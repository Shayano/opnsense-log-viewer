"""
Virtual Log Manager - Memory-optimized log manager
Streaming system with LRU cache to handle multi-GB log files
"""
import os
from typing import List, Optional, Dict, Any, Tuple
from datetime import datetime
import threading
from collections import OrderedDict

from opnsense_log_viewer.services.log_parser import OPNsenseLogParser, LogEntry

class LRUCache:
    """Simple LRU cache for log chunks"""
    
    def __init__(self, max_size: int = 50):  # 50 chunks max in memory
        self.max_size = max_size
        self.cache = OrderedDict()
        self.lock = threading.Lock()
    
    def get(self, key: str) -> Optional[List[LogEntry]]:
        """Retrieves a chunk from cache"""
        with self.lock:
            if key in self.cache:
                # Move to end (recently used)
                self.cache.move_to_end(key)
                return self.cache[key]
        return None
    
    def put(self, key: str, value: List[LogEntry]):
        """Adds a chunk to cache"""
        with self.lock:
            if key in self.cache:
                self.cache.move_to_end(key)
            else:
                self.cache[key] = value
                if len(self.cache) > self.max_size:
                    # Remove the oldest
                    self.cache.popitem(last=False)
    
    def clear(self):
        """Clears the cache"""
        with self.lock:
            self.cache.clear()
    
    def get_memory_info(self) -> Dict[str, Any]:
        """Returns memory usage information"""
        with self.lock:
            total_entries = sum(len(chunk) for chunk in self.cache.values())
            return {
                'chunks_in_memory': len(self.cache),
                'total_entries_cached': total_entries,
                'estimated_memory_mb': total_entries * 0.5 / 1024  # ~0.5KB per entry
            }

class LogFileIndex:
    """Index of a log file for fast line access"""
    
    def __init__(self, file_path: str):
        self.file_path = file_path
        self.line_offsets = []  # Offset of each line in the file
        self.total_lines = 0
        self.file_size = 0
        self.index_built = False
        self.lock = threading.Lock()
        
    def build_index(self, progress_callback=None):
        """Builds the line position index"""
        if self.index_built:
            return
            
        with self.lock:
            if self.index_built:  # Double-check
                return
                
            self.line_offsets = [0]  # First line starts at 0
            self.file_size = os.path.getsize(self.file_path)
            
            with open(self.file_path, 'rb') as f:
                offset = 0
                line_count = 0
                
                while True:
                    line = f.readline()
                    if not line:
                        break
                    
                    offset += len(line)
                    self.line_offsets.append(offset)
                    line_count += 1
                    
                    if progress_callback and line_count % 10000 == 0:
                        progress_callback(f"Indexing: {line_count:,} lines processed...")
                
                self.total_lines = line_count
                self.index_built = True
                
                if progress_callback:
                    progress_callback(f"Index complete: {self.total_lines:,} lines")
    
    def get_line_range(self, start_line: int, count: int) -> Tuple[int, int]:
        """Returns the start offset and size for a line range"""
        if not self.index_built or start_line >= self.total_lines:
            return (0, 0)
            
        end_line = min(start_line + count, self.total_lines)
        start_offset = self.line_offsets[start_line]
        end_offset = self.line_offsets[end_line] if end_line < len(self.line_offsets) else self.file_size
        
        return (start_offset, end_offset - start_offset)

class VirtualLogManager:
    """Virtual log manager with optimized memory"""
    
    def __init__(self, chunk_size: int = 1000, cache_size: int = 50, log_parser=None,
                 duckdb_cache_dir: Optional[str] = None):
        self.chunk_size = chunk_size  # Number of entries per chunk
        self.cache = LRUCache(cache_size)
        self.log_parser = log_parser if log_parser else OPNsenseLogParser()
        self.file_index = None
        self.current_file = None
        self.total_entries = 0
        self.is_filtered = False  # True when a filter is materialized in the engine
        self.duckdb_engine = None  # DuckDB fast filter engine
        # None -> the engine's default (%LOCALAPPDATA%); tests point it elsewhere.
        self.duckdb_cache_dir = duckdb_cache_dir
        # Serializes engine creation/teardown: the load thread and the filter
        # thread can both reach "no engine yet, create one" concurrently.
        self._engine_lock = threading.Lock()

    def load_file(self, file_path: str, progress_callback=None,
                  cache_status_callback=None):
        """Loads a log file (indexing only)

        ``cache_status_callback`` (optional) receives the DuckDB Parquet cache
        progress strings; unlike ``progress_callback`` it keeps being called
        (from a background thread) after this method has returned.
        """
        self.current_file = file_path
        self.cache.clear()
        self.is_filtered = False

        # A new file invalidates any previously opened engine.
        self.shutdown_duckdb_engine()

        # Build the file index
        if progress_callback:
            progress_callback("Building file index...")

        self.file_index = LogFileIndex(file_path)
        self.file_index.build_index(progress_callback)
        self.total_entries = self.file_index.total_lines

        if progress_callback:
            progress_callback(f"File indexed: {self.total_entries:,} lines ready for streaming")

        # Kick off the one-time DuckDB Parquet cache conversion right away, in
        # the background: by the time the user has composed a filter it is often
        # already done, and every filter then runs in ~1-4 s instead of a full
        # multi-GB scan. Failure here is harmless (direct scan keeps working).
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
    
    def get_chunk(self, chunk_id: int) -> List[LogEntry]:
        """Retrieves a chunk of logs (with cache)"""
        if not self.file_index or not self.file_index.index_built:
            return []
            
        cache_key = f"{self.current_file}_{chunk_id}"
        
        # Check cache
        cached_chunk = self.cache.get(cache_key)
        if cached_chunk is not None:
            return cached_chunk
        
        # Load chunk from file
        start_line = chunk_id * self.chunk_size
        if start_line >= self.file_index.total_lines:
            return []
            
        chunk_entries = []
        start_offset, size = self.file_index.get_line_range(start_line, self.chunk_size)
        
        if size > 0:
            with open(self.current_file, 'r', encoding='utf-8', errors='ignore') as f:
                f.seek(start_offset)
                lines_read = 0
                
                while lines_read < self.chunk_size:
                    line = f.readline()
                    if not line:
                        break
                        
                    # Parse the line
                    entry = self.log_parser.parse_log_line(line.strip())
                    if entry:
                        chunk_entries.append(entry)
                    lines_read += 1
        
        # Cache it
        self.cache.put(cache_key, chunk_entries)
        return chunk_entries
    
    def get_entries(self, start_index: int, count: int) -> List[LogEntry]:
        """Retrieves a range of entries (can span multiple chunks)"""
        if self.is_filtered and self.duckdb_engine is not None:
            return self.duckdb_engine.fetch_page(
                start_index, count, self.log_parser.interface_mapping)
        return self._get_raw_entries(start_index, count)
    
    def _get_raw_entries(self, start_index: int, count: int) -> List[LogEntry]:
        """Retrieves raw entries (unfiltered)"""
        entries = []
        current_index = start_index
        remaining = count
        
        while remaining > 0 and current_index < self.total_entries:
            chunk_id = current_index // self.chunk_size
            chunk_offset = current_index % self.chunk_size
            
            chunk = self.get_chunk(chunk_id)
            if not chunk:
                break
                
            # Take what we can from this chunk
            take_count = min(remaining, len(chunk) - chunk_offset)
            entries.extend(chunk[chunk_offset:chunk_offset + take_count])
            
            current_index += take_count
            remaining -= take_count
        
        return entries
    
    def apply_filter_duckdb(self, log_filter, label_descriptions=None,
                            progress_callback=None, engine=None):
        """Apply a filter via the DuckDB engine (no persistent build).

        Scans the raw file once with DuckDB's compiled multi-threaded CSV engine
        and materializes only the matching rows. Sets ``is_filtered`` so the
        display/export path pulls pages straight from DuckDB instead of mapping
        line numbers. Any filter type completes in seconds even on multi-GB files.

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

    def get_total_entries(self) -> int:
        """Returns the total number of entries (filtered or not)"""
        if self.is_filtered and self.duckdb_engine is not None:
            return self.duckdb_engine.match_count
        return self.total_entries
    
    def get_memory_info(self) -> Dict[str, Any]:
        """Returns memory usage information"""
        cache_info = self.cache.get_memory_info()
        return {
            'total_file_entries': self.total_entries,
            'filtered_entries': self.get_total_entries() if self.is_filtered else 0,
            'cache_info': cache_info,
            'chunk_size': self.chunk_size,
            'estimated_total_memory_mb': cache_info['estimated_memory_mb']
        }
    
    def set_interface_mapping(self, mapping: Dict[str, str]):
        """Configures interface mapping"""
        # The parser is now shared with main_app, no need to configure it here
        # Clear cache because entries must be re-parsed with the new mapping
        self.cache.clear()
