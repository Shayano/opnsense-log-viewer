import { create } from 'zustand';
import type { QueryResult } from '@/types/query';

interface QueryState {
  // Query results
  currentResult: QueryResult | null;
  isExecuting: boolean;
  error: string | null;

  // Actions
  setResult: (result: QueryResult) => void;
  setExecuting: (executing: boolean) => void;
  setError: (error: string | null) => void;
  clearResult: () => void;
}

export const useQueryStore = create<QueryState>((set) => ({
  currentResult: null,
  isExecuting: false,
  error: null,

  setResult: (result) =>
    set({
      currentResult: result,
      isExecuting: false,
      error: null,
    }),

  setExecuting: (executing) =>
    set({
      isExecuting: executing,
      error: null,
    }),

  setError: (error) =>
    set({
      error,
      isExecuting: false,
    }),

  clearResult: () =>
    set({
      currentResult: null,
      error: null,
    }),
}));
