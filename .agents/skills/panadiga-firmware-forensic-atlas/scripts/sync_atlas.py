"""Thin wrapper forwarding Ghidra forensic atlas sync/check to `scripts.generate_firmware_atlas.main`."""

from __future__ import annotations

import sys

from scripts.generate_firmware_atlas import main

if __name__ == "__main__":
    sys.exit(main())
