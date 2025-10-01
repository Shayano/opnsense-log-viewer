"""
Parallel filtering system for OPNsense Log Viewer
Uses all CPU cores to accelerate log filtering
"""
import multiprocessing as mp
import sys
import threading
import time
import os
from typing import List, Callable, Dict, Any, Tuple
from concurrent.futures import ProcessPoolExecutor, as_completed
import pickle

from opnsense_log_viewer.services.log_parser import LogEntry

# Fix for PyInstaller multiprocessing
if __name__ == '__main__':
    mp.freeze_support()

def filter_chunk_data(args):
    """Filters a data chunk (module-level function for multiprocessing compatibility)"""
    try:
        file_path, start_offset, size, chunk_id, filter_serialized, parser_mapping, rule_labels_mapping = args

        # Deserialize the filtering function
        filter_func = pickle.loads(filter_serialized)

        # Local import to avoid multiprocessing issues
        from opnsense_log_viewer.services.log_parser import OPNsenseLogParser

        # Create a local parser with mapping
        local_parser = OPNsenseLogParser()
        if parser_mapping:
            local_parser.set_interface_mapping(parser_mapping)

        filtered_indices = []
        processed_count = 0
        chunk_start_index = chunk_id * 1000  # Default chunk size

        # Read and parse the chunk
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            f.seek(start_offset)
            bytes_read = 0
            entry_index = 0

            while bytes_read < size:
                line = f.readline()
                if not line:
                    break

                bytes_read += len(line.encode('utf-8'))

                # Parse the line
                entry = local_parser.parse_log_line(line.strip())
                if entry:
                    # Add label mapping if available
                    if rule_labels_mapping and hasattr(entry, 'parsed_data') and 'rid' in entry.parsed_data:
                        rid = str(entry.parsed_data['rid'])
                        if rid in rule_labels_mapping:
                            entry.parsed_data['__rule_label__'] = rule_labels_mapping[rid]
                        else:
                            entry.parsed_data['__rule_label__'] = ""
                    else:
                        entry.parsed_data['__rule_label__'] = ""

                    # Apply the filter
                    if filter_func(entry):
                        filtered_indices.append(chunk_start_index + entry_index)
                    entry_index += 1
                    processed_count += 1

        return {
            'chunk_id': chunk_id,
            'filtered_indices': filtered_indices,
            'processed_count': processed_count,
            'success': True
        }

    except Exception as e:
        return {
            'chunk_id': chunk_id,
            'error': str(e),
            'success': False
        }

class ParallelLogFilter:
    """Optimized parallel filtering manager"""
    
    def __init__(self, max_workers=None):
        self.max_workers = max_workers or get_max_parallel_workers()
        self.chunk_size = 1000  # Entries per chunk
        
    def apply_filter_parallel(self, virtual_log_manager, filter_func, progress_callback=None, rule_labels_mapping=None):
        """Applies a filter in parallel across all cores"""

        if not virtual_log_manager.current_file or not virtual_log_manager.file_index:
            return []

        start_time = time.time()

        # Prepare chunks for parallel processing
        file_path = virtual_log_manager.current_file
        total_entries = virtual_log_manager.total_entries
        total_chunks = (total_entries + self.chunk_size - 1) // self.chunk_size

        if progress_callback:
            progress_callback(f"Preparing {self.max_workers} cores...")

        # Prepare arguments for each worker
        worker_args = []
        parser_mapping = virtual_log_manager.log_parser.interface_mapping

        # Serialize the filtering function
        try:
            filter_serialized = pickle.dumps(filter_func)
        except Exception as e:
            if progress_callback:
                progress_callback(f"Error: Cannot serialize filter function: {e}")
            return self._fallback_sequential_filter(virtual_log_manager, filter_func, progress_callback)
        
        for chunk_id in range(total_chunks):
            start_line = chunk_id * self.chunk_size
            if start_line >= virtual_log_manager.file_index.total_lines:
                continue
                
            start_offset, size = virtual_log_manager.file_index.get_line_range(start_line, self.chunk_size)
            if size > 0:
                worker_args.append((
                    file_path,
                    start_offset,
                    size,
                    chunk_id,
                    filter_serialized,
                    parser_mapping,
                    rule_labels_mapping
                ))
        
        if progress_callback:
            progress_callback(f"Starting parallel processing of {len(worker_args)} chunks...")
        
        # Parallel processing
        all_filtered_indices = []
        total_processed = 0
        completed_chunks = 0
        
        try:
            # Use ProcessPoolExecutor for true parallelism
            with ProcessPoolExecutor(max_workers=self.max_workers) as executor:
                # Submit all jobs
                future_to_chunk = {
                    executor.submit(filter_chunk_data, args): args[3]
                    for args in worker_args
                }
                
                # Process results as they come
                for future in as_completed(future_to_chunk):
                    chunk_id = future_to_chunk[future]
                    
                    try:
                        result = future.result()
                        
                        if result['success']:
                            all_filtered_indices.extend(result['filtered_indices'])
                            total_processed += result['processed_count']
                        else:
                            if progress_callback:
                                progress_callback(f"Error in chunk {chunk_id}: {result.get('error', 'Unknown error')}")
                        
                        completed_chunks += 1
                        
                        if progress_callback:
                            progress = (completed_chunks / len(worker_args)) * 100
                            progress_callback(f"Filtering: {progress:.0f}%")
                    
                    except Exception as e:
                        if progress_callback:
                            progress_callback(f"Error processing chunk {chunk_id}: {e}")
                        completed_chunks += 1
        
        except Exception as e:
            if progress_callback:
                progress_callback(f"Parallel processing failed: {e}")
            # Fallback to sequential method
            return self._fallback_sequential_filter(virtual_log_manager, filter_func, progress_callback)
        
        # Sort indices (important for display order)
        all_filtered_indices.sort()
        
        if progress_callback:
            progress_callback(f"Found {len(all_filtered_indices):,} matches")
        
        return all_filtered_indices
    
    def _fallback_sequential_filter(self, virtual_log_manager, filter_func, progress_callback=None):
        """Threading fallback method for frozen executables"""
        if progress_callback:
            progress_callback("Using threading mode...")

        # Call threading method directly to avoid recursion
        if hasattr(virtual_log_manager, '_apply_threaded_filter'):
            virtual_log_manager._apply_threaded_filter(filter_func, progress_callback)
        else:
            virtual_log_manager._apply_sequential_filter(filter_func, progress_callback)

        virtual_log_manager.is_filtered = True
        return virtual_log_manager.filtered_indices

class OptimizedFilterFunction:
    """Wrapper to optimize filtering functions"""
    
    def __init__(self, log_filter, time_filter_enabled=False, time_range_start=None, time_range_end=None):
        self.conditions = log_filter.expression.conditions
        self.operators = log_filter.expression.operators
        self.negations = log_filter.expression.negations
        self.time_filter_enabled = time_filter_enabled
        self.time_range_start = time_range_start
        self.time_range_end = time_range_end
    
    def __call__(self, entry):
        """Evaluates the entry in an optimized way"""
        # Time filter first (faster)
        if self.time_filter_enabled:
            if self.time_range_start and entry.timestamp < self.time_range_start:
                return False
            if self.time_range_end and entry.timestamp > self.time_range_end:
                return False
        
        # Advanced filters only if necessary
        if not self.conditions:
            return True
        
        # Optimized evaluation without unnecessary copies
        return self._evaluate_conditions_fast(entry)
    
    def _evaluate_conditions_fast(self, entry):
        """Optimized evaluation of conditions"""
        if not self.conditions:
            return True
        
        # First condition
        result = self._evaluate_single_condition(self.conditions[0], entry)
        if self.negations[0]:
            result = not result
        
        # Following conditions with lazy evaluation
        for i in range(1, len(self.conditions)):
            operator = self.operators[i-1]
            
            # Lazy evaluation for AND
            if operator == 'AND' and not result:
                return False
            # Lazy evaluation for OR
            elif operator == 'OR' and result:
                return True
            
            condition_result = self._evaluate_single_condition(self.conditions[i], entry)
            if self.negations[i]:
                condition_result = not condition_result
            
            if operator == 'AND':
                result = result and condition_result
            elif operator == 'OR':
                result = result or condition_result
        
        return result
    
    def _evaluate_single_condition(self, condition, entry):
        """Evaluates a single condition in an optimized way"""
        # Special handling for interface with mapping
        if condition.field == 'interface':
            field_value = entry.get('interface', '')
            interface_display = entry.get('interface_display', '')
            
            # Quick verification
            if self._check_value_match_fast(condition, field_value) or \
               self._check_value_match_fast(condition, interface_display):
                return True
            return False
        # Special handling for labels (uses pre-calculated label)
        elif condition.field == '__label__':
            rule_label = entry.get('__rule_label__', '')
            return self._check_value_match_fast(condition, rule_label)
        else:
            field_value = entry.get(condition.field, '')
            return self._check_value_match_fast(condition, field_value)
    
    def _check_value_match_fast(self, condition, field_value):
        """Optimized value verification"""
        if not condition.case_sensitive and isinstance(field_value, str):
            field_value = field_value.lower()
            comparison_value = condition.value.lower()
        else:
            comparison_value = condition.value
        
        # Optimizations for the most common operators
        if condition.operator == '==':
            return str(field_value) == comparison_value
        elif condition.operator == 'contains':
            return comparison_value in str(field_value)
        elif condition.operator == '!=':
            return str(field_value) != comparison_value
        
        # Other operators (less optimized but rare)
        return condition._check_value_match(field_value)

def get_cpu_count():
    """Returns the number of available CPU cores"""
    return mp.cpu_count()


def get_max_parallel_workers():
    """Returns the maximum number of workers for intensive parallelism"""
    cpu_count = mp.cpu_count()
    # For heavy filters like labels, use all available cores
    if cpu_count >= 8:
        return cpu_count  # Use all cores on powerful machines
    else:
        return max(1, cpu_count - 1)  # Keep 1 core free on modest machines
