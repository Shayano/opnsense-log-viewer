import { AlertCircle } from 'lucide-react';
import { Modal } from '@/components/base/modal';
import { Button } from '@/components/base/button';

interface HashMismatchDialogProps {
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Dialog shown when source file has been modified since last index
 */
export function HashMismatchDialog({ isOpen, onConfirm, onCancel }: HashMismatchDialogProps) {
  return (
    <Modal isOpen={isOpen} onClose={onCancel} title="Source File Changed">
      <div className="space-y-4">
        <div className="flex items-start gap-3">
          <AlertCircle className="w-5 h-5 text-orange-500 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="text-gray-700 dark:text-gray-300 mb-2">
              The source file has been modified since the last index was created.
            </p>
            <p className="text-gray-600 dark:text-gray-400 text-sm">
              The existing index may not reflect the current file contents. Would you like to
              re-index the file?
            </p>
          </div>
        </div>

        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="primary" onClick={onConfirm}>
            Re-index
          </Button>
        </div>
      </div>
    </Modal>
  );
}
