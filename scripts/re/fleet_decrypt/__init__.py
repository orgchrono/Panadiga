"""Modularized Panasonic container, archive, and NOR decryptor + SHA-256 deduplicator."""

from __future__ import annotations

from scripts.re.fleet_decrypt.catalog import (
    _merge_artifacts_into_dedup,
    _persist_partition_copies,
    _safe_token,
    _write_fleet_catalog,
    decrypt_fleet,
    retry_locked_containers,
)
from scripts.re.fleet_decrypt.discovery import (
    ARCHIVE_SUFFIXES,
    DEFAULT_INPUT_ROOT,
    RAW_BIN_HINTS,
    DecryptedArtifact,
    _classify_raw_bin,
    default_base_va_hex,
    discover_inputs,
    extract_fs_elfs_via_cli,
    normalize_va_hex,
    probe_bench_runtime_vas,
    repo_relative_str,
    unpack_frm_via_cli,
)
from scripts.re.fleet_decrypt.unpacker import decrypt_frm_container

__all__ = [
    "ARCHIVE_SUFFIXES",
    "DEFAULT_INPUT_ROOT",
    "DecryptedArtifact",
    "RAW_BIN_HINTS",
    "_classify_raw_bin",
    "_merge_artifacts_into_dedup",
    "_persist_partition_copies",
    "_safe_token",
    "_write_fleet_catalog",
    "decrypt_fleet",
    "decrypt_frm_container",
    "default_base_va_hex",
    "discover_inputs",
    "extract_fs_elfs_via_cli",
    "normalize_va_hex",
    "probe_bench_runtime_vas",
    "repo_relative_str",
    "retry_locked_containers",
    "unpack_frm_via_cli",
]
