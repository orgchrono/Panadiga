"""Universal Panasonic firmware container, archive, and raw NOR decryptor + deduplicator."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import re
import shutil
import tempfile
from pathlib import Path
from typing import Final

from scripts.panadiga_cli_bridge import (
    JsonDict,
    JsonValue,
    PanadigaCliError,
    as_dict,
    as_list,
    run_panadiga_json,
)
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import (
    PROFILES_DIR,
    build_model_lookup_from_profiles,
    detect_loader_banner,
    emit_report,
    extract_frm_from_archive_or_sfx,
    infer_model_id_from_path,
    is_panasonic_frm_path,
    scan_embedded_filesystems,
    sha256_bytes,
    sha256_file,
    shannon_entropy,
    slugify_identifier,
)

DEFAULT_INPUT_ROOT: Final[Path] = ROOT / "firmware_and_images"
ARCHIVE_SUFFIXES: Final[frozenset[str]] = frozenset({".zip", ".7z", ".rar", ".exe"})
RAW_BIN_HINTS: Final[tuple[str, ...]] = ("ipl", "nor", "flash", "ic5001", "ic8651", "dev.bin")


@dataclass(frozen=True)
class DecryptedArtifact:
    """In-memory plaintext payload + JSON metadata ready for SHA-256 deduplication."""

    raw_bytes: bytes
    sha256: str
    model_id: str
    source_ref: str
    module_name: str
    partition_files: tuple[str, ...]
    metadata: JsonDict


def repo_relative_str(path: Path) -> str:
    """Return POSIX path relative to repository root when inside `ROOT`, else full POSIX path."""
    try:
        return path.resolve().relative_to(ROOT.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def normalize_va_hex(dst_va: JsonValue) -> str | None:
    """Normalize a hex virtual address string to uppercase `0xXXXXXXXX`."""
    if not isinstance(dst_va, str):
        return None
    text = dst_va.strip()
    if not text.lower().startswith("0x"):
        return None
    try:
        return f"0x{int(text.replace('_', ''), 16):08X}"
    except ValueError:
        return None


def default_base_va_hex(stage_kind: str, fmt: str = "") -> str:
    """Deterministic fallback load VA when a raw module lacks an explicit EXTRHEAD destination."""
    kind = stage_kind.upper()
    is_bluray = "bluray" in fmt.lower()
    if kind == "IPL":
        return "0x80000000"
    if kind == "DRV":
        return "0x40000000"
    if kind == "PROG":
        return "0x50800000" if is_bluray else "0x50400000"
    if kind in {"MAIN", "MAIN_LINUX_NAND", "MAIN_LINUX_KERNEL"}:
        return "0x90000000" if (is_bluray or "LINUX" in kind) else "0x50000000"
    if kind == "STB_PAYLOAD":
        return "0x80000400"
    return "0x00000000"


def unpack_frm_via_cli(
    frm_path: Path,
    out_dir: Path,
    *,
    key_file: Path | None = None,
    split_main: bool = False,
) -> JsonDict:
    """Invoke `panadiga frm unpack-all` across all Legacy/SD/Blu-ray/AES `.FRM` formats."""
    args = ["frm", "unpack-all", str(frm_path), "--out-dir", str(out_dir)]
    if key_file is not None:
        args.extend(["--key-file", str(key_file)])
    if split_main:
        args.append("--split-main")
    args.extend(["--json", "-"])
    return as_dict(run_panadiga_json(args))


def extract_fs_elfs_via_cli(
    input_path: Path,
    out_dir: Path,
    *,
    key_file: Path | None = None,
) -> JsonDict:
    """Invoke `panadiga frm extract-fs --elf-only` on a RomFS/CramFS partition or `.FRM` container."""
    args = ["frm", "extract-fs", str(input_path), "--out-dir", str(out_dir), "--elf-only"]
    if key_file is not None:
        args.extend(["--key-file", str(key_file)])
    args.extend(["--json", "-"])
    return as_dict(run_panadiga_json(args))


def probe_bench_runtime_vas(
    frm_path: Path,
    out_dir: Path,
    *,
    model_id: str | None = None,
    key_file: Path | None = None,
) -> dict[str, str]:
    """Query `panadiga re bench-runtime` for MN103S/Blu-ray stage load VAs keyed by SHA-256."""
    args = ["re", "bench-runtime", "--input", str(frm_path), "--out-dir", str(out_dir)]
    if model_id and model_id != "UNKNOWN":
        args.extend(["--model", model_id])
    if key_file is not None:
        args.extend(["--key-file", str(key_file)])
    args.extend(["--json", "-"])
    try:
        plan = as_dict(run_panadiga_json(args))
    except PanadigaCliError:
        return {}
    va_by_sha: dict[str, str] = {}
    for st in as_list(plan.get("stages", [])):
        if not isinstance(st, dict):
            continue
        sha = st.get("sha256")
        va = normalize_va_hex(st.get("base_va_hex"))
        if isinstance(sha, str) and va is not None:
            va_by_sha[sha] = va
    return va_by_sha


def _classify_raw_bin(bin_path: Path, display_ref: str, model_id: str) -> DecryptedArtifact | None:
    low = bin_path.name.lower()
    if not any(tok in low for tok in RAW_BIN_HINTS):
        return None
    raw = bin_path.read_bytes()
    sha = sha256_bytes(raw)
    stage_kind = "IPL" if "ipl" in low else ("DEV_DATA" if low == "dev.bin" else "NOR_FULL")
    _banner, ver, dt, _off = detect_loader_banner(raw, 0x4_0000)
    empty_parts: list[JsonValue] = []
    meta: JsonDict = {
        "sha256": sha,
        "size_bytes": len(raw),
        "model_id": model_id,
        "stage_kind": stage_kind,
        "base_va_hex": default_base_va_hex(stage_kind),
        "source_kind": "RAW_BINARY",
        "encoding": "RAW",
        "compression_algo": None,
        "build_name": None,
        "loader_version": ver,
        "loader_date": dt,
        "entropy": round(shannon_entropy(raw), 4),
        "toc_sum_valid": None,
        "data_sum_valid": None,
        "aes_key_name": None,
        "partitions": empty_parts,
        "filesystems": None,
    }
    return DecryptedArtifact(
        raw_bytes=raw,
        sha256=sha,
        model_id=model_id,
        source_ref=display_ref,
        module_name=bin_path.stem,
        partition_files=(),
        metadata=meta,
    )


def discover_inputs(input_path: Path, archive_work_dir: Path) -> tuple[list[tuple[Path, str]], list[tuple[Path, str]]]:
    """Collect `.FRM` containers and raw NOR/IPL `.bin` inputs from a file or directory."""
    frm_items: list[tuple[Path, str]] = []
    raw_items: list[tuple[Path, str]] = []

    def _enqueue_file(candidate: Path, ref_label: str) -> None:
        if is_panasonic_frm_path(candidate):
            frm_items.append((candidate, ref_label))
            return
        sfx = candidate.suffix.lower()
        if sfx in ARCHIVE_SUFFIXES:
            sub = archive_work_dir / slugify_identifier(candidate.stem)
            for inner_path, inner_ref in extract_frm_from_archive_or_sfx(candidate, sub):
                if is_panasonic_frm_path(inner_path):
                    frm_items.append((inner_path, inner_ref))
                elif inner_path.suffix.lower() == ".bin":
                    raw_items.append((inner_path, inner_ref))
        elif sfx == ".bin":
            raw_items.append((candidate, ref_label))

    if input_path.is_file():
        _enqueue_file(input_path, repo_relative_str(input_path))
    elif input_path.is_dir():
        for entry in sorted(input_path.rglob("*")):
            if not entry.is_file():
                continue
            if "_extracted_archives" in entry.parts:
                continue
            _enqueue_file(entry, repo_relative_str(entry))
    return frm_items, raw_items


def decrypt_frm_container(
    frm_path: Path,
    display_ref: str,
    staging_dir: Path,
    *,
    model_id: str,
    key_file: Path | None = None,
    split_main: bool = False,
    extract_rootfs_elfs: bool = False,
) -> tuple[list[DecryptedArtifact], JsonDict | None]:
    """Unpack one `.FRM` via CLI, returning extracted artifact records or a locked-container record."""
    staging_dir.mkdir(parents=True, exist_ok=True)
    need_split = split_main or extract_rootfs_elfs
    try:
        rep = unpack_frm_via_cli(
            frm_path,
            staging_dir / "unpack",
            key_file=key_file,
            split_main=need_split,
        )
    except PanadigaCliError as exc:
        return (
            [],
            {
                "path": display_ref,
                "sha256": sha256_file(frm_path),
                "size_bytes": frm_path.stat().st_size,
                "model_id": model_id,
                "error": str(exc),
            },
        )

    fmt = str(rep.get("format", "Unknown"))
    aes_key_name = rep.get("aes_key_name")
    bench_vas = probe_bench_runtime_vas(
        frm_path,
        staging_dir / "bench",
        model_id=model_id,
        key_file=key_file,
    )
    artifacts: list[DecryptedArtifact] = []

    for mod_val in as_list(rep.get("modules", [])):
        if not isinstance(mod_val, dict):
            continue
        out_path_raw = mod_val.get("out_path")
        if not isinstance(out_path_raw, str):
            continue
        out_file = Path(out_path_raw)
        if not out_file.is_file():
            continue

        payload = out_file.read_bytes()
        sha = sha256_bytes(payload)
        mod_name = str(mod_val.get("name", out_file.stem))
        raw_stage = str(mod_val.get("stage_kind", "OTHER"))
        encoding = str(mod_val.get("encoding", "RAW"))
        base_va = (
            normalize_va_hex(mod_val.get("dst_addr_hex"))
            or bench_vas.get(sha)
            or default_base_va_hex(raw_stage, fmt)
        )
        if encoding.startswith("STB"):
            raw_stage = "STB_PAYLOAD"
            base_va = "0x80000400"

        _banner, ver, dt, _off = detect_loader_banner(payload, 0x4_0000)
        part_files_raw = tuple(p for p in as_list(mod_val.get("partition_files", [])) if isinstance(p, str))
        part_names: list[JsonValue] = [Path(p).stem for p in part_files_raw]
        fs_scan: JsonDict = (
            scan_embedded_filesystems(payload)
            if encoding == "MainList"
            else {"filesystems": [], "kernel_slice_end": 0}
        )
        filesystems = as_list(fs_scan.get("filesystems", []))
        stage_kind = "MAIN_LINUX_NAND" if filesystems else raw_stage

        artifacts.append(
            DecryptedArtifact(
                raw_bytes=payload,
                sha256=sha,
                model_id=model_id,
                source_ref=display_ref,
                module_name=mod_name,
                partition_files=part_files_raw,
                metadata={
                    "sha256": sha,
                    "size_bytes": len(payload),
                    "model_id": model_id,
                    "stage_kind": stage_kind,
                    "base_va_hex": base_va,
                    "source_kind": fmt,
                    "encoding": encoding,
                    "compression_algo": mod_val.get("compression_algo"),
                    "build_name": mod_val.get("build_name"),
                    "loader_version": ver,
                    "loader_date": dt,
                    "entropy": round(shannon_entropy(payload), 4),
                    "toc_sum_valid": mod_val.get("toc_sum_valid"),
                    "data_sum_valid": mod_val.get("data_sum_valid"),
                    "aes_key_name": aes_key_name,
                    "partitions": part_names,
                    "filesystems": filesystems or None,
                },
            )
        )

        k_end = int(str(fs_scan.get("kernel_slice_end", 0)))
        if filesystems and k_end > 0x1_0000:
            kernel_bytes = payload[:k_end]
            k_sha = sha256_bytes(kernel_bytes)
            empty_parts: list[JsonValue] = []
            artifacts.append(
                DecryptedArtifact(
                    raw_bytes=kernel_bytes,
                    sha256=k_sha,
                    model_id=model_id,
                    source_ref=display_ref,
                    module_name=f"{mod_name}_kernel",
                    partition_files=(),
                    metadata={
                        "sha256": k_sha,
                        "size_bytes": len(kernel_bytes),
                        "model_id": model_id,
                        "stage_kind": "MAIN_LINUX_KERNEL",
                        "base_va_hex": "0x90000000",
                        "source_kind": "UNIPHIER_KERNEL_SLICE",
                        "encoding": "RAW_KERNEL",
                        "compression_algo": mod_val.get("compression_algo"),
                        "build_name": mod_val.get("build_name"),
                        "loader_version": None,
                        "loader_date": None,
                        "entropy": round(shannon_entropy(kernel_bytes), 4),
                        "toc_sum_valid": mod_val.get("toc_sum_valid"),
                        "data_sum_valid": mod_val.get("data_sum_valid"),
                        "aes_key_name": aes_key_name,
                        "partitions": empty_parts,
                        "filesystems": None,
                    },
                )
            )

        if extract_rootfs_elfs and part_files_raw:
            for p_raw in part_files_raw:
                part_path = Path(p_raw)
                if not part_path.is_file():
                    continue
                try:
                    fs_rep = extract_fs_elfs_via_cli(
                        part_path,
                        staging_dir / "rootfs_elfs" / part_path.stem,
                        key_file=key_file,
                    )
                except PanadigaCliError:
                    continue
                for fs_part in as_list(fs_rep.get("partitions", [])):
                    if not isinstance(fs_part, dict):
                        continue
                    part_name = str(fs_part.get("partition_name", part_path.stem))
                    sub_rep = as_dict(fs_part.get("report", {}))
                    fs_kind_label = str(sub_rep.get("fs_kind", "FS"))
                    for ent in as_list(sub_rep.get("entries", [])):
                        if not isinstance(ent, dict):
                            continue
                        wpath_raw = ent.get("written_path")
                        elf_meta = ent.get("elf")
                        if not (isinstance(wpath_raw, str) and isinstance(elf_meta, dict)):
                            continue
                        elf_file = Path(wpath_raw)
                        if not elf_file.is_file():
                            continue
                        elf_bytes = elf_file.read_bytes()
                        elf_sha = str(ent.get("sha256") or sha256_bytes(elf_bytes))
                        rel_path = str(ent.get("rel_path", elf_file.name))
                        empty_elf_parts: list[JsonValue] = []
                        artifacts.append(
                            DecryptedArtifact(
                                raw_bytes=elf_bytes,
                                sha256=elf_sha,
                                model_id=model_id,
                                source_ref=f"{display_ref}!{part_name}/{rel_path}",
                                module_name=f"{mod_name}_{part_name}_{elf_file.name}",
                                partition_files=(),
                                metadata={
                                    "sha256": elf_sha,
                                    "size_bytes": len(elf_bytes),
                                    "model_id": model_id,
                                    "stage_kind": "USER_ELF",
                                    "base_va_hex": normalize_va_hex(elf_meta.get("entry_va_hex")) or "0x00000000",
                                    "source_kind": f"UNIPHIER_{fs_kind_label.upper()}_ELF",
                                    "encoding": "ELF",
                                    "compression_algo": None,
                                    "build_name": elf_meta.get("arch_label"),
                                    "loader_version": None,
                                    "loader_date": None,
                                    "entropy": round(shannon_entropy(elf_bytes), 4),
                                    "toc_sum_valid": None,
                                    "data_sum_valid": None,
                                    "aes_key_name": aes_key_name,
                                    "partitions": empty_elf_parts,
                                    "filesystems": None,
                                },
                            )
                        )

    return artifacts, None


def _safe_token(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "_", text).strip("_") or "item"


def _persist_partition_copies(part_files: tuple[str, ...], dest_dir: Path) -> list[JsonValue]:
    if not part_files:
        return []
    dest_dir.mkdir(parents=True, exist_ok=True)
    saved: list[JsonValue] = []
    for raw_p in part_files:
        src = Path(raw_p)
        if not src.is_file():
            continue
        target = dest_dir / src.name
        if not target.exists():
            shutil.copy2(src, target)
        saved.append(str(target))
    return saved


def _merge_artifacts_into_dedup(
    all_artifacts: list[DecryptedArtifact],
    dedup: dict[str, JsonDict],
    unique_dir: Path,
    out_dir: Path,
) -> None:
    for item in all_artifacts:
        sha = item.sha256
        alias_entry: JsonDict = {
            "model_id": item.model_id,
            "source_ref": item.source_ref,
            "module_name": item.module_name,
        }
        existing = dedup.get(sha)
        if existing is not None:
            aliases = as_list(existing.get("alias_sources", []))
            if alias_entry not in aliases:
                aliases.append(alias_entry)
                existing["alias_sources"] = aliases
            continue

        safe_model = _safe_token(item.model_id)
        safe_mod = _safe_token(item.module_name)
        unique_name = f"{safe_model}_{safe_mod}_{sha[:8]}.bin"
        unique_path = unique_dir / unique_name
        unique_path.write_bytes(item.raw_bytes)

        persisted_parts = _persist_partition_copies(
            item.partition_files,
            out_dir / "partitions" / f"{safe_model}_{sha[:8]}",
        )
        entry: JsonDict = dict(item.metadata)
        entry["unique_name"] = unique_name
        entry["unique_file"] = str(unique_path)
        entry["partition_files"] = persisted_parts
        entry["alias_sources"] = [alias_entry]
        dedup[sha] = entry


def _write_fleet_catalog(
    out_dir: Path,
    *,
    input_label: str,
    sources_scanned: int,
    decryptable_sources: int,
    dedup: dict[str, JsonDict],
    locked_containers: list[JsonValue],
) -> JsonDict:
    unique_bins: list[JsonValue] = list(dedup.values())
    catalog: JsonDict = {
        "schema_version": "1.0.0",
        "input": input_label,
        "sources_scanned": sources_scanned,
        "decryptable_source_count": decryptable_sources,
        "unique_binary_count": len(unique_bins),
        "unique_binaries": unique_bins,
        "locked_containers": locked_containers,
        "excluded_non_decryptable": locked_containers,
    }
    (out_dir / "fleet_decrypted_catalog.json").write_text(
        json.dumps(catalog, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    return catalog


def decrypt_fleet(
    input_path: Path,
    out_dir: Path,
    *,
    profiles_dir: Path = PROFILES_DIR,
    key_file: Path | None = None,
    split_main: bool = False,
    extract_rootfs_elfs: bool = False,
    model_hint: str | None = None,
) -> JsonDict:
    """Decrypt containers/NOR dumps under `input_path`, deduplicate by SHA-256, and write catalog."""
    out_dir.mkdir(parents=True, exist_ok=True)
    unique_dir = out_dir / "unique"
    unique_dir.mkdir(parents=True, exist_ok=True)
    model_lookup = build_model_lookup_from_profiles(profiles_dir)

    work_root = Path(tempfile.mkdtemp(prefix="panadiga_fleet_dec_"))
    all_artifacts: list[DecryptedArtifact] = []
    locked_containers: list[JsonValue] = []
    sources_scanned = 0
    decryptable_sources = 0

    try:
        frm_items, raw_items = discover_inputs(input_path, work_root / "archives")
        for idx, (frm_path, display_ref) in enumerate(frm_items):
            sources_scanned += 1
            inferred = model_hint or infer_model_id_from_path(Path(display_ref), profiles_dir, lookup=model_lookup) or "UNKNOWN"
            arts, locked = decrypt_frm_container(
                frm_path,
                display_ref,
                work_root / f"frm_{idx:03d}",
                model_id=inferred,
                key_file=key_file,
                split_main=split_main,
                extract_rootfs_elfs=extract_rootfs_elfs,
            )
            if locked is not None:
                locked_containers.append(locked)
            else:
                decryptable_sources += 1
                all_artifacts.extend(arts)

        for raw_path, display_ref in raw_items:
            sources_scanned += 1
            inferred = model_hint or infer_model_id_from_path(Path(display_ref), profiles_dir, lookup=model_lookup) or "UNKNOWN"
            rec = _classify_raw_bin(raw_path, display_ref, inferred)
            if rec is not None:
                decryptable_sources += 1
                all_artifacts.append(rec)

        dedup: dict[str, JsonDict] = {}
        _merge_artifacts_into_dedup(all_artifacts, dedup, unique_dir, out_dir)
    finally:
        shutil.rmtree(work_root, ignore_errors=True)

    return _write_fleet_catalog(
        out_dir,
        input_label=repo_relative_str(input_path),
        sources_scanned=sources_scanned,
        decryptable_sources=decryptable_sources,
        dedup=dedup,
        locked_containers=locked_containers,
    )


def retry_locked_containers(
    catalog_path: Path,
    out_dir: Path,
    *,
    profiles_dir: Path = PROFILES_DIR,
    key_file: Path | None = None,
    split_main: bool = False,
    extract_rootfs_elfs: bool = False,
) -> JsonDict:
    """Re-attempt decryption ONLY on entries recorded in `locked_containers` (e.g. after adding a new key)."""
    out_dir.mkdir(parents=True, exist_ok=True)
    unique_dir = out_dir / "unique"
    unique_dir.mkdir(parents=True, exist_ok=True)

    prev = as_dict(json.loads(catalog_path.read_text(encoding="utf-8")))
    dedup: dict[str, JsonDict] = {}
    for entry in as_list(prev.get("unique_binaries", [])):
        if isinstance(entry, dict) and isinstance(entry.get("sha256"), str):
            dedup[str(entry["sha256"])] = dict(entry)

    work_root = Path(tempfile.mkdtemp(prefix="panadiga_retry_locked_"))
    still_locked: list[JsonValue] = []
    unlocked_artifacts: list[DecryptedArtifact] = []
    newly_unlocked = 0

    try:
        for idx, item in enumerate(as_list(prev.get("locked_containers", []))):
            if not isinstance(item, dict):
                continue
            raw_ref = str(item.get("path", "") or item.get("origin", ""))
            model_id = str(item.get("model_id", "UNKNOWN"))
            cand = Path(raw_ref)
            if not cand.is_file():
                cand = ROOT / raw_ref
            if not cand.is_file():
                still_locked.append(item)
                continue
            arts, locked = decrypt_frm_container(
                cand,
                raw_ref,
                work_root / f"retry_{idx:03d}",
                model_id=model_id,
                key_file=key_file,
                split_main=split_main,
                extract_rootfs_elfs=extract_rootfs_elfs,
            )
            if locked is not None:
                still_locked.append(locked)
            else:
                newly_unlocked += 1
                unlocked_artifacts.extend(arts)

        _merge_artifacts_into_dedup(unlocked_artifacts, dedup, unique_dir, out_dir)
    finally:
        shutil.rmtree(work_root, ignore_errors=True)

    sources_scanned = int(str(prev.get("sources_scanned", 0)))
    decryptable_sources = int(str(prev.get("decryptable_source_count", 0))) + newly_unlocked
    return _write_fleet_catalog(
        out_dir,
        input_label=str(prev.get("input", repo_relative_str(catalog_path))),
        sources_scanned=sources_scanned,
        decryptable_sources=decryptable_sources,
        dedup=dedup,
        locked_containers=still_locked,
    )


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
