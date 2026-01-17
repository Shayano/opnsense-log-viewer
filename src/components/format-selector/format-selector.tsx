import { useState, useId } from 'react';
import { Modal } from '@/components/base/modal';
import { Button } from '@/components/base/button';

type LogFormat = 'RFC3164' | 'RFC5424' | 'CSV';

export interface FormatSelectorProps {
  isOpen: boolean;
  onConfirm: (format: LogFormat) => void;
  onCancel: () => void;
}

export function FormatSelector({ isOpen, onConfirm, onCancel }: FormatSelectorProps) {
  const [selectedFormat, setSelectedFormat] = useState<LogFormat>('RFC3164');
  const radioGroupId = useId();

  return (
    <Modal isOpen={isOpen} onClose={onCancel} title="Select Log Format">
      <div className="space-y-4">
        <p className="text-sm text-gray-600 dark:text-gray-400">
          Unable to auto-detect log format. Please select manually:
        </p>

        <div className="space-y-2" role="radiogroup" aria-labelledby={radioGroupId}>
          <label className="flex items-start space-x-3 p-3 border rounded cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 dark:border-gray-600">
            <input
              type="radio"
              name="format"
              value="RFC3164"
              checked={selectedFormat === 'RFC3164'}
              onChange={() => setSelectedFormat('RFC3164')}
              className="mt-1"
            />
            <div>
              <div className="font-medium text-gray-900 dark:text-gray-100">RFC3164 (Legacy Syslog)</div>
              <div className="text-sm text-gray-500 dark:text-gray-400 font-mono">
                &lt;134&gt;Jan 15 14:30:00 firewall filterlog[123]: message
              </div>
            </div>
          </label>

          <label className="flex items-start space-x-3 p-3 border rounded cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 dark:border-gray-600">
            <input
              type="radio"
              name="format"
              value="RFC5424"
              checked={selectedFormat === 'RFC5424'}
              onChange={() => setSelectedFormat('RFC5424')}
              className="mt-1"
            />
            <div>
              <div className="font-medium text-gray-900 dark:text-gray-100">RFC5424 (Modern Syslog)</div>
              <div className="text-sm text-gray-500 dark:text-gray-400 font-mono">
                &lt;134&gt;1 2026-01-15T14:30:00Z firewall filterlog 123 - - message
              </div>
            </div>
          </label>

          <label className="flex items-start space-x-3 p-3 border rounded cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 dark:border-gray-600">
            <input
              type="radio"
              name="format"
              value="CSV"
              checked={selectedFormat === 'CSV'}
              onChange={() => setSelectedFormat('CSV')}
              className="mt-1"
            />
            <div>
              <div className="font-medium text-gray-900 dark:text-gray-100">CSV filterlog (OPNsense)</div>
              <div className="text-sm text-gray-500 dark:text-gray-400 font-mono">
                1705329000,,,vtnet0,,pass,inet,192.168.1.100,443,...
              </div>
            </div>
          </label>
        </div>

        <div className="flex justify-end space-x-2 pt-4">
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => onConfirm(selectedFormat)}>
            Confirm
          </Button>
        </div>
      </div>
    </Modal>
  );
}
