"""
Engine lifecycle controller.

Creation, cache-readiness waiting, cancellation and teardown of the DuckDB
engine live here, in one place. Before this, the lifecycle was spread over
ad-hoc flags (the GUI's is_loading, each ProgressDialog's .cancelled, the
engine's is_closed, and a wait loop coded inline in the filter worker); a
``CancellationToken`` now carries "stop this operation" explicitly, and
``wait_cache_ready`` is the single gate used by anything that needs the cache.
"""
import threading
import time
from typing import Callable, Optional


class CancellationToken:
    """Cooperative cancellation for one operation (a load, a filter...).

    ``cancel()`` flips it exactly once; workers poll ``cancelled`` or block on
    ``wait()``, which doubles as a cancellation-aware sleep.
    """

    def __init__(self):
        self._event = threading.Event()

    def cancel(self) -> None:
        self._event.set()

    @property
    def cancelled(self) -> bool:
        return self._event.is_set()

    def wait(self, timeout: float) -> bool:
        """Sleep up to ``timeout`` seconds, waking early on cancellation.
        Returns True when the token is cancelled."""
        return self._event.wait(timeout)


class EngineController:
    """Owns the DuckDB engine of the currently open file.

    Exactly one engine is alive at a time: ``open`` replaces (and closes) the
    previous one, ``ensure`` creates lazily, ``close`` tears down. All three
    are safe to call from any thread.
    """

    def __init__(self, cache_dir: Optional[str] = None):
        self._cache_dir = cache_dir
        self._engine = None
        self._lock = threading.Lock()

    @property
    def engine(self):
        """The current engine, or None (unavailable or nothing open)."""
        return self._engine

    def ensure(self, file_path: str, on_status: Optional[Callable] = None):
        """Return the current engine, creating one for ``file_path`` (and
        starting its cache build) if none exists. None when DuckDB is
        unavailable."""
        from opnsense_log_viewer.services.duckdb_filter import DuckDBLogFilter
        with self._lock:
            if self._engine is None:
                if not DuckDBLogFilter.is_available():
                    return None
                engine = DuckDBLogFilter(file_path, cache_dir=self._cache_dir)
                engine.start_cache_build(on_status)
                self._engine = engine
            return self._engine

    def open(self, file_path: str, on_status: Optional[Callable] = None):
        """Close any previous engine and open a fresh one for ``file_path``."""
        self.close()
        return self.ensure(file_path, on_status)

    def close(self) -> None:
        """Detach and close the engine, safe against a concurrent creation."""
        with self._lock:
            engine, self._engine = self._engine, None
        if engine is not None:
            engine.close()

    def wait_cache_ready(self, token: Optional[CancellationToken] = None,
                         on_progress: Optional[Callable] = None,
                         poll_interval: float = 0.5,
                         engine=None) -> str:
        """Block until the engine's Parquet cache is settled or the operation
        is cancelled, reporting build progress meanwhile.

        ``engine`` pins the exact engine the caller will use afterwards
        (default: the current one); a file switch mid-wait then shows up as
        'closed' instead of silently latching onto the new file's engine.

        Returns one of:
          'ready'       cache usable, filters are near-instant
          'failed'      cache can no longer become ready; direct scan, alone
          'closed'      the engine was torn down mid-wait (stale operation)
          'cancelled'   the token was cancelled
          'unavailable' there is no engine at all
        ``on_progress`` receives the build fraction (0-1 float, or None when
        unknown) on every poll.
        """
        if engine is None:
            engine = self._engine
        if engine is None:
            return 'unavailable'
        while True:
            if token is not None and token.cancelled:
                return 'cancelled'
            if engine.is_closed:
                return 'closed'
            if engine.cache_ready:
                return 'ready'
            if engine.cache_failed or not engine.cache_building:
                # The build may have published between the two reads above.
                return 'ready' if engine.cache_ready else 'failed'
            if on_progress:
                on_progress(engine.cache_progress())
            if token is not None:
                if token.wait(poll_interval):
                    return 'cancelled'
            else:
                time.sleep(poll_interval)
