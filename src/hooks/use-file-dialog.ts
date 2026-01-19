import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { useFileStore } from '@/stores/file-store';
import { useQueryStore } from '@/stores/query-store';
import type { FileMetadata, IndexMetadata } from '@/types/file';
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
  const clearResult = useQueryStore((state) => state.clearResult);

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
   *
   * Tauri IPC pattern: Ok(T) returns T directly, Err(String) throws JS exception.
   * We handle different error cases by parsing the error message.
   */
  const startIndexation = async (filePath: string, fileSize: number) => {
    setLoading(true);
    console.log('[MEM] use-file-dialog startIndexation: calling clearResult', { filePath });
    clearResult(); // Free previous result and avoid showing stale data from another file
    setCurrentFile({
      path: filePath,
      size: fileSize,
      format: 'UNKNOWN', // Will be determined by parser
      selectedAt: new Date(),
    });

    try {
      // Try to load existing index first (Story 1.4)
      // On success, Tauri returns IndexMetadata directly
      const metadata = await invoke<IndexMetadata>('load_index_file', { filePath });

      // Existing index found and valid
      setIndexMetadata({
        sourceFileHash: metadata.sourceFileHash || '',
        entryCount: metadata.entryCount,
        format: metadata.format,
        indexSizeBytes: metadata.indexSizeBytes || 0,
        createdAt: metadata.createdAt || new Date().toISOString(),
      });
      setLoading(false);
      toast.success('Using existing index');
    } catch (error) {
      // Tauri Err(String) becomes JS exception
      const errorMessage = String(error);

      // Case 1: No saved index found - proceed with new indexation
      if (errorMessage.includes('No saved index')) {
        await performIndexation(filePath);
        return;
      }

      // Case 2: File has been modified since last index - show dialog
      if (errorMessage.includes('modified since indexing')) {
        setPendingFile({ path: filePath, size: fileSize });
        setShowHashMismatchDialog(true);
        setLoading(false);
        return;
      }

      // Case 3: Index file corrupted - show dialog
      if (errorMessage.includes('corrupted') || errorMessage.includes('Checksum')) {
        setPendingFile({ path: filePath, size: fileSize });
        setShowCorruptionDialog(true);
        setLoading(false);
        return;
      }

      // Case 4: Unknown error
      setLoading(false);
      setError(errorMessage);
      toast.error(`Failed to open file: ${errorMessage}`);
    }
  };

  /**
   * Perform actual indexation (new index creation)
   *
   * Uses build_hybrid_index (not index_file) so that HYBRID_INDEX is populated
   * and execute_query / get_entries_by_ids work. index_file only parses and
   * returns metadata; it does not build the index or persist to .idx.
   */
  const performIndexation = async (filePath: string) => {
    try {
      const metadata = await invoke<IndexMetadata>('build_hybrid_index', {
        filePath,
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
