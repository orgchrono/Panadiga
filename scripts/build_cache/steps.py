"""Content-hash build step caching, artifact digests, and compiler cache statistics."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import threading
from pathlib import Path
from typing import Final, Mapping, Sequence

from scripts.panadiga_ops import ROOT, sha256_file
from scripts.build_cache.inventory import CacheEntryStat, iter_files_recursive, resolve_tool

CACHE_DIR_NAME: Final[str] = ".panadiga-cache"
MANIFEST_FILENAME: Final[str] = "steps.json"
_CACHE_LOCK = threading.Lock()


def _manifest_path(cache_dir: Path) -> Path:
    return cache_dir / MANIFEST_FILENAME


def default_cache_dir(root: Path = ROOT) -> Path:
    """Return the default per-repository Panadiga step cache directory."""
    return root / "target" / CACHE_DIR_NAME


def _load_manifest_unlocked(cache_dir: Path) -> dict[str, object]:
    mf = _manifest_path(cache_dir)
    if not mf.is_file():
        return {}
    try:
        loaded = json.loads(mf.read_text(encoding="utf-8"))
        if not isinstance(loaded, dict):
            return {}
        return {str(k): v for k, v in loaded.items()}
    except (OSError, ValueError):
        return {}


def _write_manifest_unlocked(cache_dir: Path, data: Mapping[str, object]) -> None:
    cache_dir.mkdir(parents=True, exist_ok=True)
    tmp = _manifest_path(cache_dir).with_suffix(".tmp")
    tmp.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    tmp.replace(_manifest_path(cache_dir))


def _relative_key(root: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def compute_tree_digest(
    root: Path,
    input_paths: Sequence[Path],
    *,
    extensions: frozenset[str] | None = None,
    extra_salt: Sequence[str] = (),
) -> str:
    """Compute deterministic SHA-256 across files, directories, and toolchain salt strings."""
    hasher = hashlib.sha256()
    seen: set[Path] = set()
    files: list[Path] = []
    for p in input_paths:
        for item in iter_files_recursive(p, extensions=extensions, ignore_prefixes=("build",)):
            resolved = item.resolve()
            if resolved not in seen:
                seen.add(resolved)
                files.append(item)

    files.sort(key=lambda f: f.as_posix())
    for item in files:
        try:
            rel = item.resolve().relative_to(root.resolve()).as_posix()
        except ValueError:
            rel = item.name
        hasher.update(rel.encode("utf-8"))
        hasher.update(b"\0")
        hasher.update(sha256_file(item).encode("ascii"))
        hasher.update(b"\n")

    for salt in extra_salt:
        hasher.update(salt.encode("utf-8", errors="replace"))
        hasher.update(b"\0")
    return hasher.hexdigest()


def is_step_fresh(
    root: Path,
    step_key: str,
    input_digest: str,
    outputs: Sequence[Path],
    *,
    cache_dir: Path | None = None,
) -> bool:
    """Return True when input_digest and all recorded output file SHA-256 digests match."""
    cdir = cache_dir or default_cache_dir(root)
    with _CACHE_LOCK:
        manifest = _load_manifest_unlocked(cdir)
    entry = manifest.get(step_key)
    if not isinstance(entry, dict):
        return False
    if entry.get("input_digest") != input_digest:
        return False
    recorded_outputs = entry.get("outputs")
    if not isinstance(recorded_outputs, dict):
        return False

    for out_path in outputs:
        if not out_path.is_file():
            return False
        rel = _relative_key(root, out_path)
        if recorded_outputs.get(rel) != sha256_file(out_path):
            return False
    return True


def record_step_cache(
    root: Path,
    step_key: str,
    input_digest: str,
    outputs: Sequence[Path],
    *,
    cache_dir: Path | None = None,
) -> None:
    """Persist input digest and output SHA-256 checksums for subsequent incremental builds."""
    cdir = cache_dir or default_cache_dir(root)
    out_hashes: dict[str, str] = {}
    for out_path in outputs:
        if out_path.is_file():
            out_hashes[_relative_key(root, out_path)] = sha256_file(out_path)

    with _CACHE_LOCK:
        manifest = _load_manifest_unlocked(cdir)
        manifest[step_key] = {"input_digest": input_digest, "outputs": out_hashes}
        _write_manifest_unlocked(cdir, manifest)


def copy_if_changed(src: Path, dst: Path) -> bool:
    """Copy src -> dst only when dst is absent or its SHA-256 differs, preserving mtime on hits."""
    if not src.is_file():
        raise FileNotFoundError(f"Source artifact missing: {src}")
    if dst.is_file() and sha256_file(src) == sha256_file(dst):
        return False
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)
    return True


def frontend_digest_and_outputs(root: Path, *, android: bool = False) -> tuple[str, tuple[Path, ...]]:
    """Compute input digest and expected artifact paths for the React/Vite UI bundle."""
    app_dir = root / "apps" / "desktop"
    inputs = (
        app_dir / "src",
        app_dir / "index.html",
        app_dir / "package.json",
        app_dir / "package-lock.json",
        app_dir / "tsconfig.json",
        app_dir / "tsconfig.node.json",
        app_dir / "vite.config.ts",
    )
    out_dir = "dist-android" if android else "dist"
    outputs = (app_dir / out_dir / "index.html",)
    return compute_tree_digest(root, inputs, extra_salt=(f"target={out_dir}",)), outputs


def pico_digest_and_outputs(root: Path, env: Mapping[str, str]) -> tuple[str, tuple[Path, ...]]:
    """Compute input digest and UF2 output paths for RP2040/RP2350 PicoFW firmware."""
    fw_dir = root / "firmware" / "picofw"
    dest_dir = root / "crates" / "panadiga-core" / "src" / "pico" / "firmware_bins"
    salt = (
        f"pico_sdk={env.get('PICO_SDK_PATH', '')}",
        f"ccache={env.get('PANADIGA_CCACHE', '')}",
    )
    exts = frozenset({".c", ".h", ".pio", ".cmake", "CMakeLists.txt"})
    inputs = (fw_dir,)
    outputs = (dest_dir / "picofw_unified.uf2",)
    return compute_tree_digest(root, inputs, extensions=exts, extra_salt=salt), outputs


def ghidra_digest_and_outputs(root: Path) -> tuple[str, tuple[Path, ...]]:
    """Compute input digest and generated Jython output paths for Ghidra SSOT scripts."""
    core_src = root / "crates" / "panadiga-core" / "src"
    inputs = (
        core_src / "ssot",
        core_src / "ghidra",
        core_src / "constants",
        core_src / "constants.rs",
    )
    ghidra_dir = root / "tools" / "ghidra"
    outputs = (
        ghidra_dir / "ghidra_am34_memory_map.py",
        ghidra_dir / "ghidra_import_symbols_panadiga.py",
        ghidra_dir / "ghidra_importar_simbolos_panadiga.py",
        ghidra_dir / "ghidra_auto_unbrick_audit.py",
        ghidra_dir / "ghidra_mn103_fix_decompilation.py",
        ghidra_dir / "ghidra_connect_debugger.py",
    )
    return compute_tree_digest(root, inputs), outputs


def _dir_size_bytes(path: Path) -> int:
    if path.is_file():
        return path.stat().st_size
    if not path.is_dir():
        return 0
    total = 0
    for dirpath_str, _, filenames in os.walk(path):
        dirpath = Path(dirpath_str)
        for fname in filenames:
            try:
                total += (dirpath / fname).stat().st_size
            except OSError:
                pass
    return total


def inspect_cache_entries(root: Path = ROOT) -> list[CacheEntryStat]:
    """Return size and existence metrics across Panadiga build caches."""
    candidates: tuple[tuple[str, Path], ...] = (
        ("Panadiga Step Cache", default_cache_dir(root)),
        ("Pico CMake Build", root / "target" / "pico-build"),
        ("Vite Cache", root / "apps" / "desktop" / "node_modules" / ".vite"),
        ("Frontend Dist", root / "apps" / "desktop" / "dist"),
        ("Android Frontend Dist", root / "apps" / "desktop" / "dist-android"),
        ("Cargo Debug Cache", root / "target" / "debug"),
        ("Cargo Release Cache", root / "target" / "release"),
        ("Cargo Android Cache", root / "target" / "android-cargo"),
    )
    return [
        CacheEntryStat(
            label=label,
            path=path,
            exists=path.exists(),
            bytes_total=_dir_size_bytes(path) if path.exists() else 0,
        )
        for label, path in candidates
    ]


def clear_caches(root: Path = ROOT, *, include_cargo: bool = False) -> list[Path]:
    """Remove Panadiga incremental step/build caches cleanly; optionally wipe target/ too."""
    targets = [
        default_cache_dir(root),
        root / "target" / "pico-build",
        root / "apps" / "desktop" / "node_modules" / ".vite",
        root / "apps" / "desktop" / "dist",
        root / "apps" / "desktop" / "dist-android",
    ]
    if include_cargo:
        targets.extend(
            [
                root / "target" / "debug",
                root / "target" / "release",
                root / "target" / "android-cargo",
            ]
        )
    removed: list[Path] = []
    for t in targets:
        if t.exists():
            shutil.rmtree(t, ignore_errors=True)
            removed.append(t)
    return removed


def query_compiler_cache_stats(env: Mapping[str, str]) -> list[str]:
    """Collect optional sccache/ccache summary strings without failing when unavailable."""
    lines: list[str] = []
    for tool, args in (("sccache", ["--show-stats"]), ("ccache", ["--print-stats"])):
        exe = resolve_tool(tool, env)
        if not exe:
            continue
        try:
            proc = subprocess.run(
                [exe, *args],
                env=dict(env),
                capture_output=True,
                text=True,
                timeout=3,
                check=False,
            )
            if proc.returncode == 0 and proc.stdout.strip():
                first_lines = " | ".join(l.strip() for l in proc.stdout.splitlines()[:4] if l.strip())
                lines.append(f"{tool}: {first_lines}")
        except (OSError, subprocess.TimeoutExpired):
            pass
    return lines
