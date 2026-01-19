import { invoke } from '@tauri-apps/api/core';
import { useEnrichmentStore } from '@/stores/enrichment-store';
import { AlertTriangle, RefreshCw, Upload, X } from 'lucide-react';
import { useState } from 'react';
import toast from 'react-hot-toast';
import { importEnrichmentData } from '@/services/enrichment-import-service';

export function OfflineBanner() {
  const connectionStatus = useEnrichmentStore((state) => state.connectionStatus);
  const lastError = useEnrichmentStore((state) => state.lastError);
  const setConnectionStatus = useEnrichmentStore((state) => state.setConnectionStatus);
  const setBackupEnrichment = useEnrichmentStore((state) => state.setBackupEnrichment);

  const [isRetrying, setIsRetrying] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [isDismissed, setIsDismissed] = useState(() => {
    return sessionStorage.getItem('offline-banner-dismissed') === 'true';
  });

  // Only show banner when disconnected and not dismissed
  if (connectionStatus !== 'disconnected' || isDismissed) {
    return null;
  }

  const handleRetry = async () => {
    setIsRetrying(true);
    try {
      const result = await invoke<any>('retry_api_connection');
      setConnectionStatus(result);
      toast.success('Reconnected successfully');
    } catch (error) {
      toast.error(`Retry failed: ${error}`);
    } finally {
      setIsRetrying(false);
    }
  };

  const handleDismiss = () => {
    setIsDismissed(true);
    sessionStorage.setItem('offline-banner-dismissed', 'true');
  };

  const handleLoadBackup = async () => {
    setIsImporting(true);
    try {
      const success = await importEnrichmentData();
      if (success) {
        // Hide banner after successful import
        setIsDismissed(true);
        sessionStorage.setItem('offline-banner-dismissed', 'true');
      }
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <div className="bg-amber-100 dark:bg-amber-900/30 border-b border-amber-200 dark:border-amber-800 px-4 py-3">
      <div className="flex items-center justify-between max-w-7xl mx-auto">
        <div className="flex items-center gap-3">
          <AlertTriangle className="h-5 w-5 text-amber-600 dark:text-amber-400" />
          <div>
            <p className="text-sm font-medium text-amber-900 dark:text-amber-100">
              API Offline - Showing raw data without enrichment
            </p>
            {lastError && (
              <p className="text-xs text-amber-700 dark:text-amber-300 mt-0.5">
                {lastError}
              </p>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleLoadBackup}
            disabled={isImporting}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-amber-900 dark:text-amber-100 hover:bg-amber-200 dark:hover:bg-amber-800/50 rounded transition-colors disabled:opacity-50"
          >
            <Upload className={`h-4 w-4 ${isImporting ? 'animate-pulse' : ''}`} />
            {isImporting ? 'Loading...' : 'Load Backup Enrichment'}
          </button>

          <button
            onClick={handleRetry}
            disabled={isRetrying}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-amber-900 dark:text-amber-100 hover:bg-amber-200 dark:hover:bg-amber-800/50 rounded transition-colors disabled:opacity-50"
          >
            <RefreshCw className={`h-4 w-4 ${isRetrying ? 'animate-spin' : ''}`} />
            Retry Connection
          </button>

          <button
            onClick={handleDismiss}
            className="p-1.5 text-amber-700 dark:text-amber-300 hover:bg-amber-200 dark:hover:bg-amber-800/50 rounded transition-colors"
            aria-label="Dismiss"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
