"""Thin skill wrapper delegating to `scripts.pico_heartbeat_ops.main`."""

from __future__ import annotations

import sys

from scripts.pico_heartbeat_ops import main

if __name__ == "__main__":
    sys.exit(main())
