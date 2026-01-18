import { useState } from 'react';
import { ChevronDown, ChevronRight, RotateCcw, Trash2, Clock, AlertCircle } from 'lucide-react';
import { formatDistanceToNow, format } from 'date-fns';
import { useSearchHistoryStore } from '@/stores/search-history-store';
import { useFilterStore } from '@/stores/filter-store';
import { useQueryStore } from '@/stores/query-store';
import { formatFilterSummary } from '@/utils/format-filters';
import { executeQuery } from '@/utils/query-client';
import toast from 'react-hot-toast';
import type { Filter } from '@/types/filter';
import type { SearchHistory } from '@/types/search-history';
import { v4 as uuidv4 } from 'uuid';

export function SearchHistorySection(): JSX.Element {
  const { searchHistory, deleteHistoryEntry, clearAllHistory } = useSearchHistoryStore();
  const { clearFilters, filters, setDraftMode } = useFilterStore();
  const { setResult, setExecuting, setError } = useQueryStore();
  const [isExpanded, setIsExpanded] = useState(true);
  const [clearDialogOpen, setClearDialogOpen] = useState(false);

  // Sort history by timestamp descending (most recent first)
  const sortedHistory = [...searchHistory].sort((a, b) => b.timestamp - a.timestamp);

  // Format timestamp for display
  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp);
    const now = Date.now();
    const daysDiff = (now - timestamp) / (1000 * 60 * 60 * 24);

    if (daysDiff > 7) {
      // More than 7 days ago - show full date
      return format(date, 'MMM d, yyyy \'at\' h:mm a');
    } else {
      // Within 7 days - show relative time
      return formatDistanceToNow(date, { addSuffix: true });
    }
  };

  // Re-run a search from history
  const handleRerun = async (historyEntry: SearchHistory): Promise<void> => {
    try {
      // Clear current filters
      clearFilters();

      // Load history filters with new runtime IDs
      const loadedFilters: Filter[] = historyEntry.filters.map((filter) => ({
        ...filter,
        id: uuidv4(),
      }));

      // Set loaded filters (bypassing normal addFilter to avoid draft mode issues)
      // We'll execute immediately, so set draft mode to false
      setDraftMode(false);
      setExecuting(true);

      const indexHash = ''; // TODO: Get from file store if available
      const result = await executeQuery(loadedFilters, indexHash);

      setResult(result);
      setExecuting(false);

      toast.success(`Re-ran search from ${formatTimestamp(historyEntry.timestamp)}`);
    } catch (error) {
      setError(String(error));
      setExecuting(false);
      toast.error(`Failed to re-run search: ${String(error)}`);
    }
  };

  // Delete a history entry
  const handleDelete = (id: string): void => {
    deleteHistoryEntry(id);
    toast.success('Search removed from history');
  };

  // Clear all history
  const handleClearAll = (): void => {
    clearAllHistory();
    toast.success('Search history cleared');
    setClearDialogOpen(false);
  };

  return (
    <>
      <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
        {/* Section Header */}
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="w-full flex items-center justify-between px-4 py-2
            hover:bg-gray-100 dark:hover:bg-gray-800 rounded transition-colors"
          aria-expanded={isExpanded}
          aria-label="Toggle search history section"
        >
          <div className="flex items-center gap-2">
            {isExpanded ? (
              <ChevronDown className="w-4 h-4 text-gray-600 dark:text-gray-400" />
            ) : (
              <ChevronRight className="w-4 h-4 text-gray-600 dark:text-gray-400" />
            )}
            <Clock className="w-4 h-4 text-gray-600 dark:text-gray-400" />
            <h3 className="text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase">
              Recent Searches ({searchHistory.length})
            </h3>
          </div>
          {searchHistory.length > 0 && isExpanded && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                setClearDialogOpen(true);
              }}
              className="text-xs text-red-600 dark:text-red-400 hover:underline"
            >
              Clear All
            </button>
          )}
        </button>

        {/* History List */}
        {isExpanded && (
          <div className="mt-2 px-4 space-y-2">
            {sortedHistory.length === 0 ? (
              <div className="text-center py-4 text-sm text-gray-500 dark:text-gray-400">
                No search history yet.
                <br />
                Execute a search to start tracking history.
              </div>
            ) : (
              sortedHistory.map((entry) => (
                <div
                  key={entry.id}
                  className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700
                    rounded p-3 space-y-2"
                  title={`Full details:\n${formatFilterSummary(entry.filters, 200)}\nExecuted: ${format(
                    entry.timestamp,
                    'PPpp'
                  )}\nExecution time: ${entry.executionTimeMs}ms`}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      {/* Filter summary */}
                      <div className="text-sm text-gray-700 dark:text-gray-300 truncate">
                        {formatFilterSummary(entry.filters, 50)}
                      </div>

                      {/* Result count and timestamp */}
                      <div className="flex items-center gap-2 mt-1 text-xs text-gray-500 dark:text-gray-400">
                        <span className="font-medium text-blue-600 dark:text-blue-400">
                          {entry.resultCount.toLocaleString()} results
                        </span>
                        <span>•</span>
                        <span>{formatTimestamp(entry.timestamp)}</span>
                      </div>
                    </div>

                    {/* Actions */}
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => handleRerun(entry)}
                        className="p-1.5 hover:bg-blue-100 dark:hover:bg-blue-900 rounded
                          text-blue-600 dark:text-blue-400 transition-colors"
                        aria-label="Re-run search"
                        title="Re-run this search"
                      >
                        <RotateCcw className="w-4 h-4" />
                      </button>
                      <button
                        onClick={() => handleDelete(entry.id)}
                        className="p-1.5 hover:bg-red-100 dark:hover:bg-red-900 rounded
                          text-red-600 dark:text-red-400 transition-colors"
                        aria-label="Delete from history"
                        title="Delete from history"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      {/* Clear All Confirmation Dialog */}
      {clearDialogOpen && (
        <div
          className="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center"
          onClick={() => setClearDialogOpen(false)}
        >
          <div
            className="bg-white dark:bg-gray-900 rounded-lg shadow-lg w-full max-w-md p-6"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-2 mb-4">
              <AlertCircle className="w-5 h-5 text-yellow-600 dark:text-yellow-400" />
              <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                Clear Search History
              </h2>
            </div>
            <p className="text-sm text-gray-700 dark:text-gray-300 mb-4">
              Clear all search history? This cannot be undone.
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setClearDialogOpen(false)}
                className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300
                  bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700
                  rounded transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleClearAll}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600
                  hover:bg-red-700 rounded transition-colors"
              >
                Clear All
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
