import { InputHTMLAttributes, forwardRef, useId } from 'react';
import { Check } from 'lucide-react';
import { cn } from '../../utils/cn';

interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: string;
}

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ label, checked, className, id, ...props }, ref) => {
    const generatedId = useId();
    const checkboxId = id || generatedId;

    return (
      <div className="flex items-center gap-2">
        <div className="relative flex items-center">
          <input
            ref={ref}
            type="checkbox"
            id={checkboxId}
            checked={checked}
            aria-checked={checked}
            aria-label={label || props['aria-label']}
            className={cn(
              'h-4 w-4 appearance-none rounded border transition-colors',
              'bg-white dark:bg-gray-800',
              'border-gray-300 dark:border-gray-600',
              'checked:bg-primary-600 checked:border-primary-600',
              'dark:checked:bg-primary-500 dark:checked:border-primary-500',
              'focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2',
              'dark:focus:ring-offset-gray-900',
              props.disabled && 'opacity-50 cursor-not-allowed',
              className
            )}
            {...props}
          />
          {checked && (
            <Check
              className="pointer-events-none absolute left-0 h-4 w-4 text-white"
              strokeWidth={3}
              aria-hidden="true"
            />
          )}
        </div>
        {label && (
          <label
            htmlFor={checkboxId}
            className={cn(
              'text-sm text-gray-700 dark:text-gray-300',
              !props.disabled && 'cursor-pointer',
              props.disabled && 'opacity-50 cursor-not-allowed'
            )}
          >
            {label}
          </label>
        )}
      </div>
    );
  }
);

Checkbox.displayName = 'Checkbox';
