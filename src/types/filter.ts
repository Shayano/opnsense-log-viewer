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

// NEW for Story 2.3 - Saved filter definition
export interface SavedFilter {
  id: string;                          // Unique ID for saved filter
  name: string;                        // User-provided name (e.g., "Nightly Port 443 Blocks")
  filters: Omit<Filter, 'id'>[];      // Filter configurations (without runtime IDs)
  timestamp: number;                   // Creation timestamp (for FIFO eviction)
  version?: number;                    // Data format version (for migration) - optional for backward compatibility
}

export interface FilterState {
  // Active filters (from Story 2.1)
  filters: Filter[];
  draftMode: boolean;                  // true = filters not executed, false = active query

  // Saved filters (NEW for Story 2.3)
  savedFilters: SavedFilter[];

  // Actions - Active filters (from Story 2.1)
  addFilter: (filter: Omit<Filter, 'id'>) => void;
  removeFilter: (id: string) => void;
  updateFilter: (id: string, updates: Partial<Filter>) => void;
  clearFilters: () => void;
  setDraftMode: (draft: boolean) => void;

  // Actions - Saved filters (NEW for Story 2.3)
  saveFilter: (name: string) => void;
  loadFilter: (savedFilterId: string) => void;
  deleteSavedFilter: (savedFilterId: string) => void;
}
