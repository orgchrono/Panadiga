"""Repository sanitization audit, deterministic SSOT sync, and read-only commit wave planner."""

from __future__ import annotations

import os
import sys

if __package__ in (None, ""):
    _here_sync = os.path.dirname(os.path.abspath(__file__))
    sys.path[:] = [os.path.dirname(_here_sync), *(x for x in sys.path if os.path.abspath(x or ".") != _here_sync)]

import shlex
import subprocess
import argparse
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Final, Sequence

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, resolve_panadiga_cli_prefix
from scripts.panadiga_ops import ROOT, build_env
from scripts.pico_heartbeat_ops import emit_json_payload

KEYSTORE_SUFFIXES: Final[frozenset[str]] = frozenset({".jks", ".keystore", ".p12", ".pem"})
TEMP_SUFFIXES: Final[frozenset[str]] = frozenset({".tmp", ".bak", ".orig"})
BINARY_SUFFIXES: Final[frozenset[str]] = frozenset(
    {".bin", ".frm", ".img", ".iso", ".uf2", ".apk", ".exe", ".dll", ".so"}
)
SKIP_WALK_DIRS: Final[frozenset[str]] = frozenset(
    {
        ".git",
        "node_modules",
        "target",
        "dist",
        "dist-android",
        "manuals_and_schematics",
        "firmware_and_images",
        "__pycache__",
        ".jscpd-report",
    }
)
MAX_UNTRACKED_BIN_BYTES: Final[int] = 1_048_576
WAVE_DEFS: Final[tuple[tuple[str, str], ...]] = (
    ("wave-1", "chore(contracts)"),
    ("wave-2", "refactor(core)"),
    ("wave-3", "feat(cli-mcp-rag)"),
    ("wave-4", "fix(desktop-fw)"),
    ("wave-5", "feat(scripts)"),
    ("wave-6", "docs(agents)"),
)

StepRunner = Callable[[Sequence[str], Path], tuple[int, str, str]]
StatusReader = Callable[[Path], list[str]]
ArsenalChecker = Callable[[Path], bool]


@dataclass(frozen=True)
class ArtifactFinding:
    """Prohibited keystore, stray temp artifact, or oversized untracked binary."""

    path: str
    reason: str
    size_bytes: int


@dataclass(frozen=True)
class GodModuleFinding:
    """Source file exceeding soft or hard LOC ceiling."""

    path: str
    loc: int
    severity: str


def default_step_runner(cmd: Sequence[str], cwd: Path) -> tuple[int, str, str]:
    """Run a non-git sync/check subprocess and capture exit code and logs."""
    proc = subprocess.run(
        list(cmd),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=str(cwd),
        env=build_env(),
        check=False,
    )
    return proc.returncode, proc.stdout, proc.stderr


def read_git_porcelain_lines(root: Path) -> list[str]:
    """Read `git status --porcelain=v1 -uall` entries cleanly without mutating git state."""
    if not (root / ".git").exists():
        return []
    proc = subprocess.run(
        ["git", "status", "--porcelain=v1", "-uall"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=str(root),
        check=False,
    )
    return [line.rstrip("\r\n") for line in proc.stdout.splitlines() if line.strip()]


def parse_porcelain_path(line: str) -> tuple[str, str]:
    """Parse `(status_code, normalized_posix_path)` from one `git status --porcelain=v1` line."""
    status = line[:2]
    raw_path = line[3:].strip() if len(line) > 3 else ""
    if " -> " in raw_path:
        raw_path = raw_path.split(" -> ", 1)[1].strip()
    if raw_path.startswith('"') and raw_path.endswith('"') and len(raw_path) >= 2:
        raw_path = raw_path[1:-1]
    return status, raw_path.replace("\\", "/")


def _list_repo_candidates(root: Path) -> tuple[list[tuple[Path, str, bool]], list[str]]:
    porcelain = read_git_porcelain_lines(root)
    untracked: set[str] = set()
    dirty_paths: list[str] = []
    for line in porcelain:
        status, rel = parse_porcelain_path(line)
        if rel:
            dirty_paths.append(rel)
            if status == "??":
                untracked.add(rel)

    items: list[tuple[Path, str, bool]] = []
    seen: set[str] = set()
    if (root / ".git").exists():
        proc = subprocess.run(
            ["git", "ls-files"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            cwd=str(root),
            check=False,
        )
        for rel in proc.stdout.splitlines():
            norm = rel.strip().replace("\\", "/")
            if norm and norm not in seen:
                seen.add(norm)
                items.append((root / norm, norm, False))
        for norm in sorted(untracked):
            if norm not in seen:
                seen.add(norm)
                items.append((root / norm, norm, True))
        return items, dirty_paths

    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in SKIP_WALK_DIRS)
        for fname in sorted(filenames):
            full = Path(dirpath) / fname
            norm = full.relative_to(root).as_posix()
            items.append((full, norm, True))
    return items, sorted(norm for _, norm, _ in items)


def scan_prohibited_artifacts(root: Path) -> list[ArtifactFinding]:
    """Flag keystores, certificates, stray temp backups, and large untracked binaries."""
    items, _ = _list_repo_candidates(root)
    findings: list[ArtifactFinding] = []
    for full, rel, is_untracked in items:
        if not full.is_file():
            continue
        suffix = full.suffix.lower()
        size = full.stat().st_size
        if suffix in KEYSTORE_SUFFIXES:
            findings.append(ArtifactFinding(rel, f"prohibited_secret_or_keystore({suffix})", size))
        elif suffix in TEMP_SUFFIXES:
            findings.append(ArtifactFinding(rel, f"stray_temp_artifact({suffix})", size))
        elif is_untracked and suffix in BINARY_SUFFIXES and size > MAX_UNTRACKED_BIN_BYTES:
            findings.append(ArtifactFinding(rel, "large_untracked_binary(>1MiB)", size))
    return findings


def _is_code_audit_target(rel: str) -> bool:
    if not (
        (rel.startswith("crates/") and rel.endswith(".rs"))
        or (rel.startswith("apps/desktop/src-tauri/src/") and rel.endswith(".rs"))
        or (rel.startswith("scripts/") and rel.endswith(".py"))
    ):
        return False
    parts = set(rel.split("/"))
    if parts & {"generated", "tests", "target", "__pycache__"}:
        return False
    name = rel.rsplit("/", 1)[-1]
    return not (name in {"tests.rs", "__init__.py"} or name.endswith("_tests.rs"))


def scan_god_modules(
    root: Path,
    *,
    max_loc_warn: int = 450,
    max_loc_fail: int = 650,
) -> list[GodModuleFinding]:
    """Identify oversized non-generated Rust/Python source modules."""
    items, _ = _list_repo_candidates(root)
    findings: list[GodModuleFinding] = []
    for full, rel, _ in items:
        if not full.is_file() or not _is_code_audit_target(rel):
            continue
        loc = len(full.read_text(encoding="utf-8", errors="replace").splitlines())
        if loc >= max_loc_fail:
            findings.append(GodModuleFinding(rel, loc, "fail"))
        elif loc >= max_loc_warn:
            findings.append(GodModuleFinding(rel, loc, "warn"))
    findings.sort(key=lambda item: (-item.loc, item.path))
    return findings


def _run_optional_checks(root: Path) -> dict[str, bool]:
    results: dict[str, bool] = {}
    for key, rel in (
        ("arsenal_in_sync", "scripts/generate_tool_arsenal.py"),
        ("atlas_in_sync", "scripts/generate_firmware_atlas.py"),
        ("traps_valid", "scripts/verify_geometry_traps.py"),
    ):
        script = root / rel
        if not script.is_file():
            results[key] = True
            continue
        rc, _, _ = default_step_runner([sys.executable, str(script), "--check"], root)
        results[key] = rc == 0
    return results


def _default_arsenal_checker(root: Path) -> bool:
    return all(_run_optional_checks(root).values())


def run_check(
    *,
    root: Path = ROOT,
    max_loc_warn: int = 450,
    max_loc_fail: int = 650,
    arsenal_checker: ArsenalChecker = _default_arsenal_checker,
) -> tuple[int, JsonDict]:
    """Run read-only hygiene, god-module LOC, arsenal/atlas drift, and anti-trap audit."""
    artifacts = scan_prohibited_artifacts(root)
    gods = scan_god_modules(root, max_loc_warn=max_loc_warn, max_loc_fail=max_loc_fail)
    sync_checks = (
        _run_optional_checks(root)
        if arsenal_checker is _default_arsenal_checker
        else {"arsenal_in_sync": arsenal_checker(root), "atlas_in_sync": True, "traps_valid": True}
    )
    all_checks_ok = all(sync_checks.values())
    _, dirty_paths = _list_repo_candidates(root)
    has_fail_god = any(g.severity == "fail" for g in gods)
    has_warn_god = any(g.severity == "warn" for g in gods)

    if artifacts or has_fail_god or not all_checks_ok:
        status = "FAIL"
        exit_code = 1
    elif has_warn_god:
        status = "WARN"
        exit_code = 0
    else:
        status = "PASS"
        exit_code = 0

    payload: JsonDict = {
        "mode": "check",
        "status": status,
        "arsenal_in_sync": sync_checks["arsenal_in_sync"],
        "atlas_in_sync": sync_checks["atlas_in_sync"],
        "traps_valid": sync_checks["traps_valid"],
        "dirty_file_count": len(dirty_paths),
        "prohibited_artifacts": [asdict(a) for a in artifacts],
        "god_modules": [asdict(g) for g in gods],
    }
    return exit_code, payload


def run_sync(
    *,
    root: Path = ROOT,
    dry_run: bool = False,
    step_runner: StepRunner = default_step_runner,
) -> tuple[int, JsonDict]:
    """Synchronize SSOT codegen projections, Ghidra scripts, tool arsenal, and forensic atlas."""
    prefix = list(resolve_panadiga_cli_prefix())
    commands: list[list[str]] = [
        [*prefix, "ssot", "export-all", "--root", str(root)],
        [*prefix, "ghidra", "export-all", "--dir", str(root / "tools" / "ghidra")],
    ]
    for rel in ("scripts/generate_tool_arsenal.py", "scripts/generate_firmware_atlas.py"):
        script_path = root / rel
        if script_path.is_file():
            commands.append([sys.executable, str(script_path), "--write"])

    step_results: list[JsonValue] = []
    for cmd in commands:
        cmd_json: list[JsonValue] = [str(part) for part in cmd]
        if dry_run:
            step_results.append({"command": cmd_json, "status": "DRY_RUN"})
            continue
        rc, out, err = step_runner(cmd, root)
        step_results.append(
            {
                "command": cmd_json,
                "returncode": rc,
                "stdout": out.strip(),
                "stderr": err.strip(),
            }
        )
        if rc != 0:
            return rc, {"mode": "sync", "status": "FAILED", "dry_run": dry_run, "steps": step_results}

    return 0, {"mode": "sync", "status": "SYNCED", "dry_run": dry_run, "steps": step_results}


def _classify_wave(path: str) -> str:
    norm = path.replace("\\", "/").removeprefix("./")
    if norm in {"Cargo.toml", "Cargo.lock", ".jscpd.json", ".gitignore", "package.json", "package-lock.json"} or norm.startswith(
        (
            "crates/panadiga-constants/",
            "crates/panadiga-profiles/",
            "profiles/",
            "crates/panadiga-core/src/ssot/",
        )
    ):
        return "wave-1"
    if norm.startswith(
        (
            "crates/panadiga-bench/",
            "crates/panadiga-vault/",
            "crates/panadiga-crypto/",
            "crates/panadiga-ir/",
            "crates/panadiga-core/",
        )
    ):
        return "wave-2"
    if norm.startswith(("crates/panadiga-cli/", "crates/panadiga-mcp/", "crates/panadiga-rag/")):
        return "wave-3"
    if norm.startswith(("apps/desktop/", "firmware/", "tools/", "packages/")):
        return "wave-4"
    if norm.startswith("scripts/"):
        return "wave-5"
    if norm.startswith((".agents/", "docs/", ".github/")) or ("/" not in norm and norm.endswith(".md")):
        return "wave-6"
    return "unclassified"


def build_commit_plan(
    *,
    root: Path = ROOT,
    exclude_paths: Sequence[str] = (),
    status_reader: StatusReader = read_git_porcelain_lines,
) -> tuple[int, JsonDict]:
    """Generate a deterministic, read-only 6-wave atomic commit plan without mutating git."""
    normalized_excludes = tuple(p.replace("\\", "/").rstrip("/") for p in exclude_paths if p.strip())
    buckets: dict[str, list[str]] = {wave_id: [] for wave_id, _ in WAVE_DEFS}
    excluded_concurrent: list[str] = []
    unclassified: list[str] = []

    for line in status_reader(root):
        _, rel = parse_porcelain_path(line)
        if not rel:
            continue
        if any(rel == ex or rel.startswith(f"{ex}/") for ex in normalized_excludes):
            excluded_concurrent.append(rel)
            continue
        wave_id = _classify_wave(rel)
        if wave_id in buckets:
            buckets[wave_id].append(rel)
        else:
            unclassified.append(rel)

    waves_out: list[JsonValue] = []
    for wave_id, message in WAVE_DEFS:
        files = sorted(set(buckets[wave_id]))
        if not files:
            continue
        quoted = " ".join(shlex.quote(f) for f in files)
        waves_out.append(
            {
                "wave": wave_id,
                "commit_message": message,
                "file_count": len(files),
                "files": list(files),
                "suggested_commands": [
                    f"git add -- {quoted}",
                    f'git commit -m "{message}"',
                ],
            }
        )

    return 0, {
        "mode": "commit-plan",
        "mutated_git": False,
        "excluded_concurrent": [str(item) for item in sorted(set(excluded_concurrent))],
        "unclassified": [str(item) for item in sorted(set(unclassified))],
        "waves": waves_out,
    }


def main(argv: Sequence[str] | None = None) -> int:
    """CLI entrypoint for Panadiga repo sanitization, sync, and read-only commit wave planning."""
    parser = argparse.ArgumentParser(
        description="Panadiga SSOT sync, repository hygiene audit, and read-only 6-wave commit planner."
    )
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--sync", action="store_true", help="Run SSOT, Ghidra, and tool arsenal sync")
    group.add_argument("--check", action="store_true", help="Run read-only artifact, LOC, and drift audit")
    group.add_argument(
        "--commit-plan",
        action="store_true",
        help="Classify working tree cleanly into 6 atomic commit waves (never commits/pushes)",
    )
    parser.add_argument("--dry-run", action="store_true", help="Preview --sync steps without executing")
    parser.add_argument("--max-loc-warn", type=int, default=450)
    parser.add_argument("--max-loc-fail", type=int, default=650)
    parser.add_argument(
        "--exclude",
        action="append",
        default=[],
        help="Path or prefix held by a concurrent task (isolated into excluded_concurrent)",
    )
    parser.add_argument("--json", default="-", help="JSON output path or '-' for stdout")
    args = parser.parse_args(argv)

    if args.sync:
        code, payload = run_sync(dry_run=args.dry_run)
    elif args.check:
        code, payload = run_check(
            max_loc_warn=args.max_loc_warn,
            max_loc_fail=args.max_loc_fail,
        )
    else:
        code, payload = build_commit_plan(exclude_paths=args.exclude)
    emit_json_payload(payload, args.json)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
