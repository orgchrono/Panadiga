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


def test_manual_spec_extractor_regex_and_t1_guards(monkeypatch: object) -> None:
    """Verify PDF hardware token extraction and non-destructive T1 updates."""
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
        assert isinstance(rep["board_updates"], list) and len(rep["board_updates"]) == 1
        assert isinstance(rep["eeprom_updates"], list) and len(rep["eeprom_updates"]) == 1

        b_after = load_json_dict(prof / "boards" / "t0_board.json")
        assert b_after["soc_ref"] == "mn2ds0009"
        assert b_after["flash_ref"] == "spansion_s29jl064h_8mb"
        assert b_after["p7501_header_present"] is True
        assert as_dict(b_after["evidence"])["tier"] == "service_manual_only"

        e_after = load_json_dict(prof / "eeprom_chips" / "timer_eep.json")
        assert e_after["service_replacement_part"] == "RFKFM6016KT"
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def test_nor_intake_analyzer_offline_synthetic_dump() -> None:
    """Verify NOR intake analyzer locates Loader banner, EXTRHEAD, BGO F101, and EEMUL 0xF1."""
    buf = bytearray(b"\xFF" * 0x20_0000)
    buf[0x120:0x13A] = b"Loader ver0.95 [06/08/14]\x00"
    buf[0x8_0000:0x8_000C] = b"EXTRHEADMAIN"
    struct.pack_into("<H", buf, 0x10_0000, 0xF101)
    buf[0x1F_0004:0x1F_0008] = b"1284"
    buf[0x1F_0005] = 0xF1

    rep = analyze_nor_bytes(bytes(buf), model_id="DMR-TEST88", source_ref="synth_nor.bin")
    assert rep["loader_version"] == "0.95"
    assert rep["loader_date"] == "06/08/14"
    assert rep["bgo_sector_count_64k"] == 1
    assert isinstance(rep["extrhead_entries"], list) and len(rep["extrhead_entries"]) == 1
    assert rep["eemul_f1_offsets_hex"] == ["0x1F0000"]

    layout = json.loads(json.dumps(rep["draft_nor_layout"]))
    manifest = json.loads(json.dumps(rep["draft_model_manifest"]))
    assert layout["id"] == "dmr_test88_nor_layout_draft"
    assert manifest["nor_layout_ref"] == "dmr_test88_nor_layout_draft"


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
