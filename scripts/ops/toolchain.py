"""Environment setup, SHA-256 manifest updates, binary locators, and MSVC discovery."""

from __future__ import annotations

from functools import lru_cache
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import threading
from typing import BinaryIO, Final, Mapping, Protocol, Sequence

from scripts.ops.proc import ROOT

TOOLS_PYCACHE_IGNORE: Final[frozenset[str]] = frozenset(
    {"__pycache__", ".mypy_cache", ".pytest_cache", ".ruff_cache"}
)
_MANIFEST_LOCK = threading.Lock()


class _HashDigestLike(Protocol):
    def hexdigest(self) -> str: ...


class _FileDigestCallable(Protocol):
    def __call__(self, fileobj: BinaryIO, digest: str, /) -> _HashDigestLike: ...


def build_env() -> dict[str, str]:
    """Return process environment with Cargo user bin prepended when present."""
    env = dict(os.environ)
    cargo_bin = Path.home() / ".cargo" / "bin"
    if cargo_bin.is_dir():
        current_path = env.get("PATH", "")
        cargo_str = str(cargo_bin)
        if cargo_str not in current_path.split(os.pathsep):
            env["PATH"] = f"{cargo_str}{os.pathsep}{current_path}" if current_path else cargo_str
    return env


def sha256_file(path: Path) -> str:
    """Compute lowercase hexadecimal SHA-256 digest using zero-copy C file_digest when available."""
    file_digest_fn: _FileDigestCallable | None = getattr(hashlib, "file_digest", None)
    with path.open("rb") as handle:
        if file_digest_fn is not None:
            return str(file_digest_fn(handle, "sha256").hexdigest())
        digest = hashlib.sha256()
        while True:
            chunk = handle.read(1 << 20)
            if not chunk:
                break
            digest.update(chunk)
        return digest.hexdigest()


def update_sha256_manifest(manifest_path: Path, artifacts: Sequence[Path]) -> None:
    """Update or create SHA256SUMS.txt idempotently and thread-safely for given artifact paths."""
    with _MANIFEST_LOCK:
        existing_lines: list[str] = []
        target_names = {art.name for art in artifacts}
        if manifest_path.is_file():
            for line in manifest_path.read_text(encoding="utf-8").splitlines():
                stripped = line.strip()
                if not stripped:
                    continue
                parts = stripped.split()
                entry_name = parts[-1] if parts else ""
                if entry_name not in target_names:
                    existing_lines.append(stripped)

        for art in artifacts:
            if art.is_file():
                existing_lines.append(f"{sha256_file(art).upper()}  {art.name}")

        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.write_text("\n".join(existing_lines) + "\n", encoding="utf-8")


def _executable_suffix(stem: str) -> str:
    """Return platform executable filename for a Cargo binary stem."""
    return f"{stem}.exe" if sys.platform == "win32" else stem


def panadiga_binary_name() -> str:
    """Return native Panadiga CLI binary filename for the current OS."""
    return _executable_suffix("panadiga")


def panadiga_desktop_binary_name() -> str:
    """Return native Panadiga Tauri Desktop GUI binary filename for the current OS."""
    return _executable_suffix("panadiga-desktop")


def is_wsl() -> bool:
    """Detect whether execution runs inside Windows Subsystem for Linux."""
    proc_ver = Path("/proc/version")
    if not proc_ver.is_file():
        return False
    return "microsoft" in proc_ver.read_text(encoding="utf-8", errors="ignore").lower()


def _locate_binary(stem: str) -> Path | None:
    """Locate a prebuilt release binary by stem in dist/ or target/release/."""
    bin_name = _executable_suffix(stem)
    candidates: list[Path] = [
        ROOT / "dist" / bin_name,
        ROOT / "target" / "release" / bin_name,
    ]
    if is_wsl():
        candidates.extend([ROOT / "dist" / f"{stem}.exe", ROOT / "target" / "release" / f"{stem}.exe"])
    for cand in candidates:
        if cand.is_file():
            return cand
    return None


def locate_panadiga_binary() -> Path | None:
    """Locate prebuilt release Panadiga CLI binary in dist/ or target/release/."""
    return _locate_binary("panadiga")


def locate_panadiga_desktop_binary() -> Path | None:
    """Locate prebuilt release Panadiga Desktop GUI binary in dist/ or target/release/."""
    return _locate_binary("panadiga-desktop")


def find_msvc_vcvars64() -> Path | None:
    """Locate MSVC vcvars64.bat via Visual Studio Locator on Windows."""
    if sys.platform != "win32":
        return None
    program_files_x86 = Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)"))
    vswhere = program_files_x86 / "Microsoft Visual Studio" / "Installer" / "vswhere.exe"
    if not vswhere.is_file():
        return None
    proc = subprocess.run(
        [str(vswhere), "-latest", "-products", "*", "-find", "**\\vcvars64.bat"],
        capture_output=True,
        text=True,
        check=False,
    )
    for line in proc.stdout.splitlines():
        cand = Path(line.strip())
        if cand.is_file():
            return cand
    return None


@lru_cache(maxsize=1)
def _discover_msvc_delta() -> dict[str, str] | None:
    """Discover MSVC x64 toolchain environment variables once per process (thread-safe cache)."""
    vcvars = find_msvc_vcvars64()
    if vcvars is None:
        return None

    probe_env: dict[str, str] = {}
    for sys_key in (
        "SystemDrive",
        "SystemRoot",
        "WINDIR",
        "ComSpec",
        "TEMP",
        "TMP",
        "ProgramData",
        "ALLUSERSPROFILE",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "CommonProgramFiles",
        "CommonProgramFiles(x86)",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
    ):
        if sys_key in os.environ:
            probe_env[sys_key] = os.environ[sys_key]
    sys_root = probe_env.get("SystemRoot") or r"C:\Windows"
    probe_env["PATH"] = f"{sys_root}\\System32;{sys_root}"
    cmd_line = f'cmd.exe /d /s /c "call "{vcvars}" >nul 2>&1 && set"'
    try:
        proc = subprocess.run(
            cmd_line,
            env=probe_env,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            timeout=20,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return None

    if proc.returncode != 0:
        return None

    delta: dict[str, str] = {}
    for line in proc.stdout.splitlines():
        key, sep, value = line.partition("=")
        if sep and key:
            delta[key] = value
    return delta


def load_msvc_env(base_env: Mapping[str, str]) -> dict[str, str]:
    """Return environment augmented with MSVC x64 toolchain variables (cached across calls)."""
    env = dict(base_env)
    if sys.platform != "win32":
        return env
    if shutil.which("cl", path=env.get("PATH")) and "INCLUDE" in env and "LIB" in env:
        return env

    delta = _discover_msvc_delta()
    if delta is None:
        return env

    merged = {k: v for k, v in env.items() if k.upper() != "PATH"}
    base_path = env.get("PATH", "")
    for key, value in delta.items():
        if key.upper() == "PATH":
            merged["PATH"] = f"{value}{os.pathsep}{base_path}" if base_path else value
        else:
            merged[key] = value
    merged.setdefault("PATH", base_path)
    return merged
