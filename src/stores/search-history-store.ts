import { create } from 'zustand';
import { persist, createJSONStorage, StateStorage } from 'zustand/middleware';
import { v4 as uuidv4 } from 'uuid';
import toast from 'react-hot-toast';
import type { SearchHistory, SearchHistoryState } from '@/types/search-history';
import { MAX_SEARCH_HISTORY } from '@/types/search-history';

// Custom storage with error handling (same pattern as filter-store)
const createCustomStorage = (): StateStorage => ({
  getItem: (name: string) => {
    try {
      const value = localStorage.getItem(name);
      return value;
    } catch (error) {
      console.error('Failed to read from localStorage:', error);
      return null;
    }
  },
  setItem: (name: string, value: string) => {
    try {
      localStorage.setItem(name, value);
    } catch (error) {
      if (error instanceof DOMException && error.name === 'QuotaExceededError') {
        toast.error('Storage full. Clear old search history to make room.');
        throw error;
      } else {
        console.error('Failed to write to localStorage:', error);
        throw error;
      }
    }
  },
  removeItem: (name: string) => {
    try {
      localStorage.removeItem(name);
    } catch (error) {
      console.error('Failed to remove from localStorage:', error);
    }
  },
});

export const useSearchHistoryStore = create<SearchHistoryState>()(
  persist(
    (set) => ({
      // Search history array (most recent last)
      searchHistory: [],

      // Add search to history with FIFO eviction
      addSearchToHistory: (entry) =>
        set((state) => {
          const historyEntry: SearchHistory = {
            id: uuidv4(),
            timestamp: Date.now(),
            ...entry,
          };

          let updatedHistory = [...state.searchHistory, historyEntry];

          // Enforce 10 entry limit (FIFO eviction - silent, no notification)
          if (updatedHistory.length > MAX_SEARCH_HISTORY) {
            // Remove oldest entry (first in array)
            updatedHistory = updatedHistory.slice(1);
          }

          return {
            searchHistory: updatedHistory,
          };
        }),

      // Delete specific history entry
      deleteHistoryEntry: (id) =>
        set((state) => ({
          searchHistory: state.searchHistory.filter((entry) => entry.id !== id),
        })),

      // Clear all history
      clearAllHistory: () =>
        set({
          searchHistory: [],
        }),
    }),
    {
      name: 'opnsense-log-viewer-search-history', // localStorage key
      storage: createJSONStorage(() => createCustomStorage()),
    }
  )
);
