import { invoke } from '@tauri-apps/api/core';
import type { QueryRequest, QueryResult } from '@/types/query';
import type { Filter } from '@/types/filter';

/**
 * Transform Filter[] from store to QueryRequest for backend
 */
export function transformFiltersToQuery(
  filters: Filter[],
  indexHash: string
): QueryRequest {
  return {
    filters: filters.map((filter) => ({
      field: filter.field,
      operator: filter.operator,
      value: filter.value,
      logic: filter.logic,
    })),
    indexHash,
  };
}

/**
 * Execute query via Tauri IPC
 */
export async function executeQuery(
  filters: Filter[],
  indexHash: string
): Promise<QueryResult> {
  const request = transformFiltersToQuery(filters, indexHash);

  try {
    const result = await invoke<QueryResult>('execute_query', { request });
    return result;
  } catch (error) {
    throw new Error(`Query execution failed: ${error}`);
  }
}
