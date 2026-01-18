import { ChevronDown } from 'lucide-react';
import type { FieldType } from '@/types/filter';

interface FieldSelectorProps {
  value?: FieldType;
  onChange: (field: FieldType) => void;
  error?: string;
}

const FIELD_OPTIONS: { value: FieldType; label: string }[] = [
  { value: 'timestamp', label: 'Timestamp Range' },
  { value: 'sourceIp', label: 'Source IP' },
  { value: 'destinationIp', label: 'Destination IP' },
  { value: 'sourcePort', label: 'Source Port' },
  { value: 'destinationPort', label: 'Destination Port' },
  { value: 'protocol', label: 'Protocol' },
  { value: 'action', label: 'Action' },
  { value: 'interface', label: 'Interface' },
  { value: 'ruleLabel', label: 'Rule Label' },
];

export function FieldSelector({ value, onChange, error }: FieldSelectorProps): JSX.Element {
  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        1. Select Field
      </label>
      <div className="relative">
        <select
          value={value || ''}
          onChange={(e) => onChange(e.target.value as FieldType)}
          className="w-full px-3 py-2 pr-10 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500 focus:border-transparent
            appearance-none cursor-pointer"
          aria-label="Select field"
        >
          <option value="">Choose a field...</option>
          {FIELD_OPTIONS.map((option) => (
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
