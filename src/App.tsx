import { ThemeToggle } from './components/theme-toggle';
import { Toaster } from './components/base';
import { ErrorBoundary } from './components/error-boundary';
import { ComponentShowcase } from './pages/component-showcase';
import { FileSelector, FileError } from './components/file-selector';
import { FilterSidebar } from './components/filter-sidebar';
import { SettingsDialog } from './components/settings-dialog';

function App() {
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
