import type { Filter, FieldType, OperatorType } from '@/types/filter';

// Map field types to human-readable labels
const FIELD_LABELS: Record<FieldType, string> = {
  timestamp: 'Time',
  sourceIp: 'Source IP',
  destinationIp: 'Dest IP',
  sourcePort: 'Source Port',
  destinationPort: 'Dest Port',
  protocol: 'Protocol',
  action: 'Action',
  interface: 'Interface',
  ruleLabel: 'Rule',
};

// Map operators to readable symbols
const OPERATOR_SYMBOLS: Partial<Record<OperatorType, string>> = {
  equals: ':',
  contains: '~',
  startsWith: '^',
  endsWith: '$',
  greaterThan: '>',
  lessThan: '<',
  notEquals: '≠',
  regex: '~regex',
  between: 'between',
  absoluteRange: 'range',
  relative: 'last',
};

// Format a single filter as readable text
function formatSingleFilter(filter: Omit<Filter, 'id'>): string {
  const fieldLabel = FIELD_LABELS[filter.field] || filter.field;
  const operatorSymbol = OPERATOR_SYMBOLS[filter.operator] || filter.operator;

  // Format value appropriately
  let valueStr: string;
  if (Array.isArray(filter.value)) {
    // Between operator or timestamp range
    valueStr = `[${filter.value[0]} - ${filter.value[1]}]`;
  } else if (filter.operator === 'relative') {
    // Relative time range
    valueStr = `${filter.value}`;
  } else {
    valueStr = String(filter.value);
  }

  return `${fieldLabel} ${operatorSymbol} ${valueStr}`;
}

// Format array of filters into readable summary with logic operators
export function formatFilterSummary(filters: Omit<Filter, 'id'>[], maxLength: number = 60): string {
  if (filters.length === 0) return 'No filters';

  // Single filter case
  if (filters.length === 1) {
    const formatted = formatSingleFilter(filters[0]);
    if (formatted.length > maxLength) {
      return formatted.substring(0, maxLength - 3) + '...';
    }
    return formatted;
  }

  // Combine filters with logic operators
  const parts: string[] = [];
  filters.forEach((filter, index) => {
    parts.push(formatSingleFilter(filter));
    if (index < filters.length - 1) {
      // Add logic operator between filters
      const logic = filter.logic || 'AND';
      parts.push(logic);
    }
  });

  const fullSummary = parts.join(' ');

  // Truncate if too long
  if (fullSummary.length > maxLength) {
    return fullSummary.substring(0, maxLength - 3) + '...';
  }

  return fullSummary;
}

// Format full filter details for tooltip (no truncation)
export function formatFullFilterDetails(filters: Omit<Filter, 'id'>[]): string {
  if (filters.length === 0) return 'No filters';

  const parts: string[] = [];
  filters.forEach((filter, index) => {
    parts.push(`${index + 1}. ${formatSingleFilter(filter)}`);
  });

  return parts.join('\n');
}
