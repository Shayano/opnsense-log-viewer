"""
Integration tests for complete log processing workflow.
"""
import pytest
from pathlib import Path

from opnsense_log_viewer.services.log_parser import OPNsenseLogParser
from opnsense_log_viewer.services.log_filter import LogFilter
from opnsense_log_viewer.services.config_parser import OPNsenseConfigParser


@pytest.mark.integration
class TestLogWorkflow:
    """Test complete log processing workflows."""

    def test_parse_and_filter_workflow(self, sample_log_file):
        """Test parsing logs and applying filters."""
        # Step 1: Parse logs
        parser = OPNsenseLogParser()
        entries = parser.parse_log_file(sample_log_file)

        assert len(entries) > 0

        # Step 2: Apply filter
        log_filter = LogFilter()
        log_filter.add_filter_condition('action', '==', 'pass')

        filtered = log_filter.filter_entries(entries)

        # Verify filtering worked
        assert len(filtered) <= len(entries)
        assert all(e['action'] == 'pass' for e in filtered)

    def test_parse_with_interface_mapping_workflow(self, sample_log_file, sample_config_file):
        """Test parsing logs with interface mapping from config."""
        # Step 1: Parse config to get interface mapping
        config_parser = OPNsenseConfigParser()
        interface_mapping = config_parser.parse_interfaces_from_xml(sample_config_file)

        assert len(interface_mapping) > 0

        # Step 2: Parse logs with mapping
        log_parser = OPNsenseLogParser()
        log_parser.set_interface_mapping(interface_mapping)

        entries = log_parser.parse_log_file(sample_log_file)

        # Verify interface mapping applied
        for entry in entries:
            if 'interface' in entry and entry['interface'] in interface_mapping:
                assert 'interface_display' in entry
                assert entry['interface_display'] == interface_mapping[entry['interface']]

    def test_complex_filter_workflow(self, sample_log_file):
        """Test workflow with complex filtering."""
        # Parse logs
        parser = OPNsenseLogParser()
        entries = parser.parse_log_file(sample_log_file)

        # Apply multiple filters
        log_filter = LogFilter()
        log_filter.add_filter_condition('action', '==', 'pass')
        log_filter.add_filter_condition('protoname', '==', 'tcp', logic_operator='AND')

        filtered = log_filter.filter_entries(entries)

        # Verify all conditions met
        for entry in filtered:
            assert entry['action'] == 'pass'
            assert entry['protoname'] == 'tcp'

    def test_parse_filter_reparse_workflow(self, sample_log_file):
        """Test parsing, filtering, and re-parsing workflow."""
        parser = OPNsenseLogParser()

        # First parse
        entries1 = parser.parse_log_file(sample_log_file)
        count1 = len(entries1)

        # Apply filter
        log_filter = LogFilter()
        log_filter.add_filter_condition('action', '==', 'block')
        filtered = log_filter.filter_entries(entries1)

        # Parse again (parser should be reusable)
        entries2 = parser.parse_log_file(sample_log_file)
        count2 = len(entries2)

        assert count1 == count2  # Same results

    def test_multiple_config_sources_workflow(self, sample_config_file):
        """Test using multiple config sources."""
        config_parser = OPNsenseConfigParser()

        # Parse interfaces
        interfaces = config_parser.parse_interfaces_from_xml(sample_config_file)

        # Parse aliases
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(sample_config_file)

        # Verify all parsed
        assert len(interfaces) > 0
        assert len(ip_aliases) > 0
        assert len(port_aliases) > 0

    def test_end_to_end_filtering_workflow(self, sample_log_file, sample_config_file):
        """Test complete end-to-end workflow."""
        # 1. Load configuration
        config_parser = OPNsenseConfigParser()
        interface_mapping = config_parser.parse_interfaces_from_xml(sample_config_file)
        ip_aliases, port_aliases = config_parser.parse_aliases_from_xml(sample_config_file)

        # 2. Setup parser with config
        log_parser = OPNsenseLogParser()
        log_parser.set_interface_mapping(interface_mapping)

        # 3. Parse logs
        entries = log_parser.parse_log_file(sample_log_file)
        assert len(entries) > 0

        # 4. Apply filters
        log_filter = LogFilter()
        log_filter.add_filter_condition('action', '==', 'pass')
        filtered = log_filter.filter_entries(entries)

        # 5. Verify results
        assert len(filtered) <= len(entries)
        for entry in filtered:
            assert entry['action'] == 'pass'
            # Should have interface display name if mapped
            if entry.get('interface') in interface_mapping:
                assert entry.get('interface_display')

    def test_incremental_filtering_workflow(self, sample_log_file):
        """Test workflow with incremental filter application."""
        parser = OPNsenseLogParser()
        entries = parser.parse_log_file(sample_log_file)
        initial_count = len(entries)

        # Filter 1: action == pass
        log_filter = LogFilter()
        log_filter.add_filter_condition('action', '==', 'pass')
        filtered1 = log_filter.filter_entries(entries)
        count1 = len(filtered1)

        # Filter 2: Add protocol filter
        log_filter.add_filter_condition('protoname', '==', 'tcp', logic_operator='AND')
        filtered2 = log_filter.filter_entries(entries)
        count2 = len(filtered2)

        # Each filter should reduce or maintain count
        assert count1 <= initial_count
        assert count2 <= count1

    def test_filter_clear_workflow(self, sample_log_file):
        """Test workflow with filter clearing."""
        parser = OPNsenseLogParser()
        entries = parser.parse_log_file(sample_log_file)

        log_filter = LogFilter()

        # Apply filter
        log_filter.add_filter_condition('action', '==', 'block')
        filtered1 = log_filter.filter_entries(entries)

        # Clear filter
        log_filter.clear_filters()

        # Apply no filter
        filtered2 = log_filter.filter_entries(entries)

        # After clearing, should get all entries
        assert len(filtered2) == len(entries)

    def test_error_recovery_workflow(self, invalid_log_file, sample_log_file):
        """Test workflow with error recovery."""
        parser = OPNsenseLogParser()

        # Parse invalid file (should handle gracefully)
        invalid_entries = parser.parse_log_file(invalid_log_file)

        # Should still be able to parse valid file
        valid_entries = parser.parse_log_file(sample_log_file)

        assert len(valid_entries) > 0

    def test_large_dataset_workflow(self, large_log_file):
        """Test workflow with larger dataset."""
        parser = OPNsenseLogParser()

        # Parse large file
        entries = parser.parse_log_file(large_log_file)

        assert len(entries) > 100

        # Apply filter to large dataset
        log_filter = LogFilter()
        log_filter.add_filter_condition('action', '==', 'pass')

        filtered = log_filter.filter_entries(entries)

        # Should complete without errors
        assert isinstance(filtered, list)

    def test_generator_workflow(self, sample_log_file):
        """Test workflow using generator parsing."""
        parser = OPNsenseLogParser()

        # Use generator
        entries = []
        for entry in parser.parse_log_file_generator(sample_log_file):
            entries.append(entry)

        assert len(entries) > 0

        # Apply filter to generated entries
        log_filter = LogFilter()
        log_filter.add_filter_condition('protoname', '==', 'tcp')

        filtered = log_filter.filter_entries(entries)

        assert all(e['protoname'] == 'tcp' for e in filtered)

    def test_interface_name_filtering_workflow(self, sample_log_file, sample_config_file):
        """Test filtering by interface display name."""
        # Setup
        config_parser = OPNsenseConfigParser()
        interface_mapping = config_parser.parse_interfaces_from_xml(sample_config_file)

        log_parser = OPNsenseLogParser()
        log_parser.set_interface_mapping(interface_mapping)

        entries = log_parser.parse_log_file(sample_log_file)

        # Filter by display name
        log_filter = LogFilter()
        # Get first interface display name
        if interface_mapping:
            first_display_name = list(interface_mapping.values())[0]
            log_filter.add_filter_condition('interface', 'contains', first_display_name)

            filtered = log_filter.filter_entries(entries)

            # Should have some results
            assert isinstance(filtered, list)

    def test_or_condition_workflow(self, sample_log_file):
        """Test workflow with OR conditions."""
        parser = OPNsenseLogParser()
        entries = parser.parse_log_file(sample_log_file)

        # Filter: action == 'pass' OR action == 'block'
        log_filter = LogFilter()
        log_filter.add_filter_condition('action', '==', 'pass')
        log_filter.add_filter_condition('action', '==', 'block', logic_operator='OR')

        filtered = log_filter.filter_entries(entries)

        # Should match both pass and block
        assert all(e['action'] in ['pass', 'block'] for e in filtered)

    def test_negation_workflow(self, sample_log_file):
        """Test workflow with negation."""
        parser = OPNsenseLogParser()
        entries = parser.parse_log_file(sample_log_file)

        # Filter: NOT (action == 'block')
        log_filter = LogFilter()
        log_filter.add_filter_condition('action', '==', 'block', negate=True)

        filtered = log_filter.filter_entries(entries)

        # Should not have any block entries
        assert all(e['action'] != 'block' for e in filtered)

    def test_stateful_parser_workflow(self, sample_log_file, temp_log_file):
        """Test that parser maintains state correctly."""
        parser = OPNsenseLogParser()

        # Set interface mapping
        parser.set_interface_mapping({'em0': 'LAN', 'em1': 'WAN'})

        # Parse first file
        entries1 = parser.parse_log_file(sample_log_file)

        # Parse second file (should retain mapping)
        entries2 = parser.parse_log_file(temp_log_file)

        # Both should have interface_display
        for entry in entries1:
            if entry.get('interface') in ['em0', 'em1']:
                assert entry.get('interface_display')

        for entry in entries2:
            if entry.get('interface') in ['em0', 'em1']:
                assert entry.get('interface_display')
