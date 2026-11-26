#!/usr/bin/env sh
set -eu

# ==============================================================================
# Panadiga Universal POSIX Launcher (Linux / macOS / BSD / WSL / Git Bash)
# Pure POSIX sh (strictly LF, no bashisms) with recursive root discovery,
# automatic Python 3.10+ bootstrap, and direct forwarding to scripts/build.py.
# ==============================================================================

resolve_script_dir() {
    src="$1"
    while [ -L "$src" ]; do
        dir=$(CDPATH= cd -- "$(dirname -- "$src")" && pwd -P)
        if command -v readlink >/dev/null 2>&1; then
            link=$(readlink "$src")
        else
            break
        fi
        case "$link" in
            /*) src="$link" ;;
            *) src="$dir/$link" ;;
        esac
    done
    CDPATH= cd -- "$(dirname -- "$src")" && pwd -P
}

locate_repo_root() {
    probe="$1"
    while [ -n "$probe" ]; do
        if [ -f "$probe/Cargo.toml" ] && [ -f "$probe/scripts/build.py" ]; then
            printf '%s\n' "$probe"
            return 0
        fi
        parent=$(dirname -- "$probe")
        if [ "$parent" = "$probe" ]; then
            return 1
        fi
        probe="$parent"
    done
    return 1
}

detect_cpu_count() {
    if command -v nproc >/dev/null 2>&1; then
        nproc 2>/dev/null && return 0
    fi
    if command -v sysctl >/dev/null 2>&1; then
        sysctl -n hw.ncpu 2>/dev/null && return 0
    fi
    if command -v getconf >/dev/null 2>&1; then
        getconf _NPROCESSORS_ONLN 2>/dev/null && return 0
    fi
    printf '4\n'
}

check_python_bin() {
    cand="$1"
    if [ -x "$cand" ] || command -v "$cand" >/dev/null 2>&1; then
        if "$cand" -c "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)" >/dev/null 2>&1; then
            printf '%s\n' "$cand"
            return 0
        fi
    fi
    return 1
}

find_python() {
    root_dir="$1"
    if [ -n "${PANADIGA_PYTHON:-}" ] && check_python_bin "$PANADIGA_PYTHON"; then
        return 0
    fi
    for venv_py in \
        "$root_dir/.venv/bin/python" \
        "$root_dir/.venv/Scripts/python.exe" \
        "$root_dir/venv/bin/python" \
        "$root_dir/venv/Scripts/python.exe"
    do
        if check_python_bin "$venv_py"; then
            return 0
        fi
    done
    for sys_py in python3.13 python3.12 python3.11 python3.10 python3 python; do
        if check_python_bin "$sys_py"; then
            return 0
        fi
    done
    if command -v py >/dev/null 2>&1 && py -3 -c "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)" >/dev/null 2>&1; then
        printf 'py3\n'
        return 0
    fi
    return 1
}

bootstrap_python() {
    if [ -n "${PANADIGA_NO_BOOTSTRAP:-}" ]; then
        return 1
    fi
    printf '[BOOTSTRAP] INFO: Python 3.10+ nao encontrado; tentando instalar...\n' >&2
    if command -v brew >/dev/null 2>&1; then
        brew install python@3.12 || true
        return 0
    fi
    SUDO=""
    if [ "$(id -u)" -ne 0 ]; then
        if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
            SUDO="sudo -n"
        else
            return 1
        fi
    fi
    if command -v apt-get >/dev/null 2>&1; then
        $SUDO apt-get update -y || true
        $SUDO env DEBIAN_FRONTEND=noninteractive apt-get install -y python3 python3-venv python3-pip || true
    elif command -v dnf >/dev/null 2>&1; then
        $SUDO dnf install -y python3 python3-pip || true
    elif command -v pacman >/dev/null 2>&1; then
        $SUDO pacman -Sy --noconfirm python python-pip || true
    elif command -v apk >/dev/null 2>&1; then
        $SUDO apk add --no-cache python3 py3-pip || true
    elif command -v zypper >/dev/null 2>&1; then
        $SUDO zypper --non-interactive install python3 python3-pip || true
    else
        return 1
    fi
    return 0
}

SCRIPT_DIR=$(resolve_script_dir "$0")
PANADIGA_ROOT=""
if root_cand=$(locate_repo_root "$PWD"); then
    PANADIGA_ROOT="$root_cand"
elif root_cand=$(locate_repo_root "$SCRIPT_DIR"); then
    PANADIGA_ROOT="$root_cand"
else
    printf '[FAIL] Nao foi possivel localizar a raiz Panadiga (Cargo.toml + scripts/build.py).\n' >&2
    exit 1
fi

if [ -d "$HOME/.cargo/bin" ]; then
    case ":${PATH:-}:" in
        *":$HOME/.cargo/bin:"*) ;;
        *) PATH="$HOME/.cargo/bin:${PATH:-}" ;;
    esac
    export PATH
fi

CPU_JOBS=$(detect_cpu_count)
export PYTHONUTF8="${PYTHONUTF8:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$CPU_JOBS}"
export CMAKE_BUILD_PARALLEL_LEVEL="${CMAKE_BUILD_PARALLEL_LEVEL:-$CPU_JOBS}"
export MAKEFLAGS="${MAKEFLAGS:--j$CPU_JOBS}"

if ! PY_SEL=$(find_python "$PANADIGA_ROOT"); then
    bootstrap_python || true
    if ! PY_SEL=$(find_python "$PANADIGA_ROOT"); then
        printf '[FAIL] Python 3.10+ nao encontrado. Instale python3 ou defina PANADIGA_PYTHON.\n' >&2
        exit 1
    fi
fi

cd "$PANADIGA_ROOT"
if [ "$PY_SEL" = "py3" ]; then
    exec py -3 "$PANADIGA_ROOT/scripts/build.py" "$@"
else
    exec "$PY_SEL" "$PANADIGA_ROOT/scripts/build.py" "$@"
fi
