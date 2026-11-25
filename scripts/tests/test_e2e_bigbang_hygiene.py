"""Opaque-box Tier 1-4 E2E suite for R1 big-bang taxonomy hygiene, R2 DigaOS SSOT safety, and M6930LB preservation."""

from __future__ import annotations

from collections.abc import Iterator
import json
from pathlib import Path
import re
import subprocess
import tomllib
from typing import Final

import pytest

from scripts.panadiga_ops import ROOT

_SELF_REL: Final[str] = "scripts/tests/test_e2e_bigbang_hygiene.py"

_LEGACY_DIR_PARTS: Final[tuple[tuple[str, ...], ...]] = (
    ("firmware", "pico" + "-p7501"),
    ("firmware", "panadiga" + "-openos"),
    ("firmware", "panadiga" + "-blackbox-fw"),
    ("firmware", "pico" + "os"),
)

_FORBIDDEN_GLOBAL_TOKENS: Final[tuple[str, ...]] = (
    "".join(("panadiga", "-blackbox-fw")),
    "".join(("PANA", "MON2")),
    "".join(("panadiga", "-openos")),
    "".join(("openos", "_ssot")),
    "".join(("OPEN", "OS>")),
    "".join(("pico", "-p7501")),
    "".join(("Pico", "OS")),
    "".join(("PICO", "OS>")),
)

_LEGACY_PROMPT_TOKEN: Final[str] = "".join(("PANA", "DIGA>"))

_SCOPED_LEGACY_PROMPT_ALLOWLIST: Final[tuple[str, ...]] = (
    "crates/panadiga-core/src/pico/",
    "crates/panadiga-core/tests/",
    "apps/desktop/src-tauri/src/pico/diagnostics/parser.rs",
    "apps/desktop/src/services/diagnosticParser/",
    "apps/desktop/src/services/bleService.test.ts",
    "apps/desktop/src-tauri/gen/android/",
    "docs/evidence/com10_",
    "docs/evidence/boottrace_",
)

_BINARY_SUFFIXES: Final[frozenset[str]] = frozenset(
    {
        ".bin",
        ".uf2",
        ".elf",
        ".pdf",
        ".png",
        ".ico",
        ".svg",
        ".zip",
        ".7z",
        ".frm",
        ".jp2",
        ".img",
        ".pyc",
        ".class",
        ".gz",
    }
)

_LETHAL_AND_IPL_RANGES: Final[tuple[tuple[int, int, str], ...]] = (
    (0xC000_100C, 0xC000_100F, "AM34 lethal WDT/bus register"),
    (0xDC0F_0000, 0xDC0F_0FFF, "AM34 SYSCON lethal aperture"),
    (0x8000_0000, 0x8003_FFFF, "Spansion SA0..SA10 IPL RO cached"),
    (0x8400_0000, 0x8403_FFFF, "Spansion SA0..SA10 IPL RO uncached"),
)


def _is_immutable_historical_artifact(rel: str) -> bool:
    if rel == _SELF_REL or rel.startswith(".jscpd-report/"):
        return True
    if rel.startswith("docs/debug_logs/") and rel.endswith(".log"):
        return True
    if rel.startswith("docs/evidence/") and rel.endswith((".log", ".json")):
        return True
    if rel.startswith("docs/audits/") and rel.endswith(".md"):
        return True
    return Path(rel).suffix.lower() in _BINARY_SUFFIXES


def _iter_active_text_files() -> Iterator[tuple[str, str]]:
    proc = subprocess.run(
        ["git", "ls-files"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=True,
    )
    for raw_rel in proc.stdout.splitlines():
        rel = raw_rel.strip().replace("\\", "/")
        if not rel or _is_immutable_historical_artifact(rel):
            continue
        path = ROOT / rel
        if not path.is_file():
            continue
        yield rel, path.read_text(encoding="utf-8", errors="ignore")


def _load_json_dict(path: Path) -> dict[str, object]:
    assert path.is_file(), f"Expected JSON file missing: {path.relative_to(ROOT)}"
    loaded: object = json.loads(path.read_text(encoding="utf-8"))
    assert isinstance(loaded, dict), f"Expected JSON object in {path.relative_to(ROOT)}"
    return {str(k): v for k, v in loaded.items()}


def _parse_addr_value(raw: object) -> int:
    if isinstance(raw, int):
        return raw
    if isinstance(raw, str):
        return int(raw.strip(), 0)
    raise AssertionError(f"Unexpected MMIO address representation: {raw!r}")


def _overlaps_forbidden_range(start: int, end: int) -> str | None:
    for lo, hi, label in _LETHAL_AND_IPL_RANGES:
        if start <= hi and end >= lo:
            return f"0x{start:08X}..0x{end:08X} overlaps {label} (0x{lo:08X}..0x{hi:08X})"
    return None


def _validate_digaos_power_profile_safety(profile: dict[str, object]) -> list[str]:
    violations: list[str] = []
    profile_id = str(profile.get("id", "<unknown>"))
    if profile.get("lethal_mmio_guard") is not True:
        violations.append(f"{profile_id}: lethal_mmio_guard must be true")
    if profile.get("ipl_ro_guard") is not True:
        violations.append(f"{profile_id}: ipl_ro_guard must be true")
    raw_writes = profile.get("allowed_mmio_writes", [])
    if not isinstance(raw_writes, list):
        violations.append(f"{profile_id}: allowed_mmio_writes must be a list")
        return violations
    for item in raw_writes:
        if isinstance(item, dict):
            start = _parse_addr_value(item.get("start", item.get("addr")))
            size = int(item.get("size", 4))
            end = start + max(size, 1) - 1
        else:
            start = _parse_addr_value(item)
            end = start + 3
        hit = _overlaps_forbidden_range(start, end)
        if hit is not None:
            violations.append(f"{profile_id}: {hit}")
    return violations


def test_canonical_firmware_topology_and_clean_room_notice() -> None:
    """R1/F4/F5/S1: Canonical firmware/picofw & firmware/digaos exist; legacy firmware trees are retired."""
    picofw_dir = ROOT / "firmware" / "picofw"
    digaos_dir = ROOT / "firmware" / "digaos"
    assert picofw_dir.is_dir(), "Missing canonical firmware/picofw/ directory"
    assert digaos_dir.is_dir(), "Missing canonical firmware/digaos/ directory"

    for parts in _LEGACY_DIR_PARTS:
        retired_path = ROOT.joinpath(*parts)
        assert not retired_path.exists(), f"Retired/forbidden firmware path still exists: {retired_path}"

    assert (picofw_dir / "generated" / "picofw_ssot.h").is_file()
    assert (picofw_dir / "generated" / "picofw_ssot_tables.c").is_file()
    assert (digaos_dir / "generated" / "digaos_ssot.h").is_file()
    assert (digaos_dir / "generated" / "digaos_ssot_tables.c").is_file()

    digaos_readme = digaos_dir / "README.md"
    assert digaos_readme.is_file() and "NOT LEGALLY CLEAN-ROOM" in digaos_readme.read_text(encoding="utf-8")

    code_files = [
        p
        for p in sorted(digaos_dir.rglob("*"))
        if p.is_file()
        and p.suffix in {".c", ".S", ".ld"}
        and "/build" not in p.as_posix()
        and "/generated/" not in p.as_posix()
    ]
    assert code_files, "Expected C11/asm/ld source files under firmware/digaos/"
    missing_notice = [
        p.relative_to(ROOT).as_posix()
        for p in code_files
        if "NOT LEGALLY CLEAN-ROOM" not in p.read_text(encoding="utf-8", errors="ignore")
    ]
    assert missing_notice == [], f"DigaOS files missing NOT LEGALLY CLEAN-ROOM notice: {missing_notice}"

    digaos_corpus = "\n".join(p.read_text(encoding="utf-8", errors="ignore") for p in code_files)
    for absorbed_sym in ("DIGAOS>", "DIGAOS_TV_SECAM", "probe_tbc_aiof_baseline", "SW_GUARDS"):
        assert absorbed_sym in digaos_corpus, f"Expected absorbed DigaOS symbol {absorbed_sym!r}"


def test_zero_stale_legacy_identifiers_and_scoped_fallback() -> None:
    """R1/F1/F7/F12/F13/S1: Zero stale tokens in active tracked text; PANADIGA> restricted to fallback parsers."""
    global_hits: list[str] = []
    prompt_hits: list[str] = []

    for rel, text in _iter_active_text_files():
        for token in _FORBIDDEN_GLOBAL_TOKENS:
            if token in text:
                global_hits.append(f"{rel}: contains forbidden token {token!r}")
        if _LEGACY_PROMPT_TOKEN in text and not any(rel.startswith(pfx) for pfx in _SCOPED_LEGACY_PROMPT_ALLOWLIST):
            prompt_hits.append(rel)

    assert global_hits == [], f"Stale legacy identifiers found:\n" + "\n".join(global_hits[:30])
    assert prompt_hits == [], f"Legacy prompt outside fallback allowlist:\n" + "\n".join(prompt_hits[:30])


def test_physical_p7501_and_stock_protocols_preserved() -> None:
    """R1/F1/F4/F7: Physical P7501 board tokens, P4_BLACKBOX, and stock Panasonic protocols stay intact."""
    expected_present: tuple[tuple[Path, str], ...] = (
        (ROOT / "crates" / "panadiga-core" / "src" / "ssot" / "spec" / "dto.rs", "P7501PinMappingDto"),
        (ROOT / "crates" / "panadiga-mcp" / "src" / "resources.rs", "panadiga://pinout-p7501-uart"),
        (ROOT / "crates" / "panadiga-profiles" / "src" / "validate.rs", "p7501_nsw5v_hiz_only"),
        (ROOT / "apps" / "desktop" / "src" / "components" / "pico" / "P7501PinoutCard.tsx", "P7501 [B6B-ZR-3R4]"),
        (ROOT / "firmware" / "picofw" / "flash" / "picofw_blackbox_codec.h", "0x50424258"),
        (ROOT / "crates" / "panadiga-core" / "src" / "pico" / "panamon1" / "mod.rs", "PANAMON1>"),
        (ROOT / "crates" / "panadiga-core" / "src" / "pico" / "ndb" / "mod.rs", "ndb>"),
    )
    for file_path, marker in expected_present:
        assert file_path.is_file(), f"Missing expected contract file: {file_path.relative_to(ROOT)}"
        assert marker in file_path.read_text(encoding="utf-8"), (
            f"Missing preserved marker {marker!r} in {file_path.relative_to(ROOT)}"
        )

    for board_token in ("VEP79138", "SW_UART", "XRESET", "NSW5V"):
        assert board_token in (ROOT / "AGENTS.md").read_text(encoding="utf-8")


def test_tool_configs_jscpd_gitleaks_gitignore_migrated() -> None:
    """R1/R4/F12/S1: .jscpd.json, .gitleaks.toml, and .gitignore reflect picofw/digaos & dual @PICO/@P7501 frames."""
    jscpd = _load_json_dict(ROOT / ".jscpd.json")
    assert jscpd.get("threshold") == 0
    paths = jscpd.get("path")
    assert isinstance(paths, list)
    assert "firmware/picofw" in paths
    assert "firmware/digaos" in paths
    legacy_pico_dir = "".join(("firmware/", "pico", "-p7501"))
    assert legacy_pico_dir not in paths

    gitleaks_raw: object = tomllib.loads((ROOT / ".gitleaks.toml").read_text(encoding="utf-8"))
    assert isinstance(gitleaks_raw, dict)
    allowlists = gitleaks_raw.get("allowlists", [])
    assert isinstance(allowlists, list)
    compiled: list[re.Pattern[str]] = []
    for entry in allowlists:
        if isinstance(entry, dict):
            raw_pats = entry.get("regexes", [])
            if isinstance(raw_pats, list):
                compiled.extend(re.compile(str(pat)) for pat in raw_pats)
    pico_frame = "@PICO_FRAME seq=1 ts_us=100 type=CRYPTO_KEY idx=0 name=test_key key=0123456789abcdef"
    legacy_frame = "@P7501_FRAME seq=1 ts_us=100 type=CRYPTO_KEY idx=0 name=test_key key=0123456789abcdef"
    assert any(rx.match(pico_frame) for rx in compiled), ".gitleaks.toml must allowlist @PICO_FRAME CRYPTO_KEY"
    assert any(rx.match(legacy_frame) for rx in compiled), ".gitleaks.toml must keep @P7501_FRAME fallback"

    gitignore_text = (ROOT / ".gitignore").read_text(encoding="utf-8")
    assert "firmware/picofw/" in gitignore_text
    assert "firmware/digaos/" in gitignore_text
    assert legacy_pico_dir not in gitignore_text


def test_digaos_ssot_schema_safety_and_multi_soc_isolation() -> None:
    """R2/F2/F3/F6/S2/S3: Draft 2020-12 DigaOS schema, safe default config, and multi-SoC capability guards."""
    schema = _load_json_dict(ROOT / "profiles" / "schema" / "digaos_config.schema.json")
    assert "2020-12" in str(schema.get("$schema", ""))

    cfg = _load_json_dict(ROOT / "profiles" / "digaos" / "default_digaos_config.json")
    for required_section in ("menus", "themes", "power_profiles", "soc_targets"):
        section = cfg.get(required_section)
        assert isinstance(section, list) and len(section) >= 1, f"Empty/missing DigaOS section {required_section}"

    power_profiles = cfg["power_profiles"]
    assert isinstance(power_profiles, list)
    for raw_prof in power_profiles:
        assert isinstance(raw_prof, dict)
        prof = {str(k): v for k, v in raw_prof.items()}
        assert _validate_digaos_power_profile_safety(prof) == []

    soc_targets = cfg["soc_targets"]
    assert isinstance(soc_targets, list)
    by_core = {
        str(t.get("soc_core_id", "")).lower(): t
        for t in soc_targets
        if isinstance(t, dict)
    }
    assert any("am34" in k and t.get("is_primary_executable") is True and t.get("mmio_verified") is True for k, t in by_core.items()), (
        "AM34 must be marked is_primary_executable=true and mmio_verified=true"
    )
    for family_tok in ("am33", "uniphier", "sparc"):
        matches = [t for k, t in by_core.items() if family_tok in k]
        assert matches, f"Missing multi-SoC extensible target matching {family_tok!r}"
        for target in matches:
            assert target.get("mmio_verified") is False
            assert target.get("is_primary_executable") is False
            assert int(str(target.get("logic_voltage_mv", 0))) <= 3300


@pytest.mark.parametrize(
    ("patch_overrides", "expected_substr"),
    [
        ({"lethal_mmio_guard": False}, "lethal_mmio_guard"),
        ({"ipl_ro_guard": False}, "ipl_ro_guard"),
        ({"allowed_mmio_writes": [{"start": "0xC000100C", "size": 4}]}, "0xC000100C"),
        ({"allowed_mmio_writes": [{"start": "0xC000100F", "size": 1}]}, "0xC000100F"),
        ({"allowed_mmio_writes": [{"start": "0xDC0F0000", "size": 4}]}, "0xDC0F0000"),
        ({"allowed_mmio_writes": [{"start": "0xDC0F0FFC", "size": 4}]}, "0xDC0F0FFF"),
        ({"allowed_mmio_writes": [{"start": "0x80000000", "size": 4}]}, "0x80000000"),
        ({"allowed_mmio_writes": [{"start": "0x8003FFFC", "size": 4}]}, "0x8003FFFF"),
        ({"allowed_mmio_writes": [{"start": "0x84000000", "size": 4}]}, "0x84000000"),
        ({"allowed_mmio_writes": [{"start": "0x8403FFFC", "size": 4}]}, "0x8403FFFF"),
    ],
)
def test_digaos_safety_boundary_validation_rejects_lethal_and_ipl_ranges(
    patch_overrides: dict[str, object],
    expected_substr: str,
) -> None:
    """Tier 2 BVA: Safety predicate rejects lethal MMIO and IPL RO boundaries while permitting safe AIOF/AVDD."""
    safe_baseline: dict[str, object] = {
        "id": "bva_probe",
        "lethal_mmio_guard": True,
        "ipl_ro_guard": True,
        "allowed_mmio_writes": [{"start": "0x8800002C", "size": 2}],
    }
    assert _validate_digaos_power_profile_safety(safe_baseline) == []
    mutated = {**safe_baseline, **patch_overrides}
    errs = _validate_digaos_power_profile_safety(mutated)
    assert errs and any(expected_substr.upper() in e.upper() for e in errs), f"Expected {expected_substr} in {errs}"


def test_m6930lb_localization_41_artifacts_and_docs_counts_preserved() -> None:
    """R1/F7/F13/S1/S6: All M6930LB uncommitted features, COM10 captures, 44 doc pairs, 28 logs, 29 skills, 6 agents stay intact."""
    loc_rs = ROOT / "crates" / "panadiga-core" / "src" / "re" / "region_compat" / "localization.rs"
    assert loc_rs.is_file()
    loc_src = loc_rs.read_text(encoding="utf-8")
    for fn_name in ("inspect_main_localization", "patch_main_localization_in_place", "LocalizationPatchMode"):
        assert fn_name in loc_src

    assert "AVDD_NPALCTL0_PRESERVED_HI_WORD" in (
        ROOT / "crates" / "panadiga-core" / "src" / "re" / "region_compat" / "tv_standard" / "mmio.rs"
    ).read_text(encoding="utf-8")
    assert "decode_market_code_name" in (
        ROOT / "crates" / "panadiga-core" / "src" / "re" / "region_compat" / "dev_bin.rs"
    ).read_text(encoding="utf-8")

    train_json = _load_json_dict(ROOT / "profiles" / "firmware_trains" / "loader_ver061_toc48_main57.json")
    builds = train_json.get("main_builds")
    assert isinstance(builds, list)
    sites_by_slot: dict[str, set[str]] = {}
    for b in builds:
        if isinstance(b, dict):
            slot = str(b.get("slot_name", ""))
            raw_sites = b.get("region_localization_sites", [])
            if isinstance(raw_sites, list):
                sites_by_slot[slot] = {str(s.get("name", "")) for s in raw_sites if isinstance(s, dict)}
    assert "m6930lb_3060np_ic3001_bh7629_iic" in sites_by_slot.get("MAIN3060NP", set())
    expected_3410_sites = {
        "m6930lb_3410pq_market_osd_unlock",
        "m6930lb_3410pq_ic3001_bh7629_iic",
        "m6930lb_3410pq_fpc_0x7f_rcv_ignore",
        "m6930lb_3410pq_fpc_0x7f_snd_ignore",
        "m6930lb_3410pq_fpc_autoclock_skip",
    }
    assert expected_3410_sites <= sites_by_slot.get("MAIN3410PQ", set())

    for log_name in (
        "com10_live_video_osd_diag_20261124.log",
        "com10_live_volatile_patch_20261124.log",
        "com10_m6930lb_3410pq_boot1_fvu_20261124.log",
        "com10_m6930lb_3410pq_boot2_main_20261124.log",
    ):
        assert (ROOT / "docs" / "evidence" / log_name).is_file()

    assert "## 9." in (ROOT / "docs" / "en-US" / "EH55_UNBRICK_BREAKTHROUGH_BGO_CH1_DAVIS_AND_REGION.md").read_text(encoding="utf-8")
    assert "## 9." in (ROOT / "docs" / "pt-BR" / "HISTORICO_UNBRICK_EH55_BGO_CH1_DAVIS_E_REGIAO.md").read_text(encoding="utf-8")
    assert len(list((ROOT / "docs" / "en-US").glob("*.md"))) == 44
    assert len(list((ROOT / "docs" / "pt-BR").glob("*.md"))) == 44
    assert len(list((ROOT / "docs" / "debug_logs").glob("*.log"))) == 28
    assert len(list((ROOT / ".agents" / "skills").glob("*/SKILL.md"))) == 29
    assert len(list((ROOT / ".agents" / "agents").glob("*.md"))) == 6
