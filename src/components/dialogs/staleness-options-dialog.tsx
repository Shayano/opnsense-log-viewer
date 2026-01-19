import * as Dialog from '@radix-ui/react-dialog';
import { X, RefreshCw, FileDown, AlertCircle, EyeOff } from 'lucide-react';
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import toast from 'react-hot-toast';
import { useEnrichmentStore } from '@/stores/enrichment-store';
import { ConfirmRawDataDialog } from './confirm-raw-data-dialog';
import { formatEnrichmentAge, formatExportTimestamp } from '@/utils/staleness-utils';

interface BackupMetadata {
  importedAt: string;
  ageDays: number;
  sourceFile: string;
  exportTimestamp: string;
  deviceId: string;
}

interface StalenessOptionsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  metadata: BackupMetadata;
}

export function StalenessOptionsDialog({ open, onOpenChange, metadata }: StalenessOptionsDialogProps) {
  const [isReconnecting, setIsReconnecting] = useState(false);
  const [isConfirmRawDataOpen, setIsConfirmRawDataOpen] = useState(false);
  const setStalenessIndicatorDismissed = useEnrichmentStore((state) => state.setStalenessIndicatorDismissed);
  const clearBackupEnrichment = useEnrichmentStore((state) => state.clearBackupEnrichment);

  const exportDate = new Date(metadata.exportTimestamp);
  const formattedTimestamp = formatExportTimestamp(exportDate);
  const ageBreakdown = formatEnrichmentAge(exportDate);

  const handleReconnectApi = async () => {
    setIsReconnecting(true);
    try {
      const result = await invoke<{
        connected: boolean;
        opnsenseVersion?: string;
        errorMessage?: string;
        interfacesCount: number;
        rulesCount: number;
        aliasesCount: number;
      }>('reconnect_api');

      if (result.connected) {
        toast.success(`Connected to API - ${result.interfacesCount} interfaces, ${result.rulesCount} rules, ${result.aliasesCount} aliases loaded`);
        clearBackupEnrichment();
        onOpenChange(false);
      } else {
        toast.error(`Failed to reconnect: ${result.errorMessage || 'Unknown error'}`);
      }
    } catch (error) {
      toast.error(`Reconnection failed: ${error}`);
    } finally {
      setIsReconnecting(false);
    }
  };

  const handleLoadDifferentBackup = async () => {
    try {
      const selectedFile = await openFileDialog({
        multiple: false,
        directory: false,
        filters: [
          {
            name: 'Enrichment JSON',
            extensions: ['json'],
          },
        ],
      });

      if (selectedFile) {
        // Trigger import workflow (assuming there's an importEnrichmentData command)
        toast.success('Loading different backup...');
        onOpenChange(false);
        // TODO: Call importEnrichmentData workflow
      }
    } catch (error) {
      toast.error(`Failed to select file: ${error}`);
    }
  };

  const handleUseRawData = () => {
    setIsConfirmRawDataOpen(true);
  };

  const handleConfirmRawData = async () => {
    try {
      await invoke('clear_backup_enrichment');
      clearBackupEnrichment();
      toast.success('Backup enrichment removed - showing raw data');
      setIsConfirmRawDataOpen(false);
      onOpenChange(false);
    } catch (error) {
      toast.error(`Failed to clear enrichment: ${error}`);
    }
  };

  const handleDismissForSession = () => {
    setStalenessIndicatorDismissed(true);
    invoke('set_staleness_indicator_dismissed', { dismissed: true }).catch((error) => {
      console.error('Failed to set dismissed state:', error);
    });
    toast('Staleness indicator hidden for this session', { icon: '👁️' });
    onOpenChange(false);
  };

  return (
    <>
      <Dialog.Root open={open} onOpenChange={onOpenChange}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 bg-black/50 z-50" />
          <Dialog.Content className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 max-w-lg z-50">
            <div className="flex justify-between items-start mb-4">
              <Dialog.Title className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                Backup Enrichment Options
              </Dialog.Title>
              <Dialog.Close asChild>
                <button
                  className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                  aria-label="Close"
                >
                  <X className="h-5 w-5" />
                </button>
              </Dialog.Close>
            </div>

            <div className="mb-6 p-4 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-md">
              <p className="text-sm text-gray-700 dark:text-gray-300">
                <span className="font-semibold">Exported:</span> {formattedTimestamp}
              </p>
              <p className="text-sm text-gray-700 dark:text-gray-300 mt-1">
                <span className="font-semibold">Age:</span> {ageBreakdown}
              </p>
              <p className="text-sm text-gray-700 dark:text-gray-300 mt-1">
                <span className="font-semibold">Source:</span> {metadata.deviceId}
              </p>
            </div>

            <div className="space-y-3">
              <button
                onClick={handleReconnectApi}
                disabled={isReconnecting}
                className="w-full flex items-center gap-3 px-4 py-3 text-left text-sm font-medium text-gray-900 dark:text-gray-100 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-md hover:bg-green-100 dark:hover:bg-green-900/30 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <RefreshCw className={`h-5 w-5 ${isReconnecting ? 'animate-spin' : ''}`} />
                <div>
                  <div className="font-semibold">Reconnect to API</div>
                  <div className="text-xs text-gray-600 dark:text-gray-400">
                    Fetch fresh enrichment from OPNsense API
                  </div>
                </div>
              </button>

              <button
                onClick={handleLoadDifferentBackup}
                className="w-full flex items-center gap-3 px-4 py-3 text-left text-sm font-medium text-gray-900 dark:text-gray-100 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md hover:bg-blue-100 dark:hover:bg-blue-900/30 transition-colors"
              >
                <FileDown className="h-5 w-5" />
                <div>
                  <div className="font-semibold">Load Different Backup</div>
                  <div className="text-xs text-gray-600 dark:text-gray-400">
                    Import another enrichment JSON file
                  </div>
                </div>
              </button>

              <button
                onClick={handleUseRawData}
                className="w-full flex items-center gap-3 px-4 py-3 text-left text-sm font-medium text-gray-900 dark:text-gray-100 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors"
              >
                <AlertCircle className="h-5 w-5" />
                <div>
                  <div className="font-semibold">Use Raw Data</div>
                  <div className="text-xs text-gray-600 dark:text-gray-400">
                    Remove enrichment, show raw interface/rule names
                  </div>
                </div>
              </button>

              <button
                onClick={handleDismissForSession}
                className="w-full flex items-center gap-3 px-4 py-3 text-left text-sm font-medium text-gray-900 dark:text-gray-100 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
              >
                <EyeOff className="h-5 w-5" />
                <div>
                  <div className="font-semibold">Dismiss for Session</div>
                  <div className="text-xs text-gray-600 dark:text-gray-400">
                    Hide indicator until app restart or new import
                  </div>
                </div>
              </button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>

      <ConfirmRawDataDialog
        open={isConfirmRawDataOpen}
        onOpenChange={setIsConfirmRawDataOpen}
        onConfirm={handleConfirmRawData}
      />
    </>
  );
}
