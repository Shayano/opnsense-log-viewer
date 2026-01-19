import { create } from 'zustand';
import type { QueryResult } from '@/types/query';
import type { LogEntry } from '@/types/log-entry';

interface QueryState {
  // Query results
  currentResult: QueryResult | null;
  currentEntries: LogEntry[] | null;
  entriesLoading: boolean;
  isExecuting: boolean;
  error: string | null;

  // Actions
  setResult: (result: QueryResult) => void;
  setCurrentEntries: (entries: LogEntry[] | null) => void;
  setEntriesLoading: (loading: boolean) => void;
  setExecuting: (executing: boolean) => void;
  setError: (error: string | null) => void;
  clearResult: () => void;
}

export const useQueryStore = create<QueryState>((set) => ({
  currentResult: null,
  currentEntries: null,
  entriesLoading: false,
  isExecuting: false,
  error: null,

  setResult: (result) => {
    console.log('[MEM] query-store setResult', {
      entryIdsCount: result?.entryIds?.length ?? 0,
      matchedCount: result?.matchedCount,
      totalCount: result?.totalCount,
    });
    set({
      currentResult: result,
      isExecuting: false,
      error: null,
    });
  },

  setCurrentEntries: (entries) => {
    console.log('[MEM] query-store setCurrentEntries', {
      length: entries?.length ?? 0,
      isNull: entries == null,
    });
    set({ currentEntries: entries });
  },

  setEntriesLoading: (loading) => set({ entriesLoading: loading }),

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

  clearResult: () => {
    console.log('[MEM] query-store clearResult');
    set({
      currentResult: null,
      currentEntries: null,
      error: null,
    });
  },
}));
