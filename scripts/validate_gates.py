"""Individual Quality Gate runners and parallel DAG scheduler for Panadiga."""

from __future__ import annotations

from concurrent.futures import FIRST_EXCEPTION, Future, ThreadPoolExecutor, wait
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import time
from typing import Callable, Final, Mapping

from scripts.panadiga_ops import (
    ProcessTracker,
    ROOT,
    load_msvc_env,
    panadiga_binary_name,
    print_banner,
    print_status,
    run_step,
    sha256_file,
    use_process_tracker,
)
from scripts.validate_c11 import C11_HOST_SOURCES as C11_HOST_SOURCES, _gate_c11_host as _gate_c11_host

DESKTOP_DIR: Final[Path] = ROOT / "apps" / "desktop"


def _discover_python_compile_targets() -> tuple[str, ...]:
    targets: list[str] = []
    for base in (ROOT / "tools" / "ghidra", ROOT / "scripts"):
        for path in sorted(base.rglob("*.py")):
            if "__pycache__" in path.parts or "tests" in path.parts:
                continue
            targets.append(path.relative_to(ROOT).as_posix())
    return tuple(targets)


PYTHON_COMPILE_TARGETS: Final[tuple[str, ...]] = _discover_python_compile_targets()


def _gate_rust(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("1/9. Validating Rust Workspace (Cargo Fmt, Clippy -D warnings & Tests)")
    run_step(["cargo", "fmt", "--all", "--", "--check"], cwd=ROOT, env=env, capture_output=capture_output)
    run_step(["cargo", "clippy", "--workspace", "--all-targets", "--no-deps", "--", "-D", "warnings"], cwd=ROOT, env=env, capture_output=capture_output)
    run_step(
        ["cargo", "test", "--workspace", "--bins", "--lib", "--tests"],
        cwd=ROOT,
        env=env,
        capture_output=capture_output,
    )
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


def _snapshot_generated_ssot_hashes(root: Path) -> dict[str, str]:
    candidates: list[Path] = [
        root / "firmware" / "picofw" / "generated" / "picofw_ssot.h",
        root / "firmware" / "picofw" / "generated" / "picofw_ssot.c",
        root / "firmware" / "picofw" / "generated" / "picofw_ssot_tables.c",
        root / "firmware" / "picofw" / "uart" / "picofw_pana_rsp_ssot.h",
        root / "firmware" / "digaos" / "generated" / "digaos_ssot.h",
        root / "firmware" / "digaos" / "generated" / "digaos_ssot.c",
        root / "firmware" / "digaos" / "generated" / "digaos_ssot_tables.c",
        root / "apps" / "desktop" / "src" / "models" / "generated" / "ssot.ts",
        root / "apps" / "desktop" / "src" / "models" / "generated" / "profiles.ts",
        root / "apps" / "desktop" / "src" / "models" / "generated" / "digaos.ts",
        root / "scripts" / "profiles_types.py",
        *sorted((root / "profiles" / "schema").glob("*.json")),
        *sorted((root / "tools" / "ghidra").glob("*.py")),
        root / "tools" / "ghidra" / "ghidra_ssot_maps.json",
    ]
    return {
        p.relative_to(root).as_posix(): sha256_file(p)
        for p in candidates
        if p.is_file()
    }


def _gate_ghidra(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("7/9. Validating SSOT & Ghidra Native Generator (panadiga ssot / ghidra)")
    ghidra_dir = str(ROOT / "tools" / "ghidra")
    target_dir = Path(env.get("CARGO_TARGET_DIR") or str(ROOT / "target"))
    cli_bin = target_dir / "debug" / panadiga_binary_name()
    prefix = [str(cli_bin)] if cli_bin.is_file() else ["cargo", "run", "-p", "panadiga-cli", "--"]
    before = _snapshot_generated_ssot_hashes(ROOT)
    run_step(
        [*prefix, "ssot", "export-all", "--root", str(ROOT)],
        cwd=ROOT,
        env=env,
        capture_output=capture_output,
    )
    run_step(
        [*prefix, "ghidra", "export-all", "--dir", ghidra_dir],
        cwd=ROOT,
        env=env,
        capture_output=capture_output,
    )
    after = _snapshot_generated_ssot_hashes(ROOT)
    drifted = sorted(
        key for key in (set(before) | set(after)) if before.get(key) != after.get(key)
    )
    if drifted:
        raise RuntimeError(
            "Gate 7 SSOT/Ghidra zero-drift violation in: " + ", ".join(drifted)
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


def _resolve_pyghidra_python(env: Mapping[str, str], ghidra_home: Path) -> str:
    explicit = (env.get("PANADIGA_GHIDRA_PYTHON") or "").strip()
    if explicit:
        explicit_path = Path(explicit)
        if not explicit_path.is_file():
            raise FileNotFoundError(f"PANADIGA_GHIDRA_PYTHON points to missing interpreter: {explicit_path}")
        return str(explicit_path)
    if importlib.util.find_spec("pyghidra") is not None:
        return sys.executable
    candidates = (
        ghidra_home.parent.parent / ".venv" / "Scripts" / "python.exe",
        ghidra_home.parent / ".venv" / "Scripts" / "python.exe",
        Path(r"C:\ghidra\.venv\Scripts\python.exe"),
        ghidra_home.parent.parent / ".venv" / "bin" / "python",
        ghidra_home.parent / ".venv" / "bin" / "python",
    )
    for cand in candidates:
        if cand.is_file():
            return str(cand)
    raise FileNotFoundError(
        "No Python interpreter with `pyghidra` found for --ghidra-smoke; "
        "set PANADIGA_GHIDRA_PYTHON or install pyghidra in the active interpreter"
    )


def _gate_ghidra_smoke(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    if not capture_output:
        print_banner("10/9. Running Opt-In PyGhidra Headless Smoke + TrapAudit (MN103S Traps)")
    raw_home = (env.get("PANADIGA_GHIDRA_PATCHED") or env.get("GHIDRA_INSTALL_DIR") or "").strip()
    if not raw_home:
        raise FileNotFoundError(
            "PANADIGA_GHIDRA_PATCHED (or GHIDRA_INSTALL_DIR) must be set for --ghidra-smoke"
        )
    ghidra_home = Path(raw_home)
    if not ghidra_home.is_dir():
        raise FileNotFoundError(
            f"PANADIGA_GHIDRA_PATCHED (or GHIDRA_INSTALL_DIR) directory does not exist: {ghidra_home}"
        )
    py_exe = _resolve_pyghidra_python(env, ghidra_home)
    run_step(
        [
            py_exe,
            "-m",
            "scripts.re.ghidra_fleet_orchestrator",
            "--ghidra-smoke",
            "--ghidra-home",
            str(ghidra_home),
        ],
        cwd=ROOT,
        env=env,
        capture_output=capture_output,
    )
    if not capture_output:
        print_status("PASS", "PyGhidra MN103S synthetic Traps #1-#8 smoke + Java TrapAudit passed!", "32")


def run_sequential_gates(env: Mapping[str, str], *, ghidra_smoke: bool = False) -> None:
    """Execute all 9 Quality Gates (plus optional Ghidra smoke) sequentially."""
    gates: list[Callable[[Mapping[str, str]], None]] = [
        _gate_rust,
        _gate_typescript,
        _gate_vitest,
        _gate_jscpd,
        _gate_uxlint,
        _gate_playwright,
        _gate_ghidra,
        _gate_c11_host,
        _gate_python,
    ]
    if ghidra_smoke:
        gates.append(_gate_ghidra_smoke)
    for gate in gates:
        gate(env)


def run_parallel_lanes(env: Mapping[str, str], max_jobs: int, *, ghidra_smoke: bool = False) -> None:
    """Execute all 9 Quality Gates via thread-pool DAG without SSOT races or Vite starvation."""
    ghidra_lane_label = "7/9 SSOT + Ghidra Native Export"
    playwright_lane_label = "6/9 Playwright E2E"
    pytest_lane_label = "9b/9 Python Compile + Pytest"
    initial_list: list[tuple[str, Callable[[Mapping[str, str]], None]]] = [
        ("1/9 Rust Workspace", lambda e: _gate_rust(e, capture_output=True)),
        ("2/9 Oxlint + TypeScript", lambda e: _gate_typescript(e, capture_output=True)),
        ("3/9 Vitest Unit Suite", lambda e: _gate_vitest(e, capture_output=True)),
        ("4/9 Copy/Paste (jscpd 0%)", lambda e: _gate_jscpd(e, capture_output=True)),
        ("5/9 UI/UX State Lint", lambda e: _gate_uxlint(e, capture_output=True)),
        ("8/9 C11 Firmware Host Tests", lambda _e: _gate_c11_host(c11_env, capture_output=True)),
        ("9a/9 Python Mypy Strict", lambda e: _gate_python_mypy(e, capture_output=True)),
    ]
    if ghidra_smoke:
        initial_list.append(("10/9 Ghidra Headless Smoke", lambda e: _gate_ghidra_smoke(e, capture_output=True)))
    initial_lanes = tuple(initial_list)
    total_lanes = len(initial_lanes) + 3
    worker_count = max(1, min(max_jobs, total_lanes))
    print_banner(
        f"PANADIGA PARALLEL QUALITY GATES ({total_lanes} lanes | max_workers={worker_count})"
    )

    tracker = ProcessTracker()

    def _timed_run(label: str, fn: Callable[[Mapping[str, str]], None]) -> tuple[str, float]:
        t0 = time.monotonic()
        with use_process_tracker(tracker):
            fn(env)
        return label, time.monotonic() - t0

    c11_env: Mapping[str, str] = load_msvc_env(env)
    ghidra_submitted = False
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
                        if done_label == ghidra_lane_label:
                            post_ssot_lanes: tuple[tuple[str, Callable[[Mapping[str, str]], None]], ...] = (
                                (playwright_lane_label, lambda e: _gate_playwright(e, capture_output=True)),
                                (pytest_lane_label, lambda e: _gate_python_compile_and_pytest(e, capture_output=True)),
                            )
                            for post_label, post_fn in post_ssot_lanes:
                                post_fut = pool.submit(_timed_run, post_label, post_fn)
                                future_map[post_fut] = post_label
                                pending.add(post_fut)
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
                if not pending and not ghidra_submitted:
                    ghidra_submitted = True
                    # Run SSOT + Ghidra regeneration only after all reader lanes finish cleanly.
                    gh_fut = pool.submit(
                        _timed_run,
                        ghidra_lane_label,
                        lambda e: _gate_ghidra(e, capture_output=True),
                    )
                    future_map[gh_fut] = ghidra_lane_label
                    pending.add(gh_fut)
        except KeyboardInterrupt:
            for p in pending:
                p.cancel()
            tracker.terminate_all()
            raise
