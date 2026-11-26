"""Dependency-ordered DAG pipeline runner, CLI argument parser, and entrypoint dispatcher."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
from typing import Callable, Final, Mapping, Sequence

from scripts.bootstrap_deps import ensure_selection_dependencies
from scripts.build_cache.inventory import discover_workspace_inventory, optimize_build_env
from scripts.build_cache.steps import (
    clear_caches,
    ghidra_digest_and_outputs,
    is_step_fresh,
    pico_digest_and_outputs,
    record_step_cache,
)
from scripts.build_lanes.android import (
    build_android,
    install_android_rust_target,
    prepare_android_env,
)
from scripts.build_lanes.concurrency import flush_output, run_concurrent_tasks
from scripts.build_lanes.desktop import build_desktop, build_frontend, launch_gui, run_cli
from scripts.build_lanes.pico import compile_pico_boards_parallel, merge_and_sync_pico_uf2, prepare_pico_env
from scripts.panadiga_ops import ROOT, build_env, print_banner, print_status
from scripts.tui import PipelineSelection, launch_tui, render_inventory_and_cache_report
from scripts.validate import run_validation


def run_build_pipeline(
    selection: PipelineSelection,
    base_env: Mapping[str, str],
    *,
    strict_toolchains: bool = False,
) -> list[tuple[str, str, float]]:
    """Execute dependency-aware build & CI DAG: parallel when independent, serial at locks."""
    total_start = time.monotonic()
    boot_env = dict(base_env)
    ensure_selection_dependencies(selection, ROOT, boot_env)
    env, accel = optimize_build_env(boot_env, root=ROOT, use_cache=selection.use_cache)
    accel_desc = ", ".join(f"{k}={v}" for k, v in accel.items())
    print_banner(f"PANADIGA PIPELINE DAG | {accel_desc}")

    summary: list[tuple[str, str, float]] = []
    need_desktop_frontend = selection.frontend or selection.desktop
    pico_env = prepare_pico_env(env) if selection.pico else env
    pico_digest = ""
    pico_outputs: tuple[Path, ...] = ()
    pico_cached = False
    run_pico = False
    if selection.pico:
        has_arm = bool(shutil.which("arm-none-eabi-gcc", path=pico_env.get("PATH"))) and bool(
            pico_env.get("PICO_SDK_PATH")
        )
        if not has_arm:
            if strict_toolchains:
                raise FileNotFoundError("arm-none-eabi-gcc or PICO_SDK_PATH missing for --pico build.")
            print_status("INFO", "Pico toolchain/SDK absent; skipping RP2040/RP2350 firmware rebuild.", "33")
            summary.append(("Pico Firmware", "SKIP", 0.0))
        else:
            pico_digest, pico_outputs = pico_digest_and_outputs(ROOT, pico_env)
            if selection.use_cache and is_step_fresh(ROOT, "pico", pico_digest, pico_outputs):
                print_status("CACHE", "Pico firmware UF2 cache hit (firmware_bins unchanged).", "36")
                summary.append(("Pico Firmware", "CACHE HIT", 0.0))
                pico_cached = True
            else:
                run_pico = True

    run_android = False
    if selection.android:
        try:
            prepare_android_env(env)
            run_android = True
        except FileNotFoundError as exc:
            if strict_toolchains:
                raise
            print_status("INFO", f"{exc} Skipping Android APK.", "33")
            summary.append(("Android APK", "SKIP", 0.0))

    # Wave 1: Parallel independent build & preparation tasks.
    wave1_tasks: list[tuple[str, Callable[[], object]]] = []
    wave1_t0 = time.monotonic()
    if need_desktop_frontend:
        wave1_tasks.append(
            (
                "frontend",
                lambda: build_frontend(env, use_cache=selection.use_cache, capture_output=True),
            )
        )
    if run_android:
        wave1_tasks.append(
            (
                "frontend_android",
                lambda: build_frontend(
                    env,
                    use_cache=selection.use_cache,
                    capture_output=True,
                    android=True,
                ),
            )
        )
    if run_pico and not pico_cached:
        wave1_tasks.append(("pico_boards", lambda: compile_pico_boards_parallel(pico_env)))
    if run_android:
        wave1_tasks.append(("android_prep", lambda: install_android_rust_target(env)))

    if wave1_tasks:
        w1 = run_concurrent_tasks(wave1_tasks)
        w1_dt = time.monotonic() - wave1_t0
        if "frontend" in w1:
            fe_hit = bool(w1["frontend"])
            summary.append(("Frontend Vite UI", "CACHE HIT" if fe_hit else "BUILT", w1_dt))
        if "frontend_android" in w1:
            fa_hit = bool(w1["frontend_android"])
            summary.append(("Frontend Android UI", "CACHE HIT" if fa_hit else "BUILT", w1_dt))
        pico_res = w1.get("pico_boards")
        if isinstance(pico_res, tuple) and len(pico_res) == 2:
            rp2040_uf2, rp2350_uf2 = pico_res
            merge_and_sync_pico_uf2(
                rp2040_uf2,
                rp2350_uf2,
                pico_env,
                pico_digest,
                pico_outputs,
                prefer_release_merge=True,
            )
            summary.append(("Pico UF2 Dual+Merge", "BUILT", time.monotonic() - wave1_t0))

    # Wave 2: Parallel 9-gate CI validation (after UF2/frontend sync, before heavy release/APK).
    if selection.ci:
        ci_dt = run_validation(env)
        gh_digest, gh_outs = ghidra_digest_and_outputs(ROOT)
        record_step_cache(ROOT, "ghidra", gh_digest, gh_outs)
        summary.append(("CI 9 Quality Gates", "PASS", ci_dt))

    # Wave 3: Compile Desktop Release (CLI+GUI unified Cargo DAG) and Android APK in parallel!
    wave3_tasks: list[tuple[str, Callable[[], tuple[str, Path, float]]]] = []
    both_native = selection.desktop and run_android

    def _run_desktop_lane() -> tuple[str, Path, float]:
        dt0 = time.monotonic()
        out = build_desktop(
            env,
            skip_frontend=True,
            use_cache=selection.use_cache,
            capture_output=both_native,
        )
        return ("Desktop Release + dist/", out, time.monotonic() - dt0)

    def _run_android_lane() -> tuple[str, Path, float]:
        dt0 = time.monotonic()
        out = build_android(
            env,
            skip_frontend=True,
            skip_prep=True,
            use_cache=selection.use_cache,
            capture_output=both_native,
        )
        return ("Android ARM64 APK", out, time.monotonic() - dt0)

    if selection.desktop:
        wave3_tasks.append(("desktop", _run_desktop_lane))
    if run_android:
        wave3_tasks.append(("android", _run_android_lane))

    if wave3_tasks:
        w3 = run_concurrent_tasks(wave3_tasks)
        for key in ("desktop", "android"):
            if key in w3:
                lbl, _, lane_dt = w3[key]
                summary.append((lbl, "BUILT", lane_dt))

    total_dt = time.monotonic() - total_start
    print_banner(f"PANADIGA PIPELINE COMPLETE (Elapsed: {total_dt:.1f}s)", ansi_color="32")
    for label, status, dt in summary:
        print_status(status, f"{label:<26} ({dt:.2f}s)", "32" if status != "SKIP" else "33")

    if selection.run_after and selection.desktop:
        launch_gui(env, rebuild=False)
    return summary


_ALIASES: Final[dict[str, tuple[str, ...]]] = {
    "tui": ("--tui",),
    "all": ("--all",),
    "all-ci": ("--all", "--ci"),
    "ci": ("--ci",),
    "desktop": ("--desktop",),
    "frontend": ("--frontend",),
    "pico": ("--pico",),
    "android": ("--android",),
    "run": ("--run",),
    "rebuild": ("--rebuild-run",),
    "cache": ("--cache-status",),
    "clean-cache": ("--clean-cache",),
    "clean-all": ("--clean-all-caches",),
    "help": ("--help",),
    "cli": ("--cli",),
}


def normalize_argv(argv: Sequence[str]) -> list[str]:
    """Expand a leading launcher alias (`pico`, `all-ci`, ...) into its flag form.

    The `.cmd`/`.sh` launchers forward arguments untouched, so alias handling lives here once
    (no shell quoting, delayed-expansion, or `!`/`&` pitfalls).
    """
    items = list(argv)
    if items:
        alias = _ALIASES.get(items[0].lower())
        if alias is not None:
            return [*alias, *items[1:]]
    return items


def build_parser() -> argparse.ArgumentParser:
    """Build the Panadiga build orchestrator argument parser."""
    parser = argparse.ArgumentParser(description="Panadiga unified build, CI, cache, and TUI orchestrator.")
    parser.add_argument("--tui", action="store_true", help="Launch interactive TUI menu to choose targets, CI, and cache.")
    parser.add_argument("--ci", action="store_true", help="Execute 9 parallel Quality Gates in the pipeline.")
    parser.add_argument("--frontend", action="store_true", help="Build only the React/Vite desktop UI bundle.")
    parser.add_argument("--desktop", action="store_true", help="Build Desktop release binary, staging dist/ and release archive.")
    parser.add_argument("--android", action="store_true", help="Build Android ARM64 APK via Tauri Mobile.")
    parser.add_argument("--pico", action="store_true", help="Build RP2040, RP2350, and unified Pico UF2 firmwares.")
    parser.add_argument("--all", action="store_true", help="Build Pico (if toolchain exists), Desktop, and Android (if SDK/NDK exists).")
    parser.add_argument("--no-cache", action="store_true", help="Disable Panadiga hash cache and force rebuild of selected steps.")
    parser.add_argument("--cache-status", action="store_true", help="Print recursive workspace discovery, toolchains, and cache sizes.")
    parser.add_argument("--clean-cache", action="store_true", help="Clear Panadiga step/pico/vite caches.")
    parser.add_argument("--clean-all-caches", action="store_true", help="Clear all caches including Cargo target/debug and target/release.")
    parser.add_argument("--strict-toolchains", action="store_true", help="Fail instead of skipping if Pico/Android toolchains are absent in --all.")
    parser.add_argument("--run", action="store_true", help="Launch Desktop GUI (auto-builds if missing).")
    parser.add_argument("--rebuild-run", action="store_true", help="Force rebuild Desktop release and launch GUI.")
    parser.add_argument("--cli", nargs=argparse.REMAINDER, help="Forward remaining arguments to the native Panadiga CLI.")
    return parser


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    """Parse Panadiga build orchestrator CLI arguments (launcher aliases accepted)."""
    raw = list(sys.argv[1:] if argv is None else argv)
    return build_parser().parse_args(normalize_argv(raw))


def main(argv: Sequence[str] | None = None) -> int:
    """Dispatch build/CI/cache/TUI/run actions according to CLI flags."""
    raw_argv = list(sys.argv[1:] if argv is None else argv)
    if not raw_argv:
        if sys.stdin.isatty() and sys.stdout.isatty():
            raw_argv = ["--tui"]
        else:
            build_parser().print_help()
            return 0
    args = parse_args(raw_argv)
    base_env = build_env()
    use_cache = not args.no_cache
    env, _ = optimize_build_env(base_env, root=ROOT, use_cache=use_cache)

    try:
        if args.clean_all_caches or args.clean_cache:
            removed = clear_caches(ROOT, include_cargo=bool(args.clean_all_caches))
            print_status("OK", f"Cleared {len(removed)} cache directories.", "32")
            if not any((args.tui, args.ci, args.frontend, args.desktop, args.pico, args.android, args.all)):
                return 0

        if args.cache_status:
            inv = discover_workspace_inventory(ROOT, env)
            print(render_inventory_and_cache_report(ROOT, inv, env))
            return 0

        if args.tui:
            inv = discover_workspace_inventory(ROOT, env)
            chosen = launch_tui(inv, env)
            if chosen is None:
                return 0
            run_build_pipeline(chosen, env, strict_toolchains=args.strict_toolchains)
            return 0

        if args.cli is not None:
            return run_cli(args.cli, env)
        if args.rebuild_run:
            launch_gui(env, rebuild=True)
            return 0
        if args.run and not any((args.ci, args.desktop, args.pico, args.android, args.all, args.frontend)):
            launch_gui(env, rebuild=False)
            return 0

        if args.all:
            sel = PipelineSelection(
                ci=bool(args.ci),
                frontend=True,
                pico=True,
                desktop=True,
                android=True,
                use_cache=use_cache,
                run_after=bool(args.run),
            )
            run_build_pipeline(sel, env, strict_toolchains=args.strict_toolchains)
            return 0

        explicit = any((args.ci, args.frontend, args.desktop, args.pico, args.android))
        sel = PipelineSelection(
            ci=bool(args.ci),
            frontend=bool(args.frontend),
            pico=bool(args.pico),
            desktop=bool(args.desktop) or not explicit,
            android=bool(args.android),
            use_cache=use_cache,
            run_after=bool(args.run),
        )
        run_build_pipeline(sel, env, strict_toolchains=True)
        return 0
    except (subprocess.CalledProcessError, FileNotFoundError, RuntimeError) as exc:
        if isinstance(exc, subprocess.CalledProcessError):
            raw_diag: object = exc.output if exc.output else (exc.stdout if exc.stdout else exc.stderr)
            if isinstance(raw_diag, bytes):
                flush_output(raw_diag.decode("utf-8", errors="replace").strip())
            elif isinstance(raw_diag, str) and raw_diag.strip():
                flush_output(raw_diag.strip())
        print_status("FAIL", f"Build pipeline failed: {exc}", "31")
        return 1
    except KeyboardInterrupt:
        print_status("WARN", "Interrompido pelo usuario.", "33")
        return 130
    except EOFError:
        print_status("WARN", "Entrada encerrada (sem terminal interativo); nada a executar.", "33")
        return 0
    except Exception as exc:  # noqa: BLE001 - launcher must report, never dump a raw traceback
        if os.environ.get("PANADIGA_DEBUG"):
            raise
        print_status("FAIL", f"Erro inesperado ({type(exc).__name__}): {exc}", "31")
        return 1
