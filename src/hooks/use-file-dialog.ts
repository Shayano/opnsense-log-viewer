import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { useFileStore } from '@/stores/file-store';
import type { FileMetadata, IndexMetadata, LoadIndexResult } from '@/types/file';
import { toast } from '@/components/base/toaster';

const LARGE_FILE_THRESHOLD_GB = 50;
const BYTES_PER_GB = 1024 * 1024 * 1024;

/**
 * Hook for file dialog and file selection logic
 */
export function useFileDialog() {
  const [showLargeFileWarning, setShowLargeFileWarning] = useState(false);
  const [showHashMismatchDialog, setShowHashMismatchDialog] = useState(false);
  const [showCorruptionDialog, setShowCorruptionDialog] = useState(false);
  const [pendingFile, setPendingFile] = useState<{ path: string; size: number } | null>(null);

  const setCurrentFile = useFileStore((state) => state.setCurrentFile);
  const setLoading = useFileStore((state) => state.setLoading);
  const setError = useFileStore((state) => state.setError);
  const setIndexMetadata = useFileStore((state) => state.setIndexMetadata);

  /**
   * Open the native OS file picker
   */
  const openDialog = async () => {
    try {
      // Open native OS file picker
      const filePath = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: 'Log Files', extensions: ['log'] },
          { name: 'Text Files', extensions: ['txt'] },
          { name: 'CSV Files', extensions: ['csv'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });

      if (!filePath) {
        // User cancelled
        return;
      }

      // Get file metadata to check size
      const metadata = await invoke<FileMetadata>('get_file_metadata', { filePath });
      const fileSizeGB = metadata.size / BYTES_PER_GB;

      // Check if file is larger than 50GB
      if (fileSizeGB > LARGE_FILE_THRESHOLD_GB) {
        setPendingFile({ path: filePath, size: metadata.size });
        setShowLargeFileWarning(true);
        return;
      }

      // Proceed with indexation
      await startIndexation(filePath, metadata.size);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to open file';
      setError(errorMessage);
      toast.error(`Failed to open file: ${errorMessage}`);
    }
  };

  /**
   * Confirm large file selection
   */
  const confirmLargeFile = async () => {
    if (!pendingFile) return;

    setShowLargeFileWarning(false);
    await startIndexation(pendingFile.path, pendingFile.size);
    setPendingFile(null);
  };

  /**
   * Cancel large file selection
   */
  const cancelLargeFile = () => {
    setShowLargeFileWarning(false);
    setPendingFile(null);
  };

  /**
   * Try to load existing index, or start new indexation
   */
  const startIndexation = async (filePath: string, fileSize: number) => {
    try {
      setLoading(true);
      setCurrentFile({
        path: filePath,
        size: fileSize,
        format: 'UNKNOWN', // Will be determined by parser
        selectedAt: new Date(),
      });

      // Try to load existing index first (Story 1.4)
      const loadResult = await invoke<LoadIndexResult>('load_index_file', { filePath });

      if (loadResult.type === 'Success') {
        // Existing index found and valid
        setIndexMetadata({
          sourceFileHash: '', // Not needed for UI
          entryCount: loadResult.metadata.entryCount,
          format: loadResult.metadata.logFormat,
          indexSizeBytes: 0, // Not needed for UI
          createdAt: new Date().toISOString(),
        });
        toast.success('Using existing index');
        return;
      }

      if (loadResult.type === 'HashMismatch') {
        // File has been modified since last index
        setPendingFile({ path: filePath, size: fileSize });
        setShowHashMismatchDialog(true);
        setLoading(false);
        return;
      }

      // NotFound - proceed with new indexation
      await performIndexation(filePath);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to open file';

      // Check if error is corruption
      if (errorMessage.includes('corrupted') || errorMessage.includes('Checksum')) {
        setPendingFile({ path: filePath, size: fileSize });
        setShowCorruptionDialog(true);
        setLoading(false);
        return;
      }

      setLoading(false);
      setError(errorMessage);
      toast.error(`Failed to open file: ${errorMessage}`);
    }
  };

  /**
   * Perform actual indexation (new index creation)
   */
  const performIndexation = async (filePath: string) => {
    try {
      const metadata = await invoke<IndexMetadata>('index_file', {
        filePath,
        compressionLevel: 1, // Architectural decision: Zstd level 1
      });

      setIndexMetadata(metadata);
      toast.success('File indexed successfully');
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Indexation failed';
      setLoading(false);
      setError(errorMessage);
      toast.error(`Failed to index file: ${errorMessage}`);
    }
  };

  /**
   * Confirm re-indexation when hash mismatch
   */
  const confirmReindex = async () => {
    if (!pendingFile) return;

    setShowHashMismatchDialog(false);
    setLoading(true);
    await performIndexation(pendingFile.path);
    setPendingFile(null);
  };

  /**
   * Cancel re-indexation (hash mismatch or corruption)
   */
  const cancelReindex = () => {
    setShowHashMismatchDialog(false);
    setShowCorruptionDialog(false);
    setPendingFile(null);
  };

  return {
    openDialog,
    showLargeFileWarning,
    showHashMismatchDialog,
    showCorruptionDialog,
    largeFileSize: pendingFile?.size,
    confirmLargeFile,
    cancelLargeFile,
    confirmReindex,
    cancelReindex,
  };
}
