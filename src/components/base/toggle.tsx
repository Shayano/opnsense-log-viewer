import { ChangeEvent, InputHTMLAttributes, forwardRef, useId } from 'react';
import { cn } from '../../utils/cn';

interface ToggleProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: string;
}

export const Toggle = forwardRef<HTMLInputElement, ToggleProps>(
  ({ label, checked, onChange, className, id, disabled, ...props }, ref) => {
    const generatedId = useId();
    const toggleId = id || generatedId;

    return (
      <div className="flex items-center gap-2">
        <button
          type="button"
          role="switch"
          aria-checked={checked}
          aria-label={label || props['aria-label']}
          disabled={disabled}
          onClick={() => {
            if (!disabled && onChange) {
              const event = {
                target: { checked: !checked },
                currentTarget: { checked: !checked },
              } as ChangeEvent<HTMLInputElement>;
              onChange(event);
            }
          }}
          className={cn(
            'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
            'focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2',
            'dark:focus:ring-offset-gray-900',
            checked ? 'bg-primary-600 dark:bg-primary-500' : 'bg-gray-300 dark:bg-gray-600',
            disabled && 'opacity-50 cursor-not-allowed',
            !disabled && 'cursor-pointer',
            className
          )}
        >
          <span
            className={cn(
              'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
              checked ? 'translate-x-6' : 'translate-x-1'
            )}
          />
        </button>
        <input
          ref={ref}
          type="checkbox"
          id={toggleId}
          checked={checked}
          onChange={onChange}
          disabled={disabled}
          className="sr-only"
          {...props}
        />
        {label && (
          <label
            htmlFor={toggleId}
            className={cn(
              'text-sm text-gray-700 dark:text-gray-300',
              !disabled && 'cursor-pointer',
              disabled && 'opacity-50 cursor-not-allowed'
            )}
          >
            {label}
          </label>
        )}
      </div>
    );
  }
);

Toggle.displayName = 'Toggle';
