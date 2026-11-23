"""Single-container `.FRM` unpacking, Linux kernel slicing, and rootfs ELF extraction."""

from __future__ import annotations

from pathlib import Path

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, PanadigaCliError, as_dict, as_list
from scripts.re._discovery_common import (
    detect_loader_banner,
    scan_embedded_filesystems,
    sha256_bytes,
    sha256_file,
    shannon_entropy,
)
from scripts.re.fleet_decrypt.discovery import (
    DecryptedArtifact,
    default_base_va_hex,
    extract_fs_elfs_via_cli,
    normalize_va_hex,
    probe_bench_runtime_vas,
    resolve_facade_hook,
    unpack_frm_via_cli,
)

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
        rep = resolve_facade_hook("unpack_frm_via_cli", unpack_frm_via_cli)(
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
    bench_vas = resolve_facade_hook("probe_bench_runtime_vas", probe_bench_runtime_vas)(
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
                    fs_rep = resolve_facade_hook("extract_fs_elfs_via_cli", extract_fs_elfs_via_cli)(
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

