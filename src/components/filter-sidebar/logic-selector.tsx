import type { LogicOperator } from '@/types/filter';

interface LogicSelectorProps {
  value: LogicOperator;
  onChange: (logic: LogicOperator) => void;
}

const LOGIC_OPTIONS: { value: LogicOperator; label: string; color: string }[] = [
  { value: 'AND', label: 'AND', color: 'bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200' },
  { value: 'OR', label: 'OR', color: 'bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200' },
  { value: 'NOT', label: 'NOT', color: 'bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200' },
];

export function LogicSelector({ value, onChange }: LogicSelectorProps): JSX.Element {
  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-gray-500 dark:text-gray-400">Next filter:</span>
      <div className="flex gap-1">
        {LOGIC_OPTIONS.map((option) => (
          <button
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
            className={`px-2 py-1 text-xs font-semibold rounded transition-all ${
              value === option.value
                ? option.color
                : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600'
            }`}
            aria-label={`Set logic operator to ${option.value}`}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}
