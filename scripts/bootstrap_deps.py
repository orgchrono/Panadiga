"""Best-effort, idempotent toolchain bootstrap shared by `build.cmd` and `build.sh`.

Guarantees (by construction):
  * never raises: every failure becomes a `[BOOTSTRAP] WARN` line and the build continues
    (lanes whose toolchain is absent already skip or report on their own);
  * idempotent: present tools are detected in milliseconds and nothing is downloaded;
  * opt-out: set `PANADIGA_NO_BOOTSTRAP=1` to disable all network/package installs.
"""

from __future__ import annotations

import importlib.util
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path
from typing import Final, Mapping, MutableMapping, Protocol, Sequence

from scripts.build_cache.disk_guard import ensure_disk_headroom

INFO_ONLY_FLAGS: Final[frozenset[str]] = frozenset(
    {"-h", "--help", "--cache-status", "--clean-cache", "--clean-all-caches"}
)
_DOWNLOAD_TIMEOUT_S: Final[int] = 90
_INSTALL_TIMEOUT_S: Final[int] = 1800


class _SelectionLike(Protocol):
    ci: bool
    frontend: bool
    pico: bool
    desktop: bool
    android: bool


def _say(level: str, message: str) -> None:
    print(f"[BOOTSTRAP] {level}: {message}", flush=True)


def _norm_path_entry(path_str: str) -> str:
    stripped = path_str.rstrip("\\/")
    return stripped.lower() if sys.platform == "win32" else stripped


def prepend_path_dir(directory: Path, env: MutableMapping[str, str] | None = None) -> None:
    """Prepend `directory` to PATH in `env` (or `os.environ`) without duplicating entries."""
    target: MutableMapping[str, str] = env if env is not None else os.environ
    text = str(directory)
    current = target.get("PATH", "")
    existing = current.split(os.pathsep) if current else []
    wanted = _norm_path_entry(text)
    if any(_norm_path_entry(item) == wanted for item in existing if item):
        return
    target["PATH"] = f"{text}{os.pathsep}{current}" if current else text


def _run_quiet(cmd: Sequence[str], *, cwd: Path | None = None, env: Mapping[str, str] | None = None, timeout: int = _INSTALL_TIMEOUT_S) -> bool:
    try:
        proc = subprocess.run(
            list(cmd),
            cwd=str(cwd) if cwd is not None else None,
            env=dict(env) if env is not None else None,
            stdin=subprocess.DEVNULL,
            check=False,
            timeout=timeout,
        )
        return proc.returncode == 0
    except (OSError, subprocess.SubprocessError) as exc:
        _say("WARN", f"comando falhou ({' '.join(cmd[:2])}...): {exc}")
        return False


def _probe_ok(cmd: Sequence[str], *, env: Mapping[str, str] | None = None, timeout: int = 15) -> bool:
    try:
        proc = subprocess.run(
            list(cmd),
            env=dict(env) if env is not None else None,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            check=False,
            timeout=timeout,
        )
        return proc.returncode == 0
    except (OSError, subprocess.SubprocessError):
        return False


def _download(url: str, dest: Path) -> bool:
    try:
        with urllib.request.urlopen(url, timeout=_DOWNLOAD_TIMEOUT_S) as resp, dest.open("wb") as out:  # noqa: S310
            shutil.copyfileobj(resp, out)
        return dest.stat().st_size > 0
    except (OSError, ValueError) as exc:
        _say("WARN", f"download falhou ({url}): {exc}")
        return False


def _discover_java_home(env: Mapping[str, str]) -> Path | None:
    explicit = (env.get("JAVA_HOME") or "").strip()
    java_name = "java.exe" if sys.platform == "win32" else "java"
    if explicit and (Path(explicit) / "bin" / java_name).is_file():
        return Path(explicit)
    candidates: list[Path] = []
    if sys.platform == "win32":
        pf = Path(env.get("ProgramFiles", r"C:\Program Files"))
        candidates.extend([
            pf / "Android" / "Android Studio" / "jbr",
            *sorted(pf.glob("Microsoft/jdk-*/"), reverse=True),
            *sorted(pf.glob("Eclipse Adoptium/jdk-*/"), reverse=True),
            *sorted(pf.glob("Java/jdk*/"), reverse=True),
        ])
    elif sys.platform == "darwin":
        candidates.extend([
            Path("/Applications/Android Studio.app/Contents/jbr/Contents/Home"),
            *sorted(Path("/Library/Java/JavaVirtualMachines").glob("*/Contents/Home"), reverse=True),
        ])
    else:
        candidates.extend([
            Path.home() / "android-studio" / "jbr",
            Path("/opt/android-studio/jbr"),
            *sorted(Path("/usr/lib/jvm").glob("java-*"), reverse=True),
        ])
    for cand in candidates:
        if (cand / "bin" / java_name).is_file():
            return cand
    return None


def augment_host_paths(env: MutableMapping[str, str] | None = None) -> None:
    """Discover Cargo, Node, JDK/JBR, CMake, Ninja, and ARM GCC across standard OS paths."""
    target: MutableMapping[str, str] = env if env is not None else os.environ
    path_val = target.get("PATH", "")
    cargo_bin = Path.home() / ".cargo" / "bin"
    if cargo_bin.is_dir():
        prepend_path_dir(cargo_bin, target)

    if sys.platform == "win32":
        pf = Path(target.get("ProgramFiles", r"C:\Program Files"))
        pf86 = Path(target.get("ProgramFiles(x86)", r"C:\Program Files (x86)"))
        local_app = Path(target.get("LOCALAPPDATA", ""))
        for node_dir in (pf / "nodejs", local_app / "Programs" / "nodejs"):
            if (node_dir / "node.exe").is_file():
                prepend_path_dir(node_dir, target)
        if not shutil.which("arm-none-eabi-gcc", path=path_val):
            for arm_base in (pf86 / "Arm GNU Toolchain arm-none-eabi", pf / "Arm GNU Toolchain arm-none-eabi"):
                for bin_dir in sorted(arm_base.glob("*/bin"), reverse=True):
                    if (bin_dir / "arm-none-eabi-gcc.exe").is_file():
                        prepend_path_dir(bin_dir, target)
                        break
        for vs_base in (pf86 / "Microsoft Visual Studio", pf / "Microsoft Visual Studio"):
            for year in ("2022", "2019"):
                for sku in ("BuildTools", "Community", "Professional", "Enterprise"):
                    vs_cm = vs_base / year / sku / "Common7" / "IDE" / "CommonExtensions" / "Microsoft" / "CMake"
                    if (vs_cm / "CMake" / "bin" / "cmake.exe").is_file():
                        prepend_path_dir(vs_cm / "CMake" / "bin", target)
                    if (vs_cm / "Ninja" / "ninja.exe").is_file():
                        prepend_path_dir(vs_cm / "Ninja", target)
        if (pf / "CMake" / "bin" / "cmake.exe").is_file():
            prepend_path_dir(pf / "CMake" / "bin", target)
    else:
        for posix_dir in (
            Path("/opt/homebrew/bin"),
            Path("/usr/local/bin"),
            *sorted((Path.home() / ".nvm" / "versions" / "node").glob("*/bin"), reverse=True),
        ):
            if posix_dir.is_dir():
                prepend_path_dir(posix_dir, target)

    pico_sdk_tools = Path.home() / ".pico-sdk"
    if pico_sdk_tools.is_dir():
        for sub in ("cmake", "ninja", "toolchain"):
            for bin_dir in sorted((pico_sdk_tools / sub).glob("*/bin"), reverse=True):
                prepend_path_dir(bin_dir, target)
            for ver_dir in sorted((pico_sdk_tools / sub).glob("*"), reverse=True):
                if (ver_dir / ("ninja.exe" if sys.platform == "win32" else "ninja")).is_file():
                    prepend_path_dir(ver_dir, target)

    java_home = _discover_java_home(target)
    if java_home is not None:
        target.setdefault("JAVA_HOME", str(java_home))
        prepend_path_dir(java_home / "bin", target)


def _install_via_os_pkg(
    *,
    winget_ids: Sequence[str] = (),
    brew_pkgs: Sequence[str] = (),
    apt_pkgs: Sequence[str] = (),
    dnf_pkgs: Sequence[str] = (),
    pacman_pkgs: Sequence[str] = (),
    apk_pkgs: Sequence[str] = (),
    zypper_pkgs: Sequence[str] = (),
) -> bool:
    if sys.platform == "win32":
        if shutil.which("winget") is None or not winget_ids:
            return False
        ok = True
        for pkg_id in winget_ids:
            ok = _run_quiet([
                "winget", "install", "-e", "--id", pkg_id, "--silent",
                "--accept-package-agreements", "--accept-source-agreements",
            ]) and ok
        return ok
    if shutil.which("brew") is not None and brew_pkgs:
        return _run_quiet(["brew", "install", *brew_pkgs])
    is_root = hasattr(os, "geteuid") and os.geteuid() == 0
    sudo: list[str] = []
    if hasattr(os, "geteuid") and not is_root:
        if shutil.which("sudo") is None or not _probe_ok(["sudo", "-n", "true"], timeout=10):
            return False
        sudo = ["sudo", "-n"]
    if shutil.which("apt-get") is not None and apt_pkgs:
        _run_quiet([*sudo, "apt-get", "update", "-y"])
        return _run_quiet([*sudo, "env", "DEBIAN_FRONTEND=noninteractive", "apt-get", "install", "-y", *apt_pkgs])
    if shutil.which("dnf") is not None and dnf_pkgs:
        return _run_quiet([*sudo, "dnf", "install", "-y", *dnf_pkgs])
    if shutil.which("pacman") is not None and pacman_pkgs:
        return _run_quiet([*sudo, "pacman", "-Sy", "--noconfirm", *pacman_pkgs])
    if shutil.which("apk") is not None and apk_pkgs:
        return _run_quiet([*sudo, "apk", "add", "--no-cache", *apk_pkgs])
    if shutil.which("zypper") is not None and zypper_pkgs:
        return _run_quiet([*sudo, "zypper", "--non-interactive", "install", *zypper_pkgs])
    return False


def ensure_rust(*, need_lint_components: bool = False) -> bool:
    """Ensure a working `cargo` (and `rustfmt`/`clippy` when requested)."""
    augment_host_paths()
    cargo = shutil.which("cargo")
    if cargo is None or not _probe_ok([cargo, "--version"], timeout=60):
        rustup = shutil.which("rustup")
        if rustup is not None and _run_quiet([rustup, "default", "stable"]):
            cargo = shutil.which("cargo")
        elif not os.environ.get("PANADIGA_NO_BOOTSTRAP"):
            _say("INFO", "cargo nao encontrado; instalando Rust via rustup (perfil minimal)...")
            with tempfile.TemporaryDirectory(prefix="panadiga_rustup_") as tmp:
                if sys.platform == "win32":
                    arch = "aarch64" if platform.machine().lower() in ("arm64", "aarch64") else "x86_64"
                    installer = Path(tmp) / "rustup-init.exe"
                    if _download(f"https://win.rustup.rs/{arch}", installer):
                        _run_quiet([str(installer), "-y", "--profile", "minimal", "--default-toolchain", "stable"])
                else:
                    script = Path(tmp) / "rustup-init.sh"
                    if _download("https://sh.rustup.rs", script):
                        _run_quiet(["sh", str(script), "-y", "--profile", "minimal", "--default-toolchain", "stable"])
            augment_host_paths()
            cargo = shutil.which("cargo")

    if cargo is None or not _probe_ok([cargo, "--version"], timeout=60):
        _say("WARN", "cargo indisponivel; instale em https://rustup.rs")
        return False

    if need_lint_components and not os.environ.get("PANADIGA_NO_BOOTSTRAP"):
        rustup = shutil.which("rustup")
        if rustup is not None and (not _probe_ok([cargo, "fmt", "--version"]) or not _probe_ok([cargo, "clippy", "--version"])):
            _say("INFO", "instalando componentes rustfmt e clippy...")
            _run_quiet([rustup, "component", "add", "rustfmt", "clippy"])
    return True


def ensure_node() -> bool:
    """Ensure `node` and `npm` are available on PATH."""
    augment_host_paths()
    if shutil.which("node") is not None and (shutil.which("npm") is not None or shutil.which("npm.cmd") is not None):
        return True
    if os.environ.get("PANADIGA_NO_BOOTSTRAP"):
        _say("WARN", "node/npm ausentes e PANADIGA_NO_BOOTSTRAP definido; pulando instalacao.")
        return False
    _say("INFO", "node/npm nao encontrados; instalando Node.js LTS...")
    _install_via_os_pkg(
        winget_ids=("OpenJS.NodeJS.LTS",),
        brew_pkgs=("node",),
        apt_pkgs=("nodejs", "npm"),
        dnf_pkgs=("nodejs", "npm"),
        pacman_pkgs=("nodejs", "npm"),
        apk_pkgs=("nodejs", "npm"),
        zypper_pkgs=("nodejs", "npm"),
    )
    augment_host_paths()
    if shutil.which("node") is not None:
        _say("OK", "Node.js disponivel.")
        return True
    _say("WARN", "nao foi possivel instalar Node.js automaticamente; instale em https://nodejs.org")
    return False


def ensure_frontend_node_modules(root: Path, env: Mapping[str, str] | None = None) -> bool:
    """Ensure `apps/desktop/node_modules` and local bin shims are populated idempotently."""
    desktop_dir = root / "apps" / "desktop"
    nm = desktop_dir / "node_modules"
    ext = ".cmd" if sys.platform == "win32" else ""
    ready = (
        (nm / ".package-lock.json").is_file()
        and (nm / ".bin" / f"vite{ext}").is_file()
        and (nm / ".bin" / f"tsc{ext}").is_file()
        and (nm / ".bin" / f"oxlint{ext}").is_file()
        and (nm / "@orgchrono" / "oxlint-plugin" / "package.json").is_file()
    )
    if ready:
        return True
    if os.environ.get("PANADIGA_NO_BOOTSTRAP"):
        return False
    npm = shutil.which("npm", path=env.get("PATH") if env else None)
    if npm is None:
        return False
    _say("INFO", "instalando dependencias npm em apps/desktop...")
    if (desktop_dir / "package-lock.json").is_file() and _run_quiet([npm, "--prefix", str(desktop_dir), "ci", "--no-audit", "--no-fund"], cwd=root, env=env):
        return True
    return _run_quiet([npm, "--prefix", str(desktop_dir), "install", "--no-audit", "--no-fund"], cwd=root, env=env)


def ensure_python_ci_tools(root: Path) -> bool:
    """Ensure `mypy` and `pytest` are importable for Gate 9 (supports PEP 668 via local `.venv`)."""
    if importlib.util.find_spec("mypy") is not None and importlib.util.find_spec("pytest") is not None:
        return True
    if os.environ.get("PANADIGA_NO_BOOTSTRAP"):
        return False
    _say("INFO", "instalando mypy e pytest para Quality Gate Python...")
    if _run_quiet([sys.executable, "-m", "pip", "install", "--disable-pip-version-check", "--quiet", "mypy", "pytest"], cwd=root):
        return True
    if _run_quiet([sys.executable, "-m", "pip", "install", "--user", "--disable-pip-version-check", "--quiet", "mypy", "pytest"], cwd=root):
        return True
    venv_dir = root / ".venv"
    venv_py = venv_dir / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
    if not venv_py.is_file():
        _run_quiet([sys.executable, "-m", "venv", str(venv_dir)], cwd=root)
    if venv_py.is_file():
        return _run_quiet([str(venv_py), "-m", "pip", "install", "--disable-pip-version-check", "--quiet", "mypy", "pytest"], cwd=root)
    return False


def ensure_playwright_chromium(root: Path, env: Mapping[str, str] | None = None) -> bool:
    """Ensure Playwright Chromium browser binary exists for Gate 6 E2E tests."""
    src_env = env or os.environ
    cache_candidates: list[Path] = []
    if src_env.get("PLAYWRIGHT_BROWSERS_PATH"):
        cache_candidates.append(Path(src_env["PLAYWRIGHT_BROWSERS_PATH"]))
    if sys.platform == "win32" and src_env.get("LOCALAPPDATA"):
        cache_candidates.append(Path(src_env["LOCALAPPDATA"]) / "ms-playwright")
    elif sys.platform == "darwin":
        cache_candidates.append(Path.home() / "Library" / "Caches" / "ms-playwright")
    else:
        cache_candidates.append(Path.home() / ".cache" / "ms-playwright")
    if any(p.is_dir() and any(p.glob("chromium*")) for p in cache_candidates):
        return True
    if os.environ.get("PANADIGA_NO_BOOTSTRAP"):
        return False
    npx = shutil.which("npx", path=src_env.get("PATH"))
    if npx is None:
        return False
    _say("INFO", "instalando Chromium do Playwright para testes E2E...")
    return _run_quiet([npx, "--prefix", str(root / "apps" / "desktop"), "playwright", "install", "chromium"], cwd=root, env=src_env)


def ensure_pico_host_tools() -> None:
    """Best-effort install of `cmake` and `ninja` when `--pico` is explicitly requested."""
    augment_host_paths()
    need_cmake = shutil.which("cmake") is None
    need_ninja = shutil.which("ninja") is None
    if not (need_cmake or need_ninja) or os.environ.get("PANADIGA_NO_BOOTSTRAP"):
        return
    _say("INFO", "cmake/ninja ausentes; tentando instalar ferramentas CMake/Ninja...")
    _install_via_os_pkg(
        winget_ids=("Kitware.CMake", "Ninja-build.Ninja"),
        brew_pkgs=("cmake", "ninja"),
        apt_pkgs=("cmake", "ninja-build"),
        dnf_pkgs=("cmake", "ninja-build"),
        pacman_pkgs=("cmake", "ninja"),
        apk_pkgs=("cmake", "ninja"),
        zypper_pkgs=("cmake", "ninja"),
    )
    augment_host_paths()


def needs_bootstrap(argv: Sequence[str]) -> bool:
    """False for purely informational invocations and non-TTY zero-arg `--help` fallback."""
    items = list(argv)
    if not items:
        return sys.stdin.isatty() and sys.stdout.isatty()
    return not any(flag in INFO_ONLY_FLAGS for flag in items)


def ensure_dependencies(argv: Sequence[str], *, root: Path | None = None) -> None:
    """Resolve core toolchains early for CLI build invocations. Never raises."""
    try:
        augment_host_paths()
        if not needs_bootstrap(argv):
            return
        repo_root = root or Path(__file__).resolve().parent.parent
        ensure_disk_headroom(repo_root, keep_android=any(f in argv for f in ("--android", "--all")))
        need_ci = "--ci" in argv
        ensure_rust(need_lint_components=need_ci)
        if any(f in argv for f in ("--frontend", "--desktop", "--android", "--all", "--ci", "--tui")) or not any(
            f in argv for f in ("--pico", "--cli", "--run", "--rebuild-run")
        ):
            if ensure_node():
                ensure_frontend_node_modules(repo_root)
        if need_ci:
            ensure_python_ci_tools(repo_root)
            ensure_playwright_chromium(repo_root)
        if "--pico" in argv:
            ensure_pico_host_tools()
    except Exception as exc:  # noqa: BLE001 - bootstrap must never abort a build
        _say("WARN", f"bootstrap interrompido ({type(exc).__name__}): {exc}")


def ensure_selection_dependencies(selection: _SelectionLike, root: Path, env: dict[str, str]) -> None:
    """Ensure dependencies required by a chosen `PipelineSelection` (including TUI selections)."""
    try:
        augment_host_paths(env)
        ensure_disk_headroom(root, keep_android=selection.android)
        ensure_rust(need_lint_components=selection.ci)
        if selection.frontend or selection.desktop or selection.android or selection.ci:
            if ensure_node():
                augment_host_paths(env)
                ensure_frontend_node_modules(root, env)
        if selection.ci:
            ensure_python_ci_tools(root)
            ensure_playwright_chromium(root, env)
        if selection.pico:
            ensure_pico_host_tools()
            augment_host_paths(env)
    except Exception as exc:  # noqa: BLE001 - bootstrap must never abort a build
        _say("WARN", f"bootstrap de selecao interrompido ({type(exc).__name__}): {exc}")
