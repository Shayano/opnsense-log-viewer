import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import toast from 'react-hot-toast';
import type {
  ExportFormat,
  ExportScope,
  ExportMetadata,
  ExportProgress,
  ExportResult,
  ExportLogEntry,
  FilterInfo,
  ExportEstimate,
} from '@/types/export';
import type { FilterCondition } from '@/types/query';

/**
 * Prepare export metadata from current application state
 */
function prepareExportMetadata(
  totalEntries: number,
  totalInSource: number,
  filePath: string,
  fileHash: string,
  filters: FilterCondition[],
  exportScope: ExportScope,
  enrichmentActive: boolean,
  enrichmentSource?: string,
  enrichmentExportedAt?: string
): ExportMetadata {
  // Convert filters to export format
  const filtersApplied: FilterInfo[] = filters.map((filter) => ({
    field: filter.field,
    operator: filter.operator,
    value: String(filter.value),
  }));

  // Build metadata
  const metadata: ExportMetadata = {
    exportedBy: 'opnsense-log-viewer v1.0.0',
    exportDate: new Date().toISOString(),
    sourceFile: {
      path: filePath,
      sha256: fileHash,
    },
    filtersApplied,
    totalEntries,
    totalInSource,
    exportScope,
  };

  // Add enrichment status if applicable
  if (enrichmentActive) {
    metadata.enrichmentStatus = {
      active: true,
      source: enrichmentSource || 'Live API',
      exportedAt: enrichmentExportedAt,
    };
  }

  return metadata;
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

/**
 * Export filtered results workflow (Story 5.1)
 *
 * @param format Export format (CSV or JSON)
 * @param entries Log entries to export
 * @param metadata Export metadata
 * @returns true if export succeeded, false if cancelled/failed
 */
export async function exportFilteredResults(
  format: ExportFormat,
  entries: ExportLogEntry[],
  metadata: ExportMetadata
): Promise<boolean> {
  try {
    // Step 1: Open save dialog
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-').split('T')[0];
    const extension = format === 'csv' ? 'csv' : 'json';
    const defaultFilename = `opnsense_filtered_export_${timestamp}.${extension}`;

    const savePath = await save({
      defaultPath: defaultFilename,
      filters: [
        {
          name: format.toUpperCase(),
          extensions: [extension],
        },
      ],
    });

    if (!savePath) {
      return false; // User cancelled
    }

    // Step 2: Set up progress listener
    let progressModalOpen = true;
    const unlistenProgress = await listen<ExportProgress>('export-progress', (event) => {
      // Progress updates handled by component
      console.log('Export progress:', event.payload);
    });

    const unlistenComplete = await listen<ExportResult>('export-complete', (event) => {
      progressModalOpen = false;
      unlistenProgress();
      unlistenComplete();
      unlistenError();

      const result = event.payload;
      const fileSizeMB = (result.fileSizeBytes / (1024 * 1024)).toFixed(2);
      const shortChecksum = result.checksum.substring(0, 16);

      // Show success toast with verification details
      toast.success(
        (t) => (
          <div className="flex flex-col gap-2">
            <div className="font-semibold text-green-900 dark:text-green-100">
              ✅ Export completed successfully
            </div>
            <div className="text-sm text-gray-700 dark:text-gray-300 space-y-1">
              <div>Entries written: <span className="font-medium">{result.entriesWritten.toLocaleString()}</span></div>
              <div>File size: <span className="font-medium">{fileSizeMB} MB</span></div>
              <div className="flex items-center gap-2">
                <span>SHA-256: <span className="font-mono text-xs">{shortChecksum}...</span></span>
                <button
                  onClick={() => {
                    navigator.clipboard.writeText(result.checksum);
                    toast.success('Checksum copied to clipboard', { duration: 2000 });
                  }}
                  className="px-1.5 py-0.5 text-xs font-medium bg-gray-100 dark:bg-gray-800 rounded hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                  title="Copy full checksum"
                >
                  Copy
                </button>
              </div>
            </div>
            <div className="flex gap-2 mt-1">
              <button
                onClick={() => {
                  openExportLocation(result.filePath);
                  toast.dismiss(t.id);
                }}
                className="px-3 py-1 text-sm font-medium bg-green-100 dark:bg-green-800 rounded hover:bg-green-200 dark:hover:bg-green-700 transition-colors"
              >
                Open Folder
              </button>
            </div>
          </div>
        ),
        { duration: 15000 }
      );
    });

    const unlistenError = await listen<string>('export-error', (event) => {
      progressModalOpen = false;
      unlistenProgress();
      unlistenComplete();
      unlistenError();

      toast.error(`Export failed: ${event.payload}`);
    });

    // Step 3: Start export
    await invoke<ExportResult>('export_filtered_results', {
      format,
      entries,
      metadata,
      savePath,
    });

    return true;
  } catch (error) {
    console.error('Export error:', error);
    toast.error(`Export failed: ${error}`);
    return false;
  }
}

/**
 * Cancel ongoing export operation
 */
export async function cancelExport(): Promise<void> {
  try {
    await invoke('cancel_export');
    toast.info('Export cancelled');
  } catch (error) {
    console.error('Failed to cancel export:', error);
  }
}

/**
 * Get export estimate for warning dialog (Story 5.2)
 *
 * @param totalEntries Total number of entries to export
 * @param format Export format (CSV or JSON)
 * @param scope Export scope (filtered or fullDataset)
 * @returns Export estimate with file size and duration
 */
export async function estimateExport(
  totalEntries: number,
  format: ExportFormat,
  scope: ExportScope
): Promise<ExportEstimate> {
  try {
    const estimate = await invoke<ExportEstimate>('estimate_export', {
      totalEntries,
      format,
      scope,
    });
    return estimate;
  } catch (error) {
    console.error('Failed to estimate export:', error);
    // Return fallback estimate
    const bytesPerEntry = format === 'csv' ? 120 : 200;
    const entriesPerSecond = format === 'csv' ? 10000 : 5000;
    return {
      estimatedEntries: totalEntries,
      estimatedFileSizeMb: (totalEntries * bytesPerEntry) / (1024 * 1024),
      estimatedDurationSeconds: totalEntries / entriesPerSecond,
    };
  }
}

/**
 * Execute streaming export for large datasets (Story 5.2)
 *
 * @param format Export format (CSV or JSON)
 * @param entries Log entries to export (can be full dataset)
 * @param metadata Export metadata
 * @param streaming Enable streaming mode for large exports
 * @returns true if export succeeded, false if cancelled/failed
 */
export async function executeStreamingExport(
  format: ExportFormat,
  entries: ExportLogEntry[],
  metadata: ExportMetadata,
  streaming: boolean = true
): Promise<boolean> {
  try {
    // Step 1: Open save dialog
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-').split('T')[0];
    const extension = format === 'csv' ? 'csv' : 'json';
    const scopeLabel = metadata.exportScope === 'fullDataset' ? 'full' : 'filtered';
    const defaultFilename = `opnsense_${scopeLabel}_export_${timestamp}.${extension}`;

    const savePath = await save({
      defaultPath: defaultFilename,
      filters: [
        {
          name: format.toUpperCase(),
          extensions: [extension],
        },
      ],
    });

    if (!savePath) {
      return false; // User cancelled
    }

    // Step 2: Set up progress listener
    let progressModalOpen = true;
    const unlistenProgress = await listen<ExportProgress>('export-progress', (event) => {
      // Progress updates handled by component
      console.log('Export progress:', event.payload);
    });

    const unlistenComplete = await listen<ExportResult>('export-complete', (event) => {
      progressModalOpen = false;
      unlistenProgress();
      unlistenComplete();
      unlistenError();

      const result = event.payload;
      const fileSizeMB = (result.fileSizeBytes / (1024 * 1024)).toFixed(2);
      const shortChecksum = result.checksum.substring(0, 16);

      // Show success toast with verification details
      toast.success(
        (t) => (
          <div className="flex flex-col gap-2">
            <div className="font-semibold text-green-900 dark:text-green-100">
              ✅ Export completed successfully
            </div>
            <div className="text-sm text-gray-700 dark:text-gray-300 space-y-1">
              <div>Entries written: <span className="font-medium">{result.entriesWritten.toLocaleString()}</span></div>
              <div>File size: <span className="font-medium">{fileSizeMB} MB</span></div>
              <div className="flex items-center gap-2">
                <span>SHA-256: <span className="font-mono text-xs">{shortChecksum}...</span></span>
                <button
                  onClick={() => {
                    navigator.clipboard.writeText(result.checksum);
                    toast.success('Checksum copied to clipboard', { duration: 2000 });
                  }}
                  className="px-1.5 py-0.5 text-xs font-medium bg-gray-100 dark:bg-gray-800 rounded hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                  title="Copy full checksum"
                >
                  Copy
                </button>
              </div>
            </div>
            <div className="flex gap-2 mt-1">
              <button
                onClick={() => {
                  openExportLocation(result.filePath);
                  toast.dismiss(t.id);
                }}
                className="px-3 py-1 text-sm font-medium bg-green-100 dark:bg-green-800 rounded hover:bg-green-200 dark:hover:bg-green-700 transition-colors"
              >
                Open Folder
              </button>
            </div>
          </div>
        ),
        { duration: 15000 }
      );
    });

    const unlistenError = await listen<string>('export-error', (event) => {
      progressModalOpen = false;
      unlistenProgress();
      unlistenComplete();
      unlistenError();

      toast.error(`Export failed: ${event.payload}`);
    });

    // Step 3: Start export with streaming flag
    await invoke<ExportResult>('export_filtered_results', {
      format,
      entries,
      metadata,
      savePath,
      streaming,
    });

    return true;
  } catch (error) {
    console.error('Export error:', error);
    toast.error(`Export failed: ${error}`);
    return false;
  }
}

/**
 * Verify export file integrity (Story 5.3)
 *
 * Recalculates SHA-256 checksum and compares with stored checksum in file metadata
 *
 * @param filePath Path to export file to verify
 * @returns Verification result with valid/invalid status and checksums
 */
export async function verifyExportFile(filePath: string): Promise<{
  valid: boolean;
  expectedHash: string;
  actualHash: string;
  fileSizeBytes: number;
  entriesCount?: number;
}> {
  try {
    const result = await invoke<{
      valid: boolean;
      expectedHash: string;
      actualHash: string;
      fileSizeBytes: number;
      entriesCount?: number;
    }>('verify_export_file', { filePath });

    return result;
  } catch (error) {
    console.error('Failed to verify export file:', error);
    throw error;
  }
}

/**
 * Detect incomplete exports on startup (Story 5.3)
 *
 * Scans for partial export files that were not completed
 *
 * @returns Array of incomplete export file paths
 */
export async function detectIncompleteExports(): Promise<string[]> {
  try {
    const incompleteFiles = await invoke<string[]>('detect_incomplete_exports');
    return incompleteFiles;
  } catch (error) {
    console.error('Failed to detect incomplete exports:', error);
    return [];
  }
}

/**
 * Clean up a partial/incomplete export file (Story 5.3)
 *
 * @param filePath Path to partial export file to delete
 */
export async function cleanupPartialExport(filePath: string): Promise<void> {
  try {
    await invoke('cleanup_partial_export', { filePath });
    toast.success('Incomplete export file deleted');
  } catch (error) {
    console.error('Failed to cleanup partial export:', error);
    toast.error('Failed to delete incomplete export file');
  }
}

/**
 * Build export metadata from application state
 */
export { prepareExportMetadata };
