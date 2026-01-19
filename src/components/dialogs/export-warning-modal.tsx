import * as Dialog from '@radix-ui/react-dialog';
import { AlertTriangle, X } from 'lucide-react';
import type { ExportWarningModalProps } from '@/types/export';

export function ExportWarningModal({
  isOpen,
  estimate,
  onContinue,
  onCancel,
}: ExportWarningModalProps) {
  if (!estimate) return null;

  const formatTime = (seconds: number): string => {
    if (seconds < 60) {
      return `${Math.round(seconds)} seconds`;
    }
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.round(seconds % 60);
    if (remainingSeconds === 0) {
      return `${minutes} ${minutes === 1 ? 'minute' : 'minutes'}`;
    }
    return `${minutes}m ${remainingSeconds}s`;
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <Dialog.Content
          className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 max-w-md w-full z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
          aria-describedby="export-warning-description"
        >
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-2">
              <AlertTriangle className="h-5 w-5 text-yellow-600 dark:text-yellow-400" />
              <Dialog.Title className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                Large Export Warning
              </Dialog.Title>
            </div>
            <Dialog.Close asChild>
              <button
                className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                aria-label="Close"
                onClick={onCancel}
              >
                <X className="h-5 w-5" />
              </button>
            </Dialog.Close>
          </div>

          <Dialog.Description
            id="export-warning-description"
            className="mt-4 text-sm text-gray-600 dark:text-gray-400"
          >
            ⚠️ Export <span className="font-semibold">{estimate.estimatedEntries.toLocaleString()}</span> entries?
            This may take several minutes.
          </Dialog.Description>

          <div className="mt-4 space-y-3">
            {/* Estimate Details */}
            <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4 space-y-2">
              <div className="flex justify-between items-center">
                <span className="text-sm font-medium text-yellow-900 dark:text-yellow-100">
                  Estimated Time:
                </span>
                <span className="text-sm font-semibold text-yellow-700 dark:text-yellow-300">
                  ~{formatTime(estimate.estimatedDurationSeconds)} (approximate)
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-sm font-medium text-yellow-900 dark:text-yellow-100">
                  Estimated File Size:
                </span>
                <span className="text-sm font-semibold text-yellow-700 dark:text-yellow-300">
                  ~{estimate.estimatedFileSizeMb.toFixed(1)} MB
                </span>
              </div>
            </div>

            {/* Additional Warning */}
            <div className="text-xs text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-800 rounded p-3">
              <p className="font-medium mb-1">Note:</p>
              <ul className="list-disc list-inside space-y-1">
                <li>The application will remain responsive during export</li>
                <li>You can cancel the export at any time</li>
                <li>Progress will be shown in real-time</li>
              </ul>
            </div>
          </div>

          <div className="mt-6 flex justify-end gap-3">
            <button
              onClick={onCancel}
              className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-800 rounded-md hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={onContinue}
              className="px-4 py-2 text-sm font-medium text-white bg-yellow-600 rounded-md hover:bg-yellow-700 transition-colors"
            >
              Continue Export
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
