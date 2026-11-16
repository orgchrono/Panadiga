"""Facade re-exporting Panadiga toolchain discovery, subprocess tracking, and hashing helpers."""

from __future__ import annotations

from scripts.ops.proc import (
    ROOT,
    ProcessAbortedError,
    ProcessTracker,
    active_process_tracker,
    print_banner,
    print_status,
    resolve_executable,
    run_step,
    use_process_tracker,
)
from scripts.ops.toolchain import (
    TOOLS_PYCACHE_IGNORE,
    build_env,
    find_msvc_vcvars64,
    is_wsl,
    load_msvc_env,
    locate_panadiga_binary,
    locate_panadiga_desktop_binary,
    panadiga_binary_name,
    panadiga_desktop_binary_name,
    sha256_file,
    update_sha256_manifest,
)

__all__ = [
    "ROOT",
    "TOOLS_PYCACHE_IGNORE",
    "ProcessAbortedError",
    "ProcessTracker",
    "active_process_tracker",
    "build_env",
    "find_msvc_vcvars64",
    "is_wsl",
    "load_msvc_env",
    "locate_panadiga_binary",
    "locate_panadiga_desktop_binary",
    "panadiga_binary_name",
    "panadiga_desktop_binary_name",
    "print_banner",
    "print_status",
    "resolve_executable",
    "run_step",
    "sha256_file",
    "update_sha256_manifest",
    "use_process_tracker",
]
