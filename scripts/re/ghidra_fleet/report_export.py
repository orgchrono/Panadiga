"""Compact function indexing, subsystem curation, and split pseudocode bundle assembler."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, as_dict, as_list
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import PROFILES_DIR
from scripts.re.ghidra_fleet.isa_resolver import (
    SUBSYSTEM_TOKENS,
    build_ssot_isa_map,
    is_executable_target,
    resolve_isa_metadata,
)
from scripts.re.ghidra_fleet.pyghidra_runner import _is_mn103_clean

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

