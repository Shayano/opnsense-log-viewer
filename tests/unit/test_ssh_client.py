"""
Unit tests for SSH Client (with mocked connections).
"""
import pytest
from unittest.mock import MagicMock, patch
import paramiko
import socket

from opnsense_log_viewer.services.ssh_client import OPNsenseSSHClient, RuleLabelMapper
from opnsense_log_viewer.exceptions import SSHConnectionError


@pytest.mark.unit
class TestOPNsenseSSHClient:
    """Test OPNsenseSSHClient class."""

    def test_client_initialization(self, ssh_client):
        """Test SSH client initialization."""
        assert isinstance(ssh_client, OPNsenseSSHClient)
        assert ssh_client.ssh_client is None
        assert ssh_client.shell_channel is None
        assert ssh_client.connected == False

    @patch('paramiko.SSHClient')
    def test_connect_success(self, mock_ssh_class, ssh_client):
        """Test successful SSH connection."""
        # Setup mock
        mock_client = MagicMock()
        mock_channel = MagicMock()
        mock_ssh_class.return_value = mock_client
        mock_client.invoke_shell.return_value = mock_channel
        mock_channel.recv.return_value = b"Enter an option:"

        # Attempt connection
        success, message = ssh_client.connect('192.168.1.1', 'root', 'password')

        assert success == True
        assert "established" in message.lower()
        assert ssh_client.connected == True
        mock_client.connect.assert_called_once()

    @patch('paramiko.SSHClient')
    def test_connect_timeout(self, mock_ssh_class, ssh_client):
        """Test SSH connection timeout."""
        mock_client = MagicMock()
        mock_ssh_class.return_value = mock_client
        mock_client.connect.side_effect = socket.timeout()

        with pytest.raises(SSHConnectionError) as exc_info:
            ssh_client.connect('192.168.1.1', 'root', 'password', timeout=5)

        assert "timeout" in str(exc_info.value).lower()
        assert exc_info.value.error_type == "timeout"
        assert exc_info.value.hostname == '192.168.1.1'

    @patch('paramiko.SSHClient')
    def test_connect_auth_failure(self, mock_ssh_class, ssh_client):
        """Test SSH authentication failure."""
        mock_client = MagicMock()
        mock_ssh_class.return_value = mock_client
        mock_client.connect.side_effect = paramiko.AuthenticationException()

        with pytest.raises(SSHConnectionError) as exc_info:
            ssh_client.connect('192.168.1.1', 'root', 'wrong_password')

        assert "authentication" in str(exc_info.value).lower()
        assert exc_info.value.error_type == "authentication"
        assert exc_info.value.username == 'root'

    @patch('paramiko.SSHClient')
    def test_connect_ssh_exception(self, mock_ssh_class, ssh_client):
        """Test SSH connection with SSH exception."""
        mock_client = MagicMock()
        mock_ssh_class.return_value = mock_client
        mock_client.connect.side_effect = paramiko.SSHException("SSH error")

        success, message = ssh_client.connect('192.168.1.1', 'root', 'password')

        assert success == False
        assert "SSH error" in message

    @patch('paramiko.SSHClient')
    def test_connect_general_exception(self, mock_ssh_class, ssh_client):
        """Test SSH connection with general exception."""
        mock_client = MagicMock()
        mock_ssh_class.return_value = mock_client
        mock_client.connect.side_effect = Exception("Network error")

        success, message = ssh_client.connect('192.168.1.1', 'root', 'password')

        assert success == False
        assert "error" in message.lower()

    def test_wait_for_prompt(self, ssh_client):
        """Test waiting for prompt."""
        mock_channel = MagicMock()
        ssh_client.shell_channel = mock_channel

        # Mock channel behavior
        mock_channel.recv_ready.side_effect = [True, True, True]
        mock_channel.recv.side_effect = [b"some", b" output", b"Enter an option:"]

        output = ssh_client._wait_for_prompt("Enter an option:", timeout=5)

        assert "Enter an option:" in output

    def test_wait_for_prompt_timeout(self, ssh_client):
        """Test waiting for prompt with timeout."""
        mock_channel = MagicMock()
        ssh_client.shell_channel = mock_channel
        mock_channel.recv_ready.return_value = False

        with pytest.raises(TimeoutError):
            ssh_client._wait_for_prompt("Enter an option:", timeout=1)

    def test_send_command(self, ssh_client):
        """Test sending command."""
        mock_channel = MagicMock()
        ssh_client.shell_channel = mock_channel

        ssh_client._send_command("8")

        mock_channel.send.assert_called_once_with("8\n")

    def test_extract_rule_labels_not_connected(self, ssh_client):
        """Test extracting labels without connection."""
        success, message, labels = ssh_client.extract_rule_labels()

        assert success == False
        assert "No active" in message
        assert labels == {}

    @patch('paramiko.SSHClient')
    def test_extract_rule_labels_success(self, mock_ssh_class, ssh_client):
        """Test successful rule label extraction."""
        # Setup connected client
        mock_client = MagicMock()
        mock_channel = MagicMock()
        mock_ssh_class.return_value = mock_client
        mock_client.invoke_shell.return_value = mock_channel

        ssh_client.ssh_client = mock_client
        ssh_client.shell_channel = mock_channel
        ssh_client.connected = True

        # Mock responses
        rules_output = '''
block in log quick inet from {<crowdsec_blocklists>} to {any} label "031d9d1edc75c3c8c634a8aee47134ef" # CrowdSec (IPv4) in
pass out quick on em0 label "abc123def456" # Allow HTTP/HTTPS
#
'''
        mock_channel.recv_ready.side_effect = [True, True, True, True, False]
        mock_channel.recv.side_effect = [
            b"# ",
            rules_output.encode(),
            b"# ",
            b"# ",
            b""
        ]

        success, message, labels = ssh_client.extract_rule_labels()

        assert success == True
        assert len(labels) > 0
        assert '031d9d1edc75c3c8c634a8aee47134ef' in labels
        assert labels['031d9d1edc75c3c8c634a8aee47134ef'] == 'CrowdSec (IPv4) in'

    def test_parse_rules_debug_output(self, ssh_client):
        """Test parsing rules.debug output."""
        output = '''
block in log quick inet from {<crowdsec_blocklists>} to {any} label "031d9d1edc75c3c8c634a8aee47134ef" # CrowdSec (IPv4) in
pass out quick on em0 label "abc123def456" # Allow HTTP/HTTPS
block in quick on em1 label "xyz789uvw012" # Block SMTP
'''
        labels = ssh_client._parse_rules_debug_output(output)

        assert len(labels) == 3
        assert labels['031d9d1edc75c3c8c634a8aee47134ef'] == 'CrowdSec (IPv4) in'
        assert labels['abc123def456'] == 'Allow HTTP/HTTPS'
        assert labels['xyz789uvw012'] == 'Block SMTP'

    def test_parse_rules_debug_no_labels(self, ssh_client):
        """Test parsing output without labels."""
        output = '''
Some random output
Without any labels
'''
        labels = ssh_client._parse_rules_debug_output(output)

        assert labels == {}

    def test_disconnect(self, ssh_client):
        """Test disconnection."""
        mock_client = MagicMock()
        mock_channel = MagicMock()

        ssh_client.ssh_client = mock_client
        ssh_client.shell_channel = mock_channel
        ssh_client.connected = True

        ssh_client.disconnect()

        mock_channel.close.assert_called_once()
        mock_client.close.assert_called_once()
        assert ssh_client.connected == False
        assert ssh_client.ssh_client is None
        assert ssh_client.shell_channel is None

    def test_disconnect_with_exception(self, ssh_client):
        """Test disconnection with exception."""
        mock_client = MagicMock()
        mock_client.close.side_effect = Exception("Close error")

        ssh_client.ssh_client = mock_client
        ssh_client.connected = True

        # Should not raise exception
        ssh_client.disconnect()

        assert ssh_client.connected == False

    def test_destructor(self, ssh_client):
        """Test __del__ method."""
        mock_client = MagicMock()
        ssh_client.ssh_client = mock_client
        ssh_client.connected = True

        ssh_client.__del__()

        mock_client.close.assert_called_once()


@pytest.mark.unit
class TestRuleLabelMapper:
    """Test RuleLabelMapper class."""

    def test_mapper_initialization(self, rule_label_mapper):
        """Test mapper initialization."""
        assert isinstance(rule_label_mapper, RuleLabelMapper)
        assert rule_label_mapper.label_descriptions == {}

    def test_set_label_descriptions(self, rule_label_mapper, sample_rule_labels):
        """Test setting label descriptions."""
        rule_label_mapper.set_label_descriptions(sample_rule_labels)

        assert len(rule_label_mapper.label_descriptions) == len(sample_rule_labels)
        assert rule_label_mapper.label_descriptions == sample_rule_labels

    def test_set_label_descriptions_creates_copy(self, rule_label_mapper):
        """Test that setting labels creates a copy."""
        original_labels = {'hash1': 'Rule 1', 'hash2': 'Rule 2'}
        rule_label_mapper.set_label_descriptions(original_labels)

        # Modify original
        original_labels['hash3'] = 'Rule 3'

        # Mapper should not be affected
        assert 'hash3' not in rule_label_mapper.label_descriptions

    def test_get_rule_description_by_hash(self, rule_label_mapper, sample_rule_labels):
        """Test getting rule description by hash."""
        rule_label_mapper.set_label_descriptions(sample_rule_labels)

        desc = rule_label_mapper.get_rule_description_by_hash('031d9d1edc75c3c8c634a8aee47134ef')
        assert desc == 'CrowdSec (IPv4) in'

        desc = rule_label_mapper.get_rule_description_by_hash('abc123def456')
        assert desc == 'Block SMTP'

    def test_get_rule_description_not_found(self, rule_label_mapper, sample_rule_labels):
        """Test getting non-existent rule description."""
        rule_label_mapper.set_label_descriptions(sample_rule_labels)

        desc = rule_label_mapper.get_rule_description_by_hash('nonexistent')
        assert desc is None

    def test_get_rule_description_empty_mapper(self, rule_label_mapper):
        """Test getting description from empty mapper."""
        desc = rule_label_mapper.get_rule_description_by_hash('somehash')
        assert desc is None

    def test_get_mapping_stats(self, rule_label_mapper, sample_rule_labels):
        """Test getting mapping statistics."""
        rule_label_mapper.set_label_descriptions(sample_rule_labels)

        stats = rule_label_mapper.get_mapping_stats()

        assert 'total_labels' in stats
        assert stats['total_labels'] == len(sample_rule_labels)

    def test_get_mapping_stats_empty(self, rule_label_mapper):
        """Test getting stats from empty mapper."""
        stats = rule_label_mapper.get_mapping_stats()

        assert stats['total_labels'] == 0

    def test_multiple_set_operations(self, rule_label_mapper):
        """Test multiple set operations."""
        labels1 = {'hash1': 'Rule 1'}
        labels2 = {'hash2': 'Rule 2', 'hash3': 'Rule 3'}

        rule_label_mapper.set_label_descriptions(labels1)
        assert len(rule_label_mapper.label_descriptions) == 1

        rule_label_mapper.set_label_descriptions(labels2)
        assert len(rule_label_mapper.label_descriptions) == 2
        assert 'hash1' not in rule_label_mapper.label_descriptions
