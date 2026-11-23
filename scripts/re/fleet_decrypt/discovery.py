"""Input discovery, VA normalization, raw NOR triage, and panadiga CLI wrappers."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Final, TypeVar, cast

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
    detect_loader_banner,
    extract_frm_from_archive_or_sfx,
    is_panasonic_frm_path,
    sha256_bytes,
    shannon_entropy,
    slugify_identifier,
)

_FnT = TypeVar("_FnT", bound=Callable[..., object])


def resolve_facade_hook(name: str, default: _FnT) -> _FnT:
    """Resolve optional monkeypatched hook on `scripts.re.fleet_decryptor` when loaded."""
    mod = sys.modules.get("scripts.re.fleet_decryptor")
    if mod is not None:
        candidate = getattr(mod, name, None)
        if callable(candidate):
            return cast(_FnT, candidate)
    return default


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

