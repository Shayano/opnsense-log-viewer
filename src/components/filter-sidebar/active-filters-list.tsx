import { useFilterStore } from '@/stores/filter-store';
import { FilterRow } from './filter-row';
import type { Filter } from '@/types/filter';

interface ActiveFiltersListProps {
  onEditFilter: (filter: Filter) => void;
}

export function ActiveFiltersList({ onEditFilter }: ActiveFiltersListProps): JSX.Element {
  const filters = useFilterStore((state) => state.filters);
  const clearFilters = useFilterStore((state) => state.clearFilters);

  if (filters.length === 0) {
    return (
      <div className="text-center py-8 text-sm text-gray-500 dark:text-gray-400">
        No filters added yet.
        <br />
        Click "Add Filter" to start.
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase">
          Active Filters ({filters.length})
        </h3>
        <button
          onClick={clearFilters}
          className="text-xs text-red-600 dark:text-red-400 hover:underline"
          aria-label="Clear all filters"
        >
          Clear All
        </button>
      </div>

      {filters.map((filter, index) => (
        <FilterRow
          key={filter.id}
          filter={filter}
          showLogic={index < filters.length - 1}
          onEdit={onEditFilter}
        />
      ))}
    </div>
  );
}
