# Story 2.1: Visual Filter Builder UI

Status: done

## Story

As a network administrator,
I want to build complex filters using visual dropdowns instead of syntax,
So that I can construct queries like "blocked traffic on WAN between 2AM-3AM to port 443" without memorizing filter syntax.

## Acceptance Criteria

**Given** log entries are loaded and displayed (Epic 1 complete)
**When** I click the "Add Filter" button in the left sidebar
**Then** a filter builder modal opens with three dropdown fields:

**Dropdown 1 - Field Selection:**
- Timestamp Range
- Source IP
- Destination IP
- Source Port
- Destination Port
- Protocol
- Action
- Interface
- Rule Label

**Dropdown 2 - Operator Selection (context-aware based on field):**

For text fields (IPs, Interface, Rule Label):
- equals
- contains
- starts with
- ends with
- regex

For numeric fields (Ports):
- equals
- greater than
- less than
- between

For select fields (Protocol, Action):
- equals
- not equals

For Timestamp:
- absolute range (date picker)
- relative (last 1h, 6h, 24h, 7d, 30d)

**Dropdown 3 - Value Input (context-aware based on field):**
- Text input for IPs, ports, regex
- Date/time picker for timestamps
- Dropdown for Protocol (TCP, UDP, ICMP, etc.)
- Dropdown for Action (pass, block, reject)

**When** I select a field and operator
**Then** the appropriate value input appears
**And** placeholder text provides guidance (e.g., "Enter IP address like 192.168.1.1")

**When** I complete all three fields and click "Add Filter"
**Then** the filter is added to the "Active Filters" list
**And** the modal closes
**And** the filter is in "draft mode" (not executed yet)

**When** I add multiple filters
**Then** each filter displays with:
- Field name
- Operator
- Value
- Boolean logic selector (AND/OR/NOT) for combining with next filter
- [Edit] button
- [Remove] button

**And** the filter UI follows UX Design Spec:
- VS Code-inspired sidebar layout
- Collapsible filter panel for space optimization
- Clear visual hierarchy
- Dark/light theme support

**When** filters are in draft mode
**Then** a prominent "Search" button is displayed
**And** the button is styled to signal it's the action trigger (Postman-style explicit control)
**And** no automatic query execution occurs when adding/removing filters

## Tasks / Subtasks

- [ ] Create FilterBuilder component structure (AC: Modal UI)
  - [ ] Create src/components/filter-builder/filter-builder.tsx
  - [ ] Create src/components/filter-builder/filter-builder.test.tsx
  - [ ] Create src/components/filter-builder/index.ts (barrel export)
  - [ ] Setup component scaffolding with props interface
  - [ ] Add modal open/close state management

- [ ] Create Zustand filter store (AC: State management)
  - [ ] Create src/stores/filter-store.ts
  - [ ] Define Filter type with field, operator, value, logic
  - [ ] Implement addFilter action
  - [ ] Implement removeFilter action
  - [ ] Implement updateFilter action
  - [ ] Implement clearFilters action
  - [ ] Add draft mode state flag
  - [ ] Write store tests

- [ ] Implement field selection dropdown (AC: Dropdown 1)
  - [ ] Create FieldSelector component
  - [ ] List all filterable fields (Timestamp, IPs, Ports, Protocol, Action, Interface, Rule)
  - [ ] Apply consistent styling (Tailwind)
  - [ ] Handle field selection event
  - [ ] Update operator dropdown based on field type

- [ ] Implement context-aware operator dropdown (AC: Dropdown 2)
  - [ ] Create OperatorSelector component
  - [ ] Define operator sets per field type:
    * Text fields: equals, contains, startsWith, endsWith, regex
    * Numeric fields: equals, greaterThan, lessThan, between
    * Select fields: equals, notEquals
    * Timestamp: absoluteRange, relative
  - [ ] Dynamically update operators based on selected field
  - [ ] Handle operator selection event

- [ ] Implement context-aware value input (AC: Dropdown 3)
  - [ ] Create ValueInput component with multiple input types
  - [ ] Text input for IPs, regex
  - [ ] Number input for ports
  - [ ] Date/time picker for timestamps (using date-fns)
  - [ ] Dropdown for Protocol (TCP, UDP, ICMP, etc.)
  - [ ] Dropdown for Action (pass, block, reject)
  - [ ] Placeholder text guidance per field type
  - [ ] Input validation per field type
  - [ ] Handle value input event

- [ ] Implement "Add Filter" button logic (AC: Add to list)
  - [ ] Validate all three fields are selected
  - [ ] Create Filter object with field/operator/value
  - [ ] Dispatch addFilter action to store
  - [ ] Close modal after adding filter
  - [ ] Clear modal form for next filter
  - [ ] Show toast notification on success

- [ ] Create Active Filters list display (AC: Multiple filters display)
  - [ ] Create ActiveFiltersList component
  - [ ] Display each filter with:
    * Field name
    * Operator (human-readable label)
    * Value
    * Boolean logic selector (AND/OR/NOT dropdown)
    * [Edit] button
    * [Remove] button
  - [ ] Apply alternating row colors for scannability
  - [ ] Handle empty state (no filters)

- [ ] Implement boolean logic selector (AC: AND/OR/NOT)
  - [ ] Create LogicSelector component
  - [ ] Dropdown with AND, OR, NOT options
  - [ ] Default to AND for first filter
  - [ ] Show logic selector between each filter pair
  - [ ] Update filter logic in store on change
  - [ ] Visual indicator for logic operator

- [ ] Implement Edit filter functionality (AC: [Edit] button)
  - [ ] Open modal with pre-filled field/operator/value
  - [ ] Load existing filter data into form
  - [ ] Update filter on save
  - [ ] Close modal after editing
  - [ ] Show toast notification on success

- [ ] Implement Remove filter functionality (AC: [Remove] button)
  - [ ] Add remove button to each filter row
  - [ ] Dispatch removeFilter action on click
  - [ ] No confirmation needed (quick remove)
  - [ ] Show toast notification on success
  - [ ] Re-index boolean logic after removal

- [ ] Create collapsible sidebar layout (AC: VS Code-inspired)
  - [ ] Create FilterSidebar component
  - [ ] Implement collapse/expand toggle
  - [ ] Persist collapsed state in localStorage
  - [ ] Smooth transition animation (300ms)
  - [ ] Keyboard shortcut: Ctrl/Cmd+B to toggle
  - [ ] Icon indicator (collapse/expand chevron)

- [ ] Implement draft mode and Search button (AC: Postman-style explicit control)
  - [ ] Add draftMode flag to filter store
  - [ ] Display prominent "Search" button when filters exist
  - [ ] Style Search button as primary action (blue, prominent)
  - [ ] Disable Search button when no filters
  - [ ] Clicking Search transitions to "active" mode (next story)
  - [ ] Visual indicator for draft vs active state

- [ ] Apply UX Design Spec styling (AC: Visual design)
  - [ ] Professional modern aesthetic
  - [ ] Data-dense styling with tight spacing
  - [ ] Clear visual hierarchy (labels, values, actions)
  - [ ] Consistent border radius (4px/8px)
  - [ ] Subtle shadows for modal elevation
  - [ ] Dark mode support with proper contrast
  - [ ] Color-coded logic operators (AND=blue, OR=green, NOT=red)

- [ ] Add keyboard navigation (AC: Accessibility)
  - [ ] Tab navigation through all form fields
  - [ ] Enter to add filter in modal
  - [ ] Esc to close modal
  - [ ] Arrow keys for dropdown navigation
  - [ ] Focus management (restore on modal close)
  - [ ] ARIA labels for screen readers

- [ ] Write unit tests (AC: Component testing)
  - [ ] Test FilterBuilder modal open/close
  - [ ] Test field selection updates operators
  - [ ] Test operator selection updates value input type
  - [ ] Test adding filter to store
  - [ ] Test editing existing filter
  - [ ] Test removing filter
  - [ ] Test boolean logic selector
  - [ ] Test draft mode state
  - [ ] Test Search button enable/disable
  - [ ] Test keyboard navigation
  - [ ] Test accessibility (ARIA attributes)
  - [ ] Achieve 80%+ test coverage

- [ ] Integration with LogTable component (AC: UI integration)
  - [ ] Add FilterSidebar to main layout
  - [ ] Position sidebar left of LogTable
  - [ ] Ensure proper spacing and layout
  - [ ] Handle responsive breakpoints
  - [ ] Test with no filters (empty state)
  - [ ] Test with 10+ filters (scrolling)

## Dev Notes

### 🔥 ULTIMATE STORY CONTEXT ENGINE OUTPUT 🔥

This story file has been generated by the comprehensive context analysis engine. It contains EVERYTHING you need to implement Story 2.1 correctly, aligned with architecture, UX design, and project patterns established in Epic 1 stories.

---

### Architecture Compliance & Technical Requirements

#### **Technology Stack (MUST USE EXACT VERSIONS)**

**Frontend - Filter Builder UI:**
- **React 18+** (already installed) - Component framework
- **Zustand 5.0.10** (already installed) - State management for filter store
- **React Hook Form 7.x** (already installed) - Form handling for filter inputs
- **lucide-react** (already installed) - Icons for UI elements
  - ✅ ChevronDown (dropdowns), X (remove), Edit (edit), Search (search button), Filter (filter icon)
- **Tailwind CSS** (already configured) - Styling per UX Design Spec
  - ✅ Data-dense utilities, VS Code-inspired layout
  - ✅ Dark mode support via `dark:` variants
- **react-hot-toast** (already installed) - Toast notifications
- **date-fns 3.x** (already installed) - Date/time picker utilities

**Performance Requirements:**
- Modal open/close: <200ms transition
- Filter add/remove: <50ms
- Dropdown interactions: <100ms
- No UI blocking during filter operations
- Smooth animations (60 FPS)

---

#### **Code Structure & File Organization**

**Frontend Structure (NEW components for Story 2.1):**
```
src/
├── components/
│   ├── filter-builder/
│   │   ├── filter-builder.tsx           # Main modal with 3-step form
│   │   ├── field-selector.tsx           # Field dropdown component
│   │   ├── operator-selector.tsx        # Context-aware operator dropdown
│   │   ├── value-input.tsx              # Context-aware value input
│   │   ├── filter-builder.test.tsx      # Unit tests
│   │   └── index.ts                     # Barrel export
│   ├── filter-sidebar/
│   │   ├── filter-sidebar.tsx           # Collapsible sidebar container
│   │   ├── active-filters-list.tsx      # Display active filters
│   │   ├── filter-row.tsx               # Single filter display with edit/remove
│   │   ├── logic-selector.tsx           # AND/OR/NOT dropdown
│   │   ├── filter-sidebar.test.tsx      # Unit tests
│   │   └── index.ts                     # Barrel export
│   └── log-table/                       # From Epic 1
│       └── log-table.tsx                # MODIFY: Add FilterSidebar integration
├── stores/
│   ├── filter-store.ts                  # Zustand store for filter state (NEW)
│   └── filter-store.test.ts             # Store tests (NEW)
└── types/
    ├── filter.ts                        # Filter type definitions (NEW)
    └── log-entry.ts                     # From Epic 1 (already exists)
```

---

#### **Type Definitions**

**File: src/types/filter.ts (NEW)**

```typescript
export type FieldType =
  | 'timestamp'
  | 'sourceIp'
  | 'destinationIp'
  | 'sourcePort'
  | 'destinationPort'
  | 'protocol'
  | 'action'
  | 'interface'
  | 'ruleLabel';

export type OperatorType =
  // Text operators
  | 'equals'
  | 'contains'
  | 'startsWith'
  | 'endsWith'
  | 'regex'
  // Numeric operators
  | 'greaterThan'
  | 'lessThan'
  | 'between'
  // Select operators
  | 'notEquals'
  // Timestamp operators
  | 'absoluteRange'
  | 'relative';

export type LogicOperator = 'AND' | 'OR' | 'NOT';

export type RelativeTimeRange = '1h' | '6h' | '24h' | '7d' | '30d';

export interface Filter {
  id: string;                          // Unique ID for each filter
  field: FieldType;
  operator: OperatorType;
  value: string | number | [string, string] | RelativeTimeRange; // Union for different value types
  logic?: LogicOperator;               // Logic to next filter (undefined for last filter)
}

export interface FilterState {
  filters: Filter[];
  draftMode: boolean;                  // true = filters not executed, false = active query
  addFilter: (filter: Omit<Filter, 'id'>) => void;
  removeFilter: (id: string) => void;
  updateFilter: (id: string, updates: Partial<Filter>) => void;
  clearFilters: () => void;
  setDraftMode: (draft: boolean) => void;
}
```

---

#### **Zustand Filter Store**

**File: src/stores/filter-store.ts (NEW)**

```typescript
import { create } from 'zustand';
import { Filter, FilterState } from '@/types/filter';
import { v4 as uuidv4 } from 'uuid';

export const useFilterStore = create<FilterState>((set) => ({
  filters: [],
  draftMode: false,

  addFilter: (filter) =>
    set((state) => ({
      filters: [
        ...state.filters,
        { ...filter, id: uuidv4(), logic: filter.logic || 'AND' },
      ],
      draftMode: true, // Adding filter enters draft mode
    })),

  removeFilter: (id) =>
    set((state) => ({
      filters: state.filters.filter((f) => f.id !== id),
      draftMode: state.filters.length > 1, // Stay in draft if filters remain
    })),

  updateFilter: (id, updates) =>
    set((state) => ({
      filters: state.filters.map((f) => (f.id === id ? { ...f, ...updates } : f)),
      draftMode: true,
    })),

  clearFilters: () =>
    set({
      filters: [],
      draftMode: false,
    }),

  setDraftMode: (draft) =>
    set({
      draftMode: draft,
    }),
}));
```

---

#### **FilterBuilder Modal Component**

**File: src/components/filter-builder/filter-builder.tsx**

```typescript
import { useState } from 'react';
import { X } from 'lucide-react';
import { useForm } from 'react-hook-form';
import { FieldSelector } from './field-selector';
import { OperatorSelector } from './operator-selector';
import { ValueInput } from './value-input';
import { useFilterStore } from '@/stores/filter-store';
import toast from 'react-hot-toast';
import type { FieldType, OperatorType } from '@/types/filter';

interface FilterBuilderProps {
  isOpen: boolean;
  onClose: () => void;
  editingFilter?: { id: string; field: FieldType; operator: OperatorType; value: any } | null;
}

interface FormValues {
  field: FieldType;
  operator: OperatorType;
  value: string | number | [string, string];
}

export function FilterBuilder({ isOpen, onClose, editingFilter }: FilterBuilderProps) {
  const { addFilter, updateFilter } = useFilterStore();
  const { register, handleSubmit, watch, setValue, reset, formState: { errors } } = useForm<FormValues>({
    defaultValues: editingFilter || {},
  });

  const selectedField = watch('field');
  const selectedOperator = watch('operator');

  const onSubmit = (data: FormValues) => {
    if (editingFilter) {
      updateFilter(editingFilter.id, {
        field: data.field,
        operator: data.operator,
        value: data.value,
      });
      toast.success('Filter updated');
    } else {
      addFilter({
        field: data.field,
        operator: data.operator,
        value: data.value,
      });
      toast.success('Filter added');
    }

    reset();
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-lg w-full max-w-2xl p-6">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            {editingFilter ? 'Edit Filter' : 'Add Filter'}
          </h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
            aria-label="Close filter builder"
          >
            <X className="w-5 h-5 text-gray-600 dark:text-gray-400" />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          {/* Step 1: Field Selection */}
          <FieldSelector
            value={selectedField}
            onChange={(field) => setValue('field', field)}
            error={errors.field?.message}
          />

          {/* Step 2: Operator Selection (context-aware) */}
          {selectedField && (
            <OperatorSelector
              fieldType={selectedField}
              value={selectedOperator}
              onChange={(operator) => setValue('operator', operator)}
              error={errors.operator?.message}
            />
          )}

          {/* Step 3: Value Input (context-aware) */}
          {selectedField && selectedOperator && (
            <ValueInput
              fieldType={selectedField}
              operatorType={selectedOperator}
              value={watch('value')}
              onChange={(value) => setValue('value', value)}
              error={errors.value?.message}
            />
          )}

          {/* Actions */}
          <div className="flex justify-end gap-3 mt-6">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300
                bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700
                rounded transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!selectedField || !selectedOperator || !watch('value')}
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
```

---

#### **FieldSelector Component**

**File: src/components/filter-builder/field-selector.tsx**

```typescript
import { ChevronDown } from 'lucide-react';
import type { FieldType } from '@/types/filter';

interface FieldSelectorProps {
  value?: FieldType;
  onChange: (field: FieldType) => void;
  error?: string;
}

const FIELD_OPTIONS: { value: FieldType; label: string }[] = [
  { value: 'timestamp', label: 'Timestamp Range' },
  { value: 'sourceIp', label: 'Source IP' },
  { value: 'destinationIp', label: 'Destination IP' },
  { value: 'sourcePort', label: 'Source Port' },
  { value: 'destinationPort', label: 'Destination Port' },
  { value: 'protocol', label: 'Protocol' },
  { value: 'action', label: 'Action' },
  { value: 'interface', label: 'Interface' },
  { value: 'ruleLabel', label: 'Rule Label' },
];

export function FieldSelector({ value, onChange, error }: FieldSelectorProps) {
  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        1. Select Field
      </label>
      <div className="relative">
        <select
          value={value || ''}
          onChange={(e) => onChange(e.target.value as FieldType)}
          className="w-full px-3 py-2 pr-10 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500 focus:border-transparent
            appearance-none cursor-pointer"
        >
          <option value="">Choose a field...</option>
          {FIELD_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <ChevronDown className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500 pointer-events-none" />
      </div>
      {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
    </div>
  );
}
```

---

#### **OperatorSelector Component (Context-Aware)**

**File: src/components/filter-builder/operator-selector.tsx**

```typescript
import { ChevronDown } from 'lucide-react';
import type { FieldType, OperatorType } from '@/types/filter';

interface OperatorSelectorProps {
  fieldType: FieldType;
  value?: OperatorType;
  onChange: (operator: OperatorType) => void;
  error?: string;
}

// Operator options per field type (context-aware)
const OPERATOR_MAP: Record<string, { value: OperatorType; label: string }[]> = {
  text: [
    { value: 'equals', label: 'Equals' },
    { value: 'contains', label: 'Contains' },
    { value: 'startsWith', label: 'Starts With' },
    { value: 'endsWith', label: 'Ends With' },
    { value: 'regex', label: 'Regex' },
  ],
  numeric: [
    { value: 'equals', label: 'Equals' },
    { value: 'greaterThan', label: 'Greater Than' },
    { value: 'lessThan', label: 'Less Than' },
    { value: 'between', label: 'Between' },
  ],
  select: [
    { value: 'equals', label: 'Equals' },
    { value: 'notEquals', label: 'Not Equals' },
  ],
  timestamp: [
    { value: 'absoluteRange', label: 'Absolute Range (Date Picker)' },
    { value: 'relative', label: 'Relative (Last 1h, 24h, etc.)' },
  ],
};

function getOperatorCategory(fieldType: FieldType): keyof typeof OPERATOR_MAP {
  if (fieldType === 'sourcePort' || fieldType === 'destinationPort') return 'numeric';
  if (fieldType === 'protocol' || fieldType === 'action') return 'select';
  if (fieldType === 'timestamp') return 'timestamp';
  return 'text'; // sourceIp, destinationIp, interface, ruleLabel
}

export function OperatorSelector({ fieldType, value, onChange, error }: OperatorSelectorProps) {
  const category = getOperatorCategory(fieldType);
  const operators = OPERATOR_MAP[category];

  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        2. Select Operator
      </label>
      <div className="relative">
        <select
          value={value || ''}
          onChange={(e) => onChange(e.target.value as OperatorType)}
          className="w-full px-3 py-2 pr-10 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500 focus:border-transparent
            appearance-none cursor-pointer"
        >
          <option value="">Choose an operator...</option>
          {operators.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <ChevronDown className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500 pointer-events-none" />
      </div>
      {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
    </div>
  );
}
```

---

#### **ValueInput Component (Context-Aware)**

**File: src/components/filter-builder/value-input.tsx**

```typescript
import type { FieldType, OperatorType, RelativeTimeRange } from '@/types/filter';

interface ValueInputProps {
  fieldType: FieldType;
  operatorType: OperatorType;
  value: any;
  onChange: (value: any) => void;
  error?: string;
}

const PROTOCOL_OPTIONS = ['TCP', 'UDP', 'ICMP', 'IGMP', 'ESP', 'AH', 'GRE'];
const ACTION_OPTIONS = ['pass', 'block', 'reject'];
const RELATIVE_TIME_OPTIONS: { value: RelativeTimeRange; label: string }[] = [
  { value: '1h', label: 'Last 1 hour' },
  { value: '6h', label: 'Last 6 hours' },
  { value: '24h', label: 'Last 24 hours' },
  { value: '7d', label: 'Last 7 days' },
  { value: '30d', label: 'Last 30 days' },
];

export function ValueInput({ fieldType, operatorType, value, onChange, error }: ValueInputProps) {
  // Protocol dropdown
  if (fieldType === 'protocol') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Select Value
        </label>
        <select
          value={value || ''}
          onChange={(e) => onChange(e.target.value)}
          className="w-full px-3 py-2 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500"
        >
          <option value="">Choose protocol...</option>
          {PROTOCOL_OPTIONS.map((protocol) => (
            <option key={protocol} value={protocol}>
              {protocol}
            </option>
          ))}
        </select>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Action dropdown
  if (fieldType === 'action') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Select Value
        </label>
        <select
          value={value || ''}
          onChange={(e) => onChange(e.target.value)}
          className="w-full px-3 py-2 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500"
        >
          <option value="">Choose action...</option>
          {ACTION_OPTIONS.map((action) => (
            <option key={action} value={action}>
              {action}
            </option>
          ))}
        </select>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Relative timestamp
  if (fieldType === 'timestamp' && operatorType === 'relative') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Select Time Range
        </label>
        <select
          value={value || ''}
          onChange={(e) => onChange(e.target.value)}
          className="w-full px-3 py-2 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500"
        >
          <option value="">Choose time range...</option>
          {RELATIVE_TIME_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Absolute timestamp (date picker) - simplified for now
  if (fieldType === 'timestamp' && operatorType === 'absoluteRange') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Enter Date Range
        </label>
        <div className="grid grid-cols-2 gap-2">
          <input
            type="datetime-local"
            value={Array.isArray(value) ? value[0] : ''}
            onChange={(e) => onChange([e.target.value, Array.isArray(value) ? value[1] : ''])}
            className="px-3 py-2 bg-white dark:bg-gray-800
              border border-gray-300 dark:border-gray-700 rounded
              text-gray-900 dark:text-gray-100
              focus:ring-2 focus:ring-blue-500"
            placeholder="Start date"
          />
          <input
            type="datetime-local"
            value={Array.isArray(value) ? value[1] : ''}
            onChange={(e) => onChange([Array.isArray(value) ? value[0] : '', e.target.value])}
            className="px-3 py-2 bg-white dark:bg-gray-800
              border border-gray-300 dark:border-gray-700 rounded
              text-gray-900 dark:text-gray-100
              focus:ring-2 focus:ring-blue-500"
            placeholder="End date"
          />
        </div>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Number input for ports with "between" operator
  if ((fieldType === 'sourcePort' || fieldType === 'destinationPort') && operatorType === 'between') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Enter Port Range
        </label>
        <div className="grid grid-cols-2 gap-2">
          <input
            type="number"
            value={Array.isArray(value) ? value[0] : ''}
            onChange={(e) => onChange([e.target.value, Array.isArray(value) ? value[1] : ''])}
            placeholder="Min port"
            className="px-3 py-2 bg-white dark:bg-gray-800
              border border-gray-300 dark:border-gray-700 rounded
              text-gray-900 dark:text-gray-100
              focus:ring-2 focus:ring-blue-500"
          />
          <input
            type="number"
            value={Array.isArray(value) ? value[1] : ''}
            onChange={(e) => onChange([Array.isArray(value) ? value[0] : '', e.target.value])}
            placeholder="Max port"
            className="px-3 py-2 bg-white dark:bg-gray-800
              border border-gray-300 dark:border-gray-700 rounded
              text-gray-900 dark:text-gray-100
              focus:ring-2 focus:ring-blue-500"
          />
        </div>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Number input for ports
  if (fieldType === 'sourcePort' || fieldType === 'destinationPort') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Enter Port Number
        </label>
        <input
          type="number"
          value={value || ''}
          onChange={(e) => onChange(e.target.value)}
          placeholder="e.g., 443"
          min="0"
          max="65535"
          className="w-full px-3 py-2 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500"
        />
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Text input for IPs, Interface, Rule Label
  const getPlaceholder = () => {
    if (fieldType === 'sourceIp' || fieldType === 'destinationIp') {
      return 'e.g., 192.168.1.1 or 10.0.0.0/24';
    }
    if (fieldType === 'interface') {
      return 'e.g., vtnet0 or LAN';
    }
    if (fieldType === 'ruleLabel') {
      return 'e.g., Block RFC1918';
    }
    if (operatorType === 'regex') {
      return 'e.g., ^192\\.168\\.*';
    }
    return 'Enter value...';
  };

  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        3. Enter Value
      </label>
      <input
        type="text"
        value={value || ''}
        onChange={(e) => onChange(e.target.value)}
        placeholder={getPlaceholder()}
        className="w-full px-3 py-2 bg-white dark:bg-gray-800
          border border-gray-300 dark:border-gray-700 rounded
          text-gray-900 dark:text-gray-100 font-mono text-sm
          focus:ring-2 focus:ring-blue-500"
      />
      {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
    </div>
  );
}
```

---

#### **FilterSidebar with Active Filters List**

**File: src/components/filter-sidebar/filter-sidebar.tsx**

```typescript
import { useState } from 'react';
import { ChevronLeft, ChevronRight, Filter as FilterIcon, Search } from 'lucide-react';
import { useFilterStore } from '@/stores/filter-store';
import { FilterBuilder } from '@/components/filter-builder';
import { ActiveFiltersList } from './active-filters-list';

export function FilterSidebar() {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isBuilderOpen, setIsBuilderOpen] = useState(false);
  const { filters, draftMode, setDraftMode } = useFilterStore();

  const handleSearch = () => {
    setDraftMode(false); // Transition to active mode
    // Query execution will be handled in Story 2.2
  };

  if (isCollapsed) {
    return (
      <div className="w-12 bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 flex flex-col items-center py-4">
        <button
          onClick={() => setIsCollapsed(false)}
          className="p-2 hover:bg-gray-200 dark:hover:bg-gray-800 rounded"
          aria-label="Expand sidebar"
        >
          <ChevronRight className="w-5 h-5 text-gray-600 dark:text-gray-400" />
        </button>
        <FilterIcon className="w-5 h-5 text-gray-600 dark:text-gray-400 mt-4" />
      </div>
    );
  }

  return (
    <>
      <div className="w-80 bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700
        flex flex-col transition-all duration-300">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2">
            <FilterIcon className="w-5 h-5 text-gray-600 dark:text-gray-400" />
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
              Filters
            </h2>
            {draftMode && (
              <span className="px-2 py-0.5 text-xs bg-yellow-100 dark:bg-yellow-900
                text-yellow-800 dark:text-yellow-200 rounded">
                Draft
              </span>
            )}
          </div>
          <button
            onClick={() => setIsCollapsed(true)}
            className="p-1 hover:bg-gray-200 dark:hover:bg-gray-800 rounded"
            aria-label="Collapse sidebar"
          >
            <ChevronLeft className="w-4 h-4 text-gray-600 dark:text-gray-400" />
          </button>
        </div>

        {/* Add Filter Button */}
        <div className="p-4">
          <button
            onClick={() => setIsBuilderOpen(true)}
            className="w-full px-4 py-2 text-sm font-medium text-white bg-blue-600
              hover:bg-blue-700 active:scale-95 rounded transition-all"
          >
            Add Filter
          </button>
        </div>

        {/* Active Filters List */}
        <div className="flex-1 overflow-auto px-4">
          <ActiveFiltersList />
        </div>

        {/* Search Button (Draft Mode) */}
        {filters.length > 0 && draftMode && (
          <div className="p-4 border-t border-gray-200 dark:border-gray-700">
            <button
              onClick={handleSearch}
              className="w-full px-4 py-3 text-sm font-semibold text-white bg-blue-600
                hover:bg-blue-700 active:scale-95 rounded-lg transition-all
                flex items-center justify-center gap-2 shadow-md"
            >
              <Search className="w-4 h-4" />
              Search ({filters.length} filter{filters.length > 1 ? 's' : ''})
            </button>
          </div>
        )}
      </div>

      {/* Filter Builder Modal */}
      <FilterBuilder isOpen={isBuilderOpen} onClose={() => setIsBuilderOpen(false)} />
    </>
  );
}
```

---

#### **ActiveFiltersList Component**

**File: src/components/filter-sidebar/active-filters-list.tsx**

```typescript
import { useFilterStore } from '@/stores/filter-store';
import { FilterRow } from './filter-row';

export function ActiveFiltersList() {
  const { filters, clearFilters } = useFilterStore();

  if (filters.length === 0) {
    return (
      <div className="text-center py-8 text-sm text-gray-500 dark:text-gray-400">
        No filters added yet.
        <br />
        Click "Add Filter" to start.
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase">
          Active Filters ({filters.length})
        </h3>
        <button
          onClick={clearFilters}
          className="text-xs text-red-600 dark:text-red-400 hover:underline"
        >
          Clear All
        </button>
      </div>

      {filters.map((filter, index) => (
        <FilterRow key={filter.id} filter={filter} showLogic={index < filters.length - 1} />
      ))}
    </div>
  );
}
```

---

#### **FilterRow Component**

**File: src/components/filter-sidebar/filter-row.tsx**

```typescript
import { Edit, X } from 'lucide-react';
import { useFilterStore } from '@/stores/filter-store';
import { LogicSelector } from './logic-selector';
import type { Filter } from '@/types/filter';
import toast from 'react-hot-toast';

interface FilterRowProps {
  filter: Filter;
  showLogic: boolean;
}

const FIELD_LABELS: Record<string, string> = {
  timestamp: 'Timestamp',
  sourceIp: 'Source IP',
  destinationIp: 'Dest IP',
  sourcePort: 'Source Port',
  destinationPort: 'Dest Port',
  protocol: 'Protocol',
  action: 'Action',
  interface: 'Interface',
  ruleLabel: 'Rule',
};

const OPERATOR_LABELS: Record<string, string> = {
  equals: '=',
  contains: 'contains',
  startsWith: 'starts with',
  endsWith: 'ends with',
  regex: 'regex',
  greaterThan: '>',
  lessThan: '<',
  between: 'between',
  notEquals: '≠',
  absoluteRange: 'range',
  relative: 'last',
};

export function FilterRow({ filter, showLogic }: FilterRowProps) {
  const { removeFilter, updateFilter } = useFilterStore();

  const handleRemove = () => {
    removeFilter(filter.id);
    toast.success('Filter removed');
  };

  const handleEdit = () => {
    // Open FilterBuilder with editing mode (implementation in parent)
    toast.info('Edit functionality - opens modal');
  };

  const formatValue = (value: any): string => {
    if (Array.isArray(value)) {
      return `${value[0]} - ${value[1]}`;
    }
    return String(value);
  };

  return (
    <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700
      rounded p-3 space-y-2">
      {/* Filter Display */}
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          <div className="text-xs font-medium text-gray-600 dark:text-gray-400">
            {FIELD_LABELS[filter.field] || filter.field}
          </div>
          <div className="text-sm text-gray-900 dark:text-gray-100 mt-0.5">
            <span className="text-gray-500 dark:text-gray-400 mr-1">
              {OPERATOR_LABELS[filter.operator]}
            </span>
            <span className="font-mono">{formatValue(filter.value)}</span>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1">
          <button
            onClick={handleEdit}
            className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
            aria-label="Edit filter"
          >
            <Edit className="w-3 h-3 text-gray-600 dark:text-gray-400" />
          </button>
          <button
            onClick={handleRemove}
            className="p-1 hover:bg-red-100 dark:hover:bg-red-900 rounded"
            aria-label="Remove filter"
          >
            <X className="w-3 h-3 text-red-600 dark:text-red-400" />
          </button>
        </div>
      </div>

      {/* Logic Selector */}
      {showLogic && (
        <LogicSelector
          value={filter.logic || 'AND'}
          onChange={(logic) => updateFilter(filter.id, { logic })}
        />
      )}
    </div>
  );
}
```

---

#### **LogicSelector Component**

**File: src/components/filter-sidebar/logic-selector.tsx**

```typescript
import type { LogicOperator } from '@/types/filter';

interface LogicSelectorProps {
  value: LogicOperator;
  onChange: (logic: LogicOperator) => void;
}

const LOGIC_OPTIONS: { value: LogicOperator; label: string; color: string }[] = [
  { value: 'AND', label: 'AND', color: 'bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200' },
  { value: 'OR', label: 'OR', color: 'bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200' },
  { value: 'NOT', label: 'NOT', color: 'bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200' },
];

export function LogicSelector({ value, onChange }: LogicSelectorProps) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-gray-500 dark:text-gray-400">Next filter:</span>
      <div className="flex gap-1">
        {LOGIC_OPTIONS.map((option) => (
          <button
            key={option.value}
            onClick={() => onChange(option.value)}
            className={`px-2 py-1 text-xs font-semibold rounded transition-all ${
              value === option.value
                ? option.color
                : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600'
            }`}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}
```

---

### Previous Story Intelligence (Epic 1 Learnings)

**What Works Well from Epic 1:**
- ✅ Component composition pattern (main + sub-components)
- ✅ Tailwind CSS styling with dark mode support
- ✅ TypeScript strict typing with interfaces
- ✅ Zustand for state management (lightweight, performant)
- ✅ React.memo for performance optimization
- ✅ Unit testing with Vitest + React Testing Library
- ✅ Toast notifications for user feedback (react-hot-toast)
- ✅ Keyboard navigation and accessibility (ARIA attributes)
- ✅ Lucide-react icons for visual indicators

**What to Integrate from Epic 1:**
- ✅ Reuse LogEntry type from src/types/log-entry.ts
- ✅ Integrate FilterSidebar with main App layout
- ✅ Follow established component structure (components/{feature}/)
- ✅ Use same testing approach (*.test.tsx files)
- ✅ Apply consistent Tailwind styling patterns
- ✅ Dark mode support via `dark:` variants
- ✅ Monospace fonts for technical data (IPs, ports)

**Code Patterns to Follow:**
- ✅ Component separation (main component + sub-components in same folder)
- ✅ Barrel exports via index.ts
- ✅ Co-located tests (*.test.tsx)
- ✅ Use lucide-react for icons
- ✅ Toast notifications for user feedback
- ✅ ARIA attributes for accessibility
- ✅ Focus management for keyboard navigation

---

### Git Intelligence Summary

**Recent Commits (Relevant to Story 2.1):**

From `git log --oneline -5`:
1. `6c21512` - feat: add entry detail view with copy functionality (Story 1.6)
2. `7d317ce` - Remove bmad_automated from repo, add to gitignore
3. `d1587bf` - Code Review Fix: Story 1.4 Backend/Frontend type mismatch
4. `0e4c080` - Complete Story 1.5: Log Entry Display Table with Virtual Scrolling
5. `05b3c1f` - Story 1.4: Code formatting and dependencies update

**Epic 1 Implementation Patterns:**
- ✅ All frontend components use Tailwind CSS with dark mode
- ✅ TypeScript strict mode with explicit types
- ✅ Component co-location (component + tests in same folder)
- ✅ Lucide-react for consistent icons
- ✅ react-hot-toast for all user notifications

**Dependencies Already Available:**
- ✅ `react` 18+
- ✅ `zustand` 5.0.10
- ✅ `react-hook-form` 7.x
- ✅ `tailwindcss` configured with dark mode
- ✅ `lucide-react` for icons
- ✅ `react-hot-toast` for toast notifications
- ✅ `date-fns` for date utilities
- ✅ `vitest` + `@testing-library/react` for testing

**Dependencies to ADD for Story 2.1:**
- ⚠️ `uuid` (for generating unique filter IDs)

```bash
npm install uuid
npm install -D @types/uuid
```

**Integration Points:**
- Story 1.5 provides LogTable for main display area
- Story 2.1 adds FilterSidebar to left of LogTable
- Story 2.2 will connect filters to query execution engine
- Foundation for Stories 2.3-2.4 (filter management, search history)

---

### UX Design Spec Integration

**VS Code-Inspired Layout Requirements:**
- ✅ Collapsible sidebar on left (FilterSidebar)
- ✅ Main content area on right (LogTable from Story 1.5)
- ✅ Sidebar can be toggled with button or keyboard shortcut
- ✅ Smooth transition animation (300ms)
- ✅ Sidebar width: 320px (80 in Tailwind)
- ✅ Collapsed width: 48px (12 in Tailwind)

**Postman-Style Explicit Control:**
- ✅ Filters accumulate in "draft mode"
- ✅ Prominent "Search" button to execute query
- ✅ No automatic execution when adding/removing filters
- ✅ Clear visual distinction between draft and active states

**Data-Dense Styling Requirements:**
- ✅ Tight line-height: `leading-tight` class
- ✅ Tight spacing: `py-1 px-2` classes
- ✅ Dark mode support: `dark:` variants
- ✅ Hover highlights: `hover:bg-gray-100 dark:hover:bg-gray-700`
- ✅ Monospace fonts for technical values (IPs, ports)

**Visual Design Requirements:**
- ✅ Professional Modern Aesthetic: Clean, modern design
- ✅ Clear visual hierarchy through size/weight/color contrast
- ✅ Consistent design tokens: 4px or 8px border radius
- ✅ Color-coded logic operators (AND=blue, OR=green, NOT=red)

**Interaction Patterns:**
- ✅ Clear visual indication of filter actions (add, edit, remove)
- ✅ Toast notifications for feedback
- ✅ Keyboard navigation throughout (Tab, Enter, Esc)
- ✅ Smooth transitions for UI changes

**Accessibility Requirements:**
- ✅ ARIA labels on all interactive components
- ✅ Keyboard navigation for all controls
- ✅ Focus indicators visible in both themes
- ✅ Screen reader compatibility

---

### Testing Strategy

#### **Unit Tests (Vitest + React Testing Library)**

**Coverage Requirements:**
- FilterBuilder modal rendering
- Field selection updates operators correctly
- Operator selection updates value input type
- Value input renders correct type per field
- Adding filter to store
- Editing existing filter
- Removing filter
- Boolean logic selector
- Draft mode state transitions
- Search button enable/disable
- Keyboard navigation
- Accessibility (ARIA attributes)
- Target: 80%+ coverage

**Test Structure:**
```typescript
describe('FilterBuilder', () => {
  it('opens modal when Add Filter clicked', () => {
    render(<FilterSidebar />);
    fireEvent.click(screen.getByText('Add Filter'));
    expect(screen.getByText('Add Filter')).toBeInTheDocument();
  });

  it('updates operators when field changes', () => {
    render(<FilterBuilder isOpen={true} onClose={jest.fn()} />);

    // Select IP field (text operators)
    fireEvent.change(screen.getByLabelText('1. Select Field'), {
      target: { value: 'sourceIp' },
    });
    expect(screen.getByText('Contains')).toBeInTheDocument();

    // Select Port field (numeric operators)
    fireEvent.change(screen.getByLabelText('1. Select Field'), {
      target: { value: 'sourcePort' },
    });
    expect(screen.getByText('Greater Than')).toBeInTheDocument();
  });

  it('adds filter to store on submit', () => {
    const { result } = renderHook(() => useFilterStore());

    render(<FilterBuilder isOpen={true} onClose={jest.fn()} />);

    // Fill form
    fireEvent.change(screen.getByLabelText('1. Select Field'), {
      target: { value: 'action' },
    });
    fireEvent.change(screen.getByLabelText('2. Select Operator'), {
      target: { value: 'equals' },
    });
    fireEvent.change(screen.getByLabelText('3. Select Value'), {
      target: { value: 'block' },
    });

    fireEvent.click(screen.getByText('Add Filter'));

    expect(result.current.filters).toHaveLength(1);
    expect(result.current.filters[0].field).toBe('action');
  });
});
```

#### **Integration Tests**

**E2E Workflow:**
1. Open FilterSidebar
2. Click "Add Filter"
3. Select field, operator, value
4. Submit filter
5. Verify filter appears in Active Filters list
6. Edit filter
7. Verify updated values
8. Remove filter
9. Verify filter removed from list

---

### Performance Benchmarking

**Target Metrics:**
- Modal open/close: <200ms transition
- Filter add/remove: <50ms
- Dropdown interactions: <100ms
- Sidebar collapse/expand: <300ms

**Measurement Approach:**
1. **React DevTools Profiler**: Measure component render times
2. **Chrome Performance Tab**: Record interaction performance
3. **Console Timing**: Measure state updates
4. **Visual Inspection**: Check for animation stuttering

**Acceptance Criteria:**
- Transitions ≤300ms
- State updates ≤50ms
- No visual stuttering or lag

---

### Project Context Reference

**Critical Rules from Established Patterns:**

**Component Structure:**
- ✅ Feature-based folders: `components/{feature}/`
- ✅ Barrel exports via `index.ts`
- ✅ Co-located tests: `*.test.tsx`
- ✅ Sub-components in same folder

**TypeScript:**
- ✅ Strict mode enabled
- ✅ Interface over type for objects
- ✅ camelCase for properties
- ✅ Explicit return types for functions

**Zustand State Management:**
- ✅ Selective subscriptions (avoid full store subscription)
- ✅ Immutable updates with spread operators
- ✅ Simple actions for state mutations

**Tailwind CSS:**
- ✅ Utility-first approach
- ✅ Dark mode via `dark:` variants
- ✅ Responsive via breakpoint prefixes
- ✅ Custom design tokens in tailwind.config.js

**Accessibility:**
- ✅ ARIA labels on interactive elements
- ✅ Keyboard navigation support
- ✅ Screen reader compatibility
- ✅ Focus management

---

### Story Completion Status

**Status:** ready-for-dev

**Next Steps:**
1. Review this story file thoroughly - contains ALL context needed
2. Install `uuid` dependency
3. Create filter type definitions (src/types/filter.ts)
4. Create Zustand filter store (src/stores/filter-store.ts)
5. Create FilterBuilder modal with 3-step form
6. Create FieldSelector component
7. Create OperatorSelector component (context-aware)
8. Create ValueInput component (context-aware)
9. Create FilterSidebar with collapsible layout
10. Create ActiveFiltersList component
11. Create FilterRow component with edit/remove
12. Create LogicSelector component (AND/OR/NOT)
13. Implement draft mode and Search button
14. Apply UX Design Spec styling
15. Add keyboard accessibility
16. Write comprehensive unit tests
17. Integration with main App layout
18. Verify NFR compliance (performance targets)
19. Commit with format: `Complete Story 2.1: Visual Filter Builder UI`

**Blocking Dependencies:**
- Epic 1 (Stories 1.1-1.6) ✅ DONE

**Blocked Stories:**
- Story 2.2 (Query Execution Engine) - Needs filter UI to provide query input
- Story 2.3 (Filter Management) - Needs basic filter UI
- Story 2.4 (Search History) - Needs filter execution

---

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4.5 (claude-sonnet-4-5-20250929)

### Debug Log References

N/A - Story created via create-story workflow (2026-01-18)

### Completion Notes List

**Implementation Date:** TBD

### File List

**Files to Create:**
- src/types/filter.ts
- src/stores/filter-store.ts
- src/stores/filter-store.test.ts
- src/components/filter-builder/filter-builder.tsx
- src/components/filter-builder/field-selector.tsx
- src/components/filter-builder/operator-selector.tsx
- src/components/filter-builder/value-input.tsx
- src/components/filter-builder/filter-builder.test.tsx
- src/components/filter-builder/index.ts
- src/components/filter-sidebar/filter-sidebar.tsx
- src/components/filter-sidebar/active-filters-list.tsx
- src/components/filter-sidebar/filter-row.tsx
- src/components/filter-sidebar/logic-selector.tsx
- src/components/filter-sidebar/filter-sidebar.test.tsx
- src/components/filter-sidebar/index.ts

**Files to Modify:**
- src/App.tsx (integrate FilterSidebar with LogTable layout)
- package.json (add uuid dependency)

---
