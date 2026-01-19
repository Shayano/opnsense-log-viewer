import { invoke } from '@tauri-apps/api/core';
import toast from 'react-hot-toast';

/**
 * Cache status counts for export
 */
interface CacheStatus {
  interfacesCount: number;
  rulesCount: number;
  aliasesCount: number;
}

/**
 * Export command result from Rust backend
 * Mirrors ExportResult from src-tauri/src/api_client/types.rs
 */
interface ExportResult {
  jsonData: string;
  filename: string;
  warning?: string;
  cacheStatus: CacheStatus;
}

/**
 * Export enrichment data workflow (Story 4.1)
 *
 * @returns true if export succeeded, false if cancelled/failed
 */
export async function exportEnrichmentData(): Promise<boolean> {
  try {
    // Step 1: Prepare export data
    const result = await invoke<ExportResult>('export_enrichment_data');

    // Step 2: Check for warning (empty/minimal data)
    if (result.warning) {
      const proceed = await showEmptyDataWarning(result.cacheStatus, result.warning);
      if (!proceed) {
        return false; // User cancelled
      }
    }

    // Step 3: Open save dialog and write file
    const savedPath = await invoke<string>('save_enrichment_export', {
      jsonData: result.jsonData,
      filename: result.filename,
    });

    // Step 4: Show success notification with [Open Folder] button
    toast.success(
      (t) => (
        <div className="flex items-center gap-3">
          <span>Enrichment data exported successfully</span>
          <button
            onClick={() => openFolder(savedPath, t.id)}
            className="px-2 py-1 text-sm font-medium bg-green-100 dark:bg-green-800 rounded hover:bg-green-200 dark:hover:bg-green-700 transition-colors"
          >
            Open Folder
          </button>
        </div>
      ),
      { duration: 5000 }
    );

    return true;
  } catch (error) {
    if (error === 'Save dialog cancelled') {
      // User cancelled - not an error
      return false;
    }

    toast.error(`Export failed: ${error}`);
    return false;
  }
}

/**
 * Show warning dialog for empty/minimal enrichment data
 */
async function showEmptyDataWarning(
  cacheStatus: CacheStatus,
  warningMessage: string
): Promise<boolean> {
  const message =
    `${warningMessage}\n\n` +
    `Current enrichment data:\n` +
    `- ${cacheStatus.interfacesCount} interfaces\n` +
    `- ${cacheStatus.rulesCount} rule labels\n` +
    `- ${cacheStatus.aliasesCount} aliases\n\n` +
    `Export anyway?`;

  return confirm(message);
}

/**
 * Open file explorer at saved file location
 */
async function openFolder(filePath: string, toastId?: string | number) {
  try {
    await invoke('open_folder', { filePath });

    // Dismiss toast after opening folder
    if (toastId) {
      toast.dismiss(toastId);
    }
  } catch (error) {
    toast.error(`Failed to open folder: ${error}`);
  }
}
