"""
Pytest configuration and fixtures for OPNsense Log Viewer tests.
"""
import os
import sys
import pytest
import tempfile
import shutil
from pathlib import Path

# Add src directory to path for imports
src_path = Path(__file__).parent.parent / 'src'
sys.path.insert(0, str(src_path))

from opnsense_log_viewer.services.log_parser import OPNsenseLogParser, LogEntry
from opnsense_log_viewer.services.config_parser import OPNsenseConfigParser
from opnsense_log_viewer.services.log_filter import LogFilter, FilterCondition, FilterExpression
from opnsense_log_viewer.services.ssh_client import OPNsenseSSHClient, RuleLabelMapper
from opnsense_log_viewer.services.virtual_log_manager import VirtualLogManager


# Test data directory
@pytest.fixture
def fixtures_dir():
    """Return path to test fixtures directory."""
    return Path(__file__).parent / 'fixtures'


@pytest.fixture
def sample_log_file(fixtures_dir):
    """Return path to sample log file."""
    return str(fixtures_dir / 'sample_filter.log')


@pytest.fixture
def invalid_log_file(fixtures_dir):
    """Return path to invalid log file."""
    return str(fixtures_dir / 'invalid_format.log')


@pytest.fixture
def sample_config_file(fixtures_dir):
    """Return path to sample config file."""
    return str(fixtures_dir / 'sample_config.xml')


@pytest.fixture
def malformed_config_file(fixtures_dir):
    """Return path to malformed config file."""
    return str(fixtures_dir / 'malformed_config.xml')


@pytest.fixture
def temp_dir():
    """Create a temporary directory for test files."""
    temp_path = tempfile.mkdtemp()
    yield temp_path
    shutil.rmtree(temp_path, ignore_errors=True)


@pytest.fixture
def temp_log_file(temp_dir):
    """Create a temporary log file with sample data."""
    log_path = Path(temp_dir) / 'test.log'
    content = """2024-01-15T10:30:45 opnsense filterlog: 100,200,anchor1,12345,em0,match,pass,in,4,0x0,64,54321,0,none,6,tcp,60,192.168.1.100,10.0.0.50,12345,80,40,S,1234567890,0,0,
2024-01-15T10:31:12 opnsense filterlog: 101,201,anchor2,12346,em1,match,block,out,4,0x0,64,54322,0,none,17,udp,100,10.0.0.50,8.8.8.8,53,53,92
"""
    log_path.write_text(content)
    return str(log_path)


@pytest.fixture
def empty_log_file(temp_dir):
    """Create an empty log file."""
    log_path = Path(temp_dir) / 'empty.log'
    log_path.write_text('')
    return str(log_path)


# Parser fixtures
@pytest.fixture
def log_parser():
    """Create a fresh log parser instance."""
    return OPNsenseLogParser()


@pytest.fixture
def log_parser_with_mapping():
    """Create a log parser with interface mapping."""
    parser = OPNsenseLogParser()
    parser.set_interface_mapping({
        'em0': 'LAN',
        'em1': 'WAN',
        'em2': 'DMZ'
    })
    return parser


@pytest.fixture
def config_parser():
    """Create a fresh config parser instance."""
    return OPNsenseConfigParser()


@pytest.fixture
def log_filter():
    """Create a fresh log filter instance."""
    return LogFilter()


@pytest.fixture
def ssh_client():
    """Create a fresh SSH client instance."""
    return OPNsenseSSHClient()


@pytest.fixture
def rule_label_mapper():
    """Create a fresh rule label mapper instance."""
    return RuleLabelMapper()


@pytest.fixture
def virtual_log_manager():
    """Create a virtual log manager instance."""
    return VirtualLogManager(chunk_size=100, cache_size=10)


# Sample data fixtures
@pytest.fixture
def sample_log_lines():
    """Return a list of sample log lines."""
    return [
        "2024-01-15T10:30:45 opnsense filterlog: 100,200,anchor1,12345,em0,match,pass,in,4,0x0,64,54321,0,none,6,tcp,60,192.168.1.100,10.0.0.50,12345,80,40,S,1234567890,0,0,",
        "2024-01-15T10:31:12 opnsense filterlog: 101,201,anchor2,12346,em1,match,block,out,4,0x0,64,54322,0,none,17,udp,100,10.0.0.50,8.8.8.8,53,53,92",
        "2024-01-15T10:31:45 opnsense filterlog: 102,202,anchor1,12347,em0,match,pass,in,4,0x0,64,54323,0,none,1,icmp,84,192.168.1.101,10.0.0.50",
    ]


@pytest.fixture
def sample_parsed_entries(log_parser, sample_log_lines):
    """Return a list of parsed log entries."""
    entries = []
    for line in sample_log_lines:
        entry = log_parser.parse_log_line(line)
        if entry:
            entries.append(entry)
    return entries


@pytest.fixture
def sample_interface_mapping():
    """Return a sample interface mapping."""
    return {
        'em0': 'LAN Network',
        'em1': 'WAN Connection',
        'em2': 'DMZ Zone',
        'em3': 'Guest Network'
    }


@pytest.fixture
def sample_rule_labels():
    """Return sample rule label mapping."""
    return {
        '031d9d1edc75c3c8c634a8aee47134ef': 'CrowdSec (IPv4) in',
        'abc123def456': 'Block SMTP',
        'xyz789uvw012': 'Allow HTTP/HTTPS'
    }


@pytest.fixture
def sample_ip_aliases():
    """Return sample IP aliases."""
    return {
        '192.168.1.100': 'WebServer1',
        '192.168.1.101': 'WebServer2',
        '10.0.0.0/8': 'PrivateNetA',
        '172.16.0.0/12': 'PrivateNetB'
    }


@pytest.fixture
def sample_port_aliases():
    """Return sample port aliases."""
    return {
        '80': 'HTTP',
        '443': 'HTTPS',
        '22': 'SSH',
        '3389': 'RDP'
    }


# Mock fixtures
@pytest.fixture
def mock_ssh_connection(mocker):
    """Mock SSH connection for testing."""
    mock_client = mocker.MagicMock()
    mock_channel = mocker.MagicMock()

    # Mock successful connection
    mock_client.connect.return_value = None
    mock_client.invoke_shell.return_value = mock_channel

    # Mock channel recv
    mock_channel.recv_ready.return_value = True
    mock_channel.recv.return_value = b"# "

    return mock_client, mock_channel


@pytest.fixture
def mock_file_system(mocker, temp_dir):
    """Mock file system operations."""
    def mock_exists(path):
        return True

    def mock_access(path, mode):
        return True

    mocker.patch('os.path.exists', side_effect=mock_exists)
    mocker.patch('os.access', side_effect=mock_access)

    return temp_dir


# Test data generator helpers
def create_log_entry(action='pass', interface='em0', src='192.168.1.100',
                     dst='10.0.0.50', srcport='12345', dstport='80',
                     protoname='tcp', protonum='6'):
    """Helper to create a log entry for testing."""
    entry = LogEntry('')
    entry.parsed_data = {
        'action': action,
        'interface': interface,
        'interface_display': interface,
        'src': src,
        'dst': dst,
        'srcport': srcport,
        'dstport': dstport,
        'protoname': protoname,
        'protonum': protonum,
        'rulenr': '100',
        'rid': '12345'
    }
    return entry


@pytest.fixture
def log_entry_factory():
    """Factory fixture for creating log entries."""
    return create_log_entry


# Performance testing fixtures
@pytest.fixture
def large_log_file(temp_dir):
    """Create a large log file for performance testing."""
    log_path = Path(temp_dir) / 'large.log'
    base_line = "2024-01-15T10:30:45 opnsense filterlog: 100,200,anchor1,12345,em0,match,pass,in,4,0x0,64,54321,0,none,6,tcp,60,192.168.1.100,10.0.0.50,12345,80,40,S,1234567890,0,0,\n"

    with open(log_path, 'w') as f:
        for i in range(1000):  # 1000 lines for testing
            f.write(base_line)

    return str(log_path)


# Cleanup fixture
@pytest.fixture(autouse=True)
def cleanup_logs():
    """Clean up log files created during tests."""
    yield
    # Clean up any test log files
    log_dir = Path('logs')
    if log_dir.exists():
        for log_file in log_dir.glob('*.log*'):
            try:
                log_file.unlink()
            except:
                pass
