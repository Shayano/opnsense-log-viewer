# OPNsense Log Viewer - Developer Guide

## Running the Application

### Method 1: As a Module (Recommended)
```bash
cd src
python -m opnsense_log_viewer
```

### Method 2: Direct Execution
```bash
cd src
python opnsense_log_viewer/__main__.py
```

### Method 3: With Custom PYTHONPATH
```bash
export PYTHONPATH=src  # Linux/Mac
set PYTHONPATH=src     # Windows
python -m opnsense_log_viewer
```

## Development Setup

### Prerequisites
- Python 3.8+
- tkinter (usually included with Python)
- `duckdb` (fast filter engine) and `paramiko` (SSH rule-label fetch); install with
  `pip install -r requirements.txt`

### Installing in Development Mode
```bash
cd opnsense-log-viewer
pip install -e .
```

## Module Overview

### Components (`components/`)

#### ProgressDialog
```python
from opnsense_log_viewer.components import ProgressDialog

# Create progress dialog
dialog = ProgressDialog(parent_window, "Loading...")

# Update text
dialog.update_text("Processing...")

# Check if cancelled
if dialog.cancelled:
    # Handle cancellation
    pass

# Close dialog
dialog.close()
```

#### LogViewerApp
```python
from opnsense_log_viewer.components import LogViewerApp
import tkinter as tk

# Create application
root = tk.Tk()
app = LogViewerApp(root)
root.mainloop()
```

### Constants (`constants/`)

All application constants are centralized in `app_constants.py`:

```python
from opnsense_log_viewer.constants import (
    APP_NAME,                    # "OPNsense Log Viewer"
    APP_VERSION,                 # "1.0"
    DEFAULT_WINDOW_WIDTH,        # 1500
    DEFAULT_WINDOW_HEIGHT,       # 1000
    DEFAULT_PAGE_SIZE,           # 1000
    PAGE_SIZE_OPTIONS,           # ['100', '500', '1000', '2500', '5000']
    LOG_TABLE_COLUMNS,           # Column names
    COLUMN_HEADERS,              # Display headers
    FIELD_MAPPING,               # Display name -> technical name
    TAG_COLOR_BLOCKED,           # '#ffcccc'
    TAG_COLOR_PASSED,            # '#ccffcc'
    TIME_FORMAT,                 # '%Y-%m-%d %H:%M:%S'
)
```

### Services (`services/`)

#### Log Parser
```python
from opnsense_log_viewer.services.log_parser import OPNsenseLogParser, LogEntry

parser = OPNsenseLogParser()

# Set interface mapping (optional)
parser.set_interface_mapping({'vtnet0': 'WAN', 'vtnet1': 'LAN'})

# Parse a log line
entry = parser.parse_log_line(log_line)
if entry:
    print(entry.timestamp)
    print(entry.get('action'))
    print(entry.get('src'))
```

#### Config Parser
```python
from opnsense_log_viewer.services.config_parser import OPNsenseConfigParser

config = OPNsenseConfigParser()

# Parse interfaces from XML
interfaces = config.parse_interfaces_from_xml('config.xml')

# Parse aliases
ip_aliases, port_aliases = config.parse_aliases_from_xml('config.xml')

# Get alias for IP
alias = config.get_ip_alias('192.168.1.1')
```

#### Log Filter
```python
from opnsense_log_viewer.services.log_filter import LogFilter

filter_obj = LogFilter()

# Add filter conditions
filter_obj.add_filter_condition('action', '==', 'block')
filter_obj.add_filter_condition('src', 'contains', '192.168', logic='AND')

# Set time range
from datetime import datetime
start = datetime(2024, 1, 1)
end = datetime(2024, 12, 31)
filter_obj.set_time_range(start, end)

# Apply filter
if filter_obj.matches(log_entry):
    print("Entry matches filter")
```

#### Virtual Log Manager
```python
from opnsense_log_viewer.services.virtual_log_manager import VirtualLogManager

manager = VirtualLogManager(chunk_size=1000, cache_size=50, log_parser=parser)

# Load log file
def progress_callback(message):
    print(message)

manager.load_file('firewall.log', progress_callback)

# Get entries
entries = manager.get_entries(start_index=0, count=100)

# Apply filter
manager.apply_filter(filter_function, progress_callback)

# Get filtered entries
filtered_entries = manager.get_entries(0, 100)

# Get memory info
info = manager.get_memory_info()
print(f"Memory usage: {info['estimated_total_memory_mb']:.1f}MB")
```

#### SSH Client
```python
from opnsense_log_viewer.services.ssh_client import OPNsenseSSHClient, RuleLabelMapper

# Connect to OPNsense
client = OPNsenseSSHClient()
success, message = client.connect('192.168.1.1', 'user', 'password', port=22)

if success:
    # Extract rule labels
    success, message, labels = client.extract_rule_labels()

    # Use rule mapper
    mapper = RuleLabelMapper()
    mapper.set_label_descriptions(labels)

    # Get description by hash
    desc = mapper.get_rule_description_by_hash('abc123...')

    client.disconnect()
```

#### Parallel Filter
```python
from opnsense_log_viewer.services.parallel_filter import (
    ParallelLogFilter, OptimizedFilterFunction,
    get_cpu_count, get_max_parallel_workers
)

# Check system capabilities
cpu_count = get_cpu_count()
max_workers = get_max_parallel_workers()

# Create optimized filter
filter_func = OptimizedFilterFunction(
    log_filter,
    time_filter_enabled=True,
    time_range_start=start_time,
    time_range_end=end_time
)

# Apply filter in parallel
parallel_filter = ParallelLogFilter()
filtered_indices = parallel_filter.apply_filter_parallel(
    virtual_log_manager,
    filter_func,
    progress_callback,
    rule_labels_mapping=None
)
```

### Utils (`utils/`)

#### Resource Utils
```python
from opnsense_log_viewer.utils import get_resource_path

# Get path to resource (works in dev and PyInstaller)
icon_path = get_resource_path('icon/icon64.ico')
```

#### File Utils
```python
from opnsense_log_viewer.utils import read_file_tail

# Read last 1000 lines from file
lines = read_file_tail('large_file.log', 1000)
```

## Adding New Features

### Adding a New Component
1. Create file in `src/opnsense_log_viewer/components/`
2. Add proper docstrings
3. Export in `components/__init__.py`
4. Import where needed

Example:
```python
# components/new_component.py
"""
Description of the component.
"""
import tkinter as tk

class NewComponent:
    """Component description"""
    def __init__(self, parent):
        self.parent = parent
        # ...
```

### Adding New Constants
1. Add to `constants/app_constants.py`
2. Group related constants together
3. Add comments explaining usage
4. Import where needed

Example:
```python
# In app_constants.py
# New Feature Settings
NEW_FEATURE_ENABLED = True
NEW_FEATURE_TIMEOUT = 30
NEW_FEATURE_MAX_RETRIES = 3
```

### Adding New Utility Functions
1. Create function in appropriate utils file
2. Add proper docstrings and type hints
3. Export in `utils/__init__.py`
4. Write unit tests (when test framework is added)

Example:
```python
# utils/file_utils.py
def count_file_lines(file_path: str) -> int:
    """
    Count total lines in a file.

    Args:
        file_path: Path to file

    Returns:
        Number of lines in file
    """
    with open(file_path, 'r') as f:
        return sum(1 for _ in f)
```

## Code Style Guidelines

### Imports
- Use absolute imports: `from opnsense_log_viewer.module import Class`
- Group imports: stdlib, third-party, local
- Sort imports alphabetically within groups

### Docstrings
- Use triple quotes for all docstrings
- Include description, Args, Returns for functions
- Document class purpose in class docstring

Example:
```python
def process_entry(entry: LogEntry, options: Dict[str, Any]) -> bool:
    """
    Process a log entry with given options.

    Args:
        entry: Log entry to process
        options: Processing options

    Returns:
        True if processing succeeded, False otherwise
    """
    # Implementation
    pass
```

### Constants
- Use UPPER_CASE for constants
- Group related constants
- Add comments for non-obvious values

### Type Hints
- Add type hints to function signatures
- Use typing module for complex types
- Optional parameters should have Optional[] type

## Testing

The project ships a pytest suite under `tests/` (run with `python -m pytest`).
When adding features, add or update tests under `tests/unit/` and
`tests/integration/`; the DuckDB filter engine in particular is validated by a
semantic cross-check against the parser reference (`tests/unit/test_duckdb_filter.py`).

### Unit Tests
```python
# tests/test_log_parser.py
import unittest
from opnsense_log_viewer.services.log_parser import OPNsenseLogParser

class TestLogParser(unittest.TestCase):
    def setUp(self):
        self.parser = OPNsenseLogParser()

    def test_parse_valid_line(self):
        line = "..."
        entry = self.parser.parse_log_line(line)
        self.assertIsNotNone(entry)
        self.assertEqual(entry.get('action'), 'block')
```

### Integration Tests
```python
# tests/test_integration.py
# Test complete workflows
```

## Common Tasks

### Changing Window Size
Edit `constants/app_constants.py`:
```python
DEFAULT_WINDOW_WIDTH = 1600  # Change from 1500
DEFAULT_WINDOW_HEIGHT = 1200  # Change from 1000
```

### Adding a New Column
1. Add to `LOG_TABLE_COLUMNS` in `constants/app_constants.py`
2. Add width to `COLUMN_WIDTHS`
3. Add header to `COLUMN_HEADERS`
4. Update `log_viewer.py` to populate the column

### Modifying Page Size Options
Edit `constants/app_constants.py`:
```python
PAGE_SIZE_OPTIONS = ['100', '500', '1000', '2500', '5000', '10000']
```

### Adding a Preset Filter
In `log_viewer.py`, add button in `setup_ui()`:
```python
ttk.Button(preset_frame, text="ICMP",
          command=lambda: self.apply_preset_filter('icmp')).pack(...)
```

Add handler in `apply_preset_filter()`:
```python
elif preset_name == 'icmp':
    self.log_filter.add_filter_condition('protoname', '==', 'icmp')
```

## Troubleshooting

### Import Errors
- Ensure you're running from `src/` directory
- Check PYTHONPATH is set correctly
- Verify all __init__.py files exist

### Module Not Found
```bash
# Solution: Run from correct directory
cd src
python -m opnsense_log_viewer
```

### Circular Import
- Check for circular dependencies between modules
- Ensure imports are at module level, not inside functions
- Consider restructuring if needed

## Performance Considerations

### Memory Management
- VirtualLogManager uses LRU cache
- Default chunk size: 1000 entries
- Default cache size: 50 chunks
- Adjust in `constants/app_constants.py` if needed

### Filtering (DuckDB engine)
- Filters are answered by `services/duckdb_filter.py` (`DuckDBLogFilter`), which
  reads the raw log with DuckDB's compiled, multi-threaded CSV engine and filters
  it in place; the first filter on a 14 GB file completes in tens of seconds
- The file is read as a single `line` column; every field is extracted on demand
  with `split_part()` plus a `CASE WHEN ipversion='6'` for the version-dependent
  fields, mirroring `OPNsenseLogParser._parse_fields` (IPv4 and IPv6) exactly
- `FilterCondition`/`FilterExpression` are translated to a DuckDB `WHERE` (left-to-
  right fold, per-condition negation). The special `interface` (physical + logical
  name) and `__label__` (rule description) filters are resolved in Python against
  the current mappings, so nothing has to be rebuilt when they change
- A filter materializes only the matching rows (raw line + resolved display fields,
  newest first) into a `matches` table; the GUI pages through it instantly
- **Parquet cache**: when a file is loaded, `load_file` also starts a one-time
  background conversion of the valid rows to a ZSTD Parquet cache (raw line +
  typed timestamp + resolved fields; ~1.7 GB and ~2-3 min for a 14 GB source;
  do NOT add a `row_number()` column to the conversion, a windowed column
  serializes the parquet write and makes it 5x slower). Once ready,
  `build_matches` transparently reads the cache instead of the raw file and a
  typical filter answers in ~0.5-5 s. Caches live under
  `%LOCALAPPDATA%\OPNsenseLogViewer\parquet_cache`, are keyed by
  (path, size, mtime) so any file change invalidates them, are written
  atomically, validated at open, and pruned by age (14 days) and total size
  (20 GB). If the conversion fails the direct scan keeps working: the cache is
  an accelerator, never a prerequisite
- A legacy in-memory parallel filter (`parallel_filter.py`) remains as a fallback
  if DuckDB is unavailable

### File I/O
- Uses streaming for large files
- Fast tail mode for end of file
- Buffered reading with 8KB chunks

## Building for Distribution

### PyInstaller Configuration
```bash
pyinstaller --name="OPNsense Log Viewer" \
            --windowed \
            --onefile \
            --icon=icon/icon64.ico \
            --add-data="icon:icon" \
            src/opnsense_log_viewer/__main__.py
```

### Executable Location
- Windows: `dist/OPNsense Log Viewer.exe`
- Linux/Mac: `dist/OPNsense Log Viewer`

## Resources

- Application Constants: `src/opnsense_log_viewer/constants/app_constants.py`
- Filter engine: `src/opnsense_log_viewer/services/duckdb_filter.py`

## Contributing

When contributing:
1. Follow the existing code style
2. Add docstrings to all public functions/classes
3. Update this guide if adding new modules
4. Test thoroughly before committing
5. Keep functionality separated by module

## License

[Add license information here]
