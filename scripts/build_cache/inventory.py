"""Recursive workspace discovery and build environment/toolchain optimization."""

from __future__ import annotations

from dataclasses import dataclass
import os
import platform
import shutil
import sys
from pathlib import Path
from typing import Final, Mapping

from scripts.panadiga_ops import ROOT, load_msvc_env

BUILD_IGNORE_DIRS: Final[frozenset[str]] = frozenset(
    {
        ".git",
        ".github",
        ".jscpd-report",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        ".vite",
        "__pycache__",
        "build",
        "dist",
        "dist-android",
        "node_modules",
        "pico-build",
        "playwright-report",
        "target",
        "test-results",
    }
)


@dataclass(frozen=True)
class WorkspaceInventory:
    """Discovered workspace layout, host capabilities, toolchains, and active caches."""

    root: Path
    os_name: str
    arch: str
    cpu_count: int
    crates: tuple[str, ...]
    apps: tuple[str, ...]
    firmware_targets: tuple[str, ...]
    tools: dict[str, str]


@dataclass(frozen=True)
class CacheEntryStat:
    """Size metadata for a build or compiler cache directory."""

    label: str
    path: Path
    exists: bool
    bytes_total: int


def find_repo_root(start: Path | None = None) -> Path:
    """Walk recursively upward to find the Panadiga repository root (Cargo.toml + scripts/build.py)."""
    probe = (start or Path.cwd()).resolve()
    if probe.is_file():
        probe = probe.parent
    current = probe
    while True:
        if (current / "Cargo.toml").is_file() and (current / "scripts" / "build.py").is_file():
            return current
        parent = current.parent
        if parent == current:
            return ROOT
        current = parent


def iter_files_recursive(
    base: Path,
    *,
    extensions: frozenset[str] | None = None,
    ignore_prefixes: tuple[str, ...] = (),
) -> list[Path]:
    """Recursively collect files sorted deterministically while skipping build artifacts."""
    if not base.exists():
        return []
    if base.is_file():
        return [base]

    collected: list[Path] = []
    for dirpath_str, dirnames, filenames in os.walk(base):
        dirnames[:] = sorted(
            d
            for d in dirnames
            if d not in BUILD_IGNORE_DIRS
            and not any(d.startswith(prefix) for prefix in ignore_prefixes)
        )
        dirpath = Path(dirpath_str)
        for fname in sorted(filenames):
            fpath = dirpath / fname
            if extensions is not None and fpath.suffix.lower() not in extensions and fname not in extensions:
                continue
            collected.append(fpath)
    collected.sort(key=lambda p: p.as_posix())
    return collected


def _discover_manifests_recursive(root: Path, subdir: str, marker_filename: str) -> tuple[str, ...]:
    """Recursively discover workspace subprojects containing a specific manifest file."""
    base = root / subdir
    if not base.is_dir():
        return ()
    found: list[str] = []
    for path in iter_files_recursive(base, extensions=frozenset({marker_filename}), ignore_prefixes=("pico-build",)):
        if path.name == marker_filename:
            found.append(path.parent.relative_to(root).as_posix())
    return tuple(sorted(set(found)))


def discover_pico_sdk(root: Path, env: Mapping[str, str]) -> Path | None:
    """Locate Raspberry Pi Pico SDK across environment variables and common install paths."""
    sdk_env = env.get("PICO_SDK_PATH")
    if sdk_env and Path(sdk_env).is_dir():
        return Path(sdk_env).resolve()
    candidates = (
        Path(r"C:\Program Files\Raspberry Pi\Pico SDK v2.0.0\pico-sdk"),
        Path(r"C:\Program Files\Raspberry Pi\Pico SDK v1.5.1\pico-sdk"),
        Path(r"C:\pico\pico-sdk"),
        Path(r"C:\pico-sdk"),
        Path.home() / "pico-sdk",
        Path.home() / ".pico-sdk" / "sdk" / "2.1.1",
        Path.home() / ".pico-sdk" / "sdk" / "2.0.0",
        Path.home() / ".pico-sdk" / "sdk" / "1.5.1",
        root / "pico-sdk",
    )
    for cand in candidates:
        if cand.is_dir():
            return cand.resolve()
    return None


def _discover_android_paths(env: Mapping[str, str]) -> tuple[Path | None, Path | None]:
    """Detect Android SDK and NDK directories without raising when absent."""
    sdk_env = env.get("ANDROID_HOME") or env.get("ANDROID_SDK_ROOT")
    sdk_dir: Path | None = Path(sdk_env) if sdk_env and Path(sdk_env).is_dir() else None
    if sdk_dir is None:
        fallback = (
            Path(env.get("LOCALAPPDATA", "")) / "Android" / "Sdk"
            if sys.platform == "win32"
            else Path.home() / "Android" / "Sdk"
        )
        if fallback.is_dir():
            sdk_dir = fallback
    ndk_env = env.get("NDK_HOME") or env.get("ANDROID_NDK_ROOT")
    ndk_dir: Path | None = Path(ndk_env) if ndk_env and Path(ndk_env).is_dir() else None
    if ndk_dir is None and sdk_dir is not None and (sdk_dir / "ndk").is_dir():
        ndk_versions = sorted(p for p in (sdk_dir / "ndk").iterdir() if p.is_dir())
        if ndk_versions:
            ndk_dir = ndk_versions[-1]
    return sdk_dir, ndk_dir


def _augment_windows_toolchain_path(env: dict[str, str]) -> None:
    """Prepend standard Windows ARM GCC and VS CMake/Ninja folders if not yet on PATH."""
    if sys.platform != "win32":
        return
    extra_dirs: list[Path] = []
    current_path = env.get("PATH", "")
    if not shutil.which("arm-none-eabi-gcc", path=current_path):
        for arm_base in (
            Path(r"C:\Program Files (x86)\Arm GNU Toolchain arm-none-eabi"),
            Path(r"C:\Program Files\Arm GNU Toolchain arm-none-eabi"),
        ):
            if arm_base.is_dir():
                for bin_dir in sorted(arm_base.glob("*/bin"), reverse=True):
                    if (bin_dir / "arm-none-eabi-gcc.exe").is_file():
                        extra_dirs.append(bin_dir)
                        break
    vs_cmake = Path(
        r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake"
    )
    if not shutil.which("cmake", path=current_path) and (vs_cmake / "CMake" / "bin").is_dir():
        extra_dirs.append(vs_cmake / "CMake" / "bin")
    if not shutil.which("ninja", path=current_path) and (vs_cmake / "Ninja").is_dir():
        extra_dirs.append(vs_cmake / "Ninja")

    if extra_dirs:
        prefix = os.pathsep.join(str(d) for d in extra_dirs)
        env["PATH"] = f"{prefix}{os.pathsep}{current_path}" if current_path else prefix


def resolve_tool(name: str, env: Mapping[str, str]) -> str | None:
    """Resolve an executable on PATH or under `~/.cargo/bin`."""
    path_str = env.get("PATH")
    found = shutil.which(name, path=path_str)
    if found:
        return found
    cargo_candidate = Path.home() / ".cargo" / "bin" / (f"{name}.exe" if sys.platform == "win32" else name)
    return str(cargo_candidate) if cargo_candidate.is_file() else None


def optimize_build_env(
    base_env: Mapping[str, str],
    *,
    root: Path = ROOT,
    use_cache: bool = True,
) -> tuple[dict[str, str], dict[str, str]]:
    """Return environment configured for maximum parallelism and available compiler caches."""
    env = dict(base_env)
    active: dict[str, str] = {}

    jobs = str(max(1, os.cpu_count() or 4))
    env.setdefault("CARGO_BUILD_JOBS", jobs)
    env.setdefault("CMAKE_BUILD_PARALLEL_LEVEL", jobs)
    env.setdefault("NINJA_STATUS", "[%f/%t %es] ")
    env.setdefault("PYTHONUTF8", "1")
    active["jobs"] = env["CARGO_BUILD_JOBS"]

    if sys.platform != "win32":
        env.setdefault("MAKEFLAGS", f"-j{jobs}")

    _augment_windows_toolchain_path(env)

    pico_sdk = discover_pico_sdk(root, env)
    if pico_sdk is not None:
        env.setdefault("PICO_SDK_PATH", str(pico_sdk))

    if use_cache:
        sccache_bin = resolve_tool("sccache", env)
        if sccache_bin and not env.get("RUSTC_WRAPPER"):
            env["RUSTC_WRAPPER"] = sccache_bin
            active["rustc_wrapper"] = sccache_bin
        ccache_bin = resolve_tool("ccache", env)
        if ccache_bin:
            env.setdefault("PANADIGA_CCACHE", ccache_bin)
            env.setdefault("CCACHE_COMPILERCHECK", "content")
            active["ccache"] = ccache_bin
        lock_src = root / "apps" / "desktop" / "package-lock.json"
        lock_dst = root / "apps" / "desktop" / "node_modules" / ".package-lock.json"
        if lock_src.is_file() and lock_dst.is_file():
            env.setdefault("npm_config_prefer_offline", "true")
            active["npm_offline"] = "true"
        active["step_cache"] = "enabled"
    else:
        active["step_cache"] = "disabled"

    return env, active


def discover_workspace_inventory(root: Path, env: Mapping[str, str]) -> WorkspaceInventory:
    """Recursively scan workspace components and available cross-platform toolchains."""
    opt_env, _ = optimize_build_env(env, root=root, use_cache=True)
    path_val = opt_env.get("PATH", "")

    crates = _discover_manifests_recursive(root, "crates", "Cargo.toml") + _discover_manifests_recursive(
        root, "apps", "Cargo.toml"
    )
    apps = _discover_manifests_recursive(root, "apps", "package.json")
    firmware_targets = _discover_manifests_recursive(root, "firmware", "CMakeLists.txt")

    tools: dict[str, str] = {}
    for tool_name in (
        "cargo",
        "rustc",
        "node",
        "npm",
        "arm-none-eabi-gcc",
        "cmake",
        "ninja",
        "sccache",
        "ccache",
        "java",
    ):
        resolved = resolve_tool(tool_name, opt_env)
        if resolved:
            tools[tool_name] = resolved

    for c_cand in ("gcc", "clang"):
        resolved_c = shutil.which(c_cand, path=path_val)
        if resolved_c:
            tools["c11_host"] = resolved_c
            break
    if "c11_host" not in tools and sys.platform == "win32":
        msvc_env = load_msvc_env(opt_env)
        cl_path = shutil.which("cl", path=msvc_env.get("PATH"))
        if cl_path:
            tools["c11_host"] = cl_path

    pico_sdk = discover_pico_sdk(root, opt_env)
    if pico_sdk is not None:
        tools["pico_sdk"] = str(pico_sdk)

    android_sdk, android_ndk = _discover_android_paths(opt_env)
    if android_sdk is not None:
        tools["android_sdk"] = str(android_sdk)
    if android_ndk is not None:
        tools["android_ndk"] = str(android_ndk)

    return WorkspaceInventory(
        root=root,
        os_name=platform.system(),
        arch=platform.machine(),
        cpu_count=max(1, os.cpu_count() or 4),
        crates=crates,
        apps=apps,
        firmware_targets=firmware_targets,
        tools=tools,
    )
