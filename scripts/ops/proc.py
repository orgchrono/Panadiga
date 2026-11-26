"""Subprocess process-tree tracking, fail-fast execution, and synchronized console output."""

from __future__ import annotations

from contextlib import contextmanager
from pathlib import Path
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
from typing import Final, Iterator, Mapping, Sequence, TypedDict

AnyPopen = subprocess.Popen[bytes] | subprocess.Popen[str]

ROOT: Final[Path] = Path(__file__).resolve().parent.parent.parent
_PRINT_LOCK = threading.Lock()


def resolve_executable(name: str, env: Mapping[str, str] | None = None) -> str:
    """Resolve an executable path using PATH (handles .cmd/.exe cleanly on Windows)."""
    path_arg = env.get("PATH") if env is not None else None
    found = shutil.which(name, path=path_arg)
    if found is None:
        raise FileNotFoundError(f"Required executable '{name}' not found on PATH.")
    return found


class ProcessAbortedError(RuntimeError):
    """Raised when a step is skipped or terminated after sibling cancellation."""


class _TrackerLocal(threading.local):
    tracker: ProcessTracker | None = None


_TRACKER_LOCAL = _TrackerLocal()


class _SubprocessGroupKwargs(TypedDict, total=False):
    creationflags: int
    start_new_session: bool


def _subprocess_group_kwargs() -> _SubprocessGroupKwargs:
    if sys.platform == "win32":
        flag = int(getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0))
        return {"creationflags": flag} if flag else {}
    return {"start_new_session": True}


def _terminate_process_tree(proc: AnyPopen) -> None:
    """Force-terminate a child process and its descendants without raising if already exited."""
    if proc.poll() is not None:
        return
    if sys.platform == "win32":
        subprocess.run(
            ["taskkill", "/F", "/T", "/PID", str(proc.pid)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    else:
        killpg = getattr(os, "killpg", None)
        sigkill = getattr(signal, "SIGKILL", None)
        if callable(killpg) and sigkill is not None:
            try:
                killpg(proc.pid, sigkill)
            except OSError:
                pass
    try:
        if proc.poll() is None:
            proc.kill()
    except OSError:
        pass


class ProcessTracker:
    """Thread-safe registry of active child process trees for fail-fast sibling cancellation."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._aborted = False
        self._procs: set[AnyPopen] = set()

    def is_aborted(self) -> bool:
        """Return True once sibling cancellation has been triggered."""
        with self._lock:
            return self._aborted

    def register(self, proc: AnyPopen) -> bool:
        """Track a newly spawned process; returns False and terminates if already aborted."""
        with self._lock:
            if self._aborted:
                should_terminate = True
            else:
                self._procs.add(proc)
                should_terminate = False
        if should_terminate:
            _terminate_process_tree(proc)
            return False
        return True

    def unregister(self, proc: AnyPopen) -> None:
        """Stop tracking a finished child process."""
        with self._lock:
            self._procs.discard(proc)

    def terminate_all(self) -> None:
        """Mark tracker aborted and terminate every active child process tree."""
        with self._lock:
            self._aborted = True
            snapshot = list(self._procs)
            self._procs.clear()
        for proc in snapshot:
            _terminate_process_tree(proc)


def active_process_tracker() -> ProcessTracker | None:
    """Return the ProcessTracker bound to the current thread, if any."""
    return _TRACKER_LOCAL.tracker


@contextmanager
def use_process_tracker(tracker: ProcessTracker | None) -> Iterator[None]:
    """Bind a ProcessTracker to the current thread for fail-fast child cancellation."""
    previous = _TRACKER_LOCAL.tracker
    _TRACKER_LOCAL.tracker = tracker
    try:
        yield
    finally:
        _TRACKER_LOCAL.tracker = previous


def _run_captured_with_tempfile(
    argv: list[str],
    cwd_str: str,
    env_dict: dict[str, str] | None,
    tracker: ProcessTracker | None,
    cmd_label: str,
) -> str:
    """Run subprocess redirecting stdout+stderr to a temp file so background daemons cannot hang PIPE EOF."""
    with tempfile.TemporaryFile(mode="w+b") as log_fh:
        if tracker is None:
            proc = subprocess.run(
                argv,
                cwd=cwd_str,
                env=env_dict,
                stdin=subprocess.DEVNULL,
                stdout=log_fh,
                stderr=subprocess.STDOUT,
                check=False,
            )
            log_fh.seek(0)
            output = log_fh.read().decode("utf-8", errors="replace")
            if proc.returncode != 0:
                raise subprocess.CalledProcessError(proc.returncode, argv, output=output)
            return output

        group_kwargs = _subprocess_group_kwargs()
        child: subprocess.Popen[bytes] = subprocess.Popen(
            argv,
            cwd=cwd_str,
            env=env_dict,
            stdin=subprocess.DEVNULL,
            stdout=log_fh,
            stderr=subprocess.STDOUT,
            **group_kwargs,
        )
        if not tracker.register(child):
            try:
                child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                pass
            raise ProcessAbortedError(f"Aborted step during launch: {cmd_label}")

        try:
            returncode = child.wait()
        finally:
            tracker.unregister(child)

        log_fh.seek(0)
        stdout_data = log_fh.read().decode("utf-8", errors="replace")
        if returncode != 0:
            if tracker.is_aborted():
                raise ProcessAbortedError(f"Terminated aborted step: {cmd_label}")
            raise subprocess.CalledProcessError(returncode, argv, output=stdout_data)
        return stdout_data


def _run_step_untracked(
    argv: list[str],
    cwd_str: str,
    env_dict: dict[str, str] | None,
    capture_output: bool,
) -> str:
    if capture_output:
        return _run_captured_with_tempfile(argv, cwd_str, env_dict, None, argv[0])
    subprocess.run(argv, cwd=cwd_str, env=env_dict, check=True)
    return ""


def run_step(
    cmd: Sequence[str],
    *,
    cwd: Path | None = None,
    env: Mapping[str, str] | None = None,
    capture_output: bool = False,
) -> str:
    """Execute a command strictly and fail immediately on non-zero exit code."""
    if not cmd:
        raise ValueError("Cannot execute empty command sequence.")
    tracker = active_process_tracker()
    if tracker is not None and tracker.is_aborted():
        raise ProcessAbortedError(f"Skipped aborted step: {cmd[0]}")

    resolved = resolve_executable(cmd[0], env=env)
    argv = [resolved, *cmd[1:]]
    env_dict = dict(env) if env else None
    cwd_str = str(cwd or ROOT)

    if capture_output:
        return _run_captured_with_tempfile(argv, cwd_str, env_dict, tracker, cmd[0])
    if tracker is None:
        return _run_step_untracked(argv, cwd_str, env_dict, False)

    group_kwargs = _subprocess_group_kwargs()
    child: subprocess.Popen[bytes] = subprocess.Popen(
        argv,
        cwd=cwd_str,
        env=env_dict,
        stdin=subprocess.DEVNULL,
        **group_kwargs,
    )
    if not tracker.register(child):
        try:
            child.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass
        raise ProcessAbortedError(f"Aborted step during launch: {cmd[0]}")

    try:
        returncode = child.wait()
    finally:
        tracker.unregister(child)

    if returncode != 0:
        if tracker.is_aborted():
            raise ProcessAbortedError(f"Terminated aborted step: {cmd[0]}")
        raise subprocess.CalledProcessError(returncode, argv, output="")
    return ""


def print_banner(title: str, ansi_color: str = "36") -> None:
    """Print a standardized section banner."""
    sep = "=" * 70
    use_color = sys.stdout.isatty()
    prefix = f"\033[{ansi_color}m" if use_color else ""
    suffix = "\033[0m" if use_color else ""
    with _PRINT_LOCK:
        print(f"\n{prefix}{sep}\n>> {title}\n{sep}{suffix}", flush=True)


def print_status(tag: str, message: str, ansi_color: str) -> None:
    """Print a colored status tag line ([PASS], [FAIL], [INFO], [OK])."""
    with _PRINT_LOCK:
        if sys.stdout.isatty():
            print(f"\033[{ansi_color}m[{tag}]\033[0m {message}", flush=True)
        else:
            print(f"[{tag}] {message}", flush=True)
