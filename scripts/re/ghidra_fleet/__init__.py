"""Modularized SSOT-driven Ghidra fleet orchestrator package."""

from __future__ import annotations

from scripts.re.ghidra_fleet.drift_smoke import check_catalog_drift, run_ghidra_smoke
from scripts.re.ghidra_fleet.isa_resolver import (
    DEFAULT_SMOKE_FIXTURE,
    EXECUTABLE_KINDS,
    JAVA_DIR,
    SUBSYSTEM_TOKENS,
    TOOLS_GHIDRA,
    VALID_SHARDS,
    IsaResolution,
    build_ssot_isa_map,
    default_ghidra_home,
    default_project_dir,
    discover_installed_ghidra_languages,
    find_analyze_headless,
    is_executable_target,
    resolve_isa_metadata,
    resolve_language_for_record,
    select_targets,
)
from scripts.re.ghidra_fleet.pyghidra_runner import (
    boot_pyghidra,
    export_full_program_artifacts,
    run_java_trap_audit,
    run_shard,
)
from scripts.re.ghidra_fleet.report_export import assemble_bundles

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
    "resolve_isa_metadata",
    "resolve_language_for_record",
    "run_ghidra_smoke",
    "run_java_trap_audit",
    "run_shard",
    "select_targets",
]
