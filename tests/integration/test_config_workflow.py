"""
Integration tests for configuration workflows.
"""
import pytest
from pathlib import Path

from opnsense_log_viewer.services.config_parser import OPNsenseConfigParser
from opnsense_log_viewer.services.log_parser import OPNsenseLogParser


@pytest.mark.integration
class TestConfigWorkflow:
    """Test configuration loading and application workflows."""

    def test_load_config_and_apply_to_parser(self, sample_config_file):
        """Test loading config and applying to parser."""
        # Load config
        config_parser = OPNsenseConfigParser()
        interface_mapping = config_parser.parse_interfaces_from_xml(sample_config_file)

        assert len(interface_mapping) > 0

        # Apply to log parser
        log_parser = OPNsenseLogParser()
        log_parser.set_interface_mapping(interface_mapping)

        # Verify mapping is set
        assert log_parser.interface_mapping == interface_mapping

    def test_parse_all_config_elements(self, sample_config_file):
        """Test parsing all elements from config."""
        config_parser = OPNsenseConfigParser()

        # Parse interfaces
        interfaces = config_parser.parse_interfaces_from_xml(sample_config_file)

        # Parse aliases
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(sample_config_file)

        # Verify all elements parsed
        assert len(interfaces) > 0
        assert 'em0' in interfaces or 'em1' in interfaces

        assert len(ip_aliases) > 0
        assert len(port_aliases) > 0

    def test_interface_mapping_application(self, sample_config_file, sample_log_file):
        """Test that interface mapping is correctly applied."""
        # Load interface mapping
        config_parser = OPNsenseConfigParser()
        interface_mapping = config_parser.parse_interfaces_from_xml(sample_config_file)

        # Create parser with mapping
        log_parser = OPNsenseLogParser()
        log_parser.set_interface_mapping(interface_mapping)

        # Parse logs
        entries = log_parser.parse_log_file(sample_log_file)

        # Check that display names are applied
        for entry in entries:
            if entry.get('interface') in interface_mapping:
                assert entry.get('interface_display') == interface_mapping[entry['interface']]

    def test_alias_lookup_workflow(self, sample_config_file):
        """Test alias lookup workflow."""
        config_parser = OPNsenseConfigParser()

        # Parse aliases
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(sample_config_file)

        # Store in parser
        config_parser.ip_aliases = ip_aliases
        config_parser.port_aliases = port_aliases

        # Lookup IP alias
        if ip_aliases:
            first_ip = list(ip_aliases.keys())[0]
            alias_name = config_parser.get_ip_alias(first_ip)
            assert alias_name is not None
            assert alias_name == ip_aliases[first_ip]

        # Lookup port alias
        if port_aliases:
            first_port = list(port_aliases.keys())[0]
            alias_name = config_parser.get_port_alias(first_port)
            assert alias_name is not None
            assert alias_name == port_aliases[first_port]

    def test_multiple_interface_types(self, sample_config_file):
        """Test handling multiple interface types."""
        config_parser = OPNsenseConfigParser()
        interface_mapping = config_parser.parse_interfaces_from_xml(sample_config_file)

        # Should have LAN, WAN, and possibly OPT interfaces
        assert len(interface_mapping) >= 2

        # Check that different interfaces are mapped
        physical_interfaces = list(interface_mapping.keys())
        assert len(set(physical_interfaces)) == len(physical_interfaces)  # All unique

    def test_alias_types_separation(self, sample_config_file):
        """Test that IP and port aliases are separated correctly."""
        config_parser = OPNsenseConfigParser()
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(sample_config_file)

        # Verify separation
        # IP aliases should have IP addresses or networks
        for ip_key in ip_aliases.keys():
            # Should contain dots (IPv4) or slashes (network)
            assert '.' in ip_key or '/' in ip_key or ':' in ip_key

        # Port aliases should have numeric ports
        for port_key in port_aliases.keys():
            # Should be numeric or numeric range
            assert port_key.replace('-', '').replace(':', '').isdigit()

    def test_config_update_workflow(self, sample_config_file):
        """Test updating configuration."""
        log_parser = OPNsenseLogParser()
        config_parser = OPNsenseConfigParser()

        # Load initial config
        mapping1 = config_parser.parse_interfaces_from_xml(sample_config_file)
        log_parser.set_interface_mapping(mapping1)

        # Update with new mapping
        mapping2 = {'em0': 'Updated LAN', 'em1': 'Updated WAN'}
        log_parser.set_interface_mapping(mapping2)

        # Verify update
        assert log_parser.interface_mapping == mapping2

    def test_partial_config_workflow(self, temp_dir):
        """Test handling config with only some elements."""
        # Create config with only interfaces, no aliases
        xml_content = """<?xml version="1.0"?>
<opnsense>
  <interfaces>
    <lan>
      <if>em0</if>
      <descr>LAN</descr>
    </lan>
  </interfaces>
</opnsense>"""
        xml_path = Path(temp_dir) / 'partial.xml'
        xml_path.write_text(xml_content)

        config_parser = OPNsenseConfigParser()

        # Parse interfaces - should work
        interfaces = config_parser.parse_interfaces_from_xml(str(xml_path))
        assert len(interfaces) > 0

        # Parse aliases - should return empty
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(str(xml_path))
        assert ip_aliases == {}
        assert port_aliases == {}

    def test_config_with_special_characters(self, temp_dir):
        """Test config with special characters in descriptions."""
        xml_content = """<?xml version="1.0"?>
<opnsense>
  <interfaces>
    <lan>
      <if>em0</if>
      <descr>LAN (Internal Network) - Main</descr>
    </lan>
  </interfaces>
</opnsense>"""
        xml_path = Path(temp_dir) / 'special.xml'
        xml_path.write_text(xml_content)

        config_parser = OPNsenseConfigParser()
        interfaces = config_parser.parse_interfaces_from_xml(str(xml_path))

        assert 'em0' in interfaces
        assert '(' in interfaces['em0'] and ')' in interfaces['em0']

    def test_reusable_config_parser(self, sample_config_file, temp_dir):
        """Test that config parser is reusable."""
        config_parser = OPNsenseConfigParser()

        # Parse first file
        interfaces1 = config_parser.parse_interfaces_from_xml(sample_config_file)
        aliases1_ip, aliases1_port = config_parser.parse_aliases_from_xml(sample_config_file)

        # Create second config
        xml_content = """<?xml version="1.0"?>
<opnsense>
  <interfaces>
    <wan>
      <if>em1</if>
      <descr>WAN2</descr>
    </wan>
  </interfaces>
</opnsense>"""
        xml_path2 = Path(temp_dir) / 'second.xml'
        xml_path2.write_text(xml_content)

        # Parse second file
        interfaces2 = config_parser.parse_interfaces_from_xml(str(xml_path2))

        # Both should work
        assert len(interfaces1) > 0
        assert len(interfaces2) > 0

    def test_interface_without_description_fallback(self, temp_dir):
        """Test fallback for interface without description."""
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

        config_parser = OPNsenseConfigParser()
        interfaces = config_parser.parse_interfaces_from_xml(str(xml_path))

        # Should fallback to interface name in uppercase
        assert 'em0' in interfaces
        assert interfaces['em0'] == 'LAN'

    def test_empty_config_handling(self, temp_dir):
        """Test handling of empty config."""
        xml_content = """<?xml version="1.0"?>
<opnsense>
</opnsense>"""
        xml_path = Path(temp_dir) / 'empty.xml'
        xml_path.write_text(xml_content)

        config_parser = OPNsenseConfigParser()

        interfaces = config_parser.parse_interfaces_from_xml(str(xml_path))
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(str(xml_path))

        # Should handle gracefully
        assert interfaces == {}
        assert ip_aliases == {}
        assert port_aliases == {}

    def test_alias_multiline_content(self, temp_dir):
        """Test parsing aliases with multiline content."""
        xml_content = """<?xml version="1.0"?>
<opnsense>
  <Firewall>
    <Alias>
      <aliases>
        <alias>
          <enabled>1</enabled>
          <name>Servers</name>
          <type>host</type>
          <content>192.168.1.10
192.168.1.11
192.168.1.12</content>
        </alias>
      </aliases>
    </Alias>
  </Firewall>
</opnsense>"""
        xml_path = Path(temp_dir) / 'multiline.xml'
        xml_path.write_text(xml_content)

        config_parser = OPNsenseConfigParser()
        ip_aliases, _ = config_parser.parse_aliases_from_xml(str(xml_path))

        # Should parse all IPs
        assert '192.168.1.10' in ip_aliases
        assert '192.168.1.11' in ip_aliases
        assert '192.168.1.12' in ip_aliases
        assert ip_aliases['192.168.1.10'] == 'Servers'

    def test_port_range_expansion(self, temp_dir):
        """Test port range expansion in aliases."""
        xml_content = """<?xml version="1.0"?>
<opnsense>
  <Firewall>
    <Alias>
      <aliases>
        <alias>
          <enabled>1</enabled>
          <name>HighPorts</name>
          <type>port</type>
          <content>8080:8085</content>
        </alias>
      </aliases>
    </Alias>
  </Firewall>
</opnsense>"""
        xml_path = Path(temp_dir) / 'port_range.xml'
        xml_path.write_text(xml_content)

        config_parser = OPNsenseConfigParser()
        _, port_aliases = config_parser.parse_aliases_from_xml(str(xml_path))

        # Should expand range
        assert '8080' in port_aliases
        assert '8082' in port_aliases
        assert '8085' in port_aliases
        assert all(port_aliases[str(p)] == 'HighPorts' for p in range(8080, 8086) if str(p) in port_aliases)
