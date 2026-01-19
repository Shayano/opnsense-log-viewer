import * as Dialog from '@radix-ui/react-dialog';
import { CheckCircle2, AlertCircle, X, Copy } from 'lucide-react';
import toast from 'react-hot-toast';
import { invoke } from '@tauri-apps/api/core';
import type { ExportResult } from '@/types/export';

interface ExportVerificationModalProps {
  isOpen: boolean;
  result: ExportResult | null;
  onClose: () => void;
}

/**
 * Open the export location in the OS file manager
 */
async function openExportLocation(filePath: string) {
  try {
    await invoke('open_export_location', { filePath });
  } catch (error) {
    console.error('Failed to open export location:', error);
    toast.error('Failed to open export location');
  }
}

export function ExportVerificationModal({
  isOpen,
  result,
  onClose,
}: ExportVerificationModalProps) {
  if (!result) return null;

  const fileSizeMB = (result.fileSizeBytes / (1024 * 1024)).toFixed(2);
  const isComplete = result.verificationPassed;

  const copyChecksum = () => {
    navigator.clipboard.writeText(result.checksum);
    toast.success('Checksum copied to clipboard', { duration: 2000 });
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <Dialog.Content
          className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 max-w-lg w-full z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
          aria-describedby="verification-modal-description"
        >
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-2">
              {isComplete ? (
                <CheckCircle2 className="h-5 w-5 text-green-600 dark:text-green-400" />
              ) : (
                <AlertCircle className="h-5 w-5 text-yellow-600 dark:text-yellow-400" />
              )}
              <Dialog.Title className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                Export Verification
              </Dialog.Title>
            </div>
            <Dialog.Close asChild>
              <button
                className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                aria-label="Close"
                onClick={onClose}
              >
                <X className="h-5 w-5" />
              </button>
            </Dialog.Close>
          </div>

          <Dialog.Description
            id="verification-modal-description"
            className="mt-4 text-sm text-gray-600 dark:text-gray-400"
          >
            Detailed verification information for the export operation.
          </Dialog.Description>

          <div className="mt-6 space-y-4">
            {/* Export Status */}
            <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4 space-y-3">
              <div className="flex justify-between items-center">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Export Status:
                </span>
                <span className={`text-sm font-semibold flex items-center gap-1 ${
                  isComplete
                    ? 'text-green-700 dark:text-green-300'
                    : 'text-yellow-700 dark:text-yellow-300'
                }`}>
                  {isComplete ? (
                    <>
                      <CheckCircle2 className="h-4 w-4" />
                      Complete
                    </>
                  ) : (
                    <>
                      <AlertCircle className="h-4 w-4" />
                      Incomplete
                    </>
                  )}
                </span>
              </div>

              <div className="flex justify-between items-center">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Entries Written:
                </span>
                <span className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                  {result.entriesWritten.toLocaleString()}
                </span>
              </div>

              <div className="flex justify-between items-center">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  File Size:
                </span>
                <span className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                  {fileSizeMB} MB
                </span>
              </div>

              <div className="flex justify-between items-center">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Duration:
                </span>
                <span className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                  {result.durationSeconds.toFixed(1)}s
                </span>
              </div>
            </div>

            {/* Checksum Section */}
            <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
              <div className="flex justify-between items-start mb-2">
                <span className="text-sm font-medium text-blue-900 dark:text-blue-100">
                  SHA-256 Checksum:
                </span>
                <button
                  onClick={copyChecksum}
                  className="flex items-center gap-1 px-2 py-1 text-xs font-medium bg-blue-100 dark:bg-blue-800 rounded hover:bg-blue-200 dark:hover:bg-blue-700 transition-colors"
                  title="Copy checksum to clipboard"
                >
                  <Copy className="h-3 w-3" />
                  Copy
                </button>
              </div>
              <code className="block text-xs font-mono text-blue-700 dark:text-blue-300 break-all">
                {result.checksum}
              </code>
            </div>

            {/* Verification Result */}
            <div className={`rounded-lg p-4 ${
              isComplete
                ? 'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800'
                : 'bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800'
            }`}>
              <div className="flex items-center gap-2">
                {isComplete ? (
                  <>
                    <CheckCircle2 className="h-4 w-4 text-green-600 dark:text-green-400" />
                    <span className="text-sm font-medium text-green-900 dark:text-green-100">
                      Verification passed - Export is complete and valid
                    </span>
                  </>
                ) : (
                  <>
                    <AlertCircle className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
                    <span className="text-sm font-medium text-yellow-900 dark:text-yellow-100">
                      Export may be incomplete - Verification failed
                    </span>
                  </>
                )}
              </div>
            </div>
          </div>

          <div className="mt-6 flex justify-end gap-3">
            <button
              onClick={() => {
                openExportLocation(result.filePath);
              }}
              className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-800 rounded-md hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
            >
              Open Folder
            </button>
            <button
              onClick={onClose}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 transition-colors"
            >
              Close
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
