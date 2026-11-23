"""PyGhidra boot, per-program artifact export, sharded batch runner, and Java TrapAudit."""

from __future__ import annotations

import importlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from types import SimpleNamespace
from typing import Any

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, as_dict, as_list
from scripts.re._discovery_common import PROFILES_DIR
from scripts.re.ghidra_fleet.isa_resolver import (
    JAVA_DIR,
    TOOLS_GHIDRA,
    build_ssot_isa_map,
    default_ghidra_home,
    discover_installed_ghidra_languages,
    find_analyze_headless,
    resolve_isa_metadata,
    select_targets,
)

def boot_pyghidra(ghidra_home: Path | None = None) -> tuple[Any, Any, Any]:
    """Boot PyGhidra + JPype while preventing `tools/ghidra/java` from shadowing `java.*`."""
    resolved = ghidra_home or default_ghidra_home()
    if resolved is not None:
        os.environ["PANADIGA_GHIDRA_PATCHED"] = str(resolved)
    os.environ.setdefault("_JAVA_OPTIONS", "-Xmx3g")
    if str(TOOLS_GHIDRA) not in sys.path:
        sys.path.insert(0, str(TOOLS_GHIDRA))

    zero_undef = importlib.import_module("ghidra_zero_undef_pipeline")
    pyghidra = zero_undef.start_ghidra()
    if str(TOOLS_GHIDRA) not in sys.path:
        sys.path.insert(0, str(TOOLS_GHIDRA))
    exporter = importlib.import_module("ghidra_export_pseudocode_bundle")
    return pyghidra, zero_undef, exporter


def _is_mn103_clean(lang_id: str, ver: JsonDict) -> bool:
    if not lang_id.startswith("mn10300"):
        return True
    return (
        int(str(ver.get("undefined_code_bytes", -1))) == 0
        and int(str(ver.get("mn_todo_in_functions", -1))) == 0
        and int(str(ver.get("bad_instruction_bookmarks", -1))) == 0
    )


def export_full_program_artifacts(
    exporter: Any,
    prog: Any,
    out_c_dir: Path,
    out_stem: str,
    model_id: str,
    *,
    timeout_sec: int = 20,
    max_workers: int = 6,
) -> JsonDict:
    """Decompile all functions in `prog` and emit `<out_stem>.c` + `<out_stem>.funcs.json`."""
    out_c_dir.mkdir(parents=True, exist_ok=True)
    t0 = time.time()
    index = exporter.collect_function_index(prog, limit_refs=12)
    funcs = [r["_func"] for r in index]
    c_map = exporter.decompile_program_parallel(prog, funcs, timeout_sec=timeout_sec, max_workers=max_workers)

    clean_index: list[JsonValue] = []
    c_lines: list[str] = [
        f"// Panadiga 100% Ghidra Decompilation Dump: {prog.getName()} ({model_id})",
        f"// Language: {prog.getLanguageID()} | Base: 0x{(prog.getImageBase().getOffset() & 0xFFFFFFFF):08X} | Functions: {len(index)}",
        "",
    ]
    decomp_ok = 0
    for rec in index:
        c_code = str(c_map.get(rec["_va"], ""))
        if c_code:
            decomp_ok += 1
        clean_index.append(
            {
                "name": rec["name"],
                "entry_va": rec["entry_va"],
                "size_bytes": rec["size_bytes"],
                "strings": rec["strings"],
                "callers": rec["callers"],
                "callees": rec["callees"],
                "decompiled": bool(c_code),
            }
        )
        c_lines.append(f"/* === FUNCTION {rec['name']} @ {rec['entry_va']} (size={rec['size_bytes']} B) ===")
        if rec["strings"]:
            c_lines.append(" * strings: " + " | ".join(rec["strings"]))
        if rec["callers"]:
            c_lines.append(" * callers: " + ", ".join(rec["callers"]))
        if rec["callees"]:
            c_lines.append(" * callees: " + ", ".join(rec["callees"]))
        c_lines.append(" */")
        c_lines.append(c_code.rstrip() if c_code else "// <decompile empty or timed out>")
        c_lines.append("")

    c_path = out_c_dir / f"{out_stem}.c"
    idx_path = out_c_dir / f"{out_stem}.funcs.json"
    c_path.write_text("\n".join(c_lines) + "\n", encoding="utf-8")
    idx_path.write_text(json.dumps(clean_index, ensure_ascii=False) + "\n", encoding="utf-8")
    return {
        "c_file": str(c_path),
        "funcs_json": str(idx_path),
        "functions_total": len(index),
        "functions_decompiled": decomp_ok,
        "export_sec": round(time.time() - t0, 2),
    }


def run_shard(
    *,
    catalog_path: Path,
    out_c_dir: Path,
    project_dir: Path,
    project_prefix: str = "Panadiga_Fleet",
    profiles_dir: Path = PROFILES_DIR,
    ghidra_home: Path | None = None,
    shard: str = "all",
    filter_pattern: str | None = None,
    language_override: str | None = None,
    no_switches: bool = False,
    max_workers: int = 6,
) -> Path:
    """Import, preflight SLEIGH via `*.ldefs`, run MN103S Zero-Undef recovery, and export full C."""
    catalog = as_dict(json.loads(catalog_path.read_text(encoding="utf-8")))
    isa_map = build_ssot_isa_map(profiles_dir)
    targets = select_targets(catalog, shard=shard, filter_pattern=filter_pattern, profiles_dir=profiles_dir)

    resolved_home = ghidra_home or default_ghidra_home()
    installed_langs = discover_installed_ghidra_languages(resolved_home) if resolved_home is not None else {}

    ckpt_dir = out_c_dir / "checkpoints"
    ckpt_dir.mkdir(parents=True, exist_ok=True)
    ckpt_path = ckpt_dir / f"shard_{shard}_verify.json"
    existing: dict[str, JsonDict] = {}
    if ckpt_path.is_file():
        for rec in as_list(json.loads(ckpt_path.read_text(encoding="utf-8"))):
            if isinstance(rec, dict) and isinstance(rec.get("unique_name"), str):
                existing[str(rec["unique_name"])] = rec

    pyghidra, zero_undef, exporter = boot_pyghidra(resolved_home)
    proj_name = f"{project_prefix}_{shard.title().replace('_', '')}"
    project_dir.mkdir(parents=True, exist_ok=True)
    gpr_missing = not (project_dir / f"{proj_name}.gpr").exists()
    proj = pyghidra.open_project(str(project_dir), proj_name, create=gpr_missing)

    try:
        for idx, rec in enumerate(targets, 1):
            uname = str(rec["unique_name"])
            out_stem = uname[:-4] if uname.endswith(".bin") else uname
            isa_meta = resolve_isa_metadata(
                rec, profiles_dir, isa_map=isa_map, language_override=language_override
            )
            if isa_meta.language_id is None:
                existing[uname] = {
                    "shard": shard,
                    "unique_name": uname,
                    "model_id": rec.get("model_id", "UNKNOWN"),
                    "sha256": rec.get("sha256"),
                    "status": "UNKNOWN_ISA",
                }
                ckpt_path.write_text(json.dumps([existing[k] for k in sorted(existing)], indent=2) + "\n", encoding="utf-8")
                continue

            lang_id = isa_meta.language_id
            comp_id = isa_meta.compiler_id
            if installed_langs and lang_id not in installed_langs:
                existing[uname] = {
                    "shard": shard,
                    "unique_name": uname,
                    "model_id": rec.get("model_id", "UNKNOWN"),
                    "sha256": rec.get("sha256"),
                    "status": "MISSING_SLEIGH_LANGUAGE",
                    "required_language_id": lang_id,
                    "required_sleigh_provider": isa_meta.sleigh_provider,
                }
                ckpt_path.write_text(json.dumps([existing[k] for k in sorted(existing)], indent=2) + "\n", encoding="utf-8")
                continue

            prog_path = f"/{uname}"
            prev = existing.get(uname)
            if prev is not None:
                prev_exp = as_dict(prev.get("export", {}))
                prev_ver = as_dict(prev.get("verify", {}))
                c_exists = isinstance(prev_exp.get("c_file"), str) and Path(str(prev_exp["c_file"])).is_file()
                if c_exists and _is_mn103_clean(lang_id, prev_ver):
                    print(f"[{shard}] ({idx}/{len(targets)}) skip cached clean: {uname}")
                    continue

            df = proj.getProjectData().getFile(prog_path)
            if df is not None:
                df.delete()

            bin_path = Path(str(rec["unique_file"]))
            base_str = str(rec.get("base_va_hex") or isa_meta.default_base_address or "0x00000000")
            base_va = int(base_str.replace("_", ""), 16)
            model_id = str(rec.get("model_id", "UNKNOWN"))
            opts = SimpleNamespace(
                model=model_id,
                no_switches=no_switches or (str(rec.get("stage_kind", "")) == "MAIN"),
                no_save=False,
            )

            t0 = time.time()
            zero_undef.import_binary(pyghidra, proj, prog_path, bin_path, base_va, lang_id, comp_id)
            zero_undef.apply_model_memory_layout(proj, pyghidra, prog_path, base_va, model_id)
            with pyghidra.program_context(proj, prog_path) as prog:
                img = zero_undef.Image(prog, bin_path)
                rep = as_dict(zero_undef._run_all_passes(prog, img, out_stem, opts, pyghidra))
                ver = as_dict(rep["verify"])
                exp = export_full_program_artifacts(
                    exporter, prog, out_c_dir, out_stem, model_id, timeout_sec=15, max_workers=max_workers
                )
            elapsed = round(time.time() - t0, 1)
            row: JsonDict = {
                "shard": shard,
                "unique_name": uname,
                "out_stem": out_stem,
                "program": prog_path,
                "project_name": proj_name,
                "model_id": model_id,
                "stage_kind": rec.get("stage_kind"),
                "sha256": rec.get("sha256"),
                "size_bytes": rec.get("size_bytes"),
                "base_va_hex": f"0x{base_va:08X}",
                "language_id": lang_id,
                "status": "ANALYZED",
                "elapsed_sec": elapsed,
                "verify": ver,
                "export": exp,
            }
            existing[uname] = row
            sorted_rows: list[JsonValue] = [existing[k] for k in sorted(existing)]
            ckpt_path.write_text(json.dumps(sorted_rows, indent=2) + "\n", encoding="utf-8")

            print(
                f"[{shard}] ({idx}/{len(targets)}) {uname} [{lang_id}] {elapsed}s "
                f"insns={ver.get('instructions')} funcs={ver.get('functions')} "
                f"undef={ver.get('undefined_code_bytes')} todo={ver.get('mn_todo_in_functions')} "
                f"bad={ver.get('bad_instruction_bookmarks')} c_ok={exp['functions_decompiled']}/{exp['functions_total']}"
            )
            if not _is_mn103_clean(lang_id, ver):
                raise RuntimeError(f"ZERO-UNDEF GATE FAILED for {uname}: {ver}")
    finally:
        proj.close()
    return ckpt_path


def run_java_trap_audit(
    *,
    out_c_dir: Path,
    project_dir: Path,
    ghidra_home: Path | None = None,
    probes: list[tuple[str, str]] | None = None,
) -> Path:
    """Run `PanadigaMn103sTrapAudit.java` across MN103S checkpoint probes."""
    analyze_headless = find_analyze_headless(ghidra_home)
    ckpt_dir = out_c_dir / "checkpoints"
    selected: list[tuple[str, str]] = list(probes or [])
    if not selected and ckpt_dir.is_dir():
        for ckpt_file in sorted(ckpt_dir.glob("shard_*_verify.json")):
            for row in as_list(json.loads(ckpt_file.read_text(encoding="utf-8"))):
                if isinstance(row, dict) and str(row.get("language_id", "")).startswith("mn10300"):
                    proj_name = row.get("project_name")
                    uname = row.get("unique_name")
                    if isinstance(proj_name, str) and isinstance(uname, str):
                        selected.append((proj_name, uname))
                        if len(selected) >= 8:
                            break

    audit_results: list[JsonValue] = []
    env = dict(os.environ)
    env.setdefault("_JAVA_OPTIONS", "-Xmx2g")
    for proj_name, prog_name in selected:
        cmd = [
            str(analyze_headless),
            str(project_dir),
            proj_name,
            "-process",
            prog_name,
            "-noanalysis",
            "-readOnly",
            "-scriptPath",
            str(JAVA_DIR),
            "-postScript",
            "PanadigaMn103sTrapAudit.java",
            "--scope=exec",
        ]
        res = subprocess.run(cmd, capture_output=True, text=True, env=env, check=False)
        lines: list[JsonValue] = [
            ln.strip()
            for ln in (res.stdout + "\n" + res.stderr).splitlines()
            if any(tok in ln for tok in ("Trap #8", "fb 9c", "Total", "Panadiga"))
        ]
        audit_results.append(
            {
                "project_name": proj_name,
                "program": prog_name,
                "exit_code": res.returncode,
                "summary_lines": lines[-20:],
            }
        )
    out_path = out_c_dir / "java_trap_audit.json"
    out_path.write_text(json.dumps(audit_results, indent=2) + "\n", encoding="utf-8")
    return out_path

