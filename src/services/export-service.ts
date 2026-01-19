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

      // Show success toast with Open Folder button
      toast.success(
        (t) => (
          <div className="flex items-center gap-3">
            <span>
              Export completed: {event.payload.entriesWritten.toLocaleString()} entries written
            </span>
            <button
              onClick={() => {
                openExportLocation(event.payload.filePath);
                toast.dismiss(t.id);
              }}
              className="px-2 py-1 text-sm font-medium bg-green-100 dark:bg-green-800 rounded hover:bg-green-200 dark:hover:bg-green-700 transition-colors"
            >
              Open Folder
            </button>
          </div>
        ),
        { duration: 10000 }
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

      // Show success toast with Open Folder button
      toast.success(
        (t) => (
          <div className="flex items-center gap-3">
            <span>
              Export completed: {event.payload.entriesWritten.toLocaleString()} entries written
            </span>
            <button
              onClick={() => {
                openExportLocation(event.payload.filePath);
                toast.dismiss(t.id);
              }}
              className="px-2 py-1 text-sm font-medium bg-green-100 dark:bg-green-800 rounded hover:bg-green-200 dark:hover:bg-green-700 transition-colors"
            >
              Open Folder
            </button>
          </div>
        ),
        { duration: 10000 }
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
 * Build export metadata from application state
 */
export { prepareExportMetadata };
