"""Typed subprocess bridge delegating Python RE and bench wrappers to native `panadiga` CLI (`--json`)."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Final, Sequence, TypeAlias

from scripts.panadiga_ops import ROOT, build_env, locate_panadiga_binary

JsonPrimitive: TypeAlias = None | bool | int | float | str
JsonValue: TypeAlias = JsonPrimitive | list["JsonValue"] | dict[str, "JsonValue"]
JsonDict: TypeAlias = dict[str, JsonValue]

_EXE_SUFFIX: Final[str] = ".exe" if sys.platform == "win32" else ""


class PanadigaCliError(RuntimeError):
    """Raised when `panadiga` CLI exits non-zero or returns malformed JSON."""


def resolve_panadiga_cli_prefix() -> tuple[str, ...]:
    """Resolve compiled `panadiga` CLI binary path cleanly before falling back to `cargo run`."""
    override = os.environ.get("PANADIGA_CLI_BIN", "").strip()
    if override and Path(override).is_file():
        return (override,)

    exe_name = f"panadiga{_EXE_SUFFIX}"
    for cand in (
        ROOT / "target" / "debug" / exe_name,
        ROOT / "target" / "release" / exe_name,
        ROOT / "dist" / exe_name,
    ):
        if cand.is_file():
            return (str(cand),)

    located = locate_panadiga_binary()
    if located is not None and located.is_file():
        return (str(located),)

    which_bin = shutil.which("panadiga")
    if which_bin is not None:
        return (which_bin,)

    return ("cargo", "run", "--quiet", "-p", "panadiga-cli", "--")


def run_panadiga_json(
    args: Sequence[str],
    *,
    stdin_bytes: bytes | None = None,
) -> JsonValue:
    """Invoke `panadiga <args> --json -` and parse compact JSON from stdout."""
    cmd: list[str] = [*resolve_panadiga_cli_prefix(), *args]
    if "--json" not in args:
        cmd.extend(["--json", "-"])

    proc = subprocess.run(
        cmd,
        input=stdin_bytes,
        capture_output=True,
        cwd=str(ROOT),
        env=build_env(),
        check=False,
    )
    if proc.returncode != 0:
        err_bytes = proc.stderr or proc.stdout
        msg = err_bytes.decode("utf-8", errors="replace").strip()
        raise PanadigaCliError(msg or f"panadiga CLI exited with status {proc.returncode}")

    stdout_text = proc.stdout.decode("utf-8", errors="replace").strip()
    if not stdout_text:
        raise PanadigaCliError(f"panadiga CLI returned empty JSON stdout for args={list(args)}")
    try:
        parsed: JsonValue = json.loads(stdout_text)
    except json.JSONDecodeError as exc:
        raise PanadigaCliError(f"Malformed JSON from panadiga CLI: {exc}") from exc
    return parsed


def as_dict(val: JsonValue) -> JsonDict:
    """Validate and narrow a JSON value to a dictionary."""
    if not isinstance(val, dict):
        raise PanadigaCliError(f"Expected JSON object, got {type(val).__name__}")
    return val


def as_list(val: JsonValue) -> list[JsonValue]:
    """Validate and narrow a JSON value to a list."""
    if not isinstance(val, list):
        raise PanadigaCliError(f"Expected JSON list, got {type(val).__name__}")
    return val


def req_str(obj: JsonDict, key: str) -> str:
    """Read a required string field from a JSON object."""
    v = obj.get(key)
    if not isinstance(v, str):
        raise PanadigaCliError(f"Missing string field '{key}'")
    return v


def opt_str(obj: JsonDict, key: str) -> str | None:
    """Read an optional string field from a JSON object."""
    v = obj.get(key)
    if v is None:
        return None
    if not isinstance(v, str):
        raise PanadigaCliError(f"Expected string|null for '{key}'")
    return v


def req_int(obj: JsonDict, key: str) -> int:
    """Read a required integer field (excluding booleans) from a JSON object."""
    v = obj.get(key)
    if isinstance(v, bool) or not isinstance(v, int):
        raise PanadigaCliError(f"Missing int field '{key}'")
    return v


def opt_int(obj: JsonDict, key: str) -> int | None:
    """Read an optional integer field from a JSON object."""
    v = obj.get(key)
    if v is None:
        return None
    if isinstance(v, bool) or not isinstance(v, int):
        raise PanadigaCliError(f"Expected int|null for '{key}'")
    return v


def opt_float(obj: JsonDict, key: str) -> float | None:
    """Read an optional float/int numeric field from a JSON object."""
    v = obj.get(key)
    if v is None:
        return None
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        raise PanadigaCliError(f"Expected number|null for '{key}'")
    return float(v)


def req_bool(obj: JsonDict, key: str) -> bool:
    """Read a required boolean field from a JSON object."""
    v = obj.get(key)
    if not isinstance(v, bool):
        raise PanadigaCliError(f"Missing bool field '{key}'")
    return v


def _resolve_ghidra_mcp_command() -> str:
    """Resolve `ghidra-mcp` executable via env override, PATH, or Windows venv fallback."""
    override = os.environ.get("GHIDRA_MCP_BIN", "").strip()
    if override and Path(override).is_file():
        return override
    on_path = shutil.which("ghidra-mcp")
    if on_path is not None:
        return on_path
    win_default = Path(r"C:\ghidra\.venv\Scripts\ghidra-mcp.exe")
    if os.name == "nt" and win_default.is_file():
        return str(win_default)
    raise PanadigaCliError(
        "ghidra-mcp executable not found; set GHIDRA_MCP_BIN or add ghidra-mcp to PATH."
    )


def main(argv: Sequence[str] | None = None) -> int:
    """Fast stdio launcher for MCP servers (`mcp` or `ghidra-mcp`), avoiding cold `cargo run` when prebuilt."""
    args = list(sys.argv[1:] if argv is None else argv)
    if args[:1] == ["mcp"]:
        cmd = [*resolve_panadiga_cli_prefix(), *args]
    elif args[:1] == ["ghidra-mcp"]:
        cmd = [_resolve_ghidra_mcp_command(), *args[1:]]
    else:
        raise SystemExit("Usage: python -m scripts.panadiga_cli_bridge <mcp|ghidra-mcp> [args...]")
    proc = subprocess.run(cmd, cwd=str(ROOT), env=build_env(), check=False)
    return int(proc.returncode)


if __name__ == "__main__":
    raise SystemExit(main())
