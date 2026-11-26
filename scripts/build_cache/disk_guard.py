"""Automatic ephemeral `target/` cache pruning when host disk space runs low."""

from __future__ import annotations

import os
import shutil
import sys
import time
from pathlib import Path

_GIB = 1024**3
_STALE_BUILD_SECONDS = 48 * 3600


def _parse_min_free_bytes(default_gb: float) -> int:
    raw = os.environ.get("PANADIGA_MIN_FREE_GB")
    if not raw:
        return int(max(0.0, default_gb) * _GIB)
    try:
        return int(max(0.0, float(raw)) * _GIB)
    except ValueError:
        return int(max(0.0, default_gb) * _GIB)


def _free_bytes(root: Path) -> int:
    probe = root if root.exists() else root.parent
    return shutil.disk_usage(probe).free


def _tree_bytes(path: Path) -> int:
    if not path.exists():
        return 0
    if path.is_file():
        try:
            return path.stat().st_size
        except OSError:
            return 0
    total = 0
    for entry in path.rglob("*"):
        if entry.is_file():
            try:
                total += entry.stat().st_size
            except OSError:
                continue
    return total


def _remove_tree(path: Path) -> int:
    if not path.exists():
        return 0
    before = _tree_bytes(path)
    if path.is_dir():
        shutil.rmtree(path, ignore_errors=True)
    else:
        try:
            path.unlink()
        except OSError:
            pass
    after = _tree_bytes(path)
    return max(0, before - after)


def _stale_build_hash_dirs(target_dir: Path, cutoff_epoch: float) -> list[Path]:
    stale: list[Path] = []
    for rel in ("debug/build", "release/build"):
        build_root = target_dir / rel
        if not build_root.is_dir():
            continue
        for crate_dir in sorted(build_root.iterdir()):
            if not crate_dir.is_dir():
                continue
            for hash_dir in sorted(crate_dir.iterdir()):
                if not hash_dir.is_dir():
                    continue
                try:
                    if hash_dir.stat().st_mtime < cutoff_epoch:
                        stale.append(hash_dir)
                except OSError:
                    continue
    return stale


def ensure_disk_headroom(root: Path, *, min_free_gb: float = 12.0, keep_android: bool = False) -> int:
    """Prune ephemeral `target/` caches until free space meets threshold. Returns reclaimed bytes."""
    if os.environ.get("PANADIGA_NO_DISK_GUARD"):
        return 0
    min_bytes = _parse_min_free_bytes(min_free_gb)
    if _free_bytes(root) >= min_bytes:
        return 0
    target_dir = root / "target"
    if not target_dir.is_dir():
        return 0

    candidates: list[Path] = [
        target_dir / "v020",
        target_dir / "tmp",
        target_dir / "debug" / "incremental",
        target_dir / "release" / "incremental",
        *sorted(target_dir.glob("flycheck*")),
    ]
    if not keep_android:
        candidates.extend(
            [
                target_dir / "android-cargo",
                target_dir / "aarch64-linux-android",
                target_dir / "armv7-linux-androideabi",
            ]
        )
    candidates.extend(_stale_build_hash_dirs(target_dir, time.time() - _STALE_BUILD_SECONDS))
    candidates.append(target_dir / "debug")

    threshold_gb = min_bytes / _GIB
    free_gb = _free_bytes(root) / _GIB
    sys.stderr.write(
        f"[BOOTSTRAP] [INFO] Low disk space ({free_gb:.2f} GiB free < {threshold_gb:.1f} GiB); pruning target/ caches...\n"
    )
    sys.stderr.flush()

    reclaimed = 0
    for candidate in candidates:
        if _free_bytes(root) >= min_bytes:
            break
        reclaimed += _remove_tree(candidate)

    if reclaimed > 0:
        sys.stderr.write(f"[BOOTSTRAP] [OK] Reclaimed {reclaimed / _GIB:.2f} GiB from target/ caches.\n")
        sys.stderr.flush()
    return reclaimed
