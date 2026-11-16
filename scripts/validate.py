"""Panadiga universal 9 Quality Gates validation suite (Windows, Linux, macOS)."""

from __future__ import annotations

import os
import sys

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path = [os.path.dirname(_SCRIPT_DIR), *(p for p in sys.path if os.path.abspath(p or ".") != _SCRIPT_DIR)]

import argparse
import subprocess
import time
from typing import Mapping

from scripts.panadiga_ops import build_env, print_banner, print_status
from scripts.validate_gates import (
    C11_HOST_SOURCES,
    DESKTOP_DIR,
    PYTHON_COMPILE_TARGETS,
    run_parallel_lanes,
    run_sequential_gates,
)

__all__ = [
    "C11_HOST_SOURCES",
    "DESKTOP_DIR",
    "PYTHON_COMPILE_TARGETS",
    "main",
    "parse_args",
    "run_validation",
]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Panadiga 9 Quality Gates runner.")
    parser.add_argument(
        "--sequential",
        action="store_true",
        help="Run all 9 quality gates sequentially with live unbuffered logs.",
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=max(2, min(9, os.cpu_count() or 4)),
        help="Maximum parallel validation lanes (default: min(9, cpu_count)).",
    )
    return parser.parse_args(argv)


def run_validation(
    env: Mapping[str, str],
    *,
    sequential: bool = False,
    max_jobs: int | None = None,
) -> float:
    """Execute all 9 Quality Gates and return total elapsed seconds."""
    start = time.monotonic()
    jobs = max_jobs if max_jobs is not None else max(2, min(9, os.cpu_count() or 4))
    if sequential:
        run_sequential_gates(env)
    else:
        run_parallel_lanes(env, jobs)

    elapsed = time.monotonic() - start
    print_banner(
        f"ALL 9 COMMIT QUALITY GATES PASSED! (Elapsed: {elapsed:.1f}s)\n"
        "Repository is in pristine condition for git commit & push.",
        ansi_color="32",
    )
    return elapsed


def main(argv: list[str] | None = None) -> int:
    """Execute all 9 mandatory Quality Gates (parallel DAG by default)."""
    args = parse_args(argv)
    env = build_env()
    try:
        run_validation(env, sequential=args.sequential, max_jobs=args.jobs)
    except (subprocess.CalledProcessError, RuntimeError, FileNotFoundError) as exc:
        print_status("FAIL", f"Validation gate failed: {exc}", "31")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
