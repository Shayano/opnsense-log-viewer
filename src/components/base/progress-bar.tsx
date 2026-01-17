import { cn } from '../../utils/cn';

interface ProgressBarProps {
  value: number;
  label?: string;
  showPercentage?: boolean;
  className?: string;
}

export function ProgressBar({ value, label, showPercentage = false, className }: ProgressBarProps) {
  const clampedValue = Math.min(100, Math.max(0, value));
  const progressId = `progress-${Math.random().toString(36).slice(2, 9)}`;

  return (
    <div className={cn('w-full', className)}>
      {(label || showPercentage) && (
        <div className="mb-2 flex items-center justify-between text-sm">
          {label && <span className="text-gray-700 dark:text-gray-300">{label}</span>}
          {showPercentage && (
            <span className="font-medium text-gray-900 dark:text-gray-100">
              {clampedValue.toFixed(0)}%
            </span>
          )}
        </div>
      )}
      <div
        role="progressbar"
        aria-valuenow={clampedValue}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
        id={progressId}
        className="h-2 w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700"
      >
        <div
          className="h-full rounded-full bg-primary-600 transition-all duration-300 ease-out dark:bg-primary-500"
          style={{ width: `${clampedValue}%` }}
        />
      </div>
    </div>
  );
}
