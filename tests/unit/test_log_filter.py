"""
Unit tests for Log Filter.
"""
import pytest
from datetime import datetime, timedelta

from opnsense_log_viewer.services.log_filter import (
    FilterCondition, FilterExpression, LogFilter
)


@pytest.mark.unit
class TestFilterCondition:
    """Test FilterCondition class."""

    def test_condition_creation(self):
        """Test creating a filter condition."""
        condition = FilterCondition('action', '==', 'pass')
        assert condition.field == 'action'
        assert condition.operator == '=='
        assert condition.value == 'pass'
        assert condition.case_sensitive == False

    def test_condition_case_sensitive(self):
        """Test case-sensitive condition."""
        condition = FilterCondition('src', 'contains', 'Test', case_sensitive=True)
        assert condition.case_sensitive == True

    def test_evaluate_equals(self, log_entry_factory):
        """Test equals operator."""
        entry = log_entry_factory(action='pass')
        condition = FilterCondition('action', '==', 'pass')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('action', '==', 'block')
        assert condition.evaluate(entry) == False

    def test_evaluate_not_equals(self, log_entry_factory):
        """Test not equals operator."""
        entry = log_entry_factory(action='pass')
        condition = FilterCondition('action', '!=', 'block')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('action', '!=', 'pass')
        assert condition.evaluate(entry) == False

    def test_evaluate_contains(self, log_entry_factory):
        """Test contains operator."""
        entry = log_entry_factory(src='192.168.1.100')
        condition = FilterCondition('src', 'contains', '168')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('src', 'contains', '999')
        assert condition.evaluate(entry) == False

    def test_evaluate_contains_case_insensitive(self, log_entry_factory):
        """Test case-insensitive contains."""
        entry = log_entry_factory(action='PASS')
        condition = FilterCondition('action', 'contains', 'pass', case_sensitive=False)
        assert condition.evaluate(entry) == True

    def test_evaluate_contains_case_sensitive(self, log_entry_factory):
        """Test case-sensitive contains."""
        entry = log_entry_factory(action='PASS')
        condition = FilterCondition('action', 'contains', 'pass', case_sensitive=True)
        assert condition.evaluate(entry) == False

        condition = FilterCondition('action', 'contains', 'PASS', case_sensitive=True)
        assert condition.evaluate(entry) == True

    def test_evaluate_startswith(self, log_entry_factory):
        """Test startswith operator."""
        entry = log_entry_factory(src='192.168.1.100')
        condition = FilterCondition('src', 'startswith', '192')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('src', 'startswith', '168')
        assert condition.evaluate(entry) == False

    def test_evaluate_endswith(self, log_entry_factory):
        """Test endswith operator."""
        entry = log_entry_factory(src='192.168.1.100')
        condition = FilterCondition('src', 'endswith', '100')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('src', 'endswith', '200')
        assert condition.evaluate(entry) == False

    def test_evaluate_regex(self, log_entry_factory):
        """Test regex operator."""
        entry = log_entry_factory(src='192.168.1.100')
        condition = FilterCondition('src', 'regex', r'\d+\.\d+\.\d+\.\d+')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('src', 'regex', r'^10\.')
        assert condition.evaluate(entry) == False

    def test_evaluate_regex_case_insensitive(self, log_entry_factory):
        """Test case-insensitive regex."""
        entry = log_entry_factory(action='PASS')
        condition = FilterCondition('action', 'regex', 'pass', case_sensitive=False)
        assert condition.evaluate(entry) == True

    def test_evaluate_greater_than(self, log_entry_factory):
        """Test greater than operator."""
        entry = log_entry_factory(srcport='8080')
        condition = FilterCondition('srcport', '>', '1000')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('srcport', '>', '9000')
        assert condition.evaluate(entry) == False

    def test_evaluate_less_than(self, log_entry_factory):
        """Test less than operator."""
        entry = log_entry_factory(srcport='80')
        condition = FilterCondition('srcport', '<', '100')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('srcport', '<', '50')
        assert condition.evaluate(entry) == False

    def test_evaluate_greater_equal(self, log_entry_factory):
        """Test greater than or equal operator."""
        entry = log_entry_factory(srcport='80')
        condition = FilterCondition('srcport', '>=', '80')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('srcport', '>=', '100')
        assert condition.evaluate(entry) == False

    def test_evaluate_less_equal(self, log_entry_factory):
        """Test less than or equal operator."""
        entry = log_entry_factory(srcport='80')
        condition = FilterCondition('srcport', '<=', '80')
        assert condition.evaluate(entry) == True

        condition = FilterCondition('srcport', '<=', '50')
        assert condition.evaluate(entry) == False

    def test_evaluate_interface_mapping(self, log_entry_factory):
        """Test interface field with display name mapping."""
        entry = log_entry_factory(interface='em0')
        entry.parsed_data['interface_display'] = 'LAN Network'

        # Should match physical interface
        condition = FilterCondition('interface', '==', 'em0')
        assert condition.evaluate(entry) == True

        # Should match display name
        condition = FilterCondition('interface', '==', 'LAN Network')
        assert condition.evaluate(entry) == True

        # Should not match wrong name
        condition = FilterCondition('interface', '==', 'WAN')
        assert condition.evaluate(entry) == False

    def test_evaluate_missing_field(self, log_entry_factory):
        """Test evaluation with missing field."""
        entry = log_entry_factory()
        condition = FilterCondition('nonexistent', '==', 'value')
        assert condition.evaluate(entry) == False

    def test_evaluate_invalid_numeric_comparison(self, log_entry_factory):
        """Test numeric comparison with non-numeric value."""
        entry = log_entry_factory(action='pass')
        condition = FilterCondition('action', '>', '100')
        assert condition.evaluate(entry) == False

    def test_evaluate_exception_handling(self, log_entry_factory):
        """Test exception handling in evaluation."""
        entry = log_entry_factory()
        # Invalid regex should not crash
        condition = FilterCondition('src', 'regex', '[invalid(regex')
        result = condition.evaluate(entry)
        # Should return False on error
        assert isinstance(result, bool)


@pytest.mark.unit
class TestFilterExpression:
    """Test FilterExpression class."""

    def test_expression_creation(self):
        """Test creating a filter expression."""
        expr = FilterExpression()
        assert expr.conditions == []
        assert expr.operators == []
        assert expr.negations == []

    def test_add_single_condition(self):
        """Test adding a single condition."""
        expr = FilterExpression()
        condition = FilterCondition('action', '==', 'pass')
        expr.add_condition(condition)

        assert len(expr.conditions) == 1
        assert len(expr.operators) == 0

    def test_add_multiple_conditions_and(self):
        """Test adding conditions with AND operator."""
        expr = FilterExpression()
        expr.add_condition(FilterCondition('action', '==', 'pass'), 'AND')
        expr.add_condition(FilterCondition('src', 'contains', '192'), 'AND')

        assert len(expr.conditions) == 2
        assert len(expr.operators) == 1
        assert expr.operators[0] == 'AND'

    def test_add_multiple_conditions_or(self):
        """Test adding conditions with OR operator."""
        expr = FilterExpression()
        expr.add_condition(FilterCondition('action', '==', 'pass'))
        expr.add_condition(FilterCondition('action', '==', 'block'), 'OR')

        assert len(expr.operators) == 1
        assert expr.operators[0] == 'OR'

    def test_add_condition_with_negation(self):
        """Test adding negated condition."""
        expr = FilterExpression()
        expr.add_condition(FilterCondition('action', '==', 'pass'), negate=True)

        assert expr.negations[0] == True

    def test_evaluate_empty_expression(self, log_entry_factory):
        """Test evaluating empty expression."""
        expr = FilterExpression()
        entry = log_entry_factory()

        assert expr.evaluate(entry) == True

    def test_evaluate_single_condition(self, log_entry_factory):
        """Test evaluating single condition."""
        expr = FilterExpression()
        expr.add_condition(FilterCondition('action', '==', 'pass'))

        entry_pass = log_entry_factory(action='pass')
        assert expr.evaluate(entry_pass) == True

        entry_block = log_entry_factory(action='block')
        assert expr.evaluate(entry_block) == False

    def test_evaluate_and_conditions(self, log_entry_factory):
        """Test evaluating AND conditions."""
        expr = FilterExpression()
        expr.add_condition(FilterCondition('action', '==', 'pass'))
        expr.add_condition(FilterCondition('protoname', '==', 'tcp'), 'AND')

        entry_match = log_entry_factory(action='pass', protoname='tcp')
        assert expr.evaluate(entry_match) == True

        entry_no_match = log_entry_factory(action='pass', protoname='udp')
        assert expr.evaluate(entry_no_match) == False

    def test_evaluate_or_conditions(self, log_entry_factory):
        """Test evaluating OR conditions."""
        expr = FilterExpression()
        expr.add_condition(FilterCondition('action', '==', 'pass'))
        expr.add_condition(FilterCondition('action', '==', 'block'), 'OR')

        entry_pass = log_entry_factory(action='pass')
        assert expr.evaluate(entry_pass) == True

        entry_block = log_entry_factory(action='block')
        assert expr.evaluate(entry_block) == True

        entry_reject = log_entry_factory(action='reject')
        assert expr.evaluate(entry_reject) == False

    def test_evaluate_negated_condition(self, log_entry_factory):
        """Test evaluating negated condition."""
        expr = FilterExpression()
        expr.add_condition(FilterCondition('action', '==', 'block'), negate=True)

        entry_pass = log_entry_factory(action='pass')
        assert expr.evaluate(entry_pass) == True

        entry_block = log_entry_factory(action='block')
        assert expr.evaluate(entry_block) == False

    def test_evaluate_complex_expression(self, log_entry_factory):
        """Test evaluating complex expression."""
        # (action == 'pass' AND protoname == 'tcp') OR (action == 'block')
        expr = FilterExpression()
        expr.add_condition(FilterCondition('action', '==', 'pass'))
        expr.add_condition(FilterCondition('protoname', '==', 'tcp'), 'AND')
        expr.add_condition(FilterCondition('action', '==', 'block'), 'OR')

        entry1 = log_entry_factory(action='pass', protoname='tcp')
        assert expr.evaluate(entry1) == True

        entry2 = log_entry_factory(action='pass', protoname='udp')
        assert expr.evaluate(entry2) == False

        entry3 = log_entry_factory(action='block', protoname='udp')
        assert expr.evaluate(entry3) == True

    def test_clear_expression(self):
        """Test clearing expression."""
        expr = FilterExpression()
        expr.add_condition(FilterCondition('action', '==', 'pass'))
        expr.add_condition(FilterCondition('src', 'contains', '192'), 'AND')

        expr.clear()

        assert expr.conditions == []
        assert expr.operators == []
        assert expr.negations == []


@pytest.mark.unit
class TestLogFilter:
    """Test LogFilter class."""

    def test_filter_creation(self, log_filter):
        """Test creating a log filter."""
        assert isinstance(log_filter, LogFilter)
        assert isinstance(log_filter.expression, FilterExpression)
        assert log_filter.time_range_start is None
        assert log_filter.time_range_end is None

    def test_set_time_range(self, log_filter):
        """Test setting time range."""
        start = datetime.now() - timedelta(hours=1)
        end = datetime.now()

        log_filter.set_time_range(start, end)

        assert log_filter.time_range_start == start
        assert log_filter.time_range_end == end

    def test_add_filter_condition(self, log_filter):
        """Test adding filter condition."""
        log_filter.add_filter_condition('action', '==', 'pass')

        assert len(log_filter.expression.conditions) == 1
        assert log_filter.expression.conditions[0].field == 'action'

    def test_add_multiple_filter_conditions(self, log_filter):
        """Test adding multiple filter conditions."""
        log_filter.add_filter_condition('action', '==', 'pass')
        log_filter.add_filter_condition('protoname', '==', 'tcp', logic_operator='AND')

        assert len(log_filter.expression.conditions) == 2
        assert len(log_filter.expression.operators) == 1

    def test_clear_filters(self, log_filter):
        """Test clearing all filters."""
        log_filter.add_filter_condition('action', '==', 'pass')
        log_filter.set_time_range(datetime.now(), datetime.now())

        log_filter.clear_filters()

        assert len(log_filter.expression.conditions) == 0
        assert log_filter.time_range_start is None
        assert log_filter.time_range_end is None

    def test_filter_entries_no_filters(self, log_filter, sample_parsed_entries):
        """Test filtering with no filters."""
        filtered = log_filter.filter_entries(sample_parsed_entries)

        assert len(filtered) == len(sample_parsed_entries)

    def test_filter_entries_by_action(self, log_filter, sample_parsed_entries):
        """Test filtering by action."""
        log_filter.add_filter_condition('action', '==', 'pass')

        filtered = log_filter.filter_entries(sample_parsed_entries)

        assert all(e['action'] == 'pass' for e in filtered)
        assert len(filtered) <= len(sample_parsed_entries)

    def test_filter_entries_by_protocol(self, log_filter, sample_parsed_entries):
        """Test filtering by protocol."""
        log_filter.add_filter_condition('protoname', '==', 'tcp')

        filtered = log_filter.filter_entries(sample_parsed_entries)

        assert all(e['protoname'] == 'tcp' for e in filtered)

    def test_filter_entries_by_source(self, log_filter, sample_parsed_entries):
        """Test filtering by source IP."""
        log_filter.add_filter_condition('src', 'contains', '192.168')

        filtered = log_filter.filter_entries(sample_parsed_entries)

        assert all('192.168' in e['src'] for e in filtered)

    def test_filter_entries_time_range(self, log_filter, sample_parsed_entries):
        """Test filtering by time range."""
        # Set time range to future (should filter out all entries)
        start = datetime.now() + timedelta(days=1)
        end = datetime.now() + timedelta(days=2)
        log_filter.set_time_range(start, end)

        filtered = log_filter.filter_entries(sample_parsed_entries)

        assert len(filtered) == 0

    def test_filter_entries_combined(self, log_filter, sample_parsed_entries):
        """Test filtering with multiple conditions."""
        log_filter.add_filter_condition('action', '==', 'pass')
        log_filter.add_filter_condition('protoname', '==', 'tcp', logic_operator='AND')

        filtered = log_filter.filter_entries(sample_parsed_entries)

        assert all(e['action'] == 'pass' and e['protoname'] == 'tcp' for e in filtered)

    def test_get_filter_summary_no_filters(self, log_filter):
        """Test filter summary with no filters."""
        summary = log_filter.get_filter_summary()
        assert summary == "No filters active"

    def test_get_filter_summary_with_conditions(self, log_filter):
        """Test filter summary with conditions."""
        log_filter.add_filter_condition('action', '==', 'pass')
        log_filter.add_filter_condition('src', 'contains', '192.168', logic_operator='AND')

        summary = log_filter.get_filter_summary()

        assert 'action' in summary
        assert 'pass' in summary
        assert 'src' in summary
        assert '192.168' in summary
        assert 'AND' in summary

    def test_get_filter_summary_with_negation(self, log_filter):
        """Test filter summary with negated condition."""
        log_filter.add_filter_condition('action', '==', 'block', negate=True)

        summary = log_filter.get_filter_summary()

        assert 'NOT' in summary
        assert 'action' in summary
        assert 'block' in summary

    def test_get_filter_summary_with_time_range(self, log_filter):
        """Test filter summary with time range."""
        start = datetime(2024, 1, 15, 10, 0, 0)
        end = datetime(2024, 1, 15, 12, 0, 0)
        log_filter.set_time_range(start, end)

        summary = log_filter.get_filter_summary()

        assert 'Time range' in summary
        assert '2024-01-15' in summary

    def test_filter_entries_empty_list(self, log_filter):
        """Test filtering empty list."""
        log_filter.add_filter_condition('action', '==', 'pass')

        filtered = log_filter.filter_entries([])

        assert filtered == []

    def test_filter_case_sensitive(self, log_filter, log_entry_factory):
        """Test case-sensitive filtering."""
        log_filter.add_filter_condition('action', '==', 'PASS', case_sensitive=True)

        entry_upper = log_entry_factory(action='PASS')
        entry_lower = log_entry_factory(action='pass')

        assert len(log_filter.filter_entries([entry_upper])) == 1
        assert len(log_filter.filter_entries([entry_lower])) == 0
