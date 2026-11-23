"""Thin skill entrypoint forwarding arsenal sync/check to `scripts.generate_tool_arsenal.main`."""

from __future__ import annotations

import sys

from scripts.generate_tool_arsenal import main

if __name__ == "__main__":
    sys.exit(main())
