"""Recursive workspace discovery, compiler/environment caching, and content-hash step cache facade."""

from __future__ import annotations

from scripts.build_cache.inventory import (
    BUILD_IGNORE_DIRS,
    CacheEntryStat,
    WorkspaceInventory,
    discover_pico_sdk,
    discover_workspace_inventory,
    find_repo_root,
    optimize_build_env,
)
from scripts.build_cache.steps import (
    CACHE_DIR_NAME,
    MANIFEST_FILENAME,
    clear_caches,
    compute_tree_digest,
    copy_if_changed,
    default_cache_dir,
    frontend_digest_and_outputs,
    ghidra_digest_and_outputs,
    inspect_cache_entries,
    is_step_fresh,
    pico_digest_and_outputs,
    query_compiler_cache_stats,
    record_step_cache,
)

__all__ = [
    "BUILD_IGNORE_DIRS",
    "CACHE_DIR_NAME",
    "CacheEntryStat",
    "MANIFEST_FILENAME",
    "WorkspaceInventory",
    "clear_caches",
    "compute_tree_digest",
    "copy_if_changed",
    "default_cache_dir",
    "discover_pico_sdk",
    "discover_workspace_inventory",
    "find_repo_root",
    "frontend_digest_and_outputs",
    "ghidra_digest_and_outputs",
    "inspect_cache_entries",
    "is_step_fresh",
    "optimize_build_env",
    "pico_digest_and_outputs",
    "query_compiler_cache_stats",
    "record_step_cache",
]
