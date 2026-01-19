import * as Dialog from '@radix-ui/react-dialog';
import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useEnrichmentStore } from '@/stores/enrichment-store';
import { invoke } from '@tauri-apps/api/core';
import toast from 'react-hot-toast';

export function ApiReconnectedPrompt() {
  const [isOpen, setIsOpen] = useState(false);
  const backupEnrichmentActive = useEnrichmentStore((state) => state.backupEnrichmentActive);
  const clearBackupEnrichment = useEnrichmentStore((state) => state.clearBackupEnrichment);

  useEffect(() => {
    const unlisten = listen('api-reconnected', (_event) => {
      // Only show if backup enrichment is currently active
      if (backupEnrichmentActive) {
        setIsOpen(true);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [backupEnrichmentActive]);

  const handleSwitchToLive = async () => {
    try {
      // Clear backup enrichment
      await invoke('clear_backup_enrichment');
      clearBackupEnrichment();

      toast.success('Switched to live API enrichment');
      setIsOpen(false);
    } catch (error) {
      toast.error(`Failed to switch to live enrichment: ${error}`);
    }
  };

  const handleKeepBackup = () => {
    setIsOpen(false);
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={setIsOpen}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 z-50" />
        <Dialog.Content className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 max-w-md z-50">
          <Dialog.Title className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            API Reconnected
          </Dialog.Title>
          <Dialog.Description className="mt-2 text-sm text-gray-600 dark:text-gray-400">
            Your OPNsense API connection has been restored. Would you like to switch to live enrichment data?
          </Dialog.Description>
          <div className="mt-6 flex justify-end gap-3">
            <button
              onClick={handleKeepBackup}
              className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-800 rounded-md hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
            >
              Keep Backup
            </button>
            <button
              onClick={handleSwitchToLive}
              className="px-4 py-2 text-sm font-medium text-white bg-green-600 rounded-md hover:bg-green-700 transition-colors"
            >
              Yes, Switch to Live
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
