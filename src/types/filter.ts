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
