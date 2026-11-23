"""Thin wrapper forwarding repo sanitization CLI to `scripts.repo_sanitize_and_sync.main`."""

from __future__ import annotations

import sys

from scripts.repo_sanitize_and_sync import main

if __name__ == "__main__":
    sys.exit(main())
