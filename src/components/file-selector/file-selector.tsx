import { useEffect } from 'react';
import { Folder } from 'lucide-react';
import { Button } from '@/components/base/button';
import { useFileDialog } from '@/hooks/use-file-dialog';
import { useCurrentFile, useIsLoading } from '@/stores/file-store';
import { LargeFileWarning } from './large-file-warning';
import { HashMismatchDialog } from './hash-mismatch-dialog';
import { CorruptionDialog } from './corruption-dialog';

/**
 * FileSelector component
 * Provides UI for opening log files via native OS file picker
 */
export function FileSelector() {
  const {
    openDialog,
    showLargeFileWarning,
    showHashMismatchDialog,
    showCorruptionDialog,
    largeFileSize,
    confirmLargeFile,
    cancelLargeFile,
    confirmReindex,
    cancelReindex,
  } = useFileDialog();
  const currentFile = useCurrentFile();
  const isLoading = useIsLoading();

  // Register keyboard shortcut (Ctrl/Cmd+O)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'o') {
        e.preventDefault();
        openDialog();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [openDialog]);

  return (
    <div className="flex flex-col gap-4">
      <Button
        onClick={openDialog}
        disabled={isLoading}
        variant="primary"
        size="md"
        aria-label="Open log file"
        className="inline-flex items-center gap-2"
      >
        <Folder size={18} />
        {isLoading ? 'Opening...' : 'Open File'}
      </Button>

      {currentFile && (
        <div className="rounded-md bg-gray-100 dark:bg-gray-800 p-4 text-sm">
          <p className="font-semibold text-gray-900 dark:text-gray-100">Selected File:</p>
          <p className="text-gray-700 dark:text-gray-300 break-all">{currentFile.path}</p>
          <p className="text-gray-600 dark:text-gray-400 mt-2">
            Size: {(currentFile.size / (1024 * 1024)).toFixed(2)} MB
          </p>
        </div>
      )}

      <LargeFileWarning
        isOpen={showLargeFileWarning}
        fileSize={largeFileSize || 0}
        onConfirm={confirmLargeFile}
        onCancel={cancelLargeFile}
      />

      <HashMismatchDialog
        isOpen={showHashMismatchDialog}
        onConfirm={confirmReindex}
        onCancel={cancelReindex}
      />

      <CorruptionDialog
        isOpen={showCorruptionDialog}
        onConfirm={confirmReindex}
        onCancel={cancelReindex}
      />
    </div>
  );
}
