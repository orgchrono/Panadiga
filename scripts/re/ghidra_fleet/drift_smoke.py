"""JVM-free catalog SHA-256 drift detection and opt-in PyGhidra TrapAudit smoke verification."""

from __future__ import annotations

import importlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, as_dict, as_list
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import PROFILES_DIR
from scripts.re.ghidra_fleet.isa_resolver import (
    DEFAULT_SMOKE_FIXTURE,
    JAVA_DIR,
    TOOLS_GHIDRA,
    build_ssot_isa_map,
    default_ghidra_home,
    discover_installed_ghidra_languages,
    find_analyze_headless,
    is_executable_target,
    resolve_isa_metadata,
)
from scripts.re.ghidra_fleet.pyghidra_runner import (
    _is_mn103_clean,
    boot_pyghidra,
    run_java_trap_audit,
)

def check_catalog_drift(
    catalog_path: Path,
    verify_path: Path,
    *,
    profiles_dir: Path = PROFILES_DIR,
) -> JsonDict:
    """Compare catalog input SHA-256 hashes against `fleet_verify.json` in < 1s without JVM."""
    catalog = as_dict(json.loads(catalog_path.read_text(encoding="utf-8")))
    verify = as_dict(json.loads(verify_path.read_text(encoding="utf-8")))
    known_exec_shas = {
        str(as_dict(p).get("sha256", ""))
        for p in as_list(verify.get("programs", []))
        if isinstance(p, dict)
    }
    known_non_exec_shas = {
        str(as_dict(n).get("sha256", ""))
        for n in as_list(verify.get("non_mn103_and_container_binaries", []))
        if isinstance(n, dict)
    }
    known_all = known_exec_shas | known_non_exec_shas
    isa_map = build_ssot_isa_map(profiles_dir)

    new_exec: list[JsonValue] = []
    new_non_exec: list[JsonValue] = []
    already_verified = 0
    for item in as_list(catalog.get("unique_binaries", [])):
        if not isinstance(item, dict):
            continue
        sha = str(item.get("sha256", ""))
        if sha in known_all:
            already_verified += 1
            continue
        isa_meta = resolve_isa_metadata(item, profiles_dir, isa_map=isa_map)
        summary: JsonDict = {
            "unique_name": item.get("unique_name"),
            "model_id": item.get("model_id"),
            "stage_kind": item.get("stage_kind"),
            "sha256": sha,
            "resolved_language_id": isa_meta.language_id,
            "isa_source": isa_meta.source,
        }
        if is_executable_target(item, profiles_dir):
            new_exec.append(summary)
        else:
            new_non_exec.append(summary)

    new_names = [re.escape(str(as_dict(x).get("unique_name", ""))) for x in new_exec if isinstance(x, dict)]
    return {
        "catalog": str(catalog_path),
        "verify_file": str(verify_path),
        "in_sync": not new_exec and not new_non_exec,
        "already_verified_count": already_verified,
        "new_executable_count": len(new_exec),
        "new_non_executable_count": len(new_non_exec),
        "new_executable_targets": new_exec,
        "new_non_executable_targets": new_non_exec,
        "recommended_filter_regex": "|".join(new_names) if new_names else None,
    }


def run_ghidra_smoke(
    ghidra_home: Path | None = None,
    *,
    fixture_path: Path = DEFAULT_SMOKE_FIXTURE,
) -> JsonDict:
    """Run real PyGhidra + `PanadigaMn103sTrapAudit.java` against synthetic MN103S Traps #1-#8 fixture."""
    resolved_home = ghidra_home or default_ghidra_home()
    if resolved_home is None:
        raise FileNotFoundError(
            "Ghidra home required for --ghidra-smoke: set PANADIGA_GHIDRA_PATCHED / GHIDRA_INSTALL_DIR or pass --ghidra-home"
        )
    if not fixture_path.is_file():
        raise FileNotFoundError(f"Synthetic MN103S smoke fixture missing at {fixture_path}")

    installed = discover_installed_ghidra_languages(resolved_home)
    if "mn10300:LE:32:default" not in installed:
        raise RuntimeError(f"Ghidra installation at {resolved_home} lacks mn10300:LE:32:default SLEIGH extension")

    work = Path(tempfile.mkdtemp(prefix="panadiga_ghidra_smoke_"))
    proj_dir = work / "projects"
    proj_dir.mkdir(parents=True, exist_ok=True)
    proj_name = "Panadiga_MN103S_Smoke"
    prog_path = "/mn103s_smoke_traps.bin"
    base_va = 0x5040_0000
    pyghidra, zero_undef, _exporter = boot_pyghidra(resolved_home)

    try:
        proj = pyghidra.open_project(str(proj_dir), proj_name, create=True)
        try:
            zero_undef.import_binary(
                pyghidra, proj, prog_path, fixture_path, base_va, "mn10300:LE:32:default", "default"
            )
            with pyghidra.program_context(proj, prog_path) as prog:
                tx_id = prog.startTransaction("smoke_seed_and_exec")
                try:
                    for blk in prog.getMemory().getBlocks():
                        if blk.isInitialized():
                            blk.setExecute(True)
                    prog.getListing().clearAll(False, None)
                    img = zero_undef.Image(prog, fixture_path)
                    opts = SimpleNamespace(model="DMR-EH55", no_switches=True, no_save=False)
                finally:
                    prog.endTransaction(tx_id, True)
                dis_cmd_cls = importlib.import_module("ghidra.app.cmd.disassemble").DisassembleCommand
                task_mon_cls = importlib.import_module("ghidra.util.task").TaskMonitor
                sym_src_type = importlib.import_module("ghidra.program.model.symbol").SourceType.USER_DEFINED

                tx2 = prog.startTransaction("smoke_entry")
                try:
                    addr = zero_undef.va_addr(prog, base_va)
                    dis_cmd_cls(addr, None, True).applyTo(prog, task_mon_cls.DUMMY)
                    prog.getFunctionManager().createFunction(
                        "smoke_entry",
                        addr,
                        prog.getAddressFactory().getAddressSet(addr, addr.add(42)),
                        sym_src_type,
                    )
                finally:
                    prog.endTransaction(tx2, True)
                rep = as_dict(zero_undef._run_all_passes(prog, img, "mn103s_smoke_traps", opts, pyghidra))
                ver = as_dict(rep.get("verify", {}))
        finally:
            proj.close()

        if not _is_mn103_clean("mn10300:LE:32:default", ver):
            raise RuntimeError(f"Ghidra smoke Zero-Undef failed: {ver}")
        if int(str(ver.get("instructions", 0))) < 10:
            raise RuntimeError(f"Ghidra smoke decoded too few instructions: {ver}")

        audit_out = run_java_trap_audit(
            out_c_dir=work,
            project_dir=proj_dir,
            ghidra_home=resolved_home,
            probes=[(proj_name, "mn103s_smoke_traps.bin")],
        )
        audit_rows = as_list(json.loads(audit_out.read_text(encoding="utf-8")))
        audit_row = as_dict(audit_rows[0]) if audit_rows else {}
        if int(str(audit_row.get("exit_code", 1))) != 0:
            raise RuntimeError(f"PanadigaMn103sTrapAudit failed in smoke: {audit_row}")
        trap_lines = [str(ln) for ln in as_list(audit_row.get("summary_lines", []))]
        trap_count = 0
        for ln in trap_lines:
            match = re.search(r"Total am33\.sinc Trap #8 SUB findings:\s*(\d+)", ln)
            if match is not None:
                trap_count = int(match.group(1))
        if trap_count < 2:
            raise RuntimeError(f"Expected >= 2 Trap #8 SUB findings in smoke fixture, got {trap_count} ({trap_lines})")

        return {
            "status": "PASS",
            "ghidra_home": str(resolved_home),
            "fixture": str(fixture_path),
            "verify": ver,
            "trap8_findings": trap_count,
            "java_audit": audit_row,
        }
    finally:
        shutil.rmtree(work, ignore_errors=True)

