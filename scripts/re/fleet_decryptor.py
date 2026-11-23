"""Universal Panasonic firmware container, archive, and raw NOR decryptor + deduplicator."""

from __future__ import annotations

import argparse
from pathlib import Path

from scripts.panadiga_cli_bridge import as_list
from scripts.re._discovery_common import PROFILES_DIR, emit_report
from scripts.re.fleet_decrypt import (
    ARCHIVE_SUFFIXES,
    DEFAULT_INPUT_ROOT,
    RAW_BIN_HINTS,
    DecryptedArtifact,
    _classify_raw_bin,
    _merge_artifacts_into_dedup,
    _persist_partition_copies,
    _safe_token,
    _write_fleet_catalog,
    decrypt_fleet,
    decrypt_frm_container,
    default_base_va_hex,
    discover_inputs,
    extract_fs_elfs_via_cli,
    normalize_va_hex,
    probe_bench_runtime_vas,
    repo_relative_str,
    retry_locked_containers,
    unpack_frm_via_cli,
)

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
    "main",
    "normalize_va_hex",
    "probe_bench_runtime_vas",
    "repo_relative_str",
    "retry_locked_containers",
    "unpack_frm_via_cli",
]


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint for `fleet_decryptor.py`."""
    parser = argparse.ArgumentParser(
        description="Decrypt and deduplicate Panasonic .FRM / archive / NOR inputs via `panadiga` CLI"
    )
    parser.add_argument("--input", type=Path, default=DEFAULT_INPUT_ROOT, help="Input file or directory")
    parser.add_argument("--out-dir", type=Path, required=True, help="Output staging directory for unique binaries + catalog")
    parser.add_argument("--profiles-dir", type=Path, default=PROFILES_DIR, help="Path to SSOT `profiles/` directory")
    parser.add_argument("--key-file", type=Path, default=None, help="Optional `keys.ukf` external keyring path")
    parser.add_argument("--model", type=str, default=None, help="Optional explicit SSOT model_id override")
    parser.add_argument("--split-main", action="store_true", help="Also extract MainList `fmaX` partitions into `<out-dir>/partitions/`")
    parser.add_argument("--extract-rootfs-elfs", action="store_true", help="Extract ELFs from MainList RomFS/CramFS partitions into `unique_binaries` (`USER_ELF`)")
    parser.add_argument("--retry-locked", type=Path, default=None, help="Retry only `locked_containers` from an existing `fleet_decrypted_catalog.json`")
    parser.add_argument("--json", type=Path, default=None, help="Emit catalog JSON to path or `-` for stdout")
    args = parser.parse_args(argv)

    if args.retry_locked is not None:
        rep = retry_locked_containers(
            args.retry_locked,
            args.out_dir,
            profiles_dir=args.profiles_dir,
            key_file=args.key_file,
            split_main=bool(args.split_main),
            extract_rootfs_elfs=bool(args.extract_rootfs_elfs),
        )
    else:
        rep = decrypt_fleet(
            args.input,
            args.out_dir,
            profiles_dir=args.profiles_dir,
            key_file=args.key_file,
            split_main=bool(args.split_main),
            extract_rootfs_elfs=bool(args.extract_rootfs_elfs),
            model_hint=args.model,
        )
    emit_report(rep, args.json)
    if args.json is None or str(args.json) != "-":
        print(
            f"[fleet_decryptor] scanned={rep['sources_scanned']} decryptable={rep['decryptable_source_count']} "
            f"unique={rep['unique_binary_count']} locked={len(as_list(rep['locked_containers']))}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
