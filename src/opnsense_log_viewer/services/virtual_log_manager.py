"""
Virtual Log Manager - serves log pages without loading the file in memory.

The raw (unfiltered) view and the filtered view are both backed by the DuckDB
engine's Parquet cache once its one-time conversion is done: pages and exact
counts come straight from the cache in milliseconds, and opening a file no
longer scans it at all (the old Python line-offset index read the whole file
once and the cache conversion read it a second time).

Until the cache is ready (or if it can never be), the raw view is served by
``SequentialRawView``: a lazy reader that parses the file from the top only as
far as the user actually browses, remembering a sparse offset every ``stride``
entries so revisiting earlier pages stays O(1). The total entry count is only
known once the cache is ready; ``total_is_exact`` tells the UI apart.
"""
import threading
from typing import List, Optional

from opnsense_log_viewer.services.log_parser import OPNsenseLogParser, LogEntry


class SequentialRawView:
    """Raw-view pages read lazily from the log file (pre-cache fallback).

    Counts *valid* entries (``parse_log_line`` accepts them), exactly like the
    Parquet cache's validity gate, so pagination does not jump when the view
    switches to the cache.
    """

    def __init__(self, file_path: str, log_parser, stride: int = 1000):
        self.file_path = file_path
        self.log_parser = log_parser
        self.stride = stride
        # _offsets[k] = byte offset of the line holding valid entry k*stride
        self._offsets = [0]
        self._known_entries = 0   # entries seen so far (== scan frontier)
        self._exhausted = False   # EOF reached -> _known_entries is the total
        self._lock = threading.Lock()

    @property
    def known_entries(self) -> int:
        return self._known_entries

    @property
    def is_complete(self) -> bool:
        return self._exhausted

    def get_entries(self, start: int, count: int) -> List[LogEntry]:
        """Return up to ``count`` valid entries starting at index ``start``,
        extending the scan frontier if the range was never visited."""
        if count <= 0 or start < 0:
            return []
        with self._lock:
            return self._read_range(start, count)

    def _read_range(self, start: int, count: int) -> List[LogEntry]:
        block = min(start // self.stride, len(self._offsets) - 1)
        entry_idx = block * self.stride
        end = start + count
        entries: List[LogEntry] = []
        with open(self.file_path, 'rb') as f:
            f.seek(self._offsets[block])
            offset = self._offsets[block]
            while entry_idx < end:
                raw = f.readline()
                if not raw:
                    self._exhausted = True
                    break
                line = raw.decode('utf-8', errors='ignore').strip()
                entry = self.log_parser.parse_log_line(line)
                offset_after = offset + len(raw)
                if entry is not None:
                    if entry_idx % self.stride == 0:
                        k = entry_idx // self.stride
                        if k == len(self._offsets):
                            self._offsets.append(offset)
                    if entry_idx >= start:
                        entries.append(entry)
                    entry_idx += 1
                offset = offset_after
            self._known_entries = max(self._known_entries, entry_idx)
        return entries


class VirtualLogManager:
    """Serves pages of a (possibly multi-GB) log file with bounded memory."""

    def __init__(self, log_parser=None, duckdb_cache_dir: Optional[str] = None):
        self.log_parser = log_parser if log_parser else OPNsenseLogParser()
        self.current_file = None
        self.is_filtered = False  # True when a filter is materialized in the engine
        self.duckdb_engine = None  # DuckDB fast filter engine
        # None -> the engine's default (%LOCALAPPDATA%); tests point it elsewhere.
        self.duckdb_cache_dir = duckdb_cache_dir
        self._raw_view: Optional[SequentialRawView] = None
        # Serializes engine creation/teardown: the load thread and the filter
        # thread can both reach "no engine yet, create one" concurrently.
        self._engine_lock = threading.Lock()

    def load_file(self, file_path: str, progress_callback=None,
                  cache_status_callback=None):
        """Open a log file. Near-instant: no upfront scan of the file.

        The one-time DuckDB Parquet conversion starts right away in the
        background; until it finishes the raw view is read lazily from the
        file. ``cache_status_callback`` (optional) receives the cache progress
        strings and keeps being called (from a background thread) after this
        method has returned.
        """
        self.current_file = file_path
        self.is_filtered = False

        # A new file invalidates any previously opened engine.
        self.shutdown_duckdb_engine()

        if progress_callback:
            progress_callback("Opening file...")

        self._raw_view = SequentialRawView(file_path, self.log_parser)
        # Fail fast on an unreadable file (permission, vanished path...):
        # priming the first page raises here, inside the caller's try.
        self._raw_view.get_entries(0, 1)

        # Kick off the one-time DuckDB Parquet cache conversion right away, in
        # the background: it is the source of the full pagination, the exact
        # count and near-instant filters. Failure here is not fatal (the lazy
        # raw view keeps working and filters fall back to the direct scan).
        self._init_duckdb_engine(cache_status_callback)

    def _init_duckdb_engine(self, cache_status_callback=None):
        """Create the DuckDB engine for the current file and start its cache."""
        try:
            self.ensure_duckdb_engine(cache_status_callback)
        except Exception:
            # Optional accelerator only; apply_filter_duckdb retries lazily.
            pass

    def ensure_duckdb_engine(self, cache_status_callback=None):
        """Create (at most once, thread-safe) and return the DuckDB engine."""
        from opnsense_log_viewer.services.duckdb_filter import DuckDBLogFilter
        with self._engine_lock:
            if self.duckdb_engine is None and self.current_file:
                if not DuckDBLogFilter.is_available():
                    return None
                engine = DuckDBLogFilter(
                    self.current_file, cache_dir=self.duckdb_cache_dir)
                engine.start_cache_build(cache_status_callback)
                self.duckdb_engine = engine
            return self.duckdb_engine

    def shutdown_duckdb_engine(self):
        """Detach and close the engine, safe against a concurrent creation."""
        with self._engine_lock:
            engine = self.duckdb_engine
            self.duckdb_engine = None
        if engine is not None:
            engine.close()

    def get_entries(self, start_index: int, count: int) -> List[LogEntry]:
        """Retrieves a range of entries (filtered or raw view)"""
        if self.is_filtered and self.duckdb_engine is not None:
            return self.duckdb_engine.fetch_page(
                start_index, count, self.log_parser.interface_mapping)
        engine = self.duckdb_engine
        if engine is not None:
            page = engine.browse_page(
                start_index, count, self.log_parser.interface_mapping)
            if page is not None:
                return page
        if self._raw_view is not None:
            return self._raw_view.get_entries(start_index, count)
        return []

    def apply_filter_duckdb(self, log_filter, label_descriptions=None,
                            progress_callback=None, engine=None):
        """Apply a filter via the DuckDB engine (no persistent build).

        Scans the Parquet cache (or the raw file when the cache is
        unavailable) and materializes only the matching rows. Sets
        ``is_filtered`` so the display/export path pulls pages from the
        engine's matches table.

        ``engine`` lets the caller pass the exact engine it already gated on (see
        the GUI's wait-for-cache loop): reusing it guarantees the filter and the
        gate act on the same object, instead of resolving a possibly-new engine
        that would build and direct-scan concurrently.
        """
        if not self.current_file:
            return

        # Clear the flag up front so that if build_matches raises, the view falls
        # back to unfiltered instead of serving a previous filter's stale matches.
        self.is_filtered = False

        # Local reference: a concurrent shutdown (file switch, cancelled load)
        # nulls the attribute, which must not crash a build already underway.
        if engine is None:
            engine = self.ensure_duckdb_engine()
        if engine is None:
            raise ImportError(
                "duckdb is required for the fast filter engine "
                "(pip install duckdb)")

        if progress_callback:
            if engine.cache_ready:
                progress_callback("Filtering with DuckDB (optimized cache)...")
            else:
                progress_callback("Filtering with DuckDB (scanning file)...")

        count = engine.build_matches(
            log_filter.expression,
            (log_filter.time_range_start, log_filter.time_range_end),
            label_descriptions or {},
            self.log_parser.interface_mapping,
        )

        self.is_filtered = True

        if progress_callback:
            progress_callback(f"Found {count:,} matches")

    def clear_filter(self):
        """Removes the filter"""
        self.is_filtered = False

    @property
    def total_entries(self) -> int:
        """Unfiltered count of valid entries.

        Exact once the Parquet cache is ready; before that, the number of
        entries the lazy raw view has discovered so far (see total_is_exact).
        """
        engine = self.duckdb_engine
        if engine is not None:
            n = engine.row_count()
            if n is not None:
                return n
        if self._raw_view is not None:
            return self._raw_view.known_entries
        return 0

    @property
    def total_is_exact(self) -> bool:
        """False while the total is still a moving frontier (cache building)."""
        if self.is_filtered and self.duckdb_engine is not None:
            return True
        engine = self.duckdb_engine
        if engine is not None and engine.row_count() is not None:
            return True
        if self._raw_view is not None:
            return self._raw_view.is_complete
        return True

    def get_total_entries(self) -> int:
        """Returns the total number of entries (filtered or not)"""
        if self.is_filtered and self.duckdb_engine is not None:
            return self.duckdb_engine.match_count
        return self.total_entries
