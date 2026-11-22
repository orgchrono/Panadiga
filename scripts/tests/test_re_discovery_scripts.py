"""Unit tests for Wave B discovery/hydration scripts (`frm_train_harvester`, `manual_spec_extractor`, `nor_intake_analyzer`, Ghidra bundle exporter)."""

from __future__ import annotations

import hashlib
import importlib
import json
import py_compile
import shutil
import struct
import tempfile
from pathlib import Path

from scripts.panadiga_cli_bridge import as_dict
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import load_json_dict, save_json_dict
from scripts.re.frm_train_harvester import harvest_frm_trains
from scripts.re.manual_spec_extractor import extract_specs_from_text, harvest_manuals_into_profiles
from scripts.re.nor_intake_analyzer import analyze_nor_bytes


def test_frm_train_harvester_matches_sha_and_protects_t3() -> None:
    """Verify dry-run/apply hydration upgrades T0/T2 matches without touching T3+ trains."""
    tmp = Path(tempfile.mkdtemp(prefix="panadiga_frm_harv_"))
    try:
        frm_dir = tmp / "frm"
        prof_dir = tmp / "profiles"
        (prof_dir / "firmware_trains").mkdir(parents=True)
        frm_dir.mkdir(parents=True)

        blob = b"PANASONIC_TEST_FRM_PAYLOAD"
        frm_path = frm_dir / "PANA_DVD.FRM"
        frm_path.write_bytes(blob)
        sha = hashlib.sha256(blob).hexdigest()

        save_json_dict(
            prof_dir / "firmware_trains" / "t0_train.json",
            {
                "id": "t0_train",
                "loader_version": "0.90",
                "loader_profile_key": "ver090",
                "frm": {"toc_entry_bytes": 48, "official_frm_sha256": sha},
                "evidence": {"tier": "community_unverified", "sources": []},
            },
        )
        save_json_dict(
            prof_dir / "firmware_trains" / "t3_train.json",
            {
                "id": "t3_train",
                "loader_version": "0.61",
                "loader_profile_key": "ver061",
                "frm": {"toc_entry_bytes": 48, "official_frm_sha256": sha},
                "evidence": {"tier": "physical_dump_verified", "sources": [{"kind": "physical_dump", "reference": "existing"}]},
            },
        )

        rep = harvest_frm_trains(frm_root=frm_dir, profiles_dir=prof_dir, apply=True, unpack_missing=False)
        assert rep["matched_count"] == 1
        updated_t0 = load_json_dict(prof_dir / "firmware_trains" / "t0_train.json")
        assert as_dict(updated_t0["evidence"])["tier"] == "official_frm_verified"
        preserved_t3 = load_json_dict(prof_dir / "firmware_trains" / "t3_train.json")
        assert as_dict(preserved_t3["evidence"])["tier"] == "physical_dump_verified"
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def test_manual_spec_extractor_regex_and_t1_quarantine_guards(monkeypatch: object) -> None:
    """Verify PDF tokens emit quarantined staging proposals with page citations and NEVER mutate profiles/*.json."""
    sample = "Main PCB VEP79138 with MN2DS0009 SoC, S29JL064H70TFI00 Flash, IC7404 C3EBFC000053 (RFKFM6016KT) and P7501 TO JIGU."
    hits = extract_specs_from_text(sample)
    assert "VEP79138" in hits["pcb_codes"]
    assert "MN2DS0009" in hits["socs"]
    assert "S29JL064H70TFI00" in hits["flash_chips"]
    assert "C3EBFC000053" in hits["eeproms"]
    assert "RFKFM6016KT" in hits["eeproms"]
    assert "P7501" in hits["p7501_markers"]

    tmp = Path(tempfile.mkdtemp(prefix="panadiga_manual_ext_"))
    try:
        manuals = tmp / "manuals"
        prof = tmp / "profiles"
        manuals.mkdir()
        for sub in ("socs", "flash_chips", "boards", "eeprom_chips"):
            (prof / sub).mkdir(parents=True)
        (manuals / "dmr_test_sm.pdf").write_bytes(b"%PDF-1.4\n")

        save_json_dict(prof / "socs" / "mn2ds0009.json", {"id": "mn2ds0009", "part_number": "MN2DS0009"})
        save_json_dict(prof / "flash_chips" / "s29jl064h.json", {"id": "spansion_s29jl064h_8mb", "part_number": "S29JL064H70TFI00"})
        save_json_dict(
            prof / "boards" / "t0_board.json",
            {
                "id": "t0_board",
                "pcb_code": "VEP79138",
                "soc_ref": None,
                "flash_ref": None,
                "p7501_header_present": False,
                "evidence": {"tier": "community_unverified", "sources": []},
            },
        )
        save_json_dict(
            prof / "eeprom_chips" / "timer_eep.json",
            {
                "id": "timer_eep",
                "part_number": "C3EBFC000053",
                "service_replacement_part": None,
                "evidence": {"tier": "community_unverified", "sources": []},
            },
        )

        import scripts.re.manual_spec_extractor as extractor

        assert hasattr(monkeypatch, "setattr")
        monkeypatch.setattr(extractor, "extract_pdf_text", lambda _p, max_pages=80: sample)
        rep = harvest_manuals_into_profiles(manuals_dir=manuals, profiles_dir=prof, apply=True)
        assert rep["direct_profile_writes"] == 0
        assert isinstance(rep["board_proposals"], list) and len(rep["board_proposals"]) == 1
        assert isinstance(rep["eeprom_proposals"], list) and len(rep["eeprom_proposals"]) == 1

        # Profiles JSONs MUST remain completely untouched (quarantine-only invariant).
        b_after = load_json_dict(prof / "boards" / "t0_board.json")
        assert b_after["soc_ref"] is None
        assert b_after["flash_ref"] is None
        assert b_after["p7501_header_present"] is False
        assert as_dict(b_after["evidence"])["tier"] == "community_unverified"

        e_after = load_json_dict(prof / "eeprom_chips" / "timer_eep.json")
        assert e_after["service_replacement_part"] is None

        staged = sorted((prof / "staging" / "manual_proposals").glob("*.proposal.json"))
        assert len(staged) == 2
        board_prop = load_json_dict(staged[0])
        assert board_prop["requires_human_review"] is True
        assert board_prop["status"] == "quarantined_pending_human_review"
        assert board_prop["proposed_fields"] == {
            "soc_ref": "mn2ds0009",
            "flash_ref": "spansion_s29jl064h_8mb",
            "p7501_header_present": True,
        }
        assert len(as_dict(board_prop["page_citations"])) >= 3
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def test_nor_intake_analyzer_offline_synthetic_dump_and_fault_detectors() -> None:
    """Verify NOR intake analyzer locates valid sections and blocks byte-swapped, mirrored, and stale headers."""
    buf = bytearray(b"\xFF" * 0x20_0000)
    buf[0x120:0x13A] = b"Loader ver0.95 [06/08/14]\x00"
    buf[0x8_0000:0x8_000C] = b"EXTRHEADMAIN"
    struct.pack_into("<III", buf, 0x8_000C, 0x5700_0000, 0x1000, 0x400)
    struct.pack_into("<H", buf, 0x10_0000, 0xF101)
    # Two EEMUL generations across 8 KiB / 64 KiB regions
    buf[0x1D_2004:0x1D_2008] = b"1284"
    struct.pack_into("<I", buf, 0x1D_2008, 0x0000_0005)
    buf[0x1F_0004:0x1F_0008] = b"1284"
    buf[0x1F_0005] = 0xF1
    struct.pack_into("<I", buf, 0x1F_0008, 0x0000_0006)
    # Ghost/stale EXTRHEAD magic inside FTL with zero dst_len (must be marked invalid)
    buf[0x12_0000:0x12_000C] = b"EXTRHEADBAD!"
    struct.pack_into("<III", buf, 0x12_000C, 0x0000_0000, 0x0, 0x0)

    rep = analyze_nor_bytes(bytes(buf), model_id="DMR-TEST88", source_ref="synth_nor.bin")
    assert rep["loader_version"] == "0.95"
    assert rep["loader_date"] == "06/08/14"
    assert rep["bgo_sector_count_64k"] == 1
    assert rep["intake_blocked"] is False
    checks = as_dict(rep["integrity_checks"])
    assert checks["byte_swapped_16bit"] is False
    assert checks["mirrored_block_size"] is None
    assert checks["valid_extrhead_count"] == 1
    assert checks["stale_extrhead_count"] == 1
    assert isinstance(checks["eemul_generations"], list) and len(checks["eemul_generations"]) == 2
    assert rep["eemul_f1_offsets_hex"] == ["0x1F0000"]

    layout = json.loads(json.dumps(rep["draft_nor_layout"]))
    manifest = json.loads(json.dumps(rep["draft_model_manifest"]))
    assert layout["id"] == "dmr_test88_nor_layout_draft"
    assert manifest["nor_layout_ref"] == "dmr_test88_nor_layout_draft"

    # Fault 1: 16-bit byte-swapped dump ("EXTRHEAD" -> "XERTDAEH") blocks T3 intake claim
    swapped = bytearray(b"\xFF" * 0x20_0000)
    swapped[0x100:0x108] = b"XERTDAEH"
    swapped_rep = analyze_nor_bytes(bytes(swapped), model_id="DMR-SWAP", source_ref="swap.bin")
    assert swapped_rep["intake_blocked"] is True
    assert as_dict(swapped_rep["integrity_checks"])["byte_swapped_16bit"] is True
    assert as_dict(as_dict(swapped_rep["draft_nor_layout"])["evidence"])["tier"] == "community_unverified"

    # Fault 2: Upper-half address-pin mirrored dump blocks intake claim
    half = bytearray(b"\xAA" * 0x10_0000)
    half[0x120:0x13A] = b"Loader ver0.61 [06/03/06]\x00"
    mirrored = bytes(half) + bytes(half)
    mir_rep = analyze_nor_bytes(mirrored, model_id="DMR-MIRROR", source_ref="mirror.bin")
    assert mir_rep["intake_blocked"] is True
    assert as_dict(mir_rep["integrity_checks"])["mirrored_block_size"] == 0x10_0000


def test_ghidra_export_pseudocode_bundle_compiles_and_classifies() -> None:
    """Verify Ghidra bundle exporter compiles cleanly in CPython and classifies domains."""
    exporter_path = ROOT / "tools" / "ghidra" / "ghidra_export_pseudocode_bundle.py"
    py_compile.compile(str(exporter_path), doraise=True)

    mod = importlib.import_module("tools.ghidra.ghidra_export_pseudocode_bundle")
    assert mod.classify_bundle_category("prog061.bin", "nfrom_read", ("ifodev2",)) == "bgo_nfrom"
    assert mod.classify_bundle_category("prog061.bin", "sd_mount", ("SDCD",)) == "sd_fat"
    assert mod.classify_bundle_category("main.bin", "hdd_cmd", ("RTSD",)) == "main_hdd_tbc"
    assert mod.classify_bundle_category("boot.bin", "reset", ()) == "boot_core"

    rendered = mod.render_plain_text_bundle(
        "DMR-EH55",
        "bgo_nfrom",
        [{"name": "nfrom_init", "entry_va": "0x50423540", "strings": ["BGO"], "callers": [], "callees": [], "c_code": "void nfrom_init(void) {}"}],
    )
    assert "DMR-EH55 / bgo_nfrom" in rendered
    assert "=== FUNCTION nfrom_init @ 0x50423540 ===" in rendered

    ssot = mod.build_model_ssot_map("DMR-EH55", {"nfrom_init": "0x50423540"}, [], ["0xC000100C"])
    assert ssot["lethal_mmio"] == ["0xC000100C"]


def test_discovery_common_filesystems_model_inference_and_zip_collision(tmp_path: object) -> None:
    """Verify embedded FS detection, depth-bounded model inference, and ZIP duplicate handling."""
    import zipfile
    from pathlib import Path

    from scripts.re._discovery_common import (
        PROFILES_DIR,
        extract_frm_from_archive_or_sfx,
        infer_model_id_from_path,
        scan_embedded_filesystems,
    )

    root_tmp = Path(str(tmp_path))
    nand = bytearray(0x48_0000)
    struct.pack_into("<II", nand, 0x40_0000, 0x28CD_3D45, 0x1_0000)
    nand[0x40_0010:0x40_0020] = b"Compressed ROMFS"
    nand[0x45_0000:0x45_0008] = b"-rom1fs-"
    struct.pack_into(">I", nand, 0x45_0008, 0x1000)
    nand[0x46_0000:0x46_0004] = b"hsqs"
    struct.pack_into("<H", nand, 0x46_001C, 4)  # SquashFS v4 s_major
    nand[0x47_0000:0x47_0004] = b"hsqs"
    struct.pack_into("<H", nand, 0x47_001C, 999)  # False SquashFS version must be ignored
    from scripts.panadiga_cli_bridge import as_list as _as_list

    fs_rep = scan_embedded_filesystems(bytes(nand))
    assert fs_rep["kernel_slice_end"] == 0x40_0000
    kinds = [as_dict(item)["kind"] for item in _as_list(fs_rep["filesystems"]) if isinstance(item, dict)]
    assert kinds == ["CramFS", "RomFS", "SquashFS-LE"]

    deep_path = Path("/eh50/nested/ignored/parent/slot_a/dmr_bs850/PANA_DVD.FRM")
    assert infer_model_id_from_path(deep_path, PROFILES_DIR) == "DMR-BS850"
    assert infer_model_id_from_path(Path("firmware_and_images/dmr-ez485v/PANA__SD.FRM"), PROFILES_DIR) == "DMR-EZ485V"

    zpath = root_tmp / "bundle.zip"
    with zipfile.ZipFile(zpath, "w") as zf:
        zf.writestr("ver1/PANA_DVD.FRM", b"ONE")
        zf.writestr("ver2/PANA_DVD.FRM", b"TWO")
    extracted = extract_frm_from_archive_or_sfx(zpath, root_tmp / "out")
    assert [p.name for p, _ in extracted] == ["PANA_DVD.FRM", "PANA_DVD_1.FRM"]


def test_fleet_decryptor_dedup_kernel_slice_and_locked_container(
    tmp_path: object,
    monkeypatch: object,
) -> None:
    """Verify SHA-256 deduplication, UniPhier Linux kernel slicing, and locked PANAEUSB logging."""
    from pathlib import Path

    from scripts.panadiga_cli_bridge import PanadigaCliError
    from scripts.re import fleet_decryptor as fd

    root_tmp = Path(str(tmp_path))
    inp = root_tmp / "inputs"
    (inp / "dmr_bs850").mkdir(parents=True)
    (inp / "dmr_eh55").mkdir(parents=True)
    (inp / "ex97eb").mkdir(parents=True)

    f_bs = inp / "dmr_bs850" / "PANA_DVD.FRM"
    f_eh = inp / "dmr_eh55" / "PANA_DVD.FRM"
    f_locked = inp / "ex97eb" / "PANAEUSB.FRM"
    f_bs.write_bytes(b"FRM_BS")
    f_eh.write_bytes(b"FRM_EH")
    f_locked.write_bytes(b"FRM_LOCKED")

    shared_prog = b"\xCF\x00\x3F\xF8" + b"\x00" * 124

    def _make_fake_cramfs_main(target: Path) -> None:
        nand = bytearray(0x22_0000)
        struct.pack_into("<II", nand, 0x20_0000, 0x28CD_3D45, 0x1_0000)
        nand[0x20_0010:0x20_0020] = b"Compressed ROMFS"
        target.write_bytes(bytes(nand))

    def _fake_unpack(
        frm_path: Path,
        out_dir: Path,
        *,
        key_file: Path | None = None,
        split_main: bool = False,
    ) -> dict[str, object]:
        out_dir.mkdir(parents=True, exist_ok=True)
        if frm_path.name == "PANAEUSB.FRM":
            raise PanadigaCliError("Missing AES/Feistel key in keyring")
        prog_p = out_dir / "PROG.bin"
        prog_p.write_bytes(shared_prog)
        mods: list[dict[str, object]] = [
            {
                "name": "PROG0.00",
                "stage_kind": "PROG",
                "encoding": "RAW_BOOT",
                "dst_addr_hex": None,
                "out_path": str(prog_p),
                "partition_files": [],
            }
        ]
        if frm_path == f_bs:
            main_p = out_dir / "MAIN.bin"
            _make_fake_cramfs_main(main_p)
            mods.append(
                {
                    "name": "MAIN3600",
                    "stage_kind": "MAIN",
                    "encoding": "MainList",
                    "compression_algo": "gzip",
                    "dst_addr_hex": "0x90000000",
                    "out_path": str(main_p),
                    "partition_files": [],
                }
            )
        return {"format": "BlurayFeistel" if frm_path == f_bs else "LegacyDvd48B", "modules": mods}

    monkeypatch.setattr(fd, "unpack_frm_via_cli", _fake_unpack)  # type: ignore[attr-defined]
    monkeypatch.setattr(fd, "probe_bench_runtime_vas", lambda *_a, **_k: {})  # type: ignore[attr-defined]

    catalog = fd.decrypt_fleet(inp, root_tmp / "out")
    assert catalog["sources_scanned"] == 3
    assert catalog["decryptable_source_count"] == 2
    assert len(catalog["locked_containers"]) == 1  # type: ignore[arg-type]
    bins = catalog["unique_binaries"]
    assert isinstance(bins, list)
    stages = {str(as_dict(b)["stage_kind"]): as_dict(b) for b in bins if isinstance(b, dict)}
    assert "PROG" in stages and len(stages["PROG"]["alias_sources"]) == 2  # type: ignore[arg-type]
    assert "MAIN_LINUX_NAND" in stages
    assert "MAIN_LINUX_KERNEL" in stages
    assert Path(str(stages["MAIN_LINUX_KERNEL"]["unique_file"])).stat().st_size == 0x20_0000


def test_ghidra_fleet_orchestrator_isa_ssot_and_sharding(tmp_path: object) -> None:
    """Verify SSOT/ELF ISA resolution and shard/filter selection in ghidra_fleet_orchestrator."""
    from pathlib import Path

    from scripts.re._discovery_common import PROFILES_DIR
    from scripts.re.ghidra_fleet_orchestrator import resolve_language_for_record, select_targets

    root_tmp = Path(str(tmp_path))
    arm_elf = root_tmp / "arm_prog.elf"
    elf_arm = bytearray(24)
    elf_arm[:6] = b"\x7fELF\x01\x01"
    struct.pack_into("<HH", elf_arm, 16, 2, 0x28)
    arm_elf.write_bytes(bytes(elf_arm))

    a64_elf = root_tmp / "a64_prog.elf"
    elf_a64 = bytearray(24)
    elf_a64[:6] = b"\x7fELF\x02\x01"
    struct.pack_into("<HH", elf_a64, 16, 2, 0xB7)
    a64_elf.write_bytes(bytes(elf_a64))

    assert resolve_language_for_record({"model_id": "DMR-EH55"}, PROFILES_DIR)[0] == "mn10300:LE:32:default"
    assert resolve_language_for_record({"model_id": "DMR-BS850"}, PROFILES_DIR)[0] == "mn10300:LE:32:default"
    assert resolve_language_for_record({"model_id": "DMR-EZ485V"}, PROFILES_DIR)[0] == "sparc:BE:32:default"
    assert resolve_language_for_record({"model_id": "UNKNOWN_X", "unique_file": str(arm_elf)}, PROFILES_DIR)[0] == "ARM:LE:32:v7"
    assert resolve_language_for_record({"model_id": "UNKNOWN_Y", "unique_file": str(a64_elf)}, PROFILES_DIR)[0] == "AARCH64:LE:64:v8A"

    catalog = {
        "unique_binaries": [
            {"unique_name": "EH55_PROG_1.bin", "model_id": "DMR-EH55", "stage_kind": "PROG", "sha256": "s1"},
            {"unique_name": "EH50_DRV_J147.bin", "model_id": "DMR-EH50", "stage_kind": "DRV", "sha256": "s2"},
            {"unique_name": "EH55_NOR_FULL.bin", "model_id": "DMR-EH55", "stage_kind": "NOR_FULL", "sha256": "s3"},
            {"unique_name": "EZ485V_NOR_FULL.bin", "model_id": "DMR-EZ485V", "stage_kind": "NOR_FULL", "sha256": "s4"},
            {"unique_name": "BS850_MAIN.bin", "model_id": "DMR-BS850", "stage_kind": "MAIN", "sha256": "s5"},
        ]
    }
    small = select_targets(catalog, shard="small", profiles_dir=PROFILES_DIR)  # type: ignore[arg-type]
    assert [r["unique_name"] for r in small] == ["EH55_PROG_1.bin", "EZ485V_NOR_FULL.bin"]
    main_a = select_targets(catalog, shard="main_a", filter_pattern="BS850", profiles_dir=PROFILES_DIR)  # type: ignore[arg-type]
    assert [r["unique_name"] for r in main_a] == ["BS850_MAIN.bin"]


def test_frm_train_harvester_single_frm_unmatched_and_t3_guard(
    tmp_path: object,
    monkeypatch: object,
) -> None:
    """Verify `--frm` single-file harvesting, T3 protection, and unmatched onboarding report."""
    import hashlib
    import shutil
    from pathlib import Path

    from scripts.re import frm_train_harvester as harv

    root_tmp = Path(str(tmp_path))
    prof = root_tmp / "profiles"
    shutil.copytree(ROOT / "profiles" / "models", prof / "models")
    trains_dir = prof / "firmware_trains"
    trains_dir.mkdir(parents=True)

    protected_frm = root_tmp / "dmr_eh55" / "PANA_DVD.FRM"
    protected_frm.parent.mkdir(parents=True)
    protected_frm.write_bytes(b"EH55_T3_FRM")
    prot_sha = hashlib.sha256(b"EH55_T3_FRM").hexdigest()
    (trains_dir / "eh55_t3.json").write_text(
        json.dumps(
            {
                "id": "eh55_t3_train",
                "frm": {"official_frm_sha256": prot_sha},
                "prog_load_va": None,
                "evidence": {"tier": "physical_dump_verified", "sources": []},
            }
        ),
        encoding="utf-8",
    )

    unmatched_frm = root_tmp / "dmr_bs850" / "PANA_DVD.FRM"
    unmatched_frm.parent.mkdir(parents=True)
    unmatched_frm.write_bytes(b"BS850_NEW_FRM")

    monkeypatch.setattr(  # type: ignore[attr-defined]
        harv,
        "_extract_va_and_banner",
        lambda *_a, **_k: {"container_format": "BlurayFeistel", "extracted": True, "prog_load_va": "0x5080_0000"},
    )

    dir_rep = harv.harvest_frm_trains(frm_root=root_tmp, profiles_dir=prof, apply=True, unpack_missing=False)
    assert dir_rep["matched_count"] == 0  # T3 train protected from mutation
    assert dir_rep["unmatched_count"] == 1
    unmatched = dir_rep["unmatched_containers"]
    assert isinstance(unmatched, list) and as_dict(unmatched[0])["inferred_model_id"] == "DMR-BS850"

    single_rep = harv.harvest_frm_trains(frm_file=unmatched_frm, profiles_dir=prof, unpack_missing=False)
    assert single_rep["frm_files_scanned"] == 1
    assert single_rep["unmatched_count"] == 1
    assert "probe" in as_dict(single_rep["unmatched_containers"][0])  # type: ignore[index]


def test_ghidra_orchestrator_ldefs_preflight_unknown_isa_and_drift(tmp_path: object) -> None:
    """Verify XML `*.ldefs` discovery, strict UNKNOWN_ISA fallback, and `--check-drift` logic."""
    from pathlib import Path

    from scripts.re._discovery_common import PROFILES_DIR
    from scripts.re.ghidra_fleet_orchestrator import (
        check_catalog_drift,
        discover_installed_ghidra_languages,
        resolve_isa_metadata,
    )

    root_tmp = Path(str(tmp_path))
    fake_ghidra = root_tmp / "ghidra" / "Ghidra" / "Processors" / "MN103" / "data" / "languages"
    fake_ghidra.mkdir(parents=True)
    (fake_ghidra / "mn103.ldefs").write_text(
        '<language_definitions><language id="mn10300:LE:32:default"><compiler id="default"/></language></language_definitions>',
        encoding="utf-8",
    )
    langs = discover_installed_ghidra_languages(root_tmp / "ghidra")
    assert "mn10300:LE:32:default" in langs

    eh55_meta = resolve_isa_metadata({"model_id": "DMR-EH55"}, PROFILES_DIR)
    assert eh55_meta.language_id == "mn10300:LE:32:default"
    assert eh55_meta.source == "ssot"
    assert eh55_meta.sleigh_provider is not None

    ex80s_meta = resolve_isa_metadata({"model_id": "DMR-EX80S"}, PROFILES_DIR)
    assert ex80s_meta.language_id == "mn10300:LE:32:default"
    assert ex80s_meta.source == "ssot_train"

    ex86_meta = resolve_isa_metadata({"model_id": "DMR-EX86"}, PROFILES_DIR)
    assert ex86_meta.language_id == "mn10300:LE:32:default"
    assert ex86_meta.source == "ssot_train"

    ex97_meta = resolve_isa_metadata({"model_id": "DMR-EX97"}, PROFILES_DIR)
    assert ex97_meta.language_id is None
    assert ex97_meta.source == "UNKNOWN_ISA"

    raw_bin = root_tmp / "unknown_raw.bin"
    raw_bin.write_bytes(b"\x00" * 64)
    unk = resolve_isa_metadata({"model_id": "DMR-UNMAPPED", "unique_file": str(raw_bin)}, PROFILES_DIR)
    assert unk.language_id is None
    assert unk.source == "UNKNOWN_ISA"

    mips_elf = root_tmp / "mips.elf"
    hdr = bytearray(28)
    hdr[:6] = b"\x7fELF\x01\x01"
    struct.pack_into("<HH", hdr, 16, 2, 0x08)
    mips_elf.write_bytes(bytes(hdr))
    mips_meta = resolve_isa_metadata({"model_id": "DMR-UNMAPPED", "unique_file": str(mips_elf)}, PROFILES_DIR)
    assert mips_meta.language_id == "MIPS:LE:32:default"

    cat_p = root_tmp / "cat.json"
    ver_p = root_tmp / "ver.json"
    cat_p.write_text(
        json.dumps(
            {
                "unique_binaries": [
                    {"unique_name": "OLD.bin", "model_id": "DMR-EH55", "stage_kind": "PROG", "sha256": "aaa"},
                    {"unique_name": "NEW.bin", "model_id": "DMR-EH55", "stage_kind": "DRV", "sha256": "bbb"},
                ]
            }
        ),
        encoding="utf-8",
    )
    ver_p.write_text(
        json.dumps({"programs": [{"sha256": "aaa"}], "non_mn103_and_container_binaries": []}),
        encoding="utf-8",
    )
    drift = check_catalog_drift(cat_p, ver_p, profiles_dir=PROFILES_DIR)
    assert drift["in_sync"] is False
    assert drift["already_verified_count"] == 1
    assert drift["new_executable_count"] == 1


def test_fleet_decryptor_retry_locked_and_extract_rootfs_elfs(
    tmp_path: object,
    monkeypatch: object,
) -> None:
    """Verify `--retry-locked` incremental unlock and `--extract-rootfs-elfs` `USER_ELF` routing."""
    from pathlib import Path

    from scripts.panadiga_cli_bridge import PanadigaCliError
    from scripts.re import fleet_decryptor as fd

    root_tmp = Path(str(tmp_path))
    frm_usb = root_tmp / "dmr_ex97" / "PANAEUSB.FRM"
    frm_usb.parent.mkdir(parents=True)
    frm_usb.write_bytes(b"USB_FRM")

    unlocked_flag = False

    def _fake_unpack(frm_path: Path, out_dir: Path, **kwargs: object) -> dict[str, object]:
        out_dir.mkdir(parents=True, exist_ok=True)
        if not unlocked_flag:
            raise PanadigaCliError("Missing EX97 key")
        part_p = out_dir / "fma5.bin"
        part_p.write_bytes(b"CramFS_stub")
        main_p = out_dir / "MAIN.bin"
        nand = bytearray(0x22_0000)
        struct.pack_into("<II", nand, 0x20_0000, 0x28CD_3D45, 0x1_0000)
        nand[0x20_0010:0x20_0020] = b"Compressed ROMFS"
        main_p.write_bytes(bytes(nand))
        split_main = bool(kwargs.get("split_main", False))
        return {
            "format": "BlurayAes",
            "aes_key_name": "ex97_ukf",
            "modules": [
                {
                    "name": "MAIN3900",
                    "stage_kind": "MAIN",
                    "encoding": "MainList",
                    "out_path": str(main_p),
                    "partition_files": [str(part_p)] if split_main else [],
                }
            ],
        }

    def _fake_extract_fs(part_path: Path, out_dir: Path, *, key_file: Path | None = None) -> dict[str, object]:
        out_dir.mkdir(parents=True, exist_ok=True)
        elf_p = out_dir / "libfmupre.so"
        elf_bytes = bytearray(64)
        elf_bytes[:6] = b"\x7fELF\x01\x01"
        struct.pack_into("<HH", elf_bytes, 16, 3, 0x59)
        elf_p.write_bytes(bytes(elf_bytes))
        return {
            "partitions": [
                {
                    "partition_name": part_path.stem,
                    "report": {
                        "fs_kind": "CramFS",
                        "entries": [
                            {
                                "rel_path": "lib/libfmupre.so",
                                "written_path": str(elf_p),
                                "elf": {"arch_label": "mn10300", "entry_va_hex": "0x00001000"},
                            }
                        ],
                    },
                }
            ]
        }

    monkeypatch.setattr(fd, "unpack_frm_via_cli", _fake_unpack)  # type: ignore[attr-defined]
    monkeypatch.setattr(fd, "probe_bench_runtime_vas", lambda *_a, **_k: {})  # type: ignore[attr-defined]
    monkeypatch.setattr(fd, "extract_fs_elfs_via_cli", _fake_extract_fs)  # type: ignore[attr-defined]

    out_dir = root_tmp / "fleet_out"
    cat1 = fd.decrypt_fleet(frm_usb, out_dir)
    assert len(as_dict(cat1)["locked_containers"]) == 1  # type: ignore[arg-type]

    unlocked_flag = True
    cat2 = fd.retry_locked_containers(out_dir / "fleet_decrypted_catalog.json", out_dir, extract_rootfs_elfs=True)
    assert len(as_dict(cat2)["locked_containers"]) == 0  # type: ignore[arg-type]
    stages = {str(as_dict(b)["stage_kind"]) for b in cat2["unique_binaries"] if isinstance(b, dict)}  # type: ignore[union-attr]
    assert "USER_ELF" in stages
