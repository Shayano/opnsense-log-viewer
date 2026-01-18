import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ThemeToggle } from './components/theme-toggle';
import { Toaster } from './components/base';
import { ErrorBoundary } from './components/error-boundary';
import { ComponentShowcase } from './pages/component-showcase';
import { FileSelector, FileError } from './components/file-selector';
import { FilterSidebar } from './components/filter-sidebar';
import { SettingsDialog } from './components/settings-dialog';
import { loadApiCredentials, testApiConnection } from './utils/api-client';
import { useEnrichmentStore } from './stores/enrichment-store';
import { loadCachedRuleLabels } from './services/enrichment-service';

interface InterfaceMappingCache {
  mappings: Record<string, string>;
  lastUpdated: string;
  deviceId: string;
}

function App() {
  const setInterfaceMappings = useEnrichmentStore((state) => state.setInterfaceMappings);

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

  return (
    <div className="min-h-screen bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 flex flex-col">
      <Toaster />
      <header className="border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex-shrink-0">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold">OPNsense Log Viewer</h1>
          <div className="flex items-center gap-2">
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
