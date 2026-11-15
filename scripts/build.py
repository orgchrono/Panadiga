"""Unified cross-platform build DAG, CI pipeline, step cache, packaging, and TUI runner."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE in sys.path:
    sys.path.remove(_HERE)
_REPO_ROOT = os.path.dirname(_HERE)
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)

from scripts.build_lanes.android import build_android, detect_android_sdk
from scripts.build_lanes.dag import main, parse_args, run_build_pipeline
from scripts.build_lanes.desktop import (
    STALE_DIST_ITEMS,
    UF2_NAMES,
    build_desktop,
    build_frontend,
    export_ghidra_ssot,
    launch_gui,
    run_cli,
)
from scripts.build_lanes.pico import build_pico

__all__ = [
    "STALE_DIST_ITEMS",
    "UF2_NAMES",
    "build_android",
    "build_desktop",
    "build_frontend",
    "build_pico",
    "detect_android_sdk",
    "export_ghidra_ssot",
    "launch_gui",
    "main",
    "parse_args",
    "run_build_pipeline",
    "run_cli",
]

if __name__ == "__main__":
    raise SystemExit(main())
