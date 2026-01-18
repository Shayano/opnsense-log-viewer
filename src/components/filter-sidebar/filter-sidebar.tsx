import { useState, useEffect } from 'react';
import { ChevronLeft, ChevronRight, Filter as FilterIcon, Search, Loader2 } from 'lucide-react';
import { useFilterStore } from '@/stores/filter-store';
import { useQueryStore } from '@/stores/query-store';
import { FilterBuilder } from '@/components/filter-builder';
import { ActiveFiltersList } from './active-filters-list';
import { executeQuery } from '@/utils/query-client';
import toast from 'react-hot-toast';
import type { Filter } from '@/types/filter';

export function FilterSidebar(): JSX.Element {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isBuilderOpen, setIsBuilderOpen] = useState(false);
  const [editingFilter, setEditingFilter] = useState<Filter | null>(null);

  const filters = useFilterStore((state) => state.filters);
  const draftMode = useFilterStore((state) => state.draftMode);
  const setDraftMode = useFilterStore((state) => state.setDraftMode);

  const { setResult, setExecuting, setError, isExecuting } = useQueryStore();

  // Persist collapsed state in localStorage
  useEffect(() => {
    const savedState = localStorage.getItem('filter-sidebar-collapsed');
    if (savedState !== null) {
      setIsCollapsed(savedState === 'true');
    }
  }, []);

  const toggleCollapse = (): void => {
    const newState = !isCollapsed;
    setIsCollapsed(newState);
    localStorage.setItem('filter-sidebar-collapsed', String(newState));
  };

  // Keyboard shortcut: Ctrl/Cmd+B to toggle
  useEffect(() => {
    const handleKeyboard = (e: KeyboardEvent): void => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
        e.preventDefault();
        toggleCollapse();
      }
    };

    window.addEventListener('keydown', handleKeyboard);
    return () => window.removeEventListener('keydown', handleKeyboard);
  }, [toggleCollapse]);

  const handleSearch = async (): Promise<void> => {
    if (filters.length === 0) {
      toast.error('Add at least one filter');
      return;
    }

    setExecuting(true);
    setDraftMode(false); // Transition to active mode

    try {
      // Get index hash from global state (placeholder for now)
      const indexHash = '';
      const result = await executeQuery(filters, indexHash);
      setResult(result);

      toast.success(
        `Found ${result.matchedCount} of ${result.totalCount} entries (${result.executionTimeMs}ms)`
      );
    } catch (error) {
      setError(String(error));
      toast.error(String(error));
      setDraftMode(true); // Return to draft mode on error
    }
  };

  const handleAddFilter = (): void => {
    setEditingFilter(null);
    setIsBuilderOpen(true);
  };

  const handleEditFilter = (filter: Filter): void => {
    setEditingFilter(filter);
    setIsBuilderOpen(true);
  };

  const handleCloseBuilder = (): void => {
    setEditingFilter(null);
    setIsBuilderOpen(false);
  };

  if (isCollapsed) {
    return (
      <div className="w-12 bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 flex flex-col items-center py-4 transition-all duration-300">
        <button
          onClick={toggleCollapse}
          className="p-2 hover:bg-gray-200 dark:hover:bg-gray-800 rounded"
          aria-label="Expand sidebar"
          title="Expand sidebar (Ctrl+B)"
        >
          <ChevronRight className="w-5 h-5 text-gray-600 dark:text-gray-400" />
        </button>
        <FilterIcon className="w-5 h-5 text-gray-600 dark:text-gray-400 mt-4" />
        {filters.length > 0 && (
          <div className="mt-2 px-2 py-1 bg-blue-600 text-white text-xs rounded-full">
            {filters.length}
          </div>
        )}
      </div>
    );
  }

  return (
    <>
      <div className="w-80 bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700
        flex flex-col transition-all duration-300">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2">
            <FilterIcon className="w-5 h-5 text-gray-600 dark:text-gray-400" />
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
              Filters
            </h2>
            {draftMode && (
              <span className="px-2 py-0.5 text-xs bg-yellow-100 dark:bg-yellow-900
                text-yellow-800 dark:text-yellow-200 rounded">
                Draft
              </span>
            )}
          </div>
          <button
            onClick={toggleCollapse}
            className="p-1 hover:bg-gray-200 dark:hover:bg-gray-800 rounded"
            aria-label="Collapse sidebar"
            title="Collapse sidebar (Ctrl+B)"
          >
            <ChevronLeft className="w-4 h-4 text-gray-600 dark:text-gray-400" />
          </button>
        </div>

        {/* Add Filter Button */}
        <div className="p-4">
          <button
            onClick={handleAddFilter}
            className="w-full px-4 py-2 text-sm font-medium text-white bg-blue-600
              hover:bg-blue-700 active:scale-95 rounded transition-all"
            aria-label="Add new filter"
          >
            Add Filter
          </button>
        </div>

        {/* Active Filters List */}
        <div className="flex-1 overflow-auto px-4">
          <ActiveFiltersList onEditFilter={handleEditFilter} />
        </div>

        {/* Search Button (Draft Mode) */}
        {filters.length > 0 && draftMode && (
          <div className="p-4 border-t border-gray-200 dark:border-gray-700">
            <button
              onClick={handleSearch}
              disabled={isExecuting}
              className="w-full px-4 py-3 text-sm font-semibold text-white bg-blue-600
                hover:bg-blue-700 active:scale-95 rounded-lg transition-all
                flex items-center justify-center gap-2 shadow-md
                disabled:bg-gray-400 disabled:cursor-not-allowed"
              aria-label={`Execute search with ${filters.length} filter${filters.length > 1 ? 's' : ''}`}
            >
              {isExecuting ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Searching...
                </>
              ) : (
                <>
                  <Search className="w-4 h-4" />
                  Search ({filters.length} filter{filters.length > 1 ? 's' : ''})
                </>
              )}
            </button>
          </div>
        )}
      </div>

      {/* Filter Builder Modal */}
      <FilterBuilder
        isOpen={isBuilderOpen}
        onClose={handleCloseBuilder}
        editingFilter={editingFilter ? {
          id: editingFilter.id,
          field: editingFilter.field,
          operator: editingFilter.operator,
          value: editingFilter.value,
        } : null}
      />
    </>
  );
}
