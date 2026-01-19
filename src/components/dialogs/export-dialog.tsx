import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { FileDown, X } from 'lucide-react';
import type { ExportFormat } from '@/types/export';

interface ExportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  totalEntries: number;
  onExport: (format: ExportFormat) => void;
}

export function ExportDialog({
  open,
  onOpenChange,
  totalEntries,
  onExport,
}: ExportDialogProps) {
  const [selectedFormat, setSelectedFormat] = useState<ExportFormat>('csv');

  const handleExport = () => {
    onExport(selectedFormat);
  };

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <Dialog.Content
          className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 max-w-md w-full z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
          aria-describedby="export-dialog-description"
        >
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-2">
              <FileDown className="h-5 w-5 text-blue-600 dark:text-blue-400" />
              <Dialog.Title className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                Export Results
              </Dialog.Title>
            </div>
            <Dialog.Close asChild>
              <button
                className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                aria-label="Close"
              >
                <X className="h-5 w-5" />
              </button>
            </Dialog.Close>
          </div>

          <Dialog.Description
            id="export-dialog-description"
            className="mt-4 text-sm text-gray-600 dark:text-gray-400"
          >
            Export your filtered search results to a file format of your choice.
          </Dialog.Description>

          <div className="mt-4 space-y-4">
            {/* Export Scope */}
            <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-3">
              <p className="text-sm font-medium text-blue-900 dark:text-blue-100">
                Export Scope
              </p>
              <p className="text-sm text-blue-700 dark:text-blue-300 mt-1">
                Filtered Results: <span className="font-semibold">{totalEntries.toLocaleString()}</span> entries
              </p>
            </div>

            {/* Format Selection */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Format
              </label>
              <div className="space-y-2">
                <label className="flex items-center gap-3 p-3 border border-gray-300 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer transition-colors">
                  <input
                    type="radio"
                    name="format"
                    value="csv"
                    checked={selectedFormat === 'csv'}
                    onChange={(e) => setSelectedFormat(e.target.value as ExportFormat)}
                    className="h-4 w-4 text-blue-600 focus:ring-blue-500"
                  />
                  <div className="flex-1">
                    <p className="text-sm font-medium text-gray-900 dark:text-gray-100">CSV</p>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Compatible with Excel, Google Sheets, and LibreOffice
                    </p>
                  </div>
                </label>

                <label className="flex items-center gap-3 p-3 border border-gray-300 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer transition-colors">
                  <input
                    type="radio"
                    name="format"
                    value="json"
                    checked={selectedFormat === 'json'}
                    onChange={(e) => setSelectedFormat(e.target.value as ExportFormat)}
                    className="h-4 w-4 text-blue-600 focus:ring-blue-500"
                  />
                  <div className="flex-1">
                    <p className="text-sm font-medium text-gray-900 dark:text-gray-100">JSON</p>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Structured data format for programmatic analysis
                    </p>
                  </div>
                </label>
              </div>
            </div>
          </div>

          <div className="mt-6 flex justify-end gap-3">
            <Dialog.Close asChild>
              <button className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-800 rounded-md hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors">
                Cancel
              </button>
            </Dialog.Close>
            <button
              onClick={handleExport}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 transition-colors flex items-center gap-2"
            >
              <FileDown className="h-4 w-4" />
              Export
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
