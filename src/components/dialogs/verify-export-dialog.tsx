import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { ShieldCheck, X, FileSearch, Loader2 } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import toast from 'react-hot-toast';
import type { VerifyExportDialogProps } from '@/types/export';

interface VerifyExportDialogInternalProps extends VerifyExportDialogProps {
  onVerify: (filePath: string) => Promise<void>;
}

export function VerifyExportDialog({
  isOpen,
  onClose,
  onVerify,
}: VerifyExportDialogInternalProps) {
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [isVerifying, setIsVerifying] = useState(false);

  const handleSelectFile = async () => {
    try {
      const file = await open({
        title: 'Select Export File to Verify',
        filters: [
          {
            name: 'Export Files',
            extensions: ['csv', 'json'],
          },
        ],
      });

      if (file) {
        setSelectedFile(file);
      }
    } catch (error) {
      console.error('Failed to select file:', error);
      toast.error('Failed to open file picker');
    }
  };

  const handleVerify = async () => {
    if (!selectedFile) {
      toast.error('Please select a file first');
      return;
    }

    setIsVerifying(true);
    try {
      await onVerify(selectedFile);
      // Reset dialog state after successful verification
      setSelectedFile(null);
    } catch (error) {
      console.error('Verification failed:', error);
      toast.error(`Verification failed: ${error}`);
    } finally {
      setIsVerifying(false);
    }
  };

  const handleClose = () => {
    if (!isVerifying) {
      setSelectedFile(null);
      onClose();
    }
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={(open) => !open && handleClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <Dialog.Content
          className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 max-w-lg w-full z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
          aria-describedby="verify-export-description"
          onEscapeKeyDown={(e) => isVerifying && e.preventDefault()}
          onPointerDownOutside={(e) => isVerifying && e.preventDefault()}
        >
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-2">
              <ShieldCheck className="h-5 w-5 text-blue-600 dark:text-blue-400" />
              <Dialog.Title className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                Verify Export File
              </Dialog.Title>
            </div>
            <Dialog.Close asChild>
              <button
                className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                aria-label="Close"
                onClick={handleClose}
                disabled={isVerifying}
              >
                <X className="h-5 w-5" />
              </button>
            </Dialog.Close>
          </div>

          <Dialog.Description
            id="verify-export-description"
            className="mt-4 text-sm text-gray-600 dark:text-gray-400"
          >
            Select an export file to verify its integrity by recalculating and comparing checksums.
          </Dialog.Description>

          <div className="mt-6 space-y-4">
            {/* File Selection */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Export File
              </label>
              <div className="flex gap-2">
                <button
                  onClick={handleSelectFile}
                  disabled={isVerifying}
                  className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-800 rounded-md hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <FileSearch className="h-4 w-4" />
                  Select File
                </button>
              </div>
            </div>

            {/* Selected File Display */}
            {selectedFile && (
              <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-3">
                <p className="text-xs font-medium text-blue-900 dark:text-blue-100 mb-1">
                  Selected File:
                </p>
                <p className="text-sm text-blue-700 dark:text-blue-300 break-all font-mono">
                  {selectedFile}
                </p>
              </div>
            )}

            {/* Info Message */}
            <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-3">
              <p className="text-xs text-gray-600 dark:text-gray-400">
                <span className="font-medium">Note:</span> This will recalculate the SHA-256 checksum
                and compare it with the checksum stored in the export file metadata.
              </p>
            </div>
          </div>

          <div className="mt-6 flex justify-end gap-3">
            <button
              onClick={handleClose}
              disabled={isVerifying}
              className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-800 rounded-md hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Cancel
            </button>
            <button
              onClick={handleVerify}
              disabled={!selectedFile || isVerifying}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              {isVerifying ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Verifying...
                </>
              ) : (
                <>
                  <ShieldCheck className="h-4 w-4" />
                  Verify
                </>
              )}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
