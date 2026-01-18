import { Check, X } from 'lucide-react';
import { useQueryStore } from '@/stores/query-store';
import { useFilterStore } from '@/stores/filter-store';

export function ResultCount(): JSX.Element | null {
  const { currentResult } = useQueryStore();
  const { filters, clearFilters } = useFilterStore();

  if (!currentResult || filters.length === 0) {
    return null;
  }

  const isEmptyResult = currentResult.matchedCount === 0;

  return (
    <div className="px-4 py-3 bg-white dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700
      flex items-center justify-between">
      <div className="flex items-center gap-3">
        {isEmptyResult ? (
          <X className="w-5 h-5 text-red-600 dark:text-red-400" />
        ) : (
          <Check className="w-5 h-5 text-green-600 dark:text-green-400" />
        )}

        <div>
          <div className="text-sm font-semibold text-gray-900 dark:text-gray-100">
            {isEmptyResult ? (
              'No matches found'
            ) : (
              <>
                Showing <span className="text-blue-600 dark:text-blue-400">{currentResult.matchedCount.toLocaleString()}</span>
                {' '}of{' '}
                <span className="text-gray-600 dark:text-gray-400">{currentResult.totalCount.toLocaleString()}</span>
                {' '}entries
              </>
            )}
          </div>
          <div className="text-xs text-gray-500 dark:text-gray-400">
            Query executed in {currentResult.executionTimeMs}ms
          </div>
        </div>
      </div>

      <button
        onClick={clearFilters}
        className="px-3 py-1.5 text-xs font-medium text-red-600 dark:text-red-400
          border border-red-300 dark:border-red-700 hover:bg-red-50 dark:hover:bg-red-900
          rounded transition-colors"
      >
        Clear Filters
      </button>
    </div>
  );
}
