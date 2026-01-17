import { AlertTriangle } from 'lucide-react';
import { Modal } from '@/components/base/modal';
import { Button } from '@/components/base/button';

interface LargeFileWarningProps {
  isOpen: boolean;
  fileSize: number;
  onConfirm: () => void;
  onCancel: () => void;
}

const BYTES_PER_GB = 1024 * 1024 * 1024;

/**
 * LargeFileWarning component
 * Displays a warning modal for files larger than 50GB
 */
export function LargeFileWarning({
  isOpen,
  fileSize,
  onConfirm,
  onCancel,
}: LargeFileWarningProps) {
  const fileSizeGB = (fileSize / BYTES_PER_GB).toFixed(1);

  return (
    <Modal isOpen={isOpen} onClose={onCancel} title="Large File Warning" size="md">
      <div className="flex flex-col gap-4">
        <div className="flex items-start gap-3">
          <AlertTriangle className="text-yellow-500 flex-shrink-0" size={24} />
          <div>
            <p className="text-gray-900 dark:text-gray-100">
              Large file may take extended time to index.
            </p>
            <p className="text-gray-700 dark:text-gray-300 mt-2">
              File size: <strong>{fileSizeGB} GB</strong>
            </p>
            <p className="text-gray-600 dark:text-gray-400 mt-2 text-sm">
              Indexing large files can take several minutes. Do you want to continue?
            </p>
          </div>
        </div>

        <div className="flex gap-3 justify-end">
          <Button variant="secondary" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="primary" onClick={onConfirm}>
            Continue
          </Button>
        </div>
      </div>
    </Modal>
  );
}
