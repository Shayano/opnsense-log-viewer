import type { Filter } from './filter';

// Maximum number of search history entries to keep
export const MAX_SEARCH_HISTORY = 10;

// Search history entry captures executed search details
export interface SearchHistory {
  id: string;                          // Unique ID for history entry
  timestamp: number;                   // When the search was executed (milliseconds since epoch)
  filters: Omit<Filter, 'id'>[];      // Filter configurations (without runtime IDs)
  resultCount: number;                 // Number of matches found (X)
  totalCount: number;                  // Total entries in dataset (Y)
  executionTimeMs: number;             // Query execution time in milliseconds
  sourceFileHash?: string;             // SHA-256 hash of source file (optional, for tracking)
}

// Store state interface
export interface SearchHistoryState {
  searchHistory: SearchHistory[];

  // Actions
  addSearchToHistory: (entry: Omit<SearchHistory, 'id' | 'timestamp'>) => void;
  deleteHistoryEntry: (id: string) => void;
  clearAllHistory: () => void;
}
