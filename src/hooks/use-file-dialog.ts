import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { useFileStore } from '@/stores/file-store';
import type { FileMetadata, IndexMetadata } from '@/types/file';
import toast from 'react-hot-toast';

const LARGE_FILE_THRESHOLD_GB = 50;
const BYTES_PER_GB = 1024 * 1024 * 1024;

/**
 * Hook for file dialog and file selection logic
 */
export function useFileDialog() {
  const [showLargeFileWarning, setShowLargeFileWarning] = useState(false);
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
   * Start indexation of the selected file
   */
  const startIndexation = async (filePath: string, fileSize: number) => {
    try {
      setLoading(true);
      setCurrentFile({
        path: filePath,
        size: fileSize,
        format: 'UNKNOWN', // Will be determined by parser in Story 1.3
        selectedAt: new Date(),
      });

      // Call Tauri IPC to index file
      const metadata = await invoke<IndexMetadata>('index_file', {
        filePath,
        compressionLevel: 1, // Architectural decision: Zstd level 1
      });

      setIndexMetadata(metadata);
      toast.success('File indexed successfully');
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Indexation failed';
      setError(errorMessage);
      toast.error(`Failed to index file: ${errorMessage}`);
    }
  };

  return {
    openDialog,
    showLargeFileWarning,
    largeFileSize: pendingFile?.size,
    confirmLargeFile,
    cancelLargeFile,
  };
}
