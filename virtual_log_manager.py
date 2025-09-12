"""
Virtual Log Manager - Memory-optimized log manager
Streaming system with LRU cache to handle multi-GB log files
"""
import os
from typing import List, Optional, Dict, Any, Tuple
from datetime import datetime
import threading
from collections import OrderedDict

from log_parser import OPNsenseLogParser, LogEntry

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
    
    def __init__(self, chunk_size: int = 1000, cache_size: int = 50, log_parser=None):
        self.chunk_size = chunk_size  # Number of entries per chunk
        self.cache = LRUCache(cache_size)
        self.log_parser = log_parser if log_parser else OPNsenseLogParser()
        self.file_index = None
        self.current_file = None
        self.total_entries = 0
        self.filtered_indices = []  # Indices of entries after filtering
        self.is_filtered = False
        
    def load_file(self, file_path: str, progress_callback=None):
        """Loads a log file (indexing only)"""
        self.current_file = file_path
        self.cache.clear()
        self.filtered_indices = []
        self.is_filtered = False
        
        # Build the file index
        if progress_callback:
            progress_callback("Building file index...")
            
        self.file_index = LogFileIndex(file_path)
        self.file_index.build_index(progress_callback)
        self.total_entries = self.file_index.total_lines
        
        if progress_callback:
            progress_callback(f"File indexed: {self.total_entries:,} lines ready for streaming")
    
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
        if self.is_filtered:
            return self._get_filtered_entries(start_index, count)
        else:
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
    
    def _get_filtered_entries(self, start_index: int, count: int) -> List[LogEntry]:
        """Retrieves filtered entries with enrichment"""
        if start_index >= len(self.filtered_indices):
            return []
            
        end_index = min(start_index + count, len(self.filtered_indices))
        result_entries = []
        
        # Use the standard get_chunk method to maintain compatibility
        # and ensure all enrichment features work
        for i in range(start_index, end_index):
            original_index = self.filtered_indices[i]
            chunk_id = original_index // self.chunk_size
            chunk_offset = original_index % self.chunk_size
            
            chunk = self.get_chunk(chunk_id)
            if chunk and chunk_offset < len(chunk):
                result_entries.append(chunk[chunk_offset])
        
        return result_entries
    
    def apply_filter(self, filter_func, progress_callback=None, use_parallel=True):
        """Apply filter and build filtered entries index"""
        self.filtered_indices = []
        
        # Skip multiprocessing for Label filters (use_parallel=False)
        # Go directly to threading for better performance
        if not use_parallel:
            if hasattr(self, '_apply_threaded_filter'):
                self._apply_threaded_filter(filter_func, progress_callback)
            else:
                self._apply_sequential_filter(filter_func, progress_callback)
            return
        
        # Use multiprocessing for regular filters
        try:
            from parallel_filter import ParallelLogFilter
            parallel_filter = ParallelLogFilter()
            self.filtered_indices = parallel_filter.apply_filter_parallel(
                self, filter_func, progress_callback
            )
            self.is_filtered = True
            return
        except Exception as e:
            if progress_callback:
                progress_callback("Using threading...")
            # Fallback to threading
            if hasattr(self, '_apply_threaded_filter'):
                self._apply_threaded_filter(filter_func, progress_callback)
            else:
                self._apply_sequential_filter(filter_func, progress_callback)
            
    def _apply_threaded_filter(self, filter_func, progress_callback=None):
        """High-performance threaded filtering to improve performance"""
        import concurrent.futures
        import threading
        import os
        
        # Intelligent detection of optimal thread count
        def get_optimal_thread_count():
            """Determines the optimal number of threads based on system resources"""
            try:
                # Number of logical cores (with hyperthreading)
                logical_cores = os.cpu_count() or 4
                
                # For I/O-bound threading, we can be more aggressive
                if logical_cores >= 16:
                    # Powerful machine: use almost all cores
                    return max(12, logical_cores - 2)  # Keep only 2 cores for the system
                elif logical_cores >= 8:
                    # Mid-high machine: use most cores
                    return max(6, logical_cores - 1)  # Keep 1 core for the system
                elif logical_cores >= 4:
                    # Average machine: use all except 1
                    return max(3, logical_cores - 1)
                else:
                    # Limited machine: use all cores
                    return logical_cores
                    
            except Exception as e:
                print(f"Resource detection error: {e}")
                return 4  # Safe fallback
        
        optimal_threads = get_optimal_thread_count()
        
        # Load entire file into memory once to avoid I/O conflicts
        if progress_callback:
            progress_callback("Loading file...")
            
        # Read entire file at once
        try:
            with open(self.current_file, 'r', encoding='utf-8', errors='ignore') as f:
                all_lines = f.readlines()
        except Exception as e:
            if progress_callback:
                progress_callback(f"Read error: {e}")
            return
        
        # Optimize work distribution
        lines_per_thread = max(50, len(all_lines) // optimal_threads)
        max_threads = min(optimal_threads, (len(all_lines) + lines_per_thread - 1) // lines_per_thread)
        
        if progress_callback:
            progress_callback(f"Filtering with {max_threads} threads...")
        
        def process_lines_batch(start_idx, end_idx):
            """Processes a batch of lines in a thread"""
            local_indices = []
            # Import locally to avoid conflicts
            from log_parser import OPNsenseLogParser
            local_parser = OPNsenseLogParser()  # Local parser to avoid conflicts
            
            try:
                for line_idx in range(start_idx, min(end_idx, len(all_lines))):
                    line = all_lines[line_idx].strip()
                    if line:
                        try:
                            entry = local_parser.parse_log_line(line)
                            if entry and filter_func(entry):
                                local_indices.append(line_idx)
                        except Exception:
                            pass  # Ignore unparseable lines
                
                return local_indices
            except Exception as e:
                print(f"Error processing lines {start_idx}-{end_idx}: {e}")
                return []
        
        # Variables for progress tracking
        completed_batches = 0
        total_batches = max_threads
        progress_lock = threading.Lock()
        
        def update_progress():
            nonlocal completed_batches
            with progress_lock:
                completed_batches += 1
                if progress_callback:
                    progress = (completed_batches / total_batches) * 100
                    progress_callback(f"Filtering: {progress:.0f}%")
        
        # High-performance threaded processing
        batch_futures = []
        with concurrent.futures.ThreadPoolExecutor(max_workers=max_threads) as executor:
            # Create work batches
            for i in range(max_threads):
                start_idx = i * lines_per_thread
                end_idx = min(start_idx + lines_per_thread, len(all_lines))
                if start_idx < len(all_lines):
                    future = executor.submit(process_lines_batch, start_idx, end_idx)
                    batch_futures.append(future)
            
            # Collect results
            for future in concurrent.futures.as_completed(batch_futures):
                batch_indices = future.result()
                self.filtered_indices.extend(batch_indices)
                update_progress()
        
        # Sort indices to maintain line order
        self.filtered_indices.sort()
        self.is_filtered = True
        
        if progress_callback:
            progress_callback(f"Found {len(self.filtered_indices):,} matches")
    
    def _apply_sequential_filter(self, filter_func, progress_callback=None):
        """Sequential fallback method"""
        total_chunks = (self.total_entries + self.chunk_size - 1) // self.chunk_size
        processed_entries = 0
        
        for chunk_id in range(total_chunks):
            if progress_callback:
                progress = (chunk_id / total_chunks) * 100
                progress_callback(f"Filtering: {progress:.0f}%")
            
            chunk = self.get_chunk(chunk_id)
            chunk_start_index = chunk_id * self.chunk_size
            
            for i, entry in enumerate(chunk):
                if filter_func(entry):
                    self.filtered_indices.append(chunk_start_index + i)
                processed_entries += 1
        
        self.is_filtered = True
        
        if progress_callback:
            progress_callback(f"Found {len(self.filtered_indices):,} matches")
    
    def clear_filter(self):
        """Removes the filter"""
        self.filtered_indices = []
        self.is_filtered = False
    
    def get_total_entries(self) -> int:
        """Returns the total number of entries (filtered or not)"""
        if self.is_filtered:
            return len(self.filtered_indices)
        return self.total_entries
    
    def get_memory_info(self) -> Dict[str, Any]:
        """Returns memory usage information"""
        cache_info = self.cache.get_memory_info()
        return {
            'total_file_entries': self.total_entries,
            'filtered_entries': len(self.filtered_indices) if self.is_filtered else 0,
            'cache_info': cache_info,
            'chunk_size': self.chunk_size,
            'estimated_total_memory_mb': cache_info['estimated_memory_mb']
        }
    
    def set_interface_mapping(self, mapping: Dict[str, str]):
        """Configures interface mapping"""
        # The parser is now shared with main_app, no need to configure it here
        # Clear cache because entries must be re-parsed with the new mapping
        self.cache.clear()
