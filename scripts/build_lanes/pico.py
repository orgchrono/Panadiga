"""Parallel RP2040/RP2350 CMake/Ninja firmware build and unified UF2 merge lane."""

from __future__ import annotations

from pathlib import Path
from typing import Mapping, Sequence

from scripts.build_cache.inventory import discover_pico_sdk, optimize_build_env
from scripts.build_cache.steps import (
    copy_if_changed,
    is_step_fresh,
    pico_digest_and_outputs,
    record_step_cache,
)
from scripts.build_lanes.concurrency import flush_output, run_concurrent_tasks
from scripts.build_lanes.desktop import sync_firmware_bins_to_dist
from scripts.panadiga_ops import ROOT, load_msvc_env, locate_panadiga_binary, print_banner, print_status, run_step


def prepare_pico_env(base_env: Mapping[str, str]) -> dict[str, str]:
    """Augment build environment with MSVC host env, Pico SDK, and ARM embedded toolchain."""
    pico_env, _ = optimize_build_env(load_msvc_env(base_env), root=ROOT, use_cache=True)
    pico_env.setdefault("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
    if not pico_env.get("PICO_SDK_PATH"):
        sdk = discover_pico_sdk(ROOT, pico_env)
        if sdk is not None:
            pico_env["PICO_SDK_PATH"] = str(sdk)
    return pico_env


def _build_single_pico_board(
    board: str,
    fw_dir: Path,
    build_root: Path,
    pico_env: Mapping[str, str],
) -> tuple[str, Path, str]:
    """Configure and compile one Pico board target cleanly out-of-tree."""
    bdir = build_root / board
    cmake_cmd = [
        "cmake",
        "-S",
        str(fw_dir),
        "-B",
        str(bdir),
        "-G",
        "Ninja",
        "-DCMAKE_BUILD_TYPE=Release",
        f"-DPICO_BOARD={board}",
        "-DCMAKE_POLICY_VERSION_MINIMUM=3.5",
    ]
    ccache_bin = pico_env.get("PANADIGA_CCACHE")
    if ccache_bin:
        cmake_cmd.extend(
            [
                f"-DCMAKE_C_COMPILER_LAUNCHER={ccache_bin}",
                f"-DCMAKE_CXX_COMPILER_LAUNCHER={ccache_bin}",
            ]
        )

    log_a = run_step(cmake_cmd, cwd=ROOT, env=pico_env, capture_output=True)
    log_b = run_step(
        ["cmake", "--build", str(bdir), "--config", "Release", "--parallel"],
        cwd=ROOT,
        env=pico_env,
        capture_output=True,
    )
    built_uf2 = bdir / "panasonic_p7501_uart.uf2"
    if not built_uf2.is_file():
        raise FileNotFoundError(f"Expected compiled UF2 missing: {built_uf2}")
    return board, built_uf2, f"{log_a}{log_b}"


def compile_pico_boards_parallel(pico_env: Mapping[str, str]) -> tuple[Path, Path]:
    """Build RP2040 and RP2350 CMake/Ninja targets simultaneously."""
    fw_dir = ROOT / "firmware" / "pico-p7501"
    build_root = ROOT / "target" / "pico-build"
    results = run_concurrent_tasks(
        [
            ("pico", lambda: _build_single_pico_board("pico", fw_dir, build_root, pico_env)),
            ("pico2", lambda: _build_single_pico_board("pico2", fw_dir, build_root, pico_env)),
        ]
    )
    rp2040_uf2 = Path(results["pico"][1])
    rp2350_uf2 = Path(results["pico2"][1])
    for key in ("pico", "pico2"):
        logs = str(results[key][2])
        if logs:
            flush_output(f"[{key.upper()}]\n{logs}")
    return rp2040_uf2, rp2350_uf2


def merge_and_sync_pico_uf2(
    rp2040_uf2: Path,
    rp2350_uf2: Path,
    pico_env: Mapping[str, str],
    digest: str,
    outputs: Sequence[Path],
    *,
    prefer_release_merge: bool = False,
) -> None:
    """Merge dual UF2s and sync firmware_bins/ only where SHA-256 changed."""
    build_root = ROOT / "target" / "pico-build"
    dest_dir = ROOT / "crates" / "panadiga-core" / "src" / "pico" / "firmware_bins"
    dest_dir.mkdir(parents=True, exist_ok=True)
    staged_unified = build_root / "panasonic_p7501_uart_unified.uf2"

    existing_cli = locate_panadiga_binary()
    if existing_cli is not None:
        merge_cmd: list[str] = [str(existing_cli)]
    elif prefer_release_merge:
        merge_cmd = ["cargo", "run", "--release", "-p", "panadiga-cli", "--"]
    else:
        merge_cmd = ["cargo", "run", "-p", "panadiga-cli", "--"]

    run_step(
        [
            *merge_cmd,
            "pico",
            "merge-uf2",
            "--rp2040",
            str(rp2040_uf2),
            "--rp2350",
            str(rp2350_uf2),
            "--out",
            str(staged_unified),
        ],
        cwd=ROOT,
        env=pico_env,
    )
    for obsolete_name in ("panasonic_p7501_uart_rp2040.uf2", "panasonic_p7501_uart_rp2350.uf2"):
        (dest_dir / obsolete_name).unlink(missing_ok=True)
    changed_unified = copy_if_changed(staged_unified, dest_dir / "panasonic_p7501_uart_unified.uf2")
    dist_dir = ROOT / "dist"
    if dist_dir.is_dir():
        sync_firmware_bins_to_dist(dist_dir)
    record_step_cache(ROOT, "pico", digest, outputs)
    if changed_unified:
        print_status("OK", f"Synced updated UF2 binaries into {dest_dir}", "32")
    else:
        print_status("CACHE", "Rebuilt UF2 bytes identical to firmware_bins/ (zero-touch skip).", "36")


def build_pico(
    env: Mapping[str, str],
    *,
    prefer_release_merge: bool = False,
    use_cache: bool = True,
) -> bool:
    """Compile RP2040 + RP2350 Pico firmwares in parallel, merge UF2, and sync firmware_bins/."""
    print_banner("BUILD PICO: Parallel RP2040 + RP2350 + Unified UF2")
    pico_env = prepare_pico_env(env)
    digest, outputs = pico_digest_and_outputs(ROOT, pico_env)
    if use_cache and is_step_fresh(ROOT, "pico", digest, outputs):
        if (ROOT / "dist").is_dir():
            sync_firmware_bins_to_dist(ROOT / "dist")
        print_status("CACHE", "Pico firmware UF2 cache hit (firmware_bins unchanged).", "36")
        return True

    rp2040_uf2, rp2350_uf2 = compile_pico_boards_parallel(pico_env)
    merge_and_sync_pico_uf2(
        rp2040_uf2,
        rp2350_uf2,
        pico_env,
        digest,
        outputs,
        prefer_release_merge=prefer_release_merge,
    )
    return False
