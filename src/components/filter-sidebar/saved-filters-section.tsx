import { useState } from 'react';
import { ChevronDown, ChevronRight, FolderOpen, Trash2 } from 'lucide-react';
import { useFilterStore } from '@/stores/filter-store';
import { DeleteConfirmDialog } from './delete-confirm-dialog';
import toast from 'react-hot-toast';

export function SavedFiltersSection() {
  const { savedFilters, loadFilter, deleteSavedFilter } = useFilterStore();
  const [isExpanded, setIsExpanded] = useState(true);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [selectedFilterId, setSelectedFilterId] = useState<string | null>(null);

  const handleLoad = (filterId: string) => {
    const filter = savedFilters.find((f) => f.id === filterId);
    if (!filter) return;

    loadFilter(filterId);
    toast.success(`Filter "${filter.name}" loaded`);
  };

  const handleDeleteClick = (filterId: string) => {
    setSelectedFilterId(filterId);
    setDeleteDialogOpen(true);
  };

  const handleDeleteConfirm = () => {
    if (!selectedFilterId) return;

    const filter = savedFilters.find((f) => f.id === selectedFilterId);
    deleteSavedFilter(selectedFilterId);
    toast.success(`Filter "${filter?.name}" deleted`);
    setDeleteDialogOpen(false);
    setSelectedFilterId(null);
  };

  const selectedFilter = savedFilters.find((f) => f.id === selectedFilterId);

  return (
    <>
      <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
        {/* Section Header */}
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="w-full flex items-center justify-between px-4 py-2
            hover:bg-gray-100 dark:hover:bg-gray-800 rounded transition-colors"
          aria-expanded={isExpanded}
          aria-label="Toggle saved filters section"
        >
          <div className="flex items-center gap-2">
            {isExpanded ? (
              <ChevronDown className="w-4 h-4 text-gray-600 dark:text-gray-400" />
            ) : (
              <ChevronRight className="w-4 h-4 text-gray-600 dark:text-gray-400" />
            )}
            <h3 className="text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase">
              Saved Filters ({savedFilters.length})
            </h3>
          </div>
        </button>

        {/* Saved Filters List */}
        {isExpanded && (
          <div className="mt-2 px-4 space-y-2">
            {savedFilters.length === 0 ? (
              <div className="text-center py-4 text-sm text-gray-500 dark:text-gray-400">
                No saved filters yet.
                <br />
                Save your current filters to reuse them later.
              </div>
            ) : (
              savedFilters.map((savedFilter) => (
                <div
                  key={savedFilter.id}
                  className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700
                    rounded p-3 space-y-2"
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                        {savedFilter.name}
                      </div>
                      <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                        {savedFilter.filters.length} filter{savedFilter.filters.length > 1 ? 's' : ''}
                      </div>
                    </div>

                    {/* Actions */}
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => handleLoad(savedFilter.id)}
                        className="p-1.5 hover:bg-blue-100 dark:hover:bg-blue-900 rounded
                          text-blue-600 dark:text-blue-400 transition-colors"
                        aria-label={`Load filter ${savedFilter.name}`}
                        title="Load"
                      >
                        <FolderOpen className="w-4 h-4" />
                      </button>
                      <button
                        onClick={() => handleDeleteClick(savedFilter.id)}
                        className="p-1.5 hover:bg-red-100 dark:hover:bg-red-900 rounded
                          text-red-600 dark:text-red-400 transition-colors"
                        aria-label={`Delete filter ${savedFilter.name}`}
                        title="Delete"
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

      {/* Delete Confirmation Dialog */}
      <DeleteConfirmDialog
        isOpen={deleteDialogOpen}
        filterName={selectedFilter?.name || ''}
        onConfirm={handleDeleteConfirm}
        onCancel={() => {
          setDeleteDialogOpen(false);
          setSelectedFilterId(null);
        }}
      />
    </>
  );
}
