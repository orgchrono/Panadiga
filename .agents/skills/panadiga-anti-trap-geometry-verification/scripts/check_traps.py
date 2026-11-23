"""Thin wrapper delegating anti-trap verification to `scripts.verify_geometry_traps.main`."""

from __future__ import annotations

import sys

from scripts.verify_geometry_traps import main

if __name__ == "__main__":
    sys.exit(main())
