"""Thread-safe output flushing and fail-fast concurrent worker pool."""

from __future__ import annotations

from concurrent.futures import FIRST_EXCEPTION, Future, ThreadPoolExecutor, wait
import threading
from typing import Callable, Sequence, TypeVar

from scripts.panadiga_ops import ProcessTracker, use_process_tracker

_T = TypeVar("_T")
_FLUSH_LOCK = threading.Lock()


def flush_output(text: str) -> None:
    """Print buffered child output atomically under a process-wide lock."""
    if text:
        with _FLUSH_LOCK:
            print(text, end="" if text.endswith("\n") else "\n", flush=True)


def run_concurrent_tasks(
    tasks: Sequence[tuple[str, Callable[[], _T]]],
) -> dict[str, _T]:
    """Run independent callables concurrently with fail-fast child cancellation."""
    if not tasks:
        return {}
    if len(tasks) == 1:
        name, fn = tasks[0]
        return {name: fn()}

    tracker = ProcessTracker()

    def _wrapped(fn: Callable[[], _T]) -> _T:
        with use_process_tracker(tracker):
            return fn()

    results: dict[str, _T] = {}
    with ThreadPoolExecutor(max_workers=len(tasks)) as pool:
        fmap: dict[Future[_T], str] = {
            pool.submit(_wrapped, fn): name for name, fn in tasks
        }
        pending: set[Future[_T]] = set(fmap)
        try:
            while pending:
                done, pending = wait(pending, return_when=FIRST_EXCEPTION)
                for fut in done:
                    name = fmap[fut]
                    try:
                        results[name] = fut.result()
                    except BaseException:
                        for p in pending:
                            p.cancel()
                        tracker.terminate_all()
                        raise
        except KeyboardInterrupt:
            for p in pending:
                p.cancel()
            tracker.terminate_all()
            raise
    return results
