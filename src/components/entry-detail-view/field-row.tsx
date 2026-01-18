import { CopyButton } from './copy-button';

interface FieldRowProps {
  label: string;
  value: string;
}

export function FieldRow({ label, value }: FieldRowProps) {
  return (
    <div className="flex items-center justify-between py-1 px-2 hover:bg-gray-50 dark:hover:bg-gray-800 rounded">
      <div className="flex items-center gap-3 flex-1 min-w-0">
        <span className="text-xs font-medium text-gray-600 dark:text-gray-400 w-32 flex-shrink-0">
          {label}:
        </span>
        <span className="text-xs font-mono text-gray-900 dark:text-gray-100 truncate" title={value}>
          {value}
        </span>
      </div>
      <CopyButton value={value} compact />
    </div>
  );
}
