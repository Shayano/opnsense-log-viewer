import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useSearchHistoryStore } from './search-history-store';
import { MAX_SEARCH_HISTORY } from '@/types/search-history';

describe('useSearchHistoryStore', () => {
  beforeEach(() => {
    // Reset store before each test
    useSearchHistoryStore.setState({
      searchHistory: [],
    });

    // Clear localStorage
    localStorage.clear();
  });

  describe('addSearchToHistory', () => {
    it('should add search to history with generated ID and timestamp', () => {
      const { addSearchToHistory } = useSearchHistoryStore.getState();

      const mockEntry = {
        filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'block' }],
        resultCount: 100,
        totalCount: 1000,
        executionTimeMs: 250,
      };

      addSearchToHistory(mockEntry);

      const state = useSearchHistoryStore.getState();
      expect(state.searchHistory).toHaveLength(1);
      expect(state.searchHistory[0]).toMatchObject(mockEntry);
      expect(state.searchHistory[0].id).toBeDefined();
      expect(typeof state.searchHistory[0].id).toBe('string');
      expect(state.searchHistory[0].timestamp).toBeDefined();
      expect(typeof state.searchHistory[0].timestamp).toBe('number');
    });

    it('should add sourceFileHash if provided', () => {
      const { addSearchToHistory } = useSearchHistoryStore.getState();

      const mockEntry = {
        filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'block' }],
        resultCount: 50,
        totalCount: 500,
        executionTimeMs: 120,
        sourceFileHash: 'abc123def456',
      };

      addSearchToHistory(mockEntry);

      const state = useSearchHistoryStore.getState();
      expect(state.searchHistory[0].sourceFileHash).toBe('abc123def456');
    });

    it('should handle sourceFileHash as undefined', () => {
      const { addSearchToHistory } = useSearchHistoryStore.getState();

      const mockEntry = {
        filters: [{ field: 'protocol' as const, operator: 'equals' as const, value: 'tcp' }],
        resultCount: 200,
        totalCount: 2000,
        executionTimeMs: 350,
        sourceFileHash: undefined,
      };

      addSearchToHistory(mockEntry);

      const state = useSearchHistoryStore.getState();
      expect(state.searchHistory[0].sourceFileHash).toBeUndefined();
    });

    it('should add multiple searches in chronological order', () => {
      const { addSearchToHistory } = useSearchHistoryStore.getState();

      addSearchToHistory({
        filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'block' }],
        resultCount: 100,
        totalCount: 1000,
        executionTimeMs: 250,
      });

      addSearchToHistory({
        filters: [{ field: 'protocol' as const, operator: 'equals' as const, value: 'tcp' }],
        resultCount: 200,
        totalCount: 2000,
        executionTimeMs: 300,
      });

      addSearchToHistory({
        filters: [{ field: 'sourceIp' as const, operator: 'contains' as const, value: '192.168' }],
        resultCount: 50,
        totalCount: 1000,
        executionTimeMs: 150,
      });

      const state = useSearchHistoryStore.getState();
      expect(state.searchHistory).toHaveLength(3);
      expect(state.searchHistory[0].filters[0].field).toBe('action');
      expect(state.searchHistory[1].filters[0].field).toBe('protocol');
      expect(state.searchHistory[2].filters[0].field).toBe('sourceIp');
    });

    it('should strip filter runtime IDs when storing', () => {
      const { addSearchToHistory } = useSearchHistoryStore.getState();

      addSearchToHistory({
        filters: [
          { field: 'action' as const, operator: 'equals' as const, value: 'block', logic: 'AND' as const },
          { field: 'destinationPort' as const, operator: 'equals' as const, value: 443 },
        ],
        resultCount: 100,
        totalCount: 1000,
        executionTimeMs: 250,
      });

      const state = useSearchHistoryStore.getState();
      expect(state.searchHistory[0].filters[0]).not.toHaveProperty('id');
      expect(state.searchHistory[0].filters[1]).not.toHaveProperty('id');
    });

    describe('FIFO eviction', () => {
      it('should enforce MAX_SEARCH_HISTORY limit (10 entries)', () => {
        const { addSearchToHistory } = useSearchHistoryStore.getState();

        // Add 11 searches (exceeds limit of 10)
        for (let i = 0; i < 11; i++) {
          addSearchToHistory({
            filters: [{ field: 'sourcePort' as const, operator: 'equals' as const, value: i }],
            resultCount: i * 10,
            totalCount: 1000,
            executionTimeMs: 100,
          });
        }

        const state = useSearchHistoryStore.getState();
        expect(state.searchHistory).toHaveLength(MAX_SEARCH_HISTORY);
        expect(state.searchHistory).toHaveLength(10);
      });

      it('should remove oldest entry when exceeding limit', () => {
        const { addSearchToHistory } = useSearchHistoryStore.getState();

        // Add 10 searches (max capacity)
        for (let i = 0; i < 10; i++) {
          addSearchToHistory({
            filters: [{ field: 'sourcePort' as const, operator: 'equals' as const, value: i }],
            resultCount: i * 10,
            totalCount: 1000,
            executionTimeMs: 100,
          });
        }

        const stateBefore = useSearchHistoryStore.getState();
        const oldestEntry = stateBefore.searchHistory[0];

        // Add 11th search (should evict oldest)
        addSearchToHistory({
          filters: [{ field: 'sourcePort' as const, operator: 'equals' as const, value: 999 }],
          resultCount: 500,
          totalCount: 1000,
          executionTimeMs: 200,
        });

        const stateAfter = useSearchHistoryStore.getState();
        expect(stateAfter.searchHistory).toHaveLength(10);
        expect(stateAfter.searchHistory.find((e) => e.id === oldestEntry.id)).toBeUndefined();
        expect(stateAfter.searchHistory[9].filters[0].value).toBe(999);
      });

      it('should handle multiple evictions correctly', () => {
        const { addSearchToHistory } = useSearchHistoryStore.getState();

        // Add 15 searches (exceeds limit by 5)
        for (let i = 0; i < 15; i++) {
          addSearchToHistory({
            filters: [{ field: 'sourcePort' as const, operator: 'equals' as const, value: i }],
            resultCount: i * 10,
            totalCount: 1000,
            executionTimeMs: 100,
          });
        }

        const state = useSearchHistoryStore.getState();
        expect(state.searchHistory).toHaveLength(10);
        // Should contain searches 5-14 (oldest 0-4 evicted)
        expect(state.searchHistory[0].filters[0].value).toBe(5);
        expect(state.searchHistory[9].filters[0].value).toBe(14);
      });

      it('should silently evict without errors', () => {
        const { addSearchToHistory } = useSearchHistoryStore.getState();

        // This should not throw any errors
        expect(() => {
          for (let i = 0; i < 20; i++) {
            addSearchToHistory({
              filters: [{ field: 'action' as const, operator: 'equals' as const, value: `value-${i}` }],
              resultCount: 100,
              totalCount: 1000,
              executionTimeMs: 150,
            });
          }
        }).not.toThrow();

        const state = useSearchHistoryStore.getState();
        expect(state.searchHistory).toHaveLength(10);
      });
    });
  });

  describe('deleteHistoryEntry', () => {
    it('should delete entry by ID', () => {
      const { addSearchToHistory, deleteHistoryEntry } = useSearchHistoryStore.getState();

      addSearchToHistory({
        filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'block' }],
        resultCount: 100,
        totalCount: 1000,
        executionTimeMs: 250,
      });

      addSearchToHistory({
        filters: [{ field: 'protocol' as const, operator: 'equals' as const, value: 'tcp' }],
        resultCount: 200,
        totalCount: 2000,
        executionTimeMs: 300,
      });

      const stateBefore = useSearchHistoryStore.getState();
      const entryToDelete = stateBefore.searchHistory[0];

      deleteHistoryEntry(entryToDelete.id);

      const stateAfter = useSearchHistoryStore.getState();
      expect(stateAfter.searchHistory).toHaveLength(1);
      expect(stateAfter.searchHistory.find((e) => e.id === entryToDelete.id)).toBeUndefined();
      expect(stateAfter.searchHistory[0].filters[0].field).toBe('protocol');
    });

    it('should handle deleting non-existent entry gracefully', () => {
      const { addSearchToHistory, deleteHistoryEntry } = useSearchHistoryStore.getState();

      addSearchToHistory({
        filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'block' }],
        resultCount: 100,
        totalCount: 1000,
        executionTimeMs: 250,
      });

      const stateBefore = useSearchHistoryStore.getState();
      expect(stateBefore.searchHistory).toHaveLength(1);

      deleteHistoryEntry('non-existent-id');

      const stateAfter = useSearchHistoryStore.getState();
      expect(stateAfter.searchHistory).toHaveLength(1);
    });

    it('should delete from middle of history list', () => {
      const { addSearchToHistory, deleteHistoryEntry } = useSearchHistoryStore.getState();

      addSearchToHistory({
        filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'block' }],
        resultCount: 100,
        totalCount: 1000,
        executionTimeMs: 250,
      });

      addSearchToHistory({
        filters: [{ field: 'protocol' as const, operator: 'equals' as const, value: 'tcp' }],
        resultCount: 200,
        totalCount: 2000,
        executionTimeMs: 300,
      });

      addSearchToHistory({
        filters: [{ field: 'sourceIp' as const, operator: 'contains' as const, value: '192.168' }],
        resultCount: 50,
        totalCount: 1000,
        executionTimeMs: 150,
      });

      const stateBefore = useSearchHistoryStore.getState();
      const middleEntry = stateBefore.searchHistory[1];

      deleteHistoryEntry(middleEntry.id);

      const stateAfter = useSearchHistoryStore.getState();
      expect(stateAfter.searchHistory).toHaveLength(2);
      expect(stateAfter.searchHistory[0].filters[0].field).toBe('action');
      expect(stateAfter.searchHistory[1].filters[0].field).toBe('sourceIp');
    });
  });

  describe('clearAllHistory', () => {
    it('should clear all history entries', () => {
      const { addSearchToHistory, clearAllHistory } = useSearchHistoryStore.getState();

      // Add multiple searches
      for (let i = 0; i < 5; i++) {
        addSearchToHistory({
          filters: [{ field: 'sourcePort' as const, operator: 'equals' as const, value: i }],
          resultCount: i * 10,
          totalCount: 1000,
          executionTimeMs: 100,
        });
      }

      const stateBefore = useSearchHistoryStore.getState();
      expect(stateBefore.searchHistory).toHaveLength(5);

      clearAllHistory();

      const stateAfter = useSearchHistoryStore.getState();
      expect(stateAfter.searchHistory).toHaveLength(0);
      expect(stateAfter.searchHistory).toEqual([]);
    });

    it('should clear empty history without errors', () => {
      const { clearAllHistory } = useSearchHistoryStore.getState();

      expect(() => {
        clearAllHistory();
      }).not.toThrow();

      const state = useSearchHistoryStore.getState();
      expect(state.searchHistory).toEqual([]);
    });
  });

  describe('localStorage persistence', () => {
    it('should persist history to localStorage', () => {
      const { addSearchToHistory } = useSearchHistoryStore.getState();

      addSearchToHistory({
        filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'block' }],
        resultCount: 100,
        totalCount: 1000,
        executionTimeMs: 250,
      });

      // Check localStorage
      const stored = localStorage.getItem('opnsense-log-viewer-search-history');
      expect(stored).not.toBeNull();

      const parsed = JSON.parse(stored!);
      expect(parsed.state.searchHistory).toHaveLength(1);
      expect(parsed.state.searchHistory[0].filters[0].field).toBe('action');
    });

    // Note: Hydration and error handling tests are skipped as they require
    // special setup for Zustand persist middleware in test environment
  });
});
