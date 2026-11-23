"""Parameterized, SSOT-driven Ghidra Headless fleet analyzer, TrapAudit runner, and bundle assembler."""

from __future__ import annotations

import argparse
from pathlib import Path

from scripts.panadiga_cli_bridge import JsonDict
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import PROFILES_DIR, emit_report
from scripts.re.ghidra_fleet import (
    DEFAULT_SMOKE_FIXTURE,
    EXECUTABLE_KINDS,
    JAVA_DIR,
    SUBSYSTEM_TOKENS,
    TOOLS_GHIDRA,
    VALID_SHARDS,
    IsaResolution,
    assemble_bundles,
    boot_pyghidra,
    build_ssot_isa_map,
    check_catalog_drift,
    default_ghidra_home,
    default_project_dir,
    discover_installed_ghidra_languages,
    export_full_program_artifacts,
    find_analyze_headless,
    is_executable_target,
    resolve_isa_metadata,
    resolve_language_for_record,
    run_ghidra_smoke,
    run_java_trap_audit,
    run_shard,
    select_targets,
)

__all__ = [
    "DEFAULT_SMOKE_FIXTURE",
    "EXECUTABLE_KINDS",
    "IsaResolution",
    "JAVA_DIR",
    "SUBSYSTEM_TOKENS",
    "TOOLS_GHIDRA",
    "VALID_SHARDS",
    "assemble_bundles",
    "boot_pyghidra",
    "build_ssot_isa_map",
    "check_catalog_drift",
    "default_ghidra_home",
    "default_project_dir",
    "discover_installed_ghidra_languages",
    "export_full_program_artifacts",
    "find_analyze_headless",
    "is_executable_target",
    "main",
    "resolve_isa_metadata",
    "resolve_language_for_record",
    "run_ghidra_smoke",
    "run_java_trap_audit",
    "run_shard",
    "select_targets",
]


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
