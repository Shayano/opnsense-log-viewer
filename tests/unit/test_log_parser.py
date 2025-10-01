"""
Unit tests for OPNsense Log Parser.
"""
import pytest
import os
from datetime import datetime

from opnsense_log_viewer.services.log_parser import OPNsenseLogParser, LogEntry
from opnsense_log_viewer.exceptions import FileOperationError


@pytest.mark.unit
class TestLogEntry:
    """Test LogEntry class."""

    def test_log_entry_creation(self):
        """Test creating a log entry."""
        entry = LogEntry("test line")
        assert entry.raw_line == "test line"
        assert entry.parsed_data == {}
        assert entry.timestamp is None
        assert entry.host is None

    def test_log_entry_getitem(self):
        """Test __getitem__ method."""
        entry = LogEntry("test")
        entry.parsed_data = {'action': 'pass', 'src': '192.168.1.1'}
        assert entry['action'] == 'pass'
        assert entry['src'] == '192.168.1.1'
        assert entry['missing'] == ''

    def test_log_entry_contains(self):
        """Test __contains__ method."""
        entry = LogEntry("test")
        entry.parsed_data = {'action': 'pass'}
        assert 'action' in entry
        assert 'missing' not in entry

    def test_log_entry_get(self):
        """Test get method with default."""
        entry = LogEntry("test")
        entry.parsed_data = {'action': 'pass'}
        assert entry.get('action') == 'pass'
        assert entry.get('missing') == ''
        assert entry.get('missing', 'default') == 'default'


@pytest.mark.unit
class TestOPNsenseLogParser:
    """Test OPNsenseLogParser class."""

    def test_parser_initialization(self, log_parser):
        """Test parser initialization."""
        assert isinstance(log_parser, OPNsenseLogParser)
        assert log_parser.interface_mapping == {}

    def test_set_interface_mapping(self, log_parser, sample_interface_mapping):
        """Test setting interface mapping."""
        log_parser.set_interface_mapping(sample_interface_mapping)
        assert log_parser.interface_mapping == sample_interface_mapping

    def test_set_interface_mapping_invalid_type(self, log_parser):
        """Test setting invalid interface mapping."""
        with pytest.raises(TypeError):
            log_parser.set_interface_mapping("not a dict")

    def test_parse_valid_log_line(self, log_parser):
        """Test parsing a valid log line."""
        line = "2024-01-15T10:30:45 opnsense filterlog: 100,200,anchor1,12345,em0,match,pass,in,4,0x0,64,54321,0,none,6,tcp,60,192.168.1.100,10.0.0.50,12345,80,40,S,1234567890,0,0,"
        entry = log_parser.parse_log_line(line)

        assert entry is not None
        assert isinstance(entry, LogEntry)
        assert entry['action'] == 'pass'
        assert entry['interface'] == 'em0'
        assert entry['src'] == '192.168.1.100'
        assert entry['dst'] == '10.0.0.50'
        assert entry['srcport'] == '12345'
        assert entry['dstport'] == '80'
        assert entry['protoname'] == 'tcp'

    def test_parse_log_line_with_interface_mapping(self, log_parser):
        """Test parsing with interface mapping."""
        log_parser.set_interface_mapping({'em0': 'LAN Network'})
        line = "2024-01-15T10:30:45 opnsense filterlog: 100,200,anchor1,12345,em0,match,pass,in,4,0x0,64,54321,0,none,6,tcp,60,192.168.1.100,10.0.0.50,12345,80,40,S,1234567890,0,0,"
        entry = log_parser.parse_log_line(line)

        assert entry is not None
        assert entry['interface'] == 'em0'
        assert entry['interface_display'] == 'LAN Network'

    def test_parse_log_line_udp(self, log_parser):
        """Test parsing UDP log line."""
        line = "2024-01-15T10:31:12 opnsense filterlog: 101,201,anchor2,12346,em1,match,block,out,4,0x0,64,54322,0,none,17,udp,100,10.0.0.50,8.8.8.8,53,53,92"
        entry = log_parser.parse_log_line(line)

        assert entry is not None
        assert entry['action'] == 'block'
        assert entry['protonum'] == '17'
        assert entry['protoname'] == 'udp'
        assert entry['srcport'] == '53'
        assert entry['dstport'] == '53'

    def test_parse_log_line_icmp(self, log_parser):
        """Test parsing ICMP log line."""
        line = "2024-01-15T10:31:45 opnsense filterlog: 102,202,anchor1,12347,em0,match,pass,in,4,0x0,64,54323,0,none,1,icmp,84,192.168.1.101,10.0.0.50"
        entry = log_parser.parse_log_line(line)

        assert entry is not None
        assert entry['action'] == 'pass'
        assert entry['protonum'] == '1'
        assert entry['protoname'] == 'icmp'
        assert entry['src'] == '192.168.1.101'
        assert entry['dst'] == '10.0.0.50'

    def test_parse_log_line_without_filterlog(self, log_parser):
        """Test parsing line without filterlog keyword."""
        line = "2024-01-15T10:30:45 opnsense: some other log message"
        entry = log_parser.parse_log_line(line)
        assert entry is None

    def test_parse_log_line_empty(self, log_parser):
        """Test parsing empty line."""
        entry = log_parser.parse_log_line("")
        assert entry is None

    def test_parse_log_line_insufficient_fields(self, log_parser):
        """Test parsing line with insufficient fields."""
        line = "2024-01-15T10:30:45 opnsense filterlog: 100,200"
        entry = log_parser.parse_log_line(line)
        assert entry is None

    def test_parse_log_line_malformed(self, log_parser):
        """Test parsing malformed line."""
        line = "malformed line with filterlog but no structure"
        entry = log_parser.parse_log_line(line)
        assert entry is None

    def test_parse_fields_tcp(self, log_parser):
        """Test parsing TCP fields."""
        fields = ['100', '200', 'anchor1', '12345', 'em0', 'match', 'pass', 'in', '4',
                  '0x0', '64', '54321', '0', 'none', '6', 'tcp', '60', '192.168.1.100',
                  '10.0.0.50', '12345', '80', '40', 'S', '1234567890', '0', '0']
        rule = log_parser._parse_fields(fields)

        assert rule['rulenr'] == '100'
        assert rule['interface'] == 'em0'
        assert rule['action'] == 'pass'
        assert rule['protonum'] == '6'
        assert rule['protoname'] == 'tcp'
        assert rule['src'] == '192.168.1.100'
        assert rule['dst'] == '10.0.0.50'
        assert rule['srcport'] == '12345'
        assert rule['dstport'] == '80'
        assert rule['tcpflags'] == 'S'

    def test_parse_fields_protocol_mapping(self, log_parser):
        """Test protocol number to name mapping."""
        # TCP
        fields = ['100', '200', 'anchor', '12345', 'em0', 'match', 'pass', 'in', '4',
                  '0x0', '64', '54321', '0', 'none', '6']
        rule = log_parser._parse_fields(fields)
        assert rule['protoname'] == 'tcp'

        # UDP
        fields[14] = '17'
        rule = log_parser._parse_fields(fields)
        assert rule['protoname'] == 'udp'

        # ICMP
        fields[14] = '1'
        rule = log_parser._parse_fields(fields)
        assert rule['protoname'] == 'icmp'

        # CARP
        fields[14] = '112'
        rule = log_parser._parse_fields(fields)
        assert rule['protoname'] == 'carp'

    def test_parse_log_file_success(self, log_parser, sample_log_file):
        """Test parsing a complete log file."""
        entries = log_parser.parse_log_file(sample_log_file)

        assert len(entries) > 0
        assert all(isinstance(e, LogEntry) for e in entries)
        assert all(e['action'] in ['pass', 'block'] for e in entries)

    def test_parse_log_file_with_max_lines(self, log_parser, sample_log_file):
        """Test parsing log file with max lines limit."""
        entries = log_parser.parse_log_file(sample_log_file, max_lines=5)
        assert len(entries) <= 5

    def test_parse_log_file_not_found(self, log_parser):
        """Test parsing non-existent file."""
        with pytest.raises(FileOperationError) as exc_info:
            log_parser.parse_log_file("/nonexistent/file.log")
        assert "not found" in str(exc_info.value).lower()

    def test_parse_log_file_not_readable(self, log_parser, temp_dir, mocker):
        """Test parsing unreadable file."""
        # Create a file
        import tempfile
        fd, path = tempfile.mkstemp(dir=temp_dir, suffix='.log')
        os.close(fd)

        # Mock access to return False
        mocker.patch('os.access', return_value=False)

        with pytest.raises(FileOperationError) as exc_info:
            log_parser.parse_log_file(path)
        assert "not readable" in str(exc_info.value).lower()

    def test_parse_log_file_empty(self, log_parser, empty_log_file):
        """Test parsing empty log file."""
        entries = log_parser.parse_log_file(empty_log_file)
        assert entries == []

    def test_parse_log_file_invalid_format(self, log_parser, invalid_log_file):
        """Test parsing file with invalid format lines."""
        entries = log_parser.parse_log_file(invalid_log_file)
        # Should skip invalid lines and return empty or partial results
        assert isinstance(entries, list)

    def test_parse_log_file_generator(self, log_parser, sample_log_file):
        """Test generator parsing."""
        entries = list(log_parser.parse_log_file_generator(sample_log_file))

        assert len(entries) > 0
        assert all(isinstance(e, LogEntry) for e in entries)

    def test_parse_log_file_generator_not_found(self, log_parser):
        """Test generator with non-existent file."""
        with pytest.raises(FileOperationError):
            list(log_parser.parse_log_file_generator("/nonexistent/file.log"))

    def test_parse_log_file_generator_not_readable(self, log_parser, temp_dir, mocker):
        """Test generator with unreadable file."""
        import tempfile
        fd, path = tempfile.mkstemp(dir=temp_dir, suffix='.log')
        os.close(fd)

        mocker.patch('os.access', return_value=False)

        with pytest.raises(FileOperationError):
            list(log_parser.parse_log_file_generator(path))

    def test_digest_generation(self, log_parser):
        """Test MD5 digest generation for log lines."""
        line1 = "2024-01-15T10:30:45 opnsense filterlog: 100,200,anchor1,12345,em0,match,pass,in,4,0x0,64,54321,0,none,6,tcp,60,192.168.1.100,10.0.0.50,12345,80,40,S,1234567890,0,0,"
        line2 = "2024-01-15T10:30:45 opnsense filterlog: 101,200,anchor1,12345,em0,match,pass,in,4,0x0,64,54321,0,none,6,tcp,60,192.168.1.100,10.0.0.50,12345,80,40,S,1234567890,0,0,"

        entry1 = log_parser.parse_log_line(line1)
        entry2 = log_parser.parse_log_line(line2)

        # Different lines should have different digests
        assert entry1['__digest__'] != entry2['__digest__']
        assert len(entry1['__digest__']) == 32  # MD5 hash length

    def test_timestamp_parsing(self, log_parser):
        """Test timestamp parsing."""
        line = "2024-01-15T10:30:45 opnsense filterlog: 100,200,anchor1,12345,em0,match,pass,in,4,0x0,64,54321,0,none,6,tcp,60,192.168.1.100,10.0.0.50,12345,80,40,S,1234567890,0,0,"
        entry = log_parser.parse_log_line(line)

        assert entry.timestamp is not None
        assert isinstance(entry.timestamp, datetime)
        assert entry['__timestamp__'] == "2024-01-15T10:30:45"

    def test_host_assignment(self, log_parser):
        """Test host assignment."""
        line = "2024-01-15T10:30:45 opnsense filterlog: 100,200,anchor1,12345,em0,match,pass,in,4,0x0,64,54321,0,none,6,tcp,60,192.168.1.100,10.0.0.50,12345,80,40,S,1234567890,0,0,"
        entry = log_parser.parse_log_line(line)

        assert entry.host == 'opnsense'
        assert entry['__host__'] == 'opnsense'

    def test_parse_fields_short_list(self, log_parser):
        """Test parsing with insufficient fields."""
        fields = ['100', '200']
        rule = log_parser._parse_fields(fields)

        # Should handle gracefully with empty strings
        assert rule['rulenr'] == '100'
        assert rule['subrulenr'] == '200'
        assert rule.get('action', '') == ''

    def test_multiple_files_parsing(self, log_parser, sample_log_file, temp_log_file):
        """Test parsing multiple files sequentially."""
        entries1 = log_parser.parse_log_file(sample_log_file)
        entries2 = log_parser.parse_log_file(temp_log_file)

        assert len(entries1) > 0
        assert len(entries2) > 0
        # Ensure parser is reusable
        assert isinstance(entries1[0], LogEntry)
        assert isinstance(entries2[0], LogEntry)
