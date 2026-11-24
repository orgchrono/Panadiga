"""Pure-functional SSOT, zero-panic, unsafe-policy, FP, and blob hygiene Quality Gates."""

from __future__ import annotations

import os
import sys

if __package__ in (None, ""):
    _here_val = os.path.dirname(os.path.abspath(__file__))
    sys.path = [os.path.dirname(_here_val), *(p for p in sys.path if os.path.abspath(p or ".") != _here_val)]

import argparse
from collections.abc import Callable, Iterator, Sequence
import json
from pathlib import Path
import re
import subprocess
from typing import Final

PROD_PANIC_MACRO_ALLOWLIST: Final[frozenset[str]] = frozenset(
    {
        "crates/panadiga-core/src/ssot/pico_header/tables_gen.rs",
        "crates/panadiga-core/src/ssot/spec/commands.rs",
        "crates/panadiga-core/src/ssot/spec/hardware.rs",
    }
)

FORBID_UNSAFE_CRATE_FILES: Final[tuple[str, ...]] = (
    "crates/panadiga-bench/src/lib.rs",
    "crates/panadiga-vault/src/lib.rs",
    "crates/panadiga-crypto/src/lib.rs",
    "crates/panadiga-ir/src/lib.rs",
    "crates/panadiga-profiles/src/lib.rs",
    "crates/panadiga-rag/src/lib.rs",
    "crates/panadiga-mcp/src/lib.rs",
    "crates/panadiga-cli/src/lib.rs",
    "crates/panadiga-cli/src/main.rs",
    "apps/desktop/src-tauri/src/lib.rs",
    "apps/desktop/src-tauri/src/main.rs",
)

DENY_UNSAFE_CRATE_FILES: Final[tuple[str, ...]] = (
    "crates/panadiga-constants/src/lib.rs",
    "crates/panadiga-core/src/lib.rs",
)

LARGE_BLOB_ALLOWLIST_PREFIXES: Final[tuple[str, ...]] = (
    "manuals_and_schematics/",
    "firmware_and_images/",
    "docs/pseudocode/raw/",
    "docs/assets/images/re_dmr_eh55/",
    "profiles/symbols/",
)

TS_OOP_CLASS_ALLOWLIST: Final[frozenset[str]] = frozenset(
    {
        "apps/desktop/src/components/ui/error-boundary.tsx",
    }
)

REQUIRED_GOVERNANCE_FILES: Final[tuple[str, ...]] = (
    "rust-toolchain.toml",
    "rustfmt.toml",
    "clippy.toml",
    "deny.toml",
    "pyproject.toml",
    "SECURITY.md",
    ".github/CODEOWNERS",
    "NOTICE",
    "THIRD_PARTY_LICENSES.md",
)

_CFG_TEST_RE: Final[re.Pattern[str]] = re.compile(r"^\s*#\[cfg\(test\)\]")
_UNWRAP_RE: Final[re.Pattern[str]] = re.compile(r"\.unwrap\s*\(")
_PANIC_MACRO_RE: Final[re.Pattern[str]] = re.compile(
    r"\b(?:pan" + r"ic!|to" + r"do!|unimplem" + r"ented!)\s*\("
)
_TS_CLASS_RE: Final[re.Pattern[str]] = re.compile(
    r"^\s*(?:export\s+(?:default\s+)?)?(?:abstract\s+)?class\s+[A-Za-z_]\w*"
)
_PY_UNSAFE_RE: Final[re.Pattern[str]] = re.compile(
    r"\b(?:ev"
    + r"al|ex"
    + r"ec)\s*\(|\bpickle\.loa"
    + r"ds?\s*\(|\bshel"
    + r"l\s*=\s*True\b"
)


def _iter_prod_rust_code_lines(root: Path) -> Iterator[tuple[str, int, str]]:
    search_roots = [*sorted((root / "crates").glob("*/src")), root / "apps" / "desktop" / "src-tauri" / "src"]
    rs_files = sorted(p for base in search_roots if base.is_dir() for p in base.rglob("*.rs"))
    for path in rs_files:
        rel = path.relative_to(root).as_posix()
        if "/tests/" in rel or path.name.endswith("tests.rs") or "test_" in path.name:
            continue
        lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
        for line_no, raw_line in enumerate(lines, 1):
            if _CFG_TEST_RE.match(raw_line):
                break
            code = raw_line.split("//", 1)[0]
            if code.strip():
                yield rel, line_no, code


def _scan_prod_rust(
    root: Path,
    matcher: Callable[[str], bool],
    label: str,
    allowlist: frozenset[str] = frozenset(),
) -> tuple[str, ...]:
    hits = [
        f"{rel}:{line_no}: {label}: {code.strip()}"
        for rel, line_no, code in _iter_prod_rust_code_lines(root)
        if rel not in allowlist and matcher(code)
    ]
    return tuple(hits)


def check_prod_rust_zero_unwrap(root: Path) -> tuple[str, ...]:
    """Fail if any production Rust source outside tests contains `.unwrap()`."""
    return _scan_prod_rust(root, lambda code: _UNWRAP_RE.search(code) is not None, "forbidden .unwrap()")


def check_prod_rust_zero_panic_macros(root: Path) -> tuple[str, ...]:
    """Fail if production Rust code outside SSOT compile-time generators uses panic/todo/unimplemented."""
    return _scan_prod_rust(
        root,
        lambda code: _PANIC_MACRO_RE.search(code) is not None,
        "forbidden panic/todo/unimplemented macro",
        allowlist=PROD_PANIC_MACRO_ALLOWLIST,
    )


def _check_attr_group(root: Path, paths: Sequence[str], attr: str) -> list[str]:
    violations: list[str] = []
    for rel in paths:
        target = root / rel
        if not target.is_file():
            violations.append(f"{rel}: missing crate root/entrypoint for {attr}")
            continue
        head = "\n".join(target.read_text(encoding="utf-8", errors="ignore").splitlines()[:25])
        if attr not in head:
            violations.append(f"{rel}: missing `{attr}` in header")
    return violations


def check_crate_unsafe_code_policy(root: Path) -> tuple[str, ...]:
    """Verify pure crates enforce `#![forbid(unsafe_code)]` and OS/mmap crates `#![deny(unsafe_code)]`."""
    violations = _check_attr_group(root, FORBID_UNSAFE_CRATE_FILES, "#![forbid(unsafe_code)]")
    violations.extend(_check_attr_group(root, DENY_UNSAFE_CRATE_FILES, "#![deny(unsafe_code)]"))
    return tuple(violations)


def _git_tracked_files(root: Path) -> tuple[str, ...]:
    proc = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    raw_items = proc.stdout.decode("utf-8", errors="replace").split("\0")
    return tuple(item for item in raw_items if item)


def check_tracked_large_blobs(
    root: Path,
    max_bytes: int = 1_048_576,
    tracked_files: Sequence[str] | None = None,
) -> tuple[str, ...]:
    """Reject tracked blobs > 1 MiB outside explicitly audited evidence/asset prefixes."""
    entries = _git_tracked_files(root) if tracked_files is None else tracked_files
    violations: list[str] = []
    for rel in entries:
        if rel.startswith(LARGE_BLOB_ALLOWLIST_PREFIXES):
            continue
        path = root / rel
        if path.is_file():
            size = path.stat().st_size
            if size > max_bytes:
                violations.append(f"{rel}: tracked blob {size} bytes exceeds {max_bytes}-byte limit")
    return tuple(violations)


def check_ts_no_oop_classes(root: Path) -> tuple[str, ...]:
    """Disallow OOP class declarations in TypeScript source outside ErrorBoundary."""
    ts_root = root / "apps" / "desktop" / "src"
    if not ts_root.is_dir():
        return ()
    ts_files = sorted([*ts_root.rglob("*.ts"), *ts_root.rglob("*.tsx")])
    violations: list[str] = []
    for path in ts_files:
        rel_sub = path.relative_to(ts_root)
        if ".test." in path.name or "test" in rel_sub.parts:
            continue
        rel = path.relative_to(root).as_posix()
        if rel in TS_OOP_CLASS_ALLOWLIST:
            continue
        for line_no, raw_line in enumerate(path.read_text(encoding="utf-8", errors="ignore").splitlines(), 1):
            code = raw_line.split("//", 1)[0]
            if _TS_CLASS_RE.match(code):
                violations.append(f"{rel}:{line_no}: forbidden OOP class declaration")
    return tuple(violations)


def check_python_no_unsafe_exec(root: Path) -> tuple[str, ...]:
    """Disallow unsafe dynamic execution/deserialization or shell subprocess mode in scripts/."""
    scripts_dir = root / "scripts"
    if not scripts_dir.is_dir():
        return ()
    violations: list[str] = []
    for path in sorted(scripts_dir.rglob("*.py")):
        rel_parts = path.relative_to(scripts_dir).parts
        if "tests" in rel_parts or "__pycache__" in rel_parts:
            continue
        rel = path.relative_to(root).as_posix()
        for line_no, raw_line in enumerate(path.read_text(encoding="utf-8", errors="ignore").splitlines(), 1):
            code = raw_line.split("#", 1)[0]
            if _PY_UNSAFE_RE.search(code):
                violations.append(f"{rel}:{line_no}: forbidden unsafe Python construct")
    return tuple(violations)


def check_governance_manifests_present(root: Path) -> tuple[str, ...]:
    """Verify pinned toolchains, supply-chain policy, and security governance docs exist."""
    return tuple(
        f"{rel}: missing required governance manifest"
        for rel in REQUIRED_GOVERNANCE_FILES
        if not (root / rel).is_file()
    )


def run_ssot_and_fp_audit(root: Path) -> dict[str, tuple[str, ...]]:
    """Run all deterministic hygiene, SSOT/FP, zero-unwrap, and blob guards against `root`."""
    return {
        "prod_rust_zero_unwrap": check_prod_rust_zero_unwrap(root),
        "prod_rust_zero_panic_macros": check_prod_rust_zero_panic_macros(root),
        "crate_unsafe_code_policy": check_crate_unsafe_code_policy(root),
        "tracked_large_blobs": check_tracked_large_blobs(root),
        "ts_no_oop_classes": check_ts_no_oop_classes(root),
        "python_no_unsafe_exec": check_python_no_unsafe_exec(root),
        "governance_manifests": check_governance_manifests_present(root),
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Validate Panadiga SSOT, FP, zero-panic, and blob gates.")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--check", action="store_true", help="Exit non-zero if any violation is found.")
    parser.add_argument("--json", action="store_true", help="Emit structured JSON report.")
    args = parser.parse_args(list(argv) if argv is not None else None)

    report = run_ssot_and_fp_audit(args.root.resolve())
    total_violations = sum(len(items) for items in report.values())
    if args.json:
        payload = {
            "status": "PASS" if total_violations == 0 else "FAIL",
            "total_violations": total_violations,
            "checks": {k: list(v) for k, v in report.items()},
        }
        print(json.dumps(payload, indent=2))
    else:
        for check_name, items in report.items():
            status = "PASS" if not items else f"FAIL ({len(items)})"
            print(f"[{status}] {check_name}")
            for entry in items:
                print(f"  - {entry}")

    return 1 if (args.check and total_violations > 0) else 0


if __name__ == "__main__":
    sys.exit(main())
