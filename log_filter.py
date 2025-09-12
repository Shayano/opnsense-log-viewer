"""
Improved filtering system for OPNsense logs
Enhanced support for interface display names and better filter management
"""
import re
from typing import List, Dict, Any, Callable, Optional
from datetime import datetime
from log_parser import LogEntry

class FilterCondition:
    """Represents a filtering condition"""
    
    def __init__(self, field: str, operator: str, value: str, case_sensitive: bool = False):
        self.field = field
        self.operator = operator  # '==', '!=', 'contains', 'startswith', 'endswith', 'regex', '>', '<', '>=', '<='
        self.value = value
        self.case_sensitive = case_sensitive
        
    def evaluate(self, entry: LogEntry) -> bool:
        """Evaluate the condition on a log entry"""
        # Handle special interface field mapping
        if self.field == 'interface':
            # Check both physical and logical interface names
            field_value = entry.get('interface', '')
            interface_display = entry.get('interface_display', '')
            
            # Check if the value matches either physical or display name
            if self._check_value_match(field_value) or self._check_value_match(interface_display):
                return True
            return False
        else:
            field_value = entry.get(self.field, '')
            return self._check_value_match(field_value)
    
    def _check_value_match(self, field_value: Any) -> bool:
        """Check if field value matches the condition"""
        if not self.case_sensitive and isinstance(field_value, str):
            field_value = field_value.lower()
            comparison_value = self.value.lower()
        else:
            comparison_value = self.value
            
        try:
            if self.operator == '==':
                return str(field_value) == comparison_value
            elif self.operator == '!=':
                return str(field_value) != comparison_value
            elif self.operator == 'contains':
                return comparison_value in str(field_value)
            elif self.operator == 'startswith':
                return str(field_value).startswith(comparison_value)
            elif self.operator == 'endswith':
                return str(field_value).endswith(comparison_value)
            elif self.operator == 'regex':
                pattern = re.compile(self.value, re.IGNORECASE if not self.case_sensitive else 0)
                return bool(pattern.search(str(field_value)))
            elif self.operator == '>':
                return self._numeric_compare(field_value, comparison_value, lambda x, y: x > y)
            elif self.operator == '<':
                return self._numeric_compare(field_value, comparison_value, lambda x, y: x < y)
            elif self.operator == '>=':
                return self._numeric_compare(field_value, comparison_value, lambda x, y: x >= y)
            elif self.operator == '<=':
                return self._numeric_compare(field_value, comparison_value, lambda x, y: x <= y)
        except Exception:
            return False
            
        return False
    
    def _numeric_compare(self, field_value: Any, comparison_value: str, compare_func: Callable) -> bool:
        """Numeric comparison with error handling"""
        try:
            num_field = float(field_value)
            num_comparison = float(comparison_value)
            return compare_func(num_field, num_comparison)
        except (ValueError, TypeError):
            return False

class FilterExpression:
    """Represents a complex filtering expression with AND, OR, NOT"""
    
    def __init__(self):
        self.conditions = []
        self.operators = []  # 'AND', 'OR'
        self.negations = []  # True if corresponding condition is negated (NOT)
        
    def add_condition(self, condition: FilterCondition, operator: str = 'AND', negate: bool = False):
        """Add a condition to the expression"""
        self.conditions.append(condition)
        self.negations.append(negate)
        
        if len(self.conditions) > 1:
            self.operators.append(operator)
    
    def evaluate(self, entry: LogEntry) -> bool:
        """Evaluate the complete expression on a log entry"""
        if not self.conditions:
            return True
            
        # Evaluate first condition
        result = self.conditions[0].evaluate(entry)
        if self.negations[0]:
            result = not result
            
        # Evaluate subsequent conditions with operators
        for i in range(1, len(self.conditions)):
            condition_result = self.conditions[i].evaluate(entry)
            if self.negations[i]:
                condition_result = not condition_result
                
            operator = self.operators[i-1]
            if operator == 'AND':
                result = result and condition_result
            elif operator == 'OR':
                result = result or condition_result
                
        return result
    
    def clear(self):
        """Clear the expression"""
        self.conditions = []
        self.operators = []
        self.negations = []

class LogFilter:
    """Enhanced log filtering manager"""
    
    def __init__(self):
        self.expression = FilterExpression()
        self.time_range_start = None
        self.time_range_end = None
        
    def set_time_range(self, start: Optional[datetime] = None, end: Optional[datetime] = None):
        """Set time range for filtering"""
        self.time_range_start = start
        self.time_range_end = end
        
    def add_filter_condition(self, field: str, operator: str, value: str, 
                           logic_operator: str = 'AND', negate: bool = False, 
                           case_sensitive: bool = False):
        """Add a filter condition"""
        condition = FilterCondition(field, operator, value, case_sensitive)
        self.expression.add_condition(condition, logic_operator, negate)
        
    def clear_filters(self):
        """Clear all filters"""
        self.expression.clear()
        self.time_range_start = None
        self.time_range_end = None
        
    def filter_entries(self, entries: List[LogEntry]) -> List[LogEntry]:
        """Apply all filters to a list of entries"""
        filtered_entries = []
        
        for entry in entries:
            # Check time range
            if self.time_range_start and entry.timestamp < self.time_range_start:
                continue
            if self.time_range_end and entry.timestamp > self.time_range_end:
                continue
                
            # Check filter expression
            if self.expression.evaluate(entry):
                filtered_entries.append(entry)
                
        return filtered_entries
        
    def get_filter_summary(self) -> str:
        """Return text summary of active filters"""
        summary_parts = []
        
        # Summary of conditions
        for i, condition in enumerate(self.expression.conditions):
            negation = "NOT " if self.expression.negations[i] else ""
            condition_text = f"{negation}{condition.field} {condition.operator} '{condition.value}'"
            
            if i > 0:
                operator = self.expression.operators[i-1]
                condition_text = f" {operator} {condition_text}"
                
            summary_parts.append(condition_text)
            
        # Time range summary
        if self.time_range_start or self.time_range_end:
            time_text = "Time range: "
            if self.time_range_start:
                time_text += f"from {self.time_range_start.strftime('%Y-%m-%d %H:%M:%S')} "
            if self.time_range_end:
                time_text += f"to {self.time_range_end.strftime('%Y-%m-%d %H:%M:%S')}"
            summary_parts.append(time_text)
            
        return " | ".join(summary_parts) if summary_parts else "No filters active"
