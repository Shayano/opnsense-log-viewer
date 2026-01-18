import { ChevronDown } from 'lucide-react';
import type { FieldType, OperatorType } from '@/types/filter';

interface OperatorSelectorProps {
  fieldType: FieldType;
  value?: OperatorType;
  onChange: (operator: OperatorType) => void;
  error?: string;
}

// Operator options per field type (context-aware)
const OPERATOR_MAP: Record<string, { value: OperatorType; label: string }[]> = {
  text: [
    { value: 'equals', label: 'Equals' },
    { value: 'contains', label: 'Contains' },
    { value: 'startsWith', label: 'Starts With' },
    { value: 'endsWith', label: 'Ends With' },
    { value: 'regex', label: 'Regex' },
  ],
  numeric: [
    { value: 'equals', label: 'Equals' },
    { value: 'greaterThan', label: 'Greater Than' },
    { value: 'lessThan', label: 'Less Than' },
    { value: 'between', label: 'Between' },
  ],
  select: [
    { value: 'equals', label: 'Equals' },
    { value: 'notEquals', label: 'Not Equals' },
  ],
  timestamp: [
    { value: 'absoluteRange', label: 'Absolute Range (Date Picker)' },
    { value: 'relative', label: 'Relative (Last 1h, 24h, etc.)' },
  ],
};

function getOperatorCategory(fieldType: FieldType): keyof typeof OPERATOR_MAP {
  if (fieldType === 'sourcePort' || fieldType === 'destinationPort') return 'numeric';
  if (fieldType === 'protocol' || fieldType === 'action') return 'select';
  if (fieldType === 'timestamp') return 'timestamp';
  return 'text'; // sourceIp, destinationIp, interface, ruleLabel
}

export function OperatorSelector({ fieldType, value, onChange, error }: OperatorSelectorProps): JSX.Element {
  const category = getOperatorCategory(fieldType);
  const operators = OPERATOR_MAP[category];

  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        2. Select Operator
      </label>
      <div className="relative">
        <select
          value={value || ''}
          onChange={(e) => onChange(e.target.value as OperatorType)}
          className="w-full px-3 py-2 pr-10 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500 focus:border-transparent
            appearance-none cursor-pointer"
          aria-label="Select operator"
        >
          <option value="">Choose an operator...</option>
          {operators.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <ChevronDown className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500 pointer-events-none" />
      </div>
      {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
    </div>
  );
}
