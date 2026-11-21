"""Individual Quality Gate runners and parallel DAG scheduler for Panadiga."""

from __future__ import annotations

from concurrent.futures import FIRST_EXCEPTION, Future, ThreadPoolExecutor, wait
from pathlib import Path
import subprocess
import sys
import time
from typing import Callable, Final, Mapping

from scripts.panadiga_ops import (
    ProcessTracker,
    ROOT,
    print_banner,
    print_status,
    run_step,
    use_process_tracker,
)
from scripts.validate_c11 import C11_HOST_SOURCES as C11_HOST_SOURCES, _gate_c11_host as _gate_c11_host

DESKTOP_DIR: Final[Path] = ROOT / "apps" / "desktop"
PYTHON_COMPILE_TARGETS: Final[tuple[str, ...]] = (
    "tools/ghidra/ghidra_am34_memory_map.py", "tools/ghidra/ghidra_import_symbols_panadiga.py",
    "tools/ghidra/ghidra_importar_simbolos_panadiga.py", "tools/ghidra/ghidra_auto_unbrick_audit.py",
    "tools/ghidra/ghidra_mn103_fix_decompilation.py", "tools/ghidra/ghidra_zero_undef_pipeline.py",
    "tools/ghidra/ghidra_connect_debugger.py",
    "scripts/ops/__init__.py", "scripts/ops/proc.py", "scripts/ops/toolchain.py",
    "scripts/panadiga_ops.py", "scripts/panadiga_cli_bridge.py", "scripts/panadiga_cache.py",
    "scripts/build_cache/__init__.py", "scripts/build_cache/inventory.py", "scripts/build_cache/steps.py",
    "scripts/tui_plan.py", "scripts/tui_term.py", "scripts/tui.py", "scripts/build.py",
    "scripts/build_lanes/__init__.py", "scripts/build_lanes/concurrency.py",
    "scripts/build_lanes/desktop.py", "scripts/build_lanes/android.py",
    "scripts/build_lanes/pico.py", "scripts/build_lanes/dag.py",
    "scripts/validate_c11.py", "scripts/validate_gates.py", "scripts/validate.py", "scripts/profiles_types.py",
    "scripts/dev/__init__.py", "scripts/dev/cad_primitives.py", "scripts/dev/schematic_sheet.py",
    "scripts/dev/generate_schematics_and_cad.py",
    "scripts/re/__init__.py", "scripts/re/mn10300.py", "scripts/re/mn103_traps.py", "scripts/re/pana_frm.py",
    "scripts/bench/__init__.py", "scripts/bench/protocol.py", "scripts/bench/decoders.py",
    "scripts/bench/nor_dumper.py", "scripts/bench/rsp_frames.py", "scripts/bench/rsp_dump.py",
    "scripts/bench/boot_harness.py", "scripts/bench/runner.py",
)


def _gate_rust(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("1/9. Validating Rust Workspace (Cargo Fmt, Clippy -D warnings & Tests)")
    run_step(["cargo", "fmt", "--all", "--", "--check"], cwd=ROOT, env=env, capture_output=capture_output)
    run_step(["cargo", "clippy", "--workspace", "--all-targets", "--no-deps", "--", "-D", "warnings"], cwd=ROOT, env=env, capture_output=capture_output)
    run_step(["cargo", "test", "--workspace"], cwd=ROOT, env=env, capture_output=capture_output)
    if not capture_output:
        print_status("PASS", "Rust workspace formatted, clippy-clean (-D warnings), and all tests passed!", "32")


def _gate_typescript(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("2/9. Validating Oxlint Strict & TypeScript (oxlint --deny-warnings && tsc --noEmit)")
    run_step(["npm", "run", "lint"], cwd=DESKTOP_DIR, env=env, capture_output=capture_output)
    if not capture_output:
        print_status("PASS", "Oxlint (129 rules) and TypeScript strict check succeeded with 0 warnings/errors!", "32")


def _gate_vitest(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("3/9. Executing Desktop Frontend Unit Tests (Vitest)")
    run_step(["npm", "test"], cwd=DESKTOP_DIR, env=env, capture_output=capture_output)
    if not capture_output:
        print_status("PASS", "All Vitest frontend and ViewModel unit tests passed!", "32")


def _gate_jscpd(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("4/9. Running Copy/Paste Detector (jscpd threshold 0% / 0 clones)")
    run_step(["npx", "--prefix", "apps/desktop", "jscpd", "--config", ".jscpd.json"], cwd=ROOT, env=env, capture_output=capture_output)
    if not capture_output:
        print_status("PASS", "Code duplication verified at 0% (0 clones found)!", "32")


def _gate_uxlint(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("5/9. Running UI/UX State Linter (uxlint)")
    run_step(["npm", "run", "uxlint"], cwd=DESKTOP_DIR, env=env, capture_output=capture_output)
    if not capture_output:
        print_status("PASS", "UX state completeness audit passed!", "32")


def _gate_playwright(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("6/9. Running Playwright End-to-End Tests (Chromium)")
    run_step(["npm", "run", "test:e2e"], cwd=DESKTOP_DIR, env=env, capture_output=capture_output)
    if not capture_output:
        print_status("PASS", "All Playwright E2E browser tests passed!", "32")


def _gate_ghidra(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("7/9. Validating SSOT & Ghidra Native Generator (panadiga ssot / ghidra)")
    ghidra_dir = str(ROOT / "tools" / "ghidra")
    run_step(
        ["cargo", "run", "-p", "panadiga-cli", "--", "ssot", "export-all", "--root", str(ROOT)],
        cwd=ROOT,
        env=env,
        capture_output=capture_output,
    )
    run_step(
        ["cargo", "run", "-p", "panadiga-cli", "--", "ghidra", "export-all", "--dir", ghidra_dir],
        cwd=ROOT,
        env=env,
        capture_output=capture_output,
    )
    if not capture_output:
        print_status("PASS", "C11/TS/Schema/Ghidra SSOT projections generated cleanly from Rust!", "32")


def _gate_python_mypy(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    run_step([sys.executable, "-m", "mypy", "--strict", "scripts"], cwd=ROOT, env=env, capture_output=capture_output)


def _gate_python_compile_and_pytest(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    run_step([sys.executable, "-m", "py_compile", *PYTHON_COMPILE_TARGETS], cwd=ROOT, env=env, capture_output=capture_output)
    run_step([sys.executable, "-m", "pytest", "scripts/tests"], cwd=ROOT, env=env, capture_output=capture_output)


def _gate_python(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("9/9. Verifying Build Scripts, Ghidra Jython Syntax, Mypy Strict & Pytest")
    _gate_python_mypy(env, capture_output=capture_output)
    _gate_python_compile_and_pytest(env, capture_output=capture_output)
    if not capture_output:
        print_status("PASS", "Python build scripts, Ghidra syntax, mypy --strict, and pytest passed!", "32")


def _lane_rust_and_ghidra(env: Mapping[str, str]) -> None:
    _gate_rust(env, capture_output=True)
    _gate_ghidra(env, capture_output=True)


def run_sequential_gates(env: Mapping[str, str]) -> None:
    """Execute all 9 Quality Gates sequentially with unbuffered stdout/stderr."""
    for gate in (
        _gate_rust,
        _gate_typescript,
        _gate_vitest,
        _gate_jscpd,
        _gate_uxlint,
        _gate_playwright,
        _gate_ghidra,
        _gate_c11_host,
        _gate_python,
    ):
        gate(env)


def run_parallel_lanes(env: Mapping[str, str], max_jobs: int) -> None:
    """Execute all 9 Quality Gates via thread-pool DAG with fail-fast ProcessTracker cleanup."""
    lanes: tuple[tuple[str, Callable[[Mapping[str, str]], None]], ...] = (
        ("1+7/9 Rust Workspace + Ghidra", _lane_rust_and_ghidra),
        ("2/9 Oxlint + TypeScript", lambda e: _gate_typescript(e, capture_output=True)),
        ("3/9 Vitest Unit Suite", lambda e: _gate_vitest(e, capture_output=True)),
        ("4/9 Copy/Paste (jscpd 0%)", lambda e: _gate_jscpd(e, capture_output=True)),
        ("5/9 UI/UX State Lint", lambda e: _gate_uxlint(e, capture_output=True)),
        ("6/9 Playwright E2E", lambda e: _gate_playwright(e, capture_output=True)),
        ("8/9 C11 Firmware Host Tests", lambda e: _gate_c11_host(e, capture_output=True)),
        ("9a/9 Python Mypy Strict", lambda e: _gate_python_mypy(e, capture_output=True)),
        ("9b/9 Python Compile + Pytest", lambda e: _gate_python_compile_and_pytest(e, capture_output=True)),
    )
    worker_count = max(1, min(max_jobs, len(lanes)))
    print_banner(
        f"PANADIGA PARALLEL QUALITY GATES ({len(lanes)} lanes | max_workers={worker_count})"
    )

    tracker = ProcessTracker()

    def _timed_run(label: str, fn: Callable[[Mapping[str, str]], None]) -> tuple[str, float]:
        t0 = time.monotonic()
        with use_process_tracker(tracker):
            fn(env)
        return label, time.monotonic() - t0

    rust_lane_label = "1+7/9 Rust Workspace + Ghidra"
    pytest_lane_label = "9b/9 Python Compile + Pytest"
    initial_lanes = tuple((label, fn) for label, fn in lanes if label != pytest_lane_label)

    with ThreadPoolExecutor(max_workers=worker_count) as pool:
        future_map: dict[Future[tuple[str, float]], str] = {
            pool.submit(_timed_run, label, fn): label for label, fn in initial_lanes
        }
        pending: set[Future[tuple[str, float]]] = set(future_map)
        try:
            while pending:
                done, pending = wait(pending, return_when=FIRST_EXCEPTION)
                for fut in done:
                    label = future_map[fut]
                    try:
                        done_label, dt = fut.result()
                        print_status("PASS", f"{done_label} ({dt:.2f}s)", "32")
                        if done_label == rust_lane_label:
                            py_fut = pool.submit(
                                _timed_run,
                                pytest_lane_label,
                                lambda e: _gate_python_compile_and_pytest(e, capture_output=True),
                            )
                            future_map[py_fut] = pytest_lane_label
                            pending.add(py_fut)
                    except subprocess.CalledProcessError as exc:
                        for p in pending:
                            p.cancel()
                        tracker.terminate_all()
                        captured = exc.output or exc.stdout or exc.stderr or ""
                        print_status("FAIL", f"{label} failed (exit {exc.returncode})", "31")
                        if captured:
                            print(captured, flush=True)
                        raise
                    except Exception:
                        for p in pending:
                            p.cancel()
                        tracker.terminate_all()
                        print_status("FAIL", f"{label} failed", "31")
                        raise
        except KeyboardInterrupt:
            for p in pending:
                p.cancel()
            tracker.terminate_all()
            raise
