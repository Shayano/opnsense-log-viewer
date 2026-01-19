import { useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Loader2 } from 'lucide-react';
import { ThemeToggle } from './components/theme-toggle';
import { Toaster } from './components/base';
import { ErrorBoundary } from './components/error-boundary';
import { ComponentShowcase } from './pages/component-showcase';
import { FileSelector, FileError } from './components/file-selector';
import { FilterSidebar } from './components/filter-sidebar';
import { SettingsDialog } from './components/settings-dialog';
import { OfflineBanner } from './components/api-status/offline-banner';
import { ConnectionIndicator } from './components/api-status/connection-indicator';
import { StalenessIndicator, MinimizedStalenessIcon } from './components/enrichment';
import { ApiReconnectedPrompt } from './components/dialogs/api-reconnected-prompt';
import { ResultCount } from './components/result-count';
import { EmptyResultsState } from './components/result-count';
import { LogTable } from './components/log-table';
import { loadApiCredentials, testApiConnection } from './utils/api-client';
import { useEnrichmentStore } from './stores/enrichment-store';
import { useQueryStore } from './stores/query-store';
import { loadCachedRuleLabels, loadCachedAliases } from './services/enrichment-service';
import { detectIncompleteExports, cleanupPartialExport } from './services/export-service';
import toast from 'react-hot-toast';

interface InterfaceMappingCache {
  mappings: Record<string, string>;
  lastUpdated: string;
  deviceId: string;
}

function App() {
  const setInterfaceMappings = useEnrichmentStore((state) => state.setInterfaceMappings);
  const setConnectionStatus = useEnrichmentStore((state) => state.setConnectionStatus);
  const connectionStatus = useEnrichmentStore((state) => state.connectionStatus);
  const { currentResult, currentEntries, entriesLoading } = useQueryStore();

  // Story 3.1: Auto-load credentials on app startup (AC requirement)
  useEffect(() => {
    const autoLoadCredentials = async () => {
      try {
        const credentials = await loadApiCredentials();
        if (credentials) {
          // Silently test connection in background (non-blocking)
          testApiConnection(
            credentials.endpointUrl,
            credentials.apiKey,
            credentials.apiSecret
          ).catch(() => {
            // Silent failure on startup - user can manually test via Settings
          });
        }
      } catch (error) {
        // Silent failure on first launch (no credentials yet)
        console.debug('No credentials to auto-load');
      }
    };

    autoLoadCredentials();
  }, []);

  // Story 3.5: Connection status polling
  const pollConnectionStatus = useCallback(async () => {
    try {
      const info = await invoke<any>('get_connection_status');
      const previousStatus = connectionStatus;
      setConnectionStatus(info);

      // Show toast on status changes
      if (previousStatus && previousStatus !== info.status) {
        if (info.status === 'connected' && previousStatus === 'disconnected') {
          toast.success('API reconnected. Enrichment resumed.');
        } else if (info.status === 'disconnected' && previousStatus === 'connected') {
          toast('API connection lost. Using cached enrichment data.', {
            icon: '⚠️',
            duration: 5000,
          });
        }
      }
    } catch (error) {
      console.error('Failed to poll connection status:', error);
    }
  }, [setConnectionStatus, connectionStatus]);

  useEffect(() => {
    // Initial status check
    pollConnectionStatus();

    // Setup polling interval (every 10 seconds)
    const interval = setInterval(pollConnectionStatus, 10000);

    return () => clearInterval(interval);
  }, [pollConnectionStatus]);

  // Story 3.2: Auto-load interface mappings on app startup
  useEffect(() => {
    const loadCachedMappings = async () => {
      try {
        const cache = await invoke<InterfaceMappingCache | null>('get_interface_mappings_cmd');
        if (cache) {
          setInterfaceMappings(cache);
          console.log('Loaded cached interface mappings:', cache.mappings);
        }
      } catch (error) {
        console.error('Failed to load cached interface mappings:', error);
      }
    };

    loadCachedMappings();

    // Listen for interface mappings updates (emitted on successful connection)
    const unlistenPromise = listen<Record<string, string>>('interface-mappings-updated', (event) => {
      const mappingsCache = {
        mappings: event.payload,
        lastUpdated: new Date().toISOString(),
        deviceId: 'current', // Device ID not included in event
      };
      setInterfaceMappings(mappingsCache);
      console.log('Interface mappings updated:', event.payload);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [setInterfaceMappings]);

  // Story 3.3: Auto-load rule labels on app startup
  useEffect(() => {
    loadCachedRuleLabels();
  }, []);

  // Story 3.4: Auto-load IP aliases on app startup
  useEffect(() => {
    loadCachedAliases();
  }, []);

  // Story 5.3: Detect incomplete exports on startup
  useEffect(() => {
    const checkIncompleteExports = async () => {
      try {
        const incompleteFiles = await detectIncompleteExports();

        if (incompleteFiles.length > 0) {
          incompleteFiles.forEach((filePath) => {
            toast(
              (t) => (
                <div className="flex flex-col gap-2">
                  <div className="font-medium text-yellow-900 dark:text-yellow-100">
                    Incomplete export detected
                  </div>
                  <div className="text-sm text-gray-700 dark:text-gray-300">
                    {filePath}
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={async () => {
                        await cleanupPartialExport(filePath);
                        toast.dismiss(t.id);
                      }}
                      className="px-3 py-1 text-sm font-medium bg-red-100 dark:bg-red-800 rounded hover:bg-red-200 dark:hover:bg-red-700 transition-colors"
                    >
                      Delete
                    </button>
                    <button
                      onClick={() => toast.dismiss(t.id)}
                      className="px-3 py-1 text-sm font-medium bg-gray-100 dark:bg-gray-800 rounded hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                    >
                      Keep
                    </button>
                  </div>
                </div>
              ),
              { duration: Infinity, icon: '⚠️' }
            );
          });
        }
      } catch (error) {
        console.error('Failed to check for incomplete exports:', error);
      }
    };

    checkIncompleteExports();
  }, []);

  return (
    <div className="min-h-screen bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 flex flex-col">
      <Toaster />
      {/* Story 3.5: Offline Banner */}
      <OfflineBanner />
      {/* Story 4.3: Staleness Indicators */}
      <StalenessIndicator />
      <MinimizedStalenessIcon />
      <ApiReconnectedPrompt />

      <header className="border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex-shrink-0">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold">OPNsense Log Viewer</h1>
          <div className="flex items-center gap-2">
            {/* Story 3.5: Connection Status Indicator */}
            <ConnectionIndicator />
            <SettingsDialog />
            <ThemeToggle />
          </div>
        </div>
      </header>

      {/* Main Layout: FilterSidebar + Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* FilterSidebar */}
        <FilterSidebar />

        {/* Main Content */}
        <main className="flex-1 overflow-auto p-6">
          <div className="mx-auto max-w-7xl">
            <h2 className="text-xl font-medium mb-6">File Selection</h2>
            <p className="text-gray-600 dark:text-gray-400 mb-4">
              Select a log file to begin analyzing. Supports .log, .txt, .csv and other text formats.
            </p>

            <div className="mb-8">
              <FileError />
              <FileSelector />
            </div>

            {/* Search Results: LogTable with enrichment (rule labels, aliases) */}
            {currentResult && (
              <div className="mb-8 mt-8">
                <ResultCount />
                {entriesLoading ? (
                  <div className="flex items-center justify-center py-12 gap-2 text-gray-600 dark:text-gray-400">
                    <Loader2 className="w-6 h-6 animate-spin" />
                    <span>Loading entries...</span>
                  </div>
                ) : currentResult.matchedCount === 0 ? (
                  <EmptyResultsState />
                ) : (
                  <div className="min-h-[400px] flex flex-col border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                    <LogTable entries={currentEntries ?? []} />
                  </div>
                )}
              </div>
            )}

            <h2 className="text-xl font-medium mb-6 mt-12">Component Showcase</h2>
            <p className="text-gray-600 dark:text-gray-400 mb-8">
              Tailwind CSS design system foundation is now configured. The component showcase below
              demonstrates all base components with theme support.
            </p>
            <ErrorBoundary
              fallback={
                <div className="text-center text-error-600 dark:text-error-400">
                  Component showcase failed to load
                </div>
              }
            >
              <ComponentShowcase />
            </ErrorBoundary>
          </div>
        </main>
      </div>
    </div>
  );
}

export default App;
