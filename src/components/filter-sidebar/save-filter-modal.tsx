import { useState } from 'react';
import { X, Save } from 'lucide-react';
import { useFilterStore } from '@/stores/filter-store';
import toast from 'react-hot-toast';

interface SaveFilterModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SaveFilterModal({ isOpen, onClose }: SaveFilterModalProps) {
  const { savedFilters, saveFilter } = useFilterStore();
  const [filterName, setFilterName] = useState('');
  const [error, setError] = useState('');

  const handleSave = () => {
    const trimmedName = filterName.trim();

    // Validation
    if (!trimmedName) {
      setError('Filter name cannot be empty');
      return;
    }

    // Check for duplicate names
    const isDuplicate = savedFilters.some((sf) => sf.name === trimmedName);
    if (isDuplicate) {
      setError('A filter with this name already exists');
      return;
    }

    try {
      saveFilter(trimmedName);
      toast.success(`Filter "${trimmedName}" saved`);
      handleClose();
    } catch (error) {
      if (error instanceof DOMException && error.name === 'QuotaExceededError') {
        toast.error('Storage full. Delete old filters to make room.');
      } else {
        toast.error('Failed to save filter');
      }
    }
  };

  const handleClose = () => {
    setFilterName('');
    setError('');
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSave();
    } else if (e.key === 'Escape') {
      handleClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-lg w-full max-w-md p-6">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Save className="w-5 h-5 text-blue-600 dark:text-blue-400" />
            <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
              Save Filter
            </h2>
          </div>
          <button
            onClick={handleClose}
            className="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
            aria-label="Close save filter dialog"
          >
            <X className="w-5 h-5 text-gray-600 dark:text-gray-400" />
          </button>
        </div>

        {/* Content */}
        <div className="space-y-4">
          <div>
            <label
              htmlFor="filter-name"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
            >
              Enter filter name:
            </label>
            <input
              id="filter-name"
              type="text"
              value={filterName}
              onChange={(e) => {
                setFilterName(e.target.value);
                setError('');
              }}
              onKeyDown={handleKeyDown}
              placeholder="e.g., Nightly Port 443 Blocks"
              className="w-full px-3 py-2 bg-white dark:bg-gray-800
                border border-gray-300 dark:border-gray-700 rounded
                text-gray-900 dark:text-gray-100
                focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              autoFocus
            />
            {error && (
              <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>
            )}
          </div>

          {/* Info */}
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Saved filters can be loaded instantly without reconstructing filters manually.
          </p>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-3 mt-6">
          <button
            type="button"
            onClick={handleClose}
            className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300
              bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700
              rounded transition-colors"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={!filterName.trim()}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600
              hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed
              rounded transition-colors flex items-center gap-2"
          >
            <Save className="w-4 h-4" />
            Save Filter
          </button>
        </div>
      </div>
    </div>
  );
}
