"""Static fleet verification, pseudocode bundle, and MN103S SLEIGH patch integrity checks."""

from __future__ import annotations

import json
import re
from pathlib import Path

from scripts.panadiga_cli_bridge import as_dict, as_list, req_int
from scripts.panadiga_ops import ROOT

FLEET_VERIFY_PATH = ROOT / "docs" / "audits" / "zero_undef" / "fleet_verify.json"
RAW_PSEUDOCODE_DIR = ROOT / "docs" / "pseudocode" / "raw"
SLEIGH_PATCH_PATH = ROOT / "tools" / "ghidra" / "sleigh_patches" / "am33_panadiga_zero_undef.patch"
TRAP_AUDIT_JAVA_PATH = ROOT / "tools" / "ghidra" / "java" / "PanadigaMn103sTrapAudit.java"
SMOKE_FIXTURE_PATH = ROOT / "scripts" / "tests" / "fixtures" / "mn103s_smoke_traps.bin"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


def test_fleet_verify_json_reconciliation_and_zero_undef_invariant() -> None:
    """Ensure fleet_verify.json counts reconcile cleanly (78 catalog + 1 core HDD = 79) and all MN103S pass Zero-Undef."""
    assert FLEET_VERIFY_PATH.is_file(), f"Missing {FLEET_VERIFY_PATH}"
    report = as_dict(json.loads(FLEET_VERIFY_PATH.read_text(encoding="utf-8")))

    sources_decrypted = req_int(report, "sources_decrypted")
    excluded = as_list(report.get("excluded_non_decryptable"))
    assert sources_decrypted + len(excluded) == req_int(report, "sources_inspected_total")
    assert req_int(report, "valid_firmware_sources_total") <= req_int(report, "sources_inspected_total")

    programs = [as_dict(p) for p in as_list(report.get("programs"))]
    non_exec = [as_dict(n) for n in as_list(report.get("non_mn103_and_container_binaries"))]
    assert len(programs) == req_int(report, "analyzed_programs_total")

    prog_shas = {str(p.get("sha256", "")) for p in programs}
    non_exec_shas = {str(n.get("sha256", "")) for n in non_exec}
    assert all(SHA256_RE.match(s) for s in prog_shas | non_exec_shas), "All audited entries require 64-hex sha256"
    assert len(prog_shas & non_exec_shas) == req_int(report, "overlap_count") == 0
    assert len(programs) + len(non_exec) == req_int(report, "unique_binaries_audited_total") == 79
    assert req_int(report, "catalog_unique_binaries") + req_int(report, "core_extra_binaries") == 79

    mn_count = 0
    for p in programs:
        lang = str(p.get("language_id", ""))
        if lang.startswith("mn10300:"):
            mn_count += 1
            assert p.get("gate_all_zero_undef") is True, f"MN103S gate failed on {p.get('program')}"
            assert req_int(p, "undefined_code_bytes") == 0
            assert req_int(p, "mn_todo_in_functions") == 0
            assert req_int(p, "bad_instruction_bookmarks") == 0
    assert mn_count == req_int(report, "mn103s_programs_zero_undef_count")

    total_funcs = req_int(report, "total_functions")
    total_decomp = req_int(report, "total_decompiled_functions")
    delta = req_int(report, "undecompiled_functions_delta")
    assert total_funcs - total_decomp == delta
    by_prog = as_dict(report.get("undecompiled_by_program"))
    assert sum(int(str(v)) for v in by_prog.values()) == delta


def test_fleet_pseudocode_bundles_and_sleigh_artifacts_intact() -> None:
    """Verify exported fleet_*.txt bundles, SLEIGH patch Trap #8 rules, Java auditor, and MN103S smoke fixture."""
    bundles = sorted(RAW_PSEUDOCODE_DIR.glob("fleet_*.txt"))
    assert len(bundles) >= 6, f"Expected >= 6 fleet_*.txt bundles under {RAW_PSEUDOCODE_DIR}, found {len(bundles)}"
    for bpath in bundles:
        assert bpath.stat().st_size > 1024, f"Unexpectedly small/empty bundle: {bpath}"

    patch_text = SLEIGH_PATCH_PATH.read_text(encoding="utf-8")
    assert "op8=0xfb; op8=0x9c" in patch_text
    assert "op8=0xfb; op8=0x98" in patch_text
    assert "update_sub(Rn0, sext(SIMM8));" in patch_text

    java_text = TRAP_AUDIT_JAVA_PATH.read_text(encoding="utf-8")
    assert "scanRangeForSubTraps" in java_text

    assert SMOKE_FIXTURE_PATH.is_file(), f"Missing synthetic smoke fixture {SMOKE_FIXTURE_PATH}"
    fixture_bytes = SMOKE_FIXTURE_PATH.read_bytes()
    assert len(fixture_bytes) == 64
    assert b"\xfb\x9c\x01\x20" in fixture_bytes
    assert b"\xfb\x98\x00\x04" in fixture_bytes
    assert b"\xf0\xfc" in fixture_bytes
