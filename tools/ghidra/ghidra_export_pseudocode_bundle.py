"""Export plain-text + C pseudocode bundles and per-model SSOT JSON maps from cloned Ghidra projects.

Imports shared project guards and PyGhidra lifecycle helpers from ``ghidra_zero_undef_pipeline``
without modifying any stock or read-only reference projects.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
TOOLS_GHIDRA = pathlib.Path(__file__).resolve().parent
if str(TOOLS_GHIDRA) not in sys.path:
    sys.path.insert(0, str(TOOLS_GHIDRA))

import ghidra_zero_undef_pipeline as zero_undef  # noqa: E402

CATEGORIES = ("boot_core", "bgo_nfrom", "sd_fat", "main_hdd_tbc")


def classify_bundle_category(prog_name: str, func_name: str, string_hits: tuple[str, ...]) -> str:
    """Route a decompiled function into one of the four curated plain-text bundle domains."""
    joined = " ".join([prog_name.lower(), func_name.lower()] + [s.lower() for s in string_hits])
    if any(tok in joined for tok in ("nfrom", "bgo", "ifodev", "eemul", "f101", "f201")):
        return "bgo_nfrom"
    if any(tok in joined for tok in ("sdcd", "fat", "fmu", "fvu", "digadiag", "pana__sd", "extrhead")):
        return "sd_fat"
    if any(tok in joined for tok in ("main", "hdd", "rtsd", "atapi", "davis", "aiof", "avdd", "tbc", "pal-m")):
        return "main_hdd_tbc"
    return "boot_core"


def render_plain_text_bundle(model_id: str, category: str, entries: list[dict]) -> str:
    """Format decompiled functions, xrefs, and strings into a grep-friendly plain-text dossier."""
    lines = [
        "# Panadiga Ghidra Pseudocode Bundle: %s / %s" % (model_id, category),
        "# Functions exported: %d" % len(entries),
        "",
    ]
    for rec in entries:
        lines.append("=== FUNCTION %s @ %s ===" % (rec.get("name", "FUN_unknown"), rec.get("entry_va", "0x0")))
        if rec.get("strings"):
            lines.append("// strings: " + ", ".join(rec["strings"]))
        if rec.get("callers"):
            lines.append("// callers: " + ", ".join(rec["callers"]))
        if rec.get("callees"):
            lines.append("// callees: " + ", ".join(rec["callees"]))
        lines.append(rec.get("c_code", "// <no decompilation emitted>").rstrip())
        lines.append("")
    return "\n".join(lines) + "\n"


def build_model_ssot_map(
    model_id: str,
    symbols: dict[str, str],
    bus_windows: list[dict],
    lethal_mmio: list[str],
) -> dict:
    """Build structured `ghidra_ssot_maps_<slug>.json` feeding `profiles/socs/` & `firmware_trains/`."""
    return {
        "schema_version": "1.0.0",
        "model_id": model_id,
        "language_id": zero_undef.LANGUAGE_ID,
        "compiler_id": zero_undef.COMPILER_ID,
        "symbols": dict(sorted(symbols.items())),
        "bus_windows": bus_windows,
        "lethal_mmio": sorted(set(lethal_mmio)),
    }


def export_program_bundle(
    prog,
    model_id: str,
    max_functions: int = 250,
    timeout_sec: int = 20,
) -> tuple[dict[str, list[dict]], dict[str, str]]:
    """Decompile verified functions from an open Ghidra `Program` into categorized bundles."""
    from ghidra.app.decompiler import DecompInterface
    from ghidra.util.task import TaskMonitor

    buckets: dict[str, list[dict]] = {cat: [] for cat in CATEGORIES}
    symbols: dict[str, str] = {}
    iface = DecompInterface()
    iface.openProgram(prog)
    try:
        fm = prog.getFunctionManager()
        prog_name = prog.getName()
        listing = prog.getListing()
        count = 0
        for func in fm.getFunctions(True):
            if count >= max_functions:
                break
            entry_va = "0x%08X" % zero_undef.to_u32(func.getEntryPoint().getOffset())
            fname = func.getName()
            if not fname.startswith("FUN_"):
                symbols[fname] = entry_va
            str_hits = []
            for d in listing.getDefinedData(func.getBody(), True):
                if d.hasStringValue():
                    str_hits.append(str(d.getValue())[:80])
            res = iface.decompileFunction(func, timeout_sec, TaskMonitor.DUMMY)
            c_code = ""
            if res is not None and res.decompileCompleted() and res.getDecompiledFunction() is not None:
                c_code = res.getDecompiledFunction().getC()
            cat = classify_bundle_category(prog_name, fname, tuple(str_hits))
            buckets[cat].append(
                {
                    "program": prog_name,
                    "name": fname,
                    "entry_va": entry_va,
                    "strings": str_hits[:12],
                    "callers": ["0x%08X" % zero_undef.to_u32(c.getEntryPoint().getOffset()) for c in func.getCallingFunctions(TaskMonitor.DUMMY)][:12],
                    "callees": ["0x%08X" % zero_undef.to_u32(c.getEntryPoint().getOffset()) for c in func.getCalledFunctions(TaskMonitor.DUMMY)][:12],
                    "c_code": c_code,
                }
            )
            count += 1
    finally:
        iface.dispose()
    return buckets, symbols


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint exporting plain-text bundles + `ghidra_ssot_maps_<model>.json`."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", default=zero_undef.DEFAULT_MODEL, help="Target DIGA model ID")
    parser.add_argument("--project-dir", default=zero_undef.PROJECT_DIR, help="Cloned Ghidra project directory")
    parser.add_argument("--project-name", default=zero_undef.PROJECT_NAME, help="Cloned Ghidra project name")
    parser.add_argument("--program", required=True, help="Program path inside cloned project (e.g. /EH55_MAIN_3410PQ.bin)")
    parser.add_argument("--raw-out", type=pathlib.Path, default=REPO / "docs" / "pseudocode" / "raw", help="Output folder for plain-text pseudocode bundles")
    parser.add_argument("--map-out", type=pathlib.Path, default=None, help="Explicit output path for ghidra_ssot_maps_<model>.json")
    parser.add_argument("--max-functions", type=int, default=250, help="Max functions decompiled per run")
    opts = parser.parse_args(argv)

    zero_undef.guard_project(opts.project_name)
    pyghidra = zero_undef.start_ghidra()
    project = pyghidra.open_project(opts.project_dir, opts.project_name)
    slug = opts.model.lower().replace("-", "_").replace(" ", "_")

    with pyghidra.program_context(project, opts.program) as prog:
        buckets, symbols = export_program_bundle(prog, opts.model, max_functions=opts.max_functions)

    opts.raw_out.mkdir(parents=True, exist_ok=True)
    for cat in CATEGORIES:
        bundle_path = opts.raw_out / ("%s_%s.txt" % (slug, cat))
        bundle_path.write_text(render_plain_text_bundle(opts.model, cat, buckets[cat]), encoding="utf-8")

    mmio_windows = [
        {"name": n, "start": "0x%08X" % lo, "end": "0x%08X" % hi}
        for n, lo, hi in zero_undef._BUILTIN_MMIO_WINDOWS
    ]
    ssot_map = build_model_ssot_map(opts.model, symbols, mmio_windows, ["0xC000100C"])
    map_path = opts.map_out or (TOOLS_GHIDRA / ("ghidra_ssot_maps_%s.json" % slug))
    map_path.parent.mkdir(parents=True, exist_ok=True)
    map_path.write_text(json.dumps(ssot_map, indent=2) + "\n", encoding="utf-8")
    print("[ghidra_export_pseudocode_bundle] wrote %s and %d bundle files" % (map_path, len(CATEGORIES)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
