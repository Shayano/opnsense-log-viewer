"""
Unit tests for OPNsense Config Parser.
"""
import pytest
import xml.etree.ElementTree as ET

from opnsense_log_viewer.services.config_parser import OPNsenseConfigParser


@pytest.mark.unit
class TestOPNsenseConfigParser:
    """Test OPNsenseConfigParser class."""

    def test_parser_initialization(self, config_parser):
        """Test parser initialization."""
        assert isinstance(config_parser, OPNsenseConfigParser)
        assert config_parser.interface_mapping == {}
        assert config_parser.ip_aliases == {}
        assert config_parser.port_aliases == {}

    def test_parse_interfaces_from_xml(self, config_parser, sample_config_file):
        """Test parsing interfaces from XML."""
        mapping = config_parser.parse_interfaces_from_xml(sample_config_file)

        assert isinstance(mapping, dict)
        assert len(mapping) > 0
        assert 'em0' in mapping
        assert mapping['em0'] == 'LAN Network'
        assert 'em1' in mapping
        assert mapping['em1'] == 'WAN Connection'
        assert 'em2' in mapping
        assert mapping['em2'] == 'DMZ Zone'

    def test_parse_interfaces_nonexistent_file(self, config_parser):
        """Test parsing interfaces from non-existent file."""
        mapping = config_parser.parse_interfaces_from_xml('/nonexistent/config.xml')
        assert mapping == {}

    def test_parse_interfaces_malformed_xml(self, config_parser, malformed_config_file):
        """Test parsing interfaces from malformed XML."""
        mapping = config_parser.parse_interfaces_from_xml(malformed_config_file)
        # Should handle gracefully and return empty dict
        assert isinstance(mapping, dict)

    def test_parse_interfaces_no_description(self, config_parser, temp_dir):
        """Test parsing interface without description."""
        from pathlib import Path
        xml_content = """<?xml version="1.0"?>
<opnsense>
  <interfaces>
    <lan>
      <if>em0</if>
    </lan>
  </interfaces>
</opnsense>"""
        xml_path = Path(temp_dir) / 'no_desc.xml'
        xml_path.write_text(xml_content)

        mapping = config_parser.parse_interfaces_from_xml(str(xml_path))
        assert 'em0' in mapping
        assert mapping['em0'] == 'LAN'  # Should fallback to interface name

    def test_parse_aliases_from_xml(self, config_parser, sample_config_file):
        """Test parsing aliases from XML."""
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(sample_config_file)

        # Check IP aliases
        assert isinstance(ip_aliases, dict)
        assert '192.168.1.100' in ip_aliases
        assert ip_aliases['192.168.1.100'] == 'WebServers'
        assert '192.168.1.101' in ip_aliases
        assert ip_aliases['192.168.1.101'] == 'WebServers'

        # Check port aliases
        assert isinstance(port_aliases, dict)
        assert '80' in port_aliases
        assert port_aliases['80'] == 'HTTPPorts'
        assert '443' in port_aliases
        assert port_aliases['443'] == 'HTTPPorts'
        assert '8080' in port_aliases
        assert port_aliases['8080'] == 'HTTPPorts'

    def test_parse_aliases_disabled_alias(self, config_parser, sample_config_file):
        """Test that disabled aliases are not included."""
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(sample_config_file)

        # DisabledAlias should not be in the results
        assert '1.1.1.1' not in ip_aliases

    def test_parse_aliases_network_type(self, config_parser, sample_config_file):
        """Test parsing network type aliases."""
        ip_aliases, _ = config_parser.parse_aliases_from_xml(sample_config_file)

        # Network aliases should be parsed
        assert '10.0.0.0/8' in ip_aliases
        assert ip_aliases['10.0.0.0/8'] == 'PrivateNetwork'
        assert '172.16.0.0/12' in ip_aliases
        assert '192.168.0.0/16' in ip_aliases

    def test_parse_aliases_nonexistent_file(self, config_parser):
        """Test parsing aliases from non-existent file."""
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml('/nonexistent/config.xml')
        assert ip_aliases == {}
        assert port_aliases == {}

    def test_parse_aliases_malformed_xml(self, config_parser, malformed_config_file):
        """Test parsing aliases from malformed XML."""
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(malformed_config_file)
        assert isinstance(ip_aliases, dict)
        assert isinstance(port_aliases, dict)

    def test_parse_aliases_no_aliases_section(self, config_parser, temp_dir):
        """Test parsing XML without aliases section."""
        from pathlib import Path
        xml_content = """<?xml version="1.0"?>
<opnsense>
  <interfaces>
    <lan>
      <if>em0</if>
    </lan>
  </interfaces>
</opnsense>"""
        xml_path = Path(temp_dir) / 'no_aliases.xml'
        xml_path.write_text(xml_content)

        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(str(xml_path))
        assert ip_aliases == {}
        assert port_aliases == {}

    def test_get_alias_name(self, config_parser):
        """Test extracting alias name."""
        from xml.etree.ElementTree import Element, SubElement
        alias_elem = Element('alias')
        name_elem = SubElement(alias_elem, 'name')
        name_elem.text = 'TestAlias'

        name = config_parser._get_alias_name(alias_elem)
        assert name == 'TestAlias'

    def test_get_alias_name_missing(self, config_parser):
        """Test extracting alias name when missing."""
        from xml.etree.ElementTree import Element
        alias_elem = Element('alias')

        name = config_parser._get_alias_name(alias_elem)
        assert name == ''

    def test_get_alias_type(self, config_parser):
        """Test extracting alias type."""
        from xml.etree.ElementTree import Element, SubElement
        alias_elem = Element('alias')
        type_elem = SubElement(alias_elem, 'type')
        type_elem.text = 'host'

        alias_type = config_parser._get_alias_type(alias_elem)
        assert alias_type == 'host'

    def test_get_alias_content(self, config_parser):
        """Test extracting alias content."""
        from xml.etree.ElementTree import Element, SubElement
        alias_elem = Element('alias')
        content_elem = SubElement(alias_elem, 'content')
        content_elem.text = '192.168.1.1\n192.168.1.2'

        content = config_parser._get_alias_content(alias_elem)
        assert content == '192.168.1.1\n192.168.1.2'

    def test_process_ip_alias_single(self, config_parser):
        """Test processing single IP alias."""
        ip_aliases = {}
        config_parser._process_ip_alias('TestHost', '192.168.1.100', ip_aliases)

        assert '192.168.1.100' in ip_aliases
        assert ip_aliases['192.168.1.100'] == 'TestHost'

    def test_process_ip_alias_multiple(self, config_parser):
        """Test processing multiple IPs in alias."""
        ip_aliases = {}
        content = '192.168.1.100\n192.168.1.101\n192.168.1.102'
        config_parser._process_ip_alias('WebServers', content, ip_aliases)

        assert len(ip_aliases) == 3
        assert ip_aliases['192.168.1.100'] == 'WebServers'
        assert ip_aliases['192.168.1.101'] == 'WebServers'
        assert ip_aliases['192.168.1.102'] == 'WebServers'

    def test_process_ip_alias_comma_separated(self, config_parser):
        """Test processing comma-separated IPs."""
        ip_aliases = {}
        content = '192.168.1.100,192.168.1.101,192.168.1.102'
        config_parser._process_ip_alias('WebServers', content, ip_aliases)

        assert len(ip_aliases) == 3

    def test_process_port_alias_single(self, config_parser):
        """Test processing single port alias."""
        port_aliases = {}
        config_parser._process_port_alias('HTTP', '80', port_aliases)

        assert '80' in port_aliases
        assert port_aliases['80'] == 'HTTP'

    def test_process_port_alias_multiple(self, config_parser):
        """Test processing multiple ports in alias."""
        port_aliases = {}
        content = '80\n443\n8080'
        config_parser._process_port_alias('HTTPPorts', content, port_aliases)

        assert len(port_aliases) == 3
        assert port_aliases['80'] == 'HTTPPorts'
        assert port_aliases['443'] == 'HTTPPorts'
        assert port_aliases['8080'] == 'HTTPPorts'

    def test_process_port_alias_range_colon(self, config_parser):
        """Test processing port range with colon separator."""
        port_aliases = {}
        config_parser._process_port_alias('HighPorts', '8080:8090', port_aliases)

        # Should expand the range
        assert '8080' in port_aliases
        assert '8085' in port_aliases
        assert '8090' in port_aliases
        assert len([k for k in port_aliases if port_aliases[k] == 'HighPorts']) == 11

    def test_process_port_alias_range_dash(self, config_parser):
        """Test processing port range with dash separator."""
        port_aliases = {}
        config_parser._process_port_alias('HighPorts', '8080-8090', port_aliases)

        # Should expand the range
        assert '8080' in port_aliases
        assert '8085' in port_aliases
        assert '8090' in port_aliases

    def test_process_port_alias_invalid_range(self, config_parser):
        """Test processing invalid port range."""
        port_aliases = {}
        config_parser._process_port_alias('InvalidPorts', 'abc:def', port_aliases)

        # Should treat as simple port
        assert 'abc:def' in port_aliases

    def test_get_ip_alias(self, config_parser):
        """Test getting IP alias."""
        config_parser.ip_aliases = {
            '192.168.1.100': 'WebServer1',
            '10.0.0.0/8': 'PrivateNet'
        }

        assert config_parser.get_ip_alias('192.168.1.100') == 'WebServer1'
        assert config_parser.get_ip_alias('10.0.0.0/8') == 'PrivateNet'
        assert config_parser.get_ip_alias('1.2.3.4') is None

    def test_get_port_alias(self, config_parser):
        """Test getting port alias."""
        config_parser.port_aliases = {
            '80': 'HTTP',
            '443': 'HTTPS'
        }

        assert config_parser.get_port_alias('80') == 'HTTP'
        assert config_parser.get_port_alias('443') == 'HTTPS'
        assert config_parser.get_port_alias('22') is None

    def test_parse_complete_workflow(self, config_parser, sample_config_file):
        """Test complete workflow: parse interfaces and aliases."""
        # Parse interfaces
        interface_mapping = config_parser.parse_interfaces_from_xml(sample_config_file)
        assert len(interface_mapping) == 3

        # Parse aliases
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(sample_config_file)
        assert len(ip_aliases) > 0
        assert len(port_aliases) > 0

        # Store in parser
        config_parser.interface_mapping = interface_mapping
        config_parser.ip_aliases = ip_aliases
        config_parser.port_aliases = port_aliases

        # Verify access
        assert config_parser.get_ip_alias('192.168.1.100') is not None
        assert config_parser.get_port_alias('80') is not None

    def test_alias_reference_resolution(self, config_parser, temp_dir):
        """Test resolving alias references to other aliases."""
        from pathlib import Path
        xml_content = """<?xml version="1.0"?>
<opnsense>
  <Firewall>
    <Alias>
      <aliases>
        <alias>
          <enabled>1</enabled>
          <name>WebServer1</name>
          <type>host</type>
          <content>192.168.1.100</content>
        </alias>
        <alias>
          <enabled>1</enabled>
          <name>AllWebServers</name>
          <type>host</type>
          <content>WebServer1
192.168.1.101</content>
        </alias>
      </aliases>
    </Alias>
  </Firewall>
</opnsense>"""
        xml_path = Path(temp_dir) / 'alias_ref.xml'
        xml_path.write_text(xml_content)

        ip_aliases, _ = config_parser.parse_aliases_from_xml(str(xml_path))

        # AllWebServers should resolve WebServer1 reference
        assert '192.168.1.100' in ip_aliases
        assert '192.168.1.101' in ip_aliases

    def test_empty_alias_content(self, config_parser, temp_dir):
        """Test handling empty alias content."""
        from pathlib import Path
        xml_content = """<?xml version="1.0"?>
<opnsense>
  <Firewall>
    <Alias>
      <aliases>
        <alias>
          <enabled>1</enabled>
          <name>EmptyAlias</name>
          <type>host</type>
          <content></content>
        </alias>
      </aliases>
    </Alias>
  </Firewall>
</opnsense>"""
        xml_path = Path(temp_dir) / 'empty_alias.xml'
        xml_path.write_text(xml_content)

        ip_aliases, _ = config_parser.parse_aliases_from_xml(str(xml_path))

        # Should handle gracefully
        assert isinstance(ip_aliases, dict)
