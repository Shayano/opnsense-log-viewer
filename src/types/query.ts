// Mirror Rust types for TypeScript

export interface QueryRequest {
  filters: FilterCondition[];
  indexHash: string;
}

export interface FilterCondition {
  field: FilterField;
  operator: FilterOperator;
  value: FilterValue;
  logic?: LogicOperator;
}

export type FilterField =
  | 'timestamp'
  | 'sourceIp'
  | 'destinationIp'
  | 'sourcePort'
  | 'destinationPort'
  | 'protocol'
  | 'action'
  | 'interface'
  | 'ruleLabel';

export type FilterOperator =
  | 'equals'
  | 'contains'
  | 'startsWith'
  | 'endsWith'
  | 'regex'
  | 'greaterThan'
  | 'lessThan'
  | 'between'
  | 'notEquals'
  | 'absoluteRange'
  | 'relative';

export type FilterValue = string | number | [string, string];

export type LogicOperator = 'AND' | 'OR' | 'NOT';

export interface QueryResult {
  entryIds: number[];
  totalCount: number;
  matchedCount: number;
  executionTimeMs: number;
}
