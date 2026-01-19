import { useState, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import type {
  ExportFormat,
  ExportProgress,
  ExportLogEntry,
} from '@/types/export';
import { exportFilteredResults, cancelExport, prepareExportMetadata } from '@/services/export-service';
import type { LogEntry } from '@/types/log-entry';
import type { FilterCondition } from '@/types/query';

export interface UseExportOptions {
  entries: LogEntry[];
  totalInSource: number;
  filePath: string;
  fileHash: string;
  filters: FilterCondition[];
  enrichmentActive?: boolean;
  enrichmentSource?: string;
  enrichmentExportedAt?: string;
}

export interface UseExportReturn {
  isExportDialogOpen: boolean;
  isExportProgressOpen: boolean;
  exportProgress: ExportProgress | null;
  openExportDialog: () => void;
  closeExportDialog: () => void;
  handleExport: (format: ExportFormat) => Promise<void>;
  handleCancelExport: () => Promise<void>;
}

/**
 * Hook for managing export functionality
 *
 * Provides state and handlers for export dialog, progress modal, and export operations
 */
export function useExport(options: UseExportOptions): UseExportReturn {
  const [isExportDialogOpen, setIsExportDialogOpen] = useState(false);
  const [isExportProgressOpen, setIsExportProgressOpen] = useState(false);
  const [exportProgress, setExportProgress] = useState<ExportProgress | null>(null);

  const openExportDialog = useCallback(() => {
    setIsExportDialogOpen(true);
  }, []);

  const closeExportDialog = useCallback(() => {
    setIsExportDialogOpen(false);
  }, []);

  const handleExport = useCallback(
    async (format: ExportFormat) => {
      closeExportDialog();

      // Prepare export metadata
      const metadata = prepareExportMetadata(
        options.entries.length,
        options.totalInSource,
        options.filePath,
        options.fileHash,
        options.filters,
        'filtered', // useExport is for filtered results only
        options.enrichmentActive || false,
        options.enrichmentSource,
        options.enrichmentExportedAt
      );

      // Convert LogEntry[] to ExportLogEntry[]
      const exportEntries: ExportLogEntry[] = options.entries.map((entry) => ({
        timestamp: entry.timestamp,
        interface: entry.interface,
        sourceIp: entry.sourceIp,
        sourcePort: entry.sourcePort,
        destinationIp: entry.destinationIp,
        destinationPort: entry.destinationPort,
        protocol: entry.protocol,
        action: entry.action,
        ruleLabel: entry.ruleLabel || '',
      }));

      // Set up progress listener
      const unlistenProgress = await listen<ExportProgress>('export-progress', (event) => {
        setExportProgress(event.payload);
      });

      const unlistenComplete = await listen('export-complete', () => {
        setIsExportProgressOpen(false);
        setExportProgress(null);
        unlistenProgress();
        unlistenComplete();
        unlistenError();
      });

      const unlistenError = await listen('export-error', () => {
        setIsExportProgressOpen(false);
        setExportProgress(null);
        unlistenProgress();
        unlistenComplete();
        unlistenError();
      });

      // Show progress modal
      setIsExportProgressOpen(true);
      setExportProgress({
        current: 0,
        total: exportEntries.length,
        status: 'Starting export...',
        rowsPerSecond: 0,
        elapsedSeconds: 0,
      });

      // Start export
      await exportFilteredResults(format, exportEntries, metadata);
    },
    [options, closeExportDialog]
  );

  const handleCancelExport = useCallback(async () => {
    await cancelExport();
    setIsExportProgressOpen(false);
    setExportProgress(null);
  }, []);

  return {
    isExportDialogOpen,
    isExportProgressOpen,
    exportProgress,
    openExportDialog,
    closeExportDialog,
    handleExport,
    handleCancelExport,
  };
}
