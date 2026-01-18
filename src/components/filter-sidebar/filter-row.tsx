import { Edit, X } from 'lucide-react';
import { useFilterStore } from '@/stores/filter-store';
import { LogicSelector } from './logic-selector';
import toast from 'react-hot-toast';
import type { Filter } from '@/types/filter';

interface FilterRowProps {
  filter: Filter;
  showLogic: boolean;
  onEdit: (filter: Filter) => void;
}

const FIELD_LABELS: Record<string, string> = {
  timestamp: 'Timestamp',
  sourceIp: 'Source IP',
  destinationIp: 'Dest IP',
  sourcePort: 'Source Port',
  destinationPort: 'Dest Port',
  protocol: 'Protocol',
  action: 'Action',
  interface: 'Interface',
  ruleLabel: 'Rule',
};

const OPERATOR_LABELS: Record<string, string> = {
  equals: '=',
  contains: 'contains',
  startsWith: 'starts with',
  endsWith: 'ends with',
  regex: 'regex',
  greaterThan: '>',
  lessThan: '<',
  between: 'between',
  notEquals: '≠',
  absoluteRange: 'range',
  relative: 'last',
};

function formatValue(value: string | number | [string, string]): string {
  if (Array.isArray(value)) {
    return `${value[0]} - ${value[1]}`;
  }
  return String(value);
}

export function FilterRow({ filter, showLogic, onEdit }: FilterRowProps): JSX.Element {
  const { removeFilter, updateFilter } = useFilterStore();

  const handleRemove = (): void => {
    removeFilter(filter.id);
    toast.success('Filter removed');
  };

  const handleEdit = (): void => {
    onEdit(filter);
  };

  return (
    <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700
      rounded p-3 space-y-2">
      {/* Filter Display */}
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          <div className="text-xs font-medium text-gray-600 dark:text-gray-400">
            {FIELD_LABELS[filter.field] || filter.field}
          </div>
          <div className="text-sm text-gray-900 dark:text-gray-100 mt-0.5">
            <span className="text-gray-500 dark:text-gray-400 mr-1">
              {OPERATOR_LABELS[filter.operator]}
            </span>
            <span className="font-mono">{formatValue(filter.value)}</span>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1">
          <button
            onClick={handleEdit}
            className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
            aria-label="Edit filter"
          >
            <Edit className="w-3 h-3 text-gray-600 dark:text-gray-400" />
          </button>
          <button
            onClick={handleRemove}
            className="p-1 hover:bg-red-100 dark:hover:bg-red-900 rounded"
            aria-label="Remove filter"
          >
            <X className="w-3 h-3 text-red-600 dark:text-red-400" />
          </button>
        </div>
      </div>

      {/* Logic Selector */}
      {showLogic && (
        <LogicSelector
          value={filter.logic || 'AND'}
          onChange={(logic) => updateFilter(filter.id, { logic })}
        />
      )}
    </div>
  );
}
