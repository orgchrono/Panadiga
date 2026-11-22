"""Parameterized, SSOT-driven Ghidra Headless fleet analyzer, TrapAudit runner, and bundle assembler."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import importlib
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import xml.etree.ElementTree as ET
from pathlib import Path
from types import SimpleNamespace
from typing import Any, Final

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, as_dict, as_list
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import PROFILES_DIR, emit_report, iter_profile_entities

TOOLS_GHIDRA: Final[Path] = ROOT / "tools" / "ghidra"
JAVA_DIR: Final[Path] = TOOLS_GHIDRA / "java"
DEFAULT_SMOKE_FIXTURE: Final[Path] = ROOT / "scripts" / "tests" / "fixtures" / "mn103s_smoke_traps.bin"
EXECUTABLE_KINDS: Final[frozenset[str]] = frozenset(
    {"IPL", "PROG", "DRV", "MAIN", "MAIN_LINUX_KERNEL", "VXWORKS_BAL", "NOR_FULL", "USER_ELF"}
)
VALID_SHARDS: Final[tuple[str, ...]] = ("all", "small", "main_a", "main_b", "main_c")
SUBSYSTEM_TOKENS: Final[tuple[str, ...]] = (
    "bgo",
    "nfrom",
    "ifodev",
    "eemul",
    "sdcd",
    "fat",
    "fmu",
    "digadiag",
    "extrhead",
    "rtsd",
    "davis",
    "lba48",
    "_dev_serial",
    "slogon",
    "pass",
    "atapi",
    "aiof",
    "avdd",
    "tbc",
    "pal-m",
    "devinfo",
    "swup",
    "nand",
    "linux version",
    "start_kernel",
    "fpc",
    "testmode",
    "400gb",
    "m7920",
    "mats",
    "hita",
)


def default_ghidra_home() -> Path | None:
    """Resolve Ghidra installation path from environment variables without hardcoding user paths."""
    for env_key in ("PANADIGA_GHIDRA_PATCHED", "GHIDRA_INSTALL_DIR", "GHIDRA_HOME"):
        val = os.environ.get(env_key, "").strip()
        if val:
            cand = Path(val)
            if cand.is_dir():
                return cand
    return None


def default_project_dir() -> Path:
    """Resolve default Ghidra project directory cleanly via environment or repo scratch dir."""
    env_dir = os.environ.get("PANADIGA_GHIDRA_PROJECT_DIR", "").strip()
    return Path(env_dir) if env_dir else (ROOT / ".ghidra_projects")


def find_analyze_headless(ghidra_home: Path | None) -> Path:
    """Locate `analyzeHeadless` (.bat on Windows) inside `ghidra_home/support/`."""
    resolved = ghidra_home or default_ghidra_home()
    if resolved is None:
        raise FileNotFoundError(
            "Ghidra home not configured: pass --ghidra-home or set PANADIGA_GHIDRA_PATCHED / GHIDRA_INSTALL_DIR"
        )
    exe_name = "analyzeHeadless.bat" if os.name == "nt" else "analyzeHeadless"
    candidate = resolved / "support" / exe_name
    if not candidate.is_file():
        raise FileNotFoundError(f"Ghidra headless launcher missing at {candidate}")
    return candidate


@dataclass(frozen=True)
class IsaResolution:
    """Resolved Ghidra SLEIGH language target from SSOT `profiles/soc_cores/` or ELF header."""

    language_id: str | None
    compiler_id: str
    sleigh_provider: str | None
    default_base_address: str | None
    source: str


def discover_installed_ghidra_languages(ghidra_home: Path) -> dict[str, set[str]]:
    """Parse `*.ldefs` XML files under `ghidra_home` without booting JVM (< 50 ms)."""
    installed: dict[str, set[str]] = {}
    if not ghidra_home.is_dir():
        return installed
    for ldef in sorted(ghidra_home.rglob("*.ldefs")):
        try:
            tree = ET.parse(ldef)
        except ET.ParseError:
            continue
        for lang in tree.findall(".//language"):
            lid = lang.attrib.get("id", "").strip()
            if not lid:
                continue
            compilers = {
                comp.attrib.get("id", "").strip()
                for comp in lang.findall("./compiler")
                if comp.attrib.get("id", "").strip()
            }
            bucket = installed.setdefault(lid, set())
            bucket.update(compilers or {"default"})
    return installed


def _language_from_isa_string(isa: str, endianness: str = "little") -> tuple[str, str]:
    low = isa.lower()
    if "mn103" in low:
        return "mn10300:LE:32:default", "default"
    if "sparc" in low:
        return ("sparc:LE:32:default" if endianness == "little" else "sparc:BE:32:default"), "default"
    if "arm64" in low or "aarch64" in low or "cortex-a53" in low:
        return "AARCH64:LE:64:v8A", "default"
    if "arm" in low or "cortex-a" in low:
        return "ARM:LE:32:v7", "default"
    if "mips" in low:
        return ("MIPS:BE:32:default" if endianness == "big" else "MIPS:LE:32:default"), "default"
    return "mn10300:LE:32:default", "default"


def _isa_resolution_from_core(core: JsonDict, source: str) -> IsaResolution:
    explicit_lang = core.get("ghidra_language_id")
    explicit_comp = core.get("ghidra_compiler_id")
    if isinstance(explicit_lang, str) and explicit_lang.strip():
        lang_id = explicit_lang.strip()
        comp_id = explicit_comp.strip() if isinstance(explicit_comp, str) and explicit_comp.strip() else "default"
    else:
        lang_id, comp_id = _language_from_isa_string(
            str(core.get("isa", "")), str(core.get("endianness", "little"))
        )
    provider = core.get("ghidra_sleigh_provider")
    default_base = core.get("default_base_address")
    return IsaResolution(
        language_id=lang_id,
        compiler_id=comp_id,
        sleigh_provider=str(provider) if isinstance(provider, str) else None,
        default_base_address=str(default_base) if isinstance(default_base, str) else None,
        source=source,
    )


def build_ssot_isa_map(profiles_dir: Path = PROFILES_DIR) -> dict[str, IsaResolution]:
    """Build normalized `model_id` -> `IsaResolution` map strictly from SSOT board/soc/train chains."""
    cores: dict[str, JsonDict] = {
        str(ent.get("id", "")): ent for _p, ent in iter_profile_entities(profiles_dir, "soc_cores") if ent.get("id")
    }
    socs: dict[str, JsonDict] = {
        str(ent.get("id", "")): ent for _p, ent in iter_profile_entities(profiles_dir, "socs") if ent.get("id")
    }
    boards: dict[str, JsonDict] = {
        str(ent.get("id", "")): ent for _p, ent in iter_profile_entities(profiles_dir, "boards") if ent.get("id")
    }
    trains: dict[str, JsonDict] = {
        str(ent.get("id", "")): ent for _p, ent in iter_profile_entities(profiles_dir, "firmware_trains") if ent.get("id")
    }

    mapping: dict[str, IsaResolution] = {}
    for _mpath, model in iter_profile_entities(profiles_dir, "models"):
        model_id = str(model.get("model_id", "")).strip()
        if not model_id:
            continue

        core: JsonDict | None = None
        source = "ssot"
        board = boards.get(str(model.get("board_ref") or "").strip())
        soc = socs.get(str(board.get("soc_ref") or "").strip()) if board is not None else None
        if soc is not None:
            core = cores.get(str(soc.get("core_ref") or "").strip())

        if core is None:
            train = trains.get(str(model.get("train_ref") or "").strip())
            if train is not None:
                core = cores.get(str(train.get("soc_core_ref") or "").strip())
                source = "ssot_train"

        if core is None:
            continue

        res = _isa_resolution_from_core(core, source)
        mapping[model_id.upper()] = res
        aliases = model.get("exact_aliases")
        if isinstance(aliases, list):
            for alias in aliases:
                if isinstance(alias, str) and alias.strip():
                    mapping[alias.strip().upper()] = res
    return mapping


def _language_from_elf_header(bin_path: Path) -> tuple[str, str] | None:
    if not bin_path.is_file():
        return None
    with bin_path.open("rb") as fh:
        hdr = fh.read(20)
    if len(hdr) < 20 or hdr[:4] != b"\x7fELF":
        return None
    is_le = hdr[5] == 1
    endian = "<H" if is_le else ">H"
    e_machine = struct.unpack_from(endian, hdr, 18)[0]
    if e_machine == 0x28:
        return "ARM:LE:32:v7", "default"
    if e_machine == 0xB7:
        return "AARCH64:LE:64:v8A", "default"
    if e_machine in (0x02, 0x12, 0x2B):
        return "sparc:BE:32:default", "default"
    if e_machine in (0x59, 0x5A):
        return "mn10300:LE:32:default", "default"
    if e_machine in (0x08, 0x0A):
        return ("MIPS:LE:32:default" if is_le else "MIPS:BE:32:default"), "default"
    return None


def resolve_isa_metadata(
    rec: JsonDict,
    profiles_dir: Path = PROFILES_DIR,
    *,
    isa_map: dict[str, IsaResolution] | None = None,
    language_override: str | None = None,
) -> IsaResolution:
    """Resolve Ghidra language strictly from override, SSOT `profiles/`, or ELF header (never guess raw ISAs)."""
    if language_override and language_override.strip():
        return IsaResolution(
            language_id=language_override.strip(),
            compiler_id="default",
            sleigh_provider="override",
            default_base_address=None,
            source="override",
        )

    model_id = str(rec.get("model_id", "")).strip().upper()
    resolved_map = isa_map if isa_map is not None else build_ssot_isa_map(profiles_dir)
    if model_id in resolved_map:
        return resolved_map[model_id]

    unique_file_raw = rec.get("unique_file")
    if isinstance(unique_file_raw, str):
        elf_lang = _language_from_elf_header(Path(unique_file_raw))
        if elf_lang is not None:
            return IsaResolution(
                language_id=elf_lang[0],
                compiler_id=elf_lang[1],
                sleigh_provider="elf_header",
                default_base_address=None,
                source="elf",
            )

    return IsaResolution(
        language_id=None,
        compiler_id="default",
        sleigh_provider=None,
        default_base_address=None,
        source="UNKNOWN_ISA",
    )


def resolve_language_for_record(
    rec: JsonDict,
    profiles_dir: Path = PROFILES_DIR,
    *,
    isa_map: dict[str, IsaResolution] | None = None,
    language_override: str | None = None,
) -> tuple[str | None, str]:
    """Return `(ghidra_language_id, compiler_id)` or `(None, 'default')` when ISA is unmapped."""
    meta = resolve_isa_metadata(rec, profiles_dir, isa_map=isa_map, language_override=language_override)
    return meta.language_id, meta.compiler_id


def is_executable_target(rec: JsonDict, profiles_dir: Path = PROFILES_DIR) -> bool:
    """Filter decrypted catalog records to executable binaries eligible for Ghidra import."""
    kind = str(rec.get("stage_kind", ""))
    name = str(rec.get("unique_name", ""))
    if kind not in EXECUTABLE_KINDS:
        return False
    if "DEV.BIN" in name or "DRV_J147" in name or "DRV1J315" in name:
        return False
    if kind == "NOR_FULL":
        lang_id, _comp = resolve_language_for_record(rec, profiles_dir)
        return lang_id is not None and lang_id.startswith("sparc")
    return True


def select_targets(
    catalog: JsonDict,
    *,
    shard: str = "all",
    filter_pattern: str | None = None,
    profiles_dir: Path = PROFILES_DIR,
    core_shas: set[str] | None = None,
) -> list[JsonDict]:
    """Select ordered executable targets for a given shard and optional regex filter."""
    if shard not in VALID_SHARDS:
        raise ValueError(f"Unsupported shard {shard!r}; expected one of {VALID_SHARDS}")
    excluded_shas = core_shas or set()
    candidates = [
        as_dict(r)
        for r in as_list(catalog.get("unique_binaries", []))
        if isinstance(r, dict)
        and is_executable_target(as_dict(r), profiles_dir)
        and str(r.get("sha256", "")) not in excluded_shas
    ]
    if filter_pattern:
        rx = re.compile(filter_pattern, re.IGNORECASE)
        candidates = [
            r
            for r in candidates
            if rx.search(f"{r.get('unique_name', '')} {r.get('model_id', '')} {r.get('stage_kind', '')}")
        ]

    if shard == "all":
        return candidates
    if shard == "small":
        return [r for r in candidates if str(r.get("stage_kind", "")) != "MAIN"]

    mains = [r for r in candidates if str(r.get("stage_kind", "")) == "MAIN"]
    shard_idx = {"main_a": 0, "main_b": 1, "main_c": 2}[shard]
    return mains[shard_idx::3]


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


def _write_split_bundle(base_path: Path, chunks: list[str], max_bytes: int = 4_800_000) -> list[Path]:
    base_path.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join(chunks)
    if len(text.encode("utf-8")) <= max_bytes:
        base_path.write_text(text, encoding="utf-8")
        return [base_path]
    written: list[Path] = []
    part_idx = 1
    cur: list[str] = []
    cur_sz = 0
    for block in chunks:
        bsz = len(block.encode("utf-8")) + 1
        if cur and cur_sz + bsz > max_bytes:
            out_p = base_path.with_name(f"{base_path.stem}_part{part_idx}{base_path.suffix}")
            out_p.write_text("\n".join(cur) + "\n", encoding="utf-8")
            written.append(out_p)
            part_idx += 1
            cur, cur_sz = [], 0
        cur.append(block)
        cur_sz += bsz
    if cur:
        out_p = base_path.with_name(f"{base_path.stem}_part{part_idx}{base_path.suffix}")
        out_p.write_text("\n".join(cur) + "\n", encoding="utf-8")
        written.append(out_p)
    return written


def _extract_c_blocks(c_path: Path) -> list[tuple[str, str, str]]:
    raw = c_path.read_text(encoding="utf-8", errors="replace")
    parts = raw.split("/* === FUNCTION ")
    out: list[tuple[str, str, str]] = []
    for chunk in parts[1:]:
        head_line, _, _rest = chunk.partition("\n")
        toks = head_line.split()
        fname = toks[0] if toks else "FUN_unknown"
        va = toks[2] if len(toks) >= 3 else "0x0"
        out.append((fname, va, "/* === FUNCTION " + chunk.rstrip()))
    return out


def _select_subsystem_blocks(c_path: Path, funcs_json: Path, max_funcs: int = 90) -> list[str]:
    meta: dict[str, JsonDict] = {
        str(f["entry_va"]): as_dict(f)
        for f in as_list(json.loads(funcs_json.read_text(encoding="utf-8")))
        if isinstance(f, dict) and "entry_va" in f
    }
    scored: list[tuple[tuple[int, int, int, int], int, str]] = []
    for order, (fname, va, full_blk) in enumerate(_extract_c_blocks(c_path)):
        low_blk = full_blk.lower()
        hits = sum(1 for t in SUBSYSTEM_TOKENS if t in low_blk)
        named = 0 if fname.startswith("FUN_") else 1
        m = meta.get(va, {})
        deg = len(as_list(m.get("callers", []))) + len(as_list(m.get("callees", [])))
        if hits > 0 or named or order < 5:
            scored.append(((1 if order < 3 else 0, named, hits, deg), int(va, 16), full_blk))
    top = sorted(scored, key=lambda x: x[0], reverse=True)[:max_funcs]
    return [blk for _, _, blk in sorted(top, key=lambda x: x[1])]


def _append_compact_func_index(dst: list[str], funcs_json: Path) -> None:
    idx_list = [as_dict(f) for f in as_list(json.loads(funcs_json.read_text(encoding="utf-8"))) if isinstance(f, dict)]
    dst.append(f"// COMPLETE FUNCTION INDEX ({len(idx_list)} funcs):")
    anon_pack: list[str] = []
    for f in idx_list:
        strs = [str(s) for s in as_list(f.get("strings", []))]
        name = str(f.get("name", "FUN_unknown"))
        entry_va = str(f.get("entry_va", "0x0"))
        size_bytes = int(str(f.get("size_bytes", 0)))
        if strs or not name.startswith("FUN_"):
            if anon_pack:
                dst.append("  " + " ".join(anon_pack))
                anon_pack.clear()
            suffix = (" // " + " | ".join(strs[:3])) if strs else ""
            dst.append(f"{entry_va} {size_bytes:5d}B {name}{suffix}")
        else:
            anon_pack.append(f"{entry_va[2:]}:{size_bytes}")
            if len(anon_pack) >= 12:
                dst.append(" ".join(anon_pack))
                anon_pack.clear()
    if anon_pack:
        dst.append(" ".join(anon_pack))


def assemble_bundles(
    *,
    catalog_path: Path,
    out_c_dir: Path,
    raw_out_dir: Path,
    verify_out: Path,
    profiles_dir: Path = PROFILES_DIR,
) -> JsonDict:
    """Aggregate shard verification checkpoints + `fleet_verify.json` + split plain-text bundles."""
    catalog = as_dict(json.loads(catalog_path.read_text(encoding="utf-8")))
    ckpt_dir = out_c_dir / "checkpoints"
    all_analyzed: list[JsonDict] = []
    if ckpt_dir.is_dir():
        for ckpt in sorted(ckpt_dir.glob("shard_*_verify.json")):
            for row in as_list(json.loads(ckpt.read_text(encoding="utf-8"))):
                if isinstance(row, dict):
                    all_analyzed.append(row)

    java_path = out_c_dir / "java_trap_audit.json"
    java_audit: list[JsonValue] = (
        as_list(json.loads(java_path.read_text(encoding="utf-8"))) if java_path.is_file() else []
    )
    non_exec = [
        as_dict(r)
        for r in as_list(catalog.get("unique_binaries", []))
        if isinstance(r, dict) and not is_executable_target(as_dict(r), profiles_dir)
    ]

    verify_out.parent.mkdir(parents=True, exist_ok=True)
    prog_summaries: list[JsonValue] = []
    mn_clean = 0
    total_insns = 0
    total_funcs = 0
    total_decomp = 0
    undecomp_by_prog: dict[str, JsonValue] = {}
    core_extra_programs: list[JsonValue] = []
    catalog_shas = {
        str(as_dict(b).get("sha256", ""))
        for b in as_list(catalog.get("unique_binaries", []))
        if isinstance(b, dict)
    }
    for r in all_analyzed:
        if r.get("status") in {"UNKNOWN_ISA", "MISSING_SLEIGH_LANGUAGE"}:
            continue
        ver = as_dict(r.get("verify", {}))
        exp = as_dict(r.get("export", {}))
        lang_id = str(r.get("language_id", ""))
        clean = _is_mn103_clean(lang_id, ver)
        if lang_id.startswith("mn10300") and clean:
            mn_clean += 1
        insns = int(str(ver.get("instructions", 0)))
        funcs = int(str(ver.get("functions", 0)))
        decomp = int(str(exp.get("functions_decompiled", 0)))
        total_insns += insns
        total_funcs += funcs
        total_decomp += decomp
        prog_name = str(r.get("program", ""))
        if funcs > decomp:
            undecomp_by_prog[prog_name] = funcs - decomp
        if str(r.get("sha256", "")) not in catalog_shas:
            core_extra_programs.append(prog_name)
        prog_summaries.append(
            {
                "shard": r.get("shard"),
                "program": r.get("program"),
                "model_id": r.get("model_id", "UNKNOWN"),
                "stage_kind": r.get("stage_kind", "OTHER"),
                "sha256": r.get("sha256"),
                "size_bytes": r.get("size_bytes"),
                "base_va_hex": r.get("base_va_hex"),
                "language_id": lang_id,
                "instructions": insns,
                "functions": funcs,
                "decompiled_functions": decomp,
                "undefined_code_bytes": ver.get("undefined_code_bytes", 0),
                "mn_todo_in_functions": ver.get("mn_todo_in_functions", 0),
                "bad_instruction_bookmarks": ver.get("bad_instruction_bookmarks", 0),
                "gate_all_zero_undef": clean,
            }
        )

    excluded = as_list(catalog.get("excluded_non_decryptable", []))
    sources_decrypted = int(str(catalog.get("decryptable_source_count", 0)))
    catalog_unique = int(str(catalog.get("unique_binary_count", len(catalog_shas))))
    core_extra_count = len(core_extra_programs)
    prog_shas = {str(as_dict(p).get("sha256", "")) for p in prog_summaries if isinstance(p, dict)}
    non_shas = {str(r.get("sha256", "")) for r in non_exec}
    fleet_report: JsonDict = {
        "schema_version": "1.0.0",
        "sources_decrypted": sources_decrypted,
        "sources_inspected_total": sources_decrypted + len(excluded),
        "catalog_unique_binaries": catalog_unique,
        "core_extra_binaries": core_extra_count,
        "core_extra_programs": core_extra_programs,
        "unique_binaries_audited_total": len(prog_summaries) + len(non_exec),
        "overlap_count": len(prog_shas & non_shas),
        "unique_binaries_total": catalog_unique,
        "analyzed_programs_total": len(prog_summaries),
        "mn103s_programs_zero_undef_count": mn_clean,
        "total_instructions": total_insns,
        "total_functions": total_funcs,
        "total_decompiled_functions": total_decomp,
        "undecompiled_functions_delta": total_funcs - total_decomp,
        "undecompiled_by_program": undecomp_by_prog,
        "programs": prog_summaries,
        "non_mn103_and_container_binaries": [
            {
                "unique_name": r.get("unique_name"),
                "stage_kind": r.get("stage_kind"),
                "model_id": r.get("model_id"),
                "sha256": r.get("sha256"),
                "size_bytes": r.get("size_bytes"),
                "base_va_hex": r.get("base_va_hex"),
                "source_kind": r.get("source_kind"),
                "filesystems": r.get("filesystems"),
            }
            for r in non_exec
        ],
        "excluded_non_decryptable": excluded,
        "java_trap_audit": java_audit,
    }
    verify_out.write_text(json.dumps(fleet_report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    raw_out_dir.mkdir(parents=True, exist_ok=True)
    for old_f in raw_out_dir.glob("fleet_*.txt"):
        old_f.unlink()

    main_sub_chunks = ["# Panadiga Full-Fleet Critical Subsystem Pseudocode\n"]
    main_map_chunks = ["# Panadiga Full-Fleet Complete Function Directory\n"]
    for r in sorted(all_analyzed, key=lambda item: str(item.get("unique_name", item.get("program", "")))):
        exp = as_dict(r.get("export", {}))
        cp = Path(str(exp.get("c_file", "")))
        fp = Path(str(exp.get("funcs_json", "")))
        if not (cp.is_file() and fp.is_file()):
            continue
        ver = as_dict(r.get("verify", {}))
        hdr = (
            f"\n{'=' * 80}\n### {r.get('stage_kind')} {r.get('unique_name', r.get('program'))} "
            f"| SHA256={r.get('sha256')} | BASE={r.get('base_va_hex')} "
            f"| insns={ver.get('instructions')} | funcs={ver.get('functions')}\n{'=' * 80}\n"
        )
        main_sub_chunks.append(hdr)
        main_sub_chunks.extend(_select_subsystem_blocks(cp, fp, max_funcs=24))
        main_map_chunks.append(hdr)
        _append_compact_func_index(main_map_chunks, fp)

    bundles = _write_split_bundle(raw_out_dir / "fleet_subsystems.txt", main_sub_chunks)
    bundles.extend(_write_split_bundle(raw_out_dir / "fleet_function_maps.txt", main_map_chunks))
    fleet_report["bundle_files"] = [str(p) for p in bundles]
    return fleet_report


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


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint for `ghidra_fleet_orchestrator.py`."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=None, help="Path to `fleet_decrypted_catalog.json`")
    parser.add_argument("--ghidra-home", type=Path, default=None, help="Patched Ghidra install root (optional env fallback)")
    parser.add_argument("--project-dir", type=Path, default=default_project_dir(), help="Ghidra workspace directory")
    parser.add_argument("--project-prefix", type=str, default="Panadiga_Fleet", help="Prefix for created Ghidra projects")
    parser.add_argument("--profiles-dir", type=Path, default=PROFILES_DIR, help="Path to SSOT `profiles/` directory")
    parser.add_argument("--shard", choices=list(VALID_SHARDS), default=None, help="Shard slice to import + analyze")
    parser.add_argument("--filter", type=str, default=None, help="Optional regex filter over unique_name/model/stage")
    parser.add_argument("--language-override", type=str, default=None, help="Explicit Ghidra SLEIGH language ID override")
    parser.add_argument("--out-c-dir", type=Path, default=None, help="Output folder for `.c`, `.funcs.json`, checkpoints")
    parser.add_argument("--raw-out-dir", type=Path, default=ROOT / "docs" / "pseudocode" / "raw", help="Output folder for split bundles")
    parser.add_argument("--verify-out", type=Path, default=ROOT / "docs" / "audits" / "zero_undef" / "fleet_verify.json", help="Output path for consolidated fleet_verify.json")
    parser.add_argument("--workers", type=int, default=6, help="Decompiler worker count per program")
    parser.add_argument("--no-switches", action="store_true", help="Skip switch-table recovery pass")
    parser.add_argument("--java-audit", action="store_true", help="Run `PanadigaMn103sTrapAudit.java` over shard checkpoints")
    parser.add_argument("--assemble", action="store_true", help="Assemble `fleet_verify.json` and size-split bundles")
    parser.add_argument("--check-drift", action="store_true", help="Diff `--catalog` against `--verify-out` by SHA-256 (<1s, no JVM)")
    parser.add_argument("--ghidra-smoke", action="store_true", help="Run opt-in real PyGhidra + TrapAudit smoke test on synthetic MN103S fixture")
    parser.add_argument("--json", type=Path, default=None, help="Emit JSON summary to file or `-`")
    opts = parser.parse_args(argv)

    summary: JsonDict = {}
    if opts.ghidra_smoke:
        summary["ghidra_smoke"] = run_ghidra_smoke(opts.ghidra_home)
    if opts.check_drift:
        if opts.catalog is None:
            parser.error("--check-drift requires --catalog")
        summary["drift"] = check_catalog_drift(opts.catalog, opts.verify_out, profiles_dir=opts.profiles_dir)
    if opts.shard is not None:
        if opts.catalog is None or opts.out_c_dir is None:
            parser.error("--shard requires --catalog and --out-c-dir")
        summary["catalog"] = str(opts.catalog)
        ckpt = run_shard(
            catalog_path=opts.catalog,
            out_c_dir=opts.out_c_dir,
            project_dir=opts.project_dir,
            project_prefix=opts.project_prefix,
            profiles_dir=opts.profiles_dir,
            ghidra_home=opts.ghidra_home,
            shard=opts.shard,
            filter_pattern=opts.filter,
            language_override=opts.language_override,
            no_switches=bool(opts.no_switches),
            max_workers=opts.workers,
        )
        summary["shard_checkpoint"] = str(ckpt)
    if opts.java_audit:
        if opts.out_c_dir is None:
            parser.error("--java-audit requires --out-c-dir")
        audit_path = run_java_trap_audit(
            out_c_dir=opts.out_c_dir,
            project_dir=opts.project_dir,
            ghidra_home=opts.ghidra_home,
        )
        summary["java_audit"] = str(audit_path)
    if opts.assemble:
        if opts.catalog is None or opts.out_c_dir is None:
            parser.error("--assemble requires --catalog and --out-c-dir")
        rep = assemble_bundles(
            catalog_path=opts.catalog,
            out_c_dir=opts.out_c_dir,
            raw_out_dir=opts.raw_out_dir,
            verify_out=opts.verify_out,
            profiles_dir=opts.profiles_dir,
        )
        summary["fleet_verify"] = rep

    if not summary:
        parser.print_help()
        return 1

    emit_report(summary, opts.json)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
