import { AlertCircle } from 'lucide-react';
import { useFilterStore } from '@/stores/filter-store';
import { useQueryStore } from '@/stores/query-store';

export function EmptyResultsState(): JSX.Element | null {
  const { currentResult } = useQueryStore();
  const { clearFilters } = useFilterStore();

  if (!currentResult || currentResult.matchedCount > 0) {
    return null;
  }

  return (
    <div className="flex items-center justify-center h-full">
      <div className="text-center max-w-md px-8 py-12">
        <AlertCircle className="w-12 h-12 text-gray-400 dark:text-gray-600 mx-auto mb-4" />

        <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
          No entries match current filters
        </h3>

        <p className="text-sm text-gray-600 dark:text-gray-400 mb-6">
          Try adjusting your filter criteria or clearing all filters to see results.
        </p>

        <button
          onClick={clearFilters}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600
            hover:bg-blue-700 rounded transition-colors"
        >
          Clear All Filters
        </button>
      </div>
    </div>
  );
}
