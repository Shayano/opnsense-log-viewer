import { AlertCircle } from 'lucide-react';
import { Modal } from '@/components/base/modal';
import { Button } from '@/components/base/button';

interface CorruptionDialogProps {
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Dialog shown when index file is corrupted
 */
export function CorruptionDialog({ isOpen, onConfirm, onCancel }: CorruptionDialogProps) {
  return (
    <Modal isOpen={isOpen} onClose={onCancel} title="Index File Corrupted">
      <div className="space-y-4">
        <div className="flex items-start gap-3">
          <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="text-gray-700 dark:text-gray-300 mb-2">
              The index file appears to be corrupted and cannot be loaded.
            </p>
            <p className="text-gray-600 dark:text-gray-400 text-sm">
              This may happen if the index file was damaged or modified externally. Would you like
              to re-index the file to continue?
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
