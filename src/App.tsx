import { ThemeToggle } from './components/theme-toggle';
import { Toaster } from './components/base';
import { ComponentShowcase } from './pages/component-showcase';

function App() {
  return (
    <div className="min-h-screen bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      <Toaster />
      <header className="border-b border-gray-200 dark:border-gray-700 px-6 py-4">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold">OPNsense Log Viewer</h1>
          <ThemeToggle />
        </div>
      </header>
      <main className="p-6">
        <div className="mx-auto max-w-7xl">
          <h2 className="text-xl font-medium mb-6">Component Showcase</h2>
          <p className="text-gray-600 dark:text-gray-400 mb-8">
            Tailwind CSS design system foundation is now configured. The component showcase below
            demonstrates all base components with theme support.
          </p>
          <ComponentShowcase />
        </div>
      </main>
    </div>
  );
}

export default App;
