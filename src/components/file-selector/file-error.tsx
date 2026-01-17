import { X, AlertCircle } from 'lucide-react';
import { useFileError, useFileStore } from '@/stores/file-store';

/**
 * FileError component
 * Displays error messages for file operations
 * Note: Most errors are displayed via react-hot-toast,
 * but this component can show persistent errors in the UI
 */
export function FileError() {
  const error = useFileError();
  const clearError = useFileStore((state) => state.setError);

  if (!error) return null;

  return (
    <div
      role="alert"
      className="flex items-start gap-3 rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 p-4 text-sm"
    >
      <AlertCircle className="text-red-500 flex-shrink-0 mt-0.5" size={20} />
      <div className="flex-1">
        <p className="font-semibold text-red-900 dark:text-red-100">Error</p>
        <p className="text-red-800 dark:text-red-200 mt-1">{error}</p>
        <p className="text-red-700 dark:text-red-300 mt-2 text-xs">
          Please check the file path and permissions, then try again.
        </p>
      </div>
      <button
        onClick={() => clearError(null)}
        className="text-red-500 hover:text-red-700 dark:hover:text-red-300"
        aria-label="Close error message"
      >
        <X size={18} />
      </button>
    </div>
  );
}
