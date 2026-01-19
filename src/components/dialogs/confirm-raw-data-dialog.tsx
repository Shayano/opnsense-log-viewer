import * as AlertDialog from '@radix-ui/react-alert-dialog';

interface ConfirmRawDataDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}

export function ConfirmRawDataDialog({ open, onOpenChange, onConfirm }: ConfirmRawDataDialogProps) {
  return (
    <AlertDialog.Root open={open} onOpenChange={onOpenChange}>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="fixed inset-0 bg-black/50 z-50" />
        <AlertDialog.Content className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 max-w-md z-50">
          <AlertDialog.Title className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Remove Backup Enrichment?
          </AlertDialog.Title>
          <AlertDialog.Description className="mt-2 text-sm text-gray-600 dark:text-gray-400">
            <p className="mb-3">
              You will see raw interface names, rule hashes, and IP addresses without context.
            </p>
            <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3">
              <p className="text-red-800 dark:text-red-300 text-xs font-medium mb-1">
                Affected:
              </p>
              <ul className="text-red-700 dark:text-red-400 text-xs space-y-1">
                <li>• Interface names (vtnet0 instead of LAN)</li>
                <li>• Rule labels (hashes instead of descriptions)</li>
                <li>• Alias names (IPs without context)</li>
              </ul>
            </div>
          </AlertDialog.Description>
          <div className="mt-6 flex justify-end gap-3">
            <AlertDialog.Cancel asChild>
              <button className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-800 rounded-md hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors">
                Cancel
              </button>
            </AlertDialog.Cancel>
            <AlertDialog.Action asChild>
              <button
                onClick={onConfirm}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 transition-colors"
              >
                Continue
              </button>
            </AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}
