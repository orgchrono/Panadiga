"""Frontend, Ghidra SSOT export, Desktop release build, staging, and app/CLI launchers."""

from __future__ import annotations

import platform
import shutil
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path
from typing import Final, Mapping, Sequence

from scripts.build_cache.steps import (
    copy_if_changed,
    frontend_digest_and_outputs,
    ghidra_digest_and_outputs,
    is_step_fresh,
    record_step_cache,
)
from scripts.build_lanes.concurrency import flush_output
from scripts.panadiga_ops import (
    ROOT,
    TOOLS_PYCACHE_IGNORE,
    locate_panadiga_binary,
    locate_panadiga_desktop_binary,
    panadiga_binary_name,
    panadiga_desktop_binary_name,
    print_banner,
    print_status,
    resolve_executable,
    run_step,
    update_sha256_manifest,
)

UF2_NAMES: Final[tuple[str, ...]] = (
    "panasonic_p7501_uart_rp2040.uf2",
    "panasonic_p7501_uart_rp2350.uf2",
    "panasonic_p7501_uart_unified.uf2",
)
STALE_DIST_ITEMS: Final[tuple[str, ...]] = (
    "run.cmd",
    "run.ps1",
    "run.sh",
    "WebView2Loader.dll",
    "firmware",
    "README.md",
)


def build_frontend(env: Mapping[str, str], *, use_cache: bool = True, capture_output: bool = False) -> bool:
    """Build Desktop React/Vite bundle; returns True when satisfied from hash cache."""
    digest, outputs = frontend_digest_and_outputs(ROOT)
    if use_cache and is_step_fresh(ROOT, "frontend", digest, outputs):
        if not capture_output:
            print_status("CACHE", "Frontend Vite UI cache hit (apps/desktop/dist unchanged).", "36")
        return True

    out = run_step(
        ["npm", "--prefix", "apps/desktop", "run", "build"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )
    flush_output(out)
    record_step_cache(ROOT, "frontend", digest, outputs)
    return False


def export_ghidra_ssot(
    cli_bin: Path,
    env: Mapping[str, str],
    *,
    use_cache: bool = True,
) -> bool:
    """Export Ghidra Jython SSOT scripts; returns True when satisfied from cache."""
    digest, outputs = ghidra_digest_and_outputs(ROOT)
    if use_cache and is_step_fresh(ROOT, "ghidra", digest, outputs):
        return True

    out = run_step(
        [str(cli_bin), "ghidra", "export-all", "--dir", "tools/ghidra"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )
    flush_output(out)
    record_step_cache(ROOT, "ghidra", digest, outputs)
    return False


def _stage_tools_directory(dist_dir: Path) -> None:
    """Stage clean runtime tools into dist/tools/ per target OS without tests or dev scripts."""
    src_tools = ROOT / "tools"
    dst_tools = dist_dir / "tools"
    if dst_tools.exists():
        shutil.rmtree(dst_tools)
    if not src_tools.is_dir():
        return

    dst_tools.mkdir(parents=True, exist_ok=True)
    ignore_fn = shutil.ignore_patterns(*TOOLS_PYCACHE_IGNORE, "*.pyc")

    ghidra_src = src_tools / "ghidra"
    if ghidra_src.is_dir():
        shutil.copytree(ghidra_src, dst_tools / "ghidra", ignore=ignore_fn)

    tools_readme = src_tools / "README.md"
    if tools_readme.is_file():
        shutil.copy2(tools_readme, dst_tools / "README.md")

    if sys.platform == "win32":
        for cmd_file in sorted(src_tools.glob("*.cmd")):
            shutil.copy2(cmd_file, dst_tools / cmd_file.name)
        hxd_src = src_tools / "HxD"
        if hxd_src.is_dir():
            shutil.copytree(hxd_src, dst_tools / "HxD", ignore=ignore_fn)


def _package_desktop_archive(dist_dir: Path, releases_dir: Path) -> Path:
    """Create platform release archive (.zip on Windows, .tar.gz on POSIX) from dist/."""
    releases_dir.mkdir(parents=True, exist_ok=True)
    machine = platform.machine().lower()
    arch = "x86_64" if machine in {"amd64", "x86_64"} else ("aarch64" if machine in {"arm64", "aarch64"} else machine)

    if sys.platform == "win32":
        archive_path = releases_dir / f"panadiga-windows-{arch}.zip"
        with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            for item in sorted(dist_dir.rglob("*")):
                if item.is_file() and releases_dir not in item.parents and item != archive_path:
                    zf.write(item, arcname=item.relative_to(dist_dir))
        return archive_path

    os_slug = "macos" if sys.platform == "darwin" else "linux"
    archive_path = releases_dir / f"panadiga-{os_slug}-{arch}.tar.gz"
    with tarfile.open(archive_path, "w:gz") as tf:
        for item in sorted(dist_dir.iterdir()):
            if item != releases_dir:
                tf.add(item, arcname=item.name)
    return archive_path


def _copy_release_binary(bin_name: str, dist_dir: Path) -> Path:
    """Copy a compiled target/release binary into dist/ with executable permissions on POSIX."""
    built_bin = ROOT / "target" / "release" / bin_name
    if not built_bin.is_file():
        raise FileNotFoundError(f"Expected compiled binary missing: {built_bin}")
    dest_bin = dist_dir / bin_name
    shutil.copy2(built_bin, dest_bin)
    if sys.platform != "win32":
        dest_bin.chmod(dest_bin.stat().st_mode | 0o111)
    return dest_bin


def sync_firmware_bins_to_dist(dist_dir: Path) -> None:
    """Copy compiled P7501 UF2 images from firmware_bins/ into dist/ when dist/ is present."""
    fw_bins_dir = ROOT / "crates" / "panadiga-core" / "src" / "pico" / "firmware_bins"
    for uf2_name in UF2_NAMES:
        uf2_src = fw_bins_dir / uf2_name
        if uf2_src.is_file():
            copy_if_changed(uf2_src, dist_dir / uf2_name)


def build_desktop(
    env: Mapping[str, str],
    *,
    skip_frontend: bool = False,
    use_cache: bool = True,
    capture_output: bool = False,
) -> Path:
    """Build CLI & Desktop GUI together in one parallel Cargo DAG, export Ghidra SSOT, and stage dist/."""
    print_banner("BUILD DESKTOP: Unified Parallel Cargo (CLI + Desktop GUI Release)")
    if not skip_frontend:
        build_frontend(env, use_cache=use_cache, capture_output=capture_output)

    cargo_out = run_step(
        ["cargo", "build", "--release", "-p", "panadiga-cli", "-p", "panadiga-desktop"],
        cwd=ROOT,
        env=env,
        capture_output=capture_output,
    )
    flush_output(cargo_out)

    release_cli = ROOT / "target" / "release" / panadiga_binary_name()
    if export_ghidra_ssot(release_cli, env, use_cache=use_cache):
        print_status("CACHE", "Ghidra SSOT scripts unchanged (cache hit).", "36")

    dist_dir = ROOT / "dist"
    releases_dir = dist_dir / "releases"
    dist_dir.mkdir(parents=True, exist_ok=True)

    for stale in STALE_DIST_ITEMS:
        target = dist_dir / stale
        if target.is_dir():
            shutil.rmtree(target)
        elif target.is_file():
            target.unlink()

    cli_bin = _copy_release_binary(panadiga_binary_name(), dist_dir)
    gui_bin = _copy_release_binary(panadiga_desktop_binary_name(), dist_dir)

    _stage_tools_directory(dist_dir)
    sync_firmware_bins_to_dist(dist_dir)

    license_src = ROOT / "LICENSE"
    if license_src.is_file():
        shutil.copy2(license_src, dist_dir / "LICENSE")

    archive_path = _package_desktop_archive(dist_dir, releases_dir)
    update_sha256_manifest(releases_dir / "SHA256SUMS.txt", [archive_path])
    print_status("OK", f"Desktop release ready: {gui_bin} & {cli_bin} | Archive: {archive_path}", "32")
    return gui_bin


def launch_gui(env: Mapping[str, str], rebuild: bool = False) -> None:
    """Launch Desktop GUI detached, building desktop release first when missing."""
    binary = None if rebuild else locate_panadiga_desktop_binary()
    if binary is None:
        binary = build_desktop(env)

    print_status("INFO", f"Launching Panadiga Desktop: {binary}", "36")
    if sys.platform == "win32":
        flags = getattr(subprocess, "DETACHED_PROCESS", 0) | getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)
        subprocess.Popen([str(binary)], cwd=str(binary.parent), creationflags=flags)
    else:
        subprocess.Popen([str(binary)], cwd=str(binary.parent), start_new_session=True)


def run_cli(cli_args: Sequence[str], env: Mapping[str, str]) -> int:
    """Run native Panadiga CLI subcommand via compiled binary or cargo fallback."""
    binary = locate_panadiga_binary()
    if binary is not None:
        proc = subprocess.run([str(binary), *cli_args], cwd=str(ROOT), env=dict(env), check=False)
        return proc.returncode
    cargo_bin = resolve_executable("cargo", env=env)
    proc = subprocess.run([cargo_bin, "run", "-p", "panadiga-cli", "--", *cli_args], cwd=str(ROOT), env=dict(env), check=False)
    return proc.returncode
