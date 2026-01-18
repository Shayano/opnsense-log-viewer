import { useState, useEffect } from 'react';
import { X } from 'lucide-react';
import { FieldSelector } from './field-selector';
import { OperatorSelector } from './operator-selector';
import { ValueInput } from './value-input';
import { useFilterStore } from '@/stores/filter-store';
import toast from 'react-hot-toast';
import type { FieldType, OperatorType, RelativeTimeRange } from '@/types/filter';

interface FilterBuilderProps {
  isOpen: boolean;
  onClose: () => void;
  editingFilter?: { id: string; field: FieldType; operator: OperatorType; value: string | number | [string, string] | RelativeTimeRange } | null;
}

export function FilterBuilder({ isOpen, onClose, editingFilter }: FilterBuilderProps): JSX.Element | null {
  const { addFilter, updateFilter } = useFilterStore();

  const [field, setField] = useState<FieldType | undefined>(editingFilter?.field);
  const [operator, setOperator] = useState<OperatorType | undefined>(editingFilter?.operator);
  const [value, setValue] = useState<string | number | [string, string] | RelativeTimeRange>(editingFilter?.value || '');

  // Reset form when modal closes or when editing filter changes
  useEffect(() => {
    if (!isOpen) {
      setField(undefined);
      setOperator(undefined);
      setValue('');
    } else if (editingFilter) {
      setField(editingFilter.field);
      setOperator(editingFilter.operator);
      setValue(editingFilter.value);
    }
  }, [isOpen, editingFilter]);

  // Reset operator and value when field changes
  const handleFieldChange = (newField: FieldType): void => {
    setField(newField);
    setOperator(undefined);
    setValue('');
  };

  // Reset value when operator changes
  const handleOperatorChange = (newOperator: OperatorType): void => {
    setOperator(newOperator);
    // Reset value to appropriate type for the operator
    if (newOperator === 'between' || newOperator === 'absoluteRange') {
      setValue(['', '']);
    } else {
      setValue('');
    }
  };

  const handleSubmit = (e: React.FormEvent): void => {
    e.preventDefault();

    if (!field || !operator || !value || (Array.isArray(value) && (!value[0] || !value[1]))) {
      toast.error('Please fill in all fields');
      return;
    }

    if (editingFilter) {
      updateFilter(editingFilter.id, {
        field,
        operator,
        value,
      });
      toast.success('Filter updated');
    } else {
      addFilter({
        field,
        operator,
        value,
      });
      toast.success('Filter added');
    }

    // Reset form and close
    setField(undefined);
    setOperator(undefined);
    setValue('');
    onClose();
  };

  const handleClose = (): void => {
    setField(undefined);
    setOperator(undefined);
    setValue('');
    onClose();
  };

  // Handle Escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent): void => {
      if (e.key === 'Escape' && isOpen) {
        handleClose();
      }
    };

    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [isOpen]);

  if (!isOpen) return null;

  const isFormValid = field && operator && value && !(Array.isArray(value) && (!value[0] || !value[1]));

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-lg w-full max-w-2xl p-6">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            {editingFilter ? 'Edit Filter' : 'Add Filter'}
          </h2>
          <button
            onClick={handleClose}
            className="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
            aria-label="Close filter builder"
          >
            <X className="w-5 h-5 text-gray-600 dark:text-gray-400" />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Step 1: Field Selection */}
          <FieldSelector
            value={field}
            onChange={handleFieldChange}
          />

          {/* Step 2: Operator Selection (context-aware) */}
          {field && (
            <OperatorSelector
              fieldType={field}
              value={operator}
              onChange={handleOperatorChange}
            />
          )}

          {/* Step 3: Value Input (context-aware) */}
          {field && operator && (
            <ValueInput
              fieldType={field}
              operatorType={operator}
              value={value}
              onChange={setValue}
            />
          )}

          {/* Actions */}
          <div className="flex justify-end gap-3 mt-6">
            <button
              type="button"
              onClick={handleClose}
              className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300
                bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700
                rounded transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!isFormValid}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600
                hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed
                rounded transition-colors"
            >
              {editingFilter ? 'Update Filter' : 'Add Filter'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
