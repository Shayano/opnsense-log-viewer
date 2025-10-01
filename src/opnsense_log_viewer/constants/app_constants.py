"""
Application constants for OPNsense Log Viewer.
"""

# Window dimensions
DEFAULT_WINDOW_WIDTH = 1500
DEFAULT_WINDOW_HEIGHT = 1000

# Pagination
DEFAULT_PAGE_SIZE = 1000
PAGE_SIZE_OPTIONS = ['100', '500', '1000', '2500', '5000']

# Virtual Log Manager
VIRTUAL_LOG_CHUNK_SIZE = 1000
VIRTUAL_LOG_CACHE_SIZE = 50

# File tail reading
TAIL_READ_LINES = 2000
TAIL_CHUNK_SIZE = 8192  # 8KB chunks for file reading

# Safe navigation thresholds
SAFE_TAIL_THRESHOLD = 1000  # Stay this many entries before end when using Next

# Column configuration
LOG_TABLE_COLUMNS = ('timestamp', 'action', 'interface', 'src', 'srcport', 'dst', 'dstport', 'proto', 'label')

COLUMN_WIDTHS = {
    'timestamp': 150,
    'action': 80,
    'interface': 120,
    'src': 120,
    'srcport': 80,
    'dst': 120,
    'dstport': 80,
    'proto': 80,
    'label': 200
}

COLUMN_HEADERS = {
    'timestamp': 'Timestamp',
    'action': 'Action',
    'interface': 'Interface',
    'src': 'Source',
    'srcport': 'Source port',
    'dst': 'Destination',
    'dstport': 'Destination port',
    'proto': 'Protocol',
    'label': 'Label'
}

# Field mapping from display names to technical names
FIELD_MAPPING = {
    'Action': 'action',
    'Interface': 'interface',
    'Source': 'src',
    'Source port': 'srcport',
    'Destination': 'dst',
    'Destination port': 'dstport',
    'Protocol': 'protoname',
    'Label': 'label'
}

# Filter field options
FILTER_FIELD_OPTIONS = ['Action', 'Interface', 'Source', 'Source port',
                        'Destination', 'Destination port', 'Protocol', 'Label']

# Filter operator options
FILTER_OPERATOR_OPTIONS = ['==', '!=', 'contains', 'startswith', 'endswith', 'regex']

# Logic operator options
FILTER_LOGIC_OPTIONS = ['AND', 'OR']

# Preset filter names
PRESET_FILTER_ALL = 'all'
PRESET_FILTER_BLOCKED = 'blocked'
PRESET_FILTER_ALLOWED = 'allowed'
PRESET_FILTER_TCP = 'tcp'
PRESET_FILTER_UDP = 'udp'

# Tag colors
TAG_COLOR_BLOCKED = '#ffcccc'
TAG_COLOR_PASSED = '#ccffcc'

# SSH connection
DEFAULT_SSH_PORT = 22
SSH_TIMEOUT = 10

# Export
EXPORT_CHUNK_SIZE = 1000

# Time format
TIME_FORMAT = '%Y-%m-%d %H:%M:%S'

# Application info
APP_NAME = "OPNsense Log Viewer"
APP_VERSION = "1.1"
APP_URL = "https://github.com/Shayano/opnsense-log-viewer"
