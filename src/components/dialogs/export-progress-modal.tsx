import * as Dialog from '@radix-ui/react-dialog';
import { Loader2, X } from 'lucide-react';
import type { ExportProgress } from '@/types/export';

interface ExportProgressModalProps {
  open: boolean;
  progress: ExportProgress | null;
  onCancel: () => void;
}

export function ExportProgressModal({
  open,
  progress,
  onCancel,
}: ExportProgressModalProps) {
  const progressPercentage = progress
    ? Math.round((progress.current / progress.total) * 100)
    : 0;

  const formatTime = (seconds: number): string => {
    if (seconds < 60) {
      return `${seconds.toFixed(1)}s`;
    }
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds.toFixed(0)}s`;
  };

  return (
    <Dialog.Root open={open}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <Dialog.Content
          className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 max-w-md w-full z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
          aria-describedby="export-progress-description"
          onEscapeKeyDown={(e) => e.preventDefault()}
          onPointerDownOutside={(e) => e.preventDefault()}
        >
          <div className="flex items-center gap-3">
            <Loader2 className="h-5 w-5 text-blue-600 dark:text-blue-400 animate-spin" />
            <Dialog.Title className="text-lg font-semibold text-gray-900 dark:text-gray-100">
              Exporting...
            </Dialog.Title>
          </div>

          <Dialog.Description
            id="export-progress-description"
            className="mt-4 text-sm text-gray-600 dark:text-gray-400"
          >
            {progress?.status || 'Preparing export...'}
          </Dialog.Description>

          {progress && (
            <div className="mt-4 space-y-4">
              {/* Progress Bar */}
              <div>
                <div className="flex justify-between text-sm text-gray-600 dark:text-gray-400 mb-2">
                  <span>
                    {progress.current.toLocaleString()} of {progress.total.toLocaleString()} entries
                  </span>
                  <span className="font-semibold">{progressPercentage}%</span>
                </div>
                <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2.5 overflow-hidden">
                  <div
                    className="bg-blue-600 h-2.5 rounded-full transition-all duration-300"
                    style={{ width: `${progressPercentage}%` }}
                  />
                </div>
              </div>

              {/* Stats */}
              <div className="flex justify-between text-xs text-gray-600 dark:text-gray-400">
                <div>
                  <span className="font-medium">Speed:</span>{' '}
                  {Math.round(progress.rowsPerSecond).toLocaleString()} rows/sec
                </div>
                <div>
                  <span className="font-medium">Elapsed:</span>{' '}
                  {formatTime(progress.elapsedSeconds)}
                </div>
              </div>
            </div>
          )}

          <div className="mt-6 flex justify-end">
            <button
              onClick={onCancel}
              className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-800 rounded-md hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors flex items-center gap-2"
            >
              <X className="h-4 w-4" />
              Cancel
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
