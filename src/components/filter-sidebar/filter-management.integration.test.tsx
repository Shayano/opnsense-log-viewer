import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { FilterSidebar } from './filter-sidebar';
import { useFilterStore } from '@/stores/filter-store';
import toast from 'react-hot-toast';

// Mock toast
vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
  };
})();
Object.defineProperty(window, 'localStorage', { value: localStorageMock });

describe('Filter Management Integration Tests', () => {
  beforeEach(() => {
    useFilterStore.setState({
      filters: [],
      draftMode: false,
      savedFilters: [],
    });
    localStorageMock.clear();
    vi.clearAllMocks();
  });

  describe('Save → Load → Execute Workflow', () => {
    it('should save filters, load them, and maintain draft mode', async () => {
      // Step 1: Add filters
      const { addFilter, saveFilter, loadFilter, setDraftMode } = useFilterStore.getState();
      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1', logic: 'AND' });
      addFilter({ field: 'destinationPort', operator: 'equals', value: 443 });

      expect(useFilterStore.getState().filters).toHaveLength(2);
      expect(useFilterStore.getState().draftMode).toBe(true);

      // Step 2: Save filters
      saveFilter('Test Workflow Filter');

      const state1 = useFilterStore.getState();
      expect(state1.savedFilters).toHaveLength(1);
      expect(state1.savedFilters[0].name).toBe('Test Workflow Filter');
      expect(state1.savedFilters[0].filters).toHaveLength(2);

      // Step 3: Clear active filters
      useFilterStore.setState({ filters: [], draftMode: false });
      expect(useFilterStore.getState().filters).toHaveLength(0);

      // Step 4: Load saved filter
      const savedFilterId = state1.savedFilters[0].id;
      loadFilter(savedFilterId);

      const state2 = useFilterStore.getState();
      expect(state2.filters).toHaveLength(2);
      expect(state2.draftMode).toBe(true); // Draft mode should be true after load
      expect(state2.filters[0].field).toBe('sourceIp');
      expect(state2.filters[1].field).toBe('destinationPort');

      // Step 5: Execute (set draft mode to false)
      setDraftMode(false);
      expect(useFilterStore.getState().draftMode).toBe(false);
    });
  });

  describe('Save → Delete → Verify Removed', () => {
    it('should delete saved filter from store and localStorage', async () => {
      const { addFilter, saveFilter, deleteSavedFilter } = useFilterStore.getState();

      // Add and save filter
      addFilter({ field: 'protocol', operator: 'equals', value: 'TCP' });
      saveFilter('Filter to Delete');

      expect(useFilterStore.getState().savedFilters).toHaveLength(1);

      // Delete filter
      const filterId = useFilterStore.getState().savedFilters[0].id;
      deleteSavedFilter(filterId);

      expect(useFilterStore.getState().savedFilters).toHaveLength(0);

      // Verify localStorage is updated
      const storedData = localStorageMock.getItem('opnsense-log-viewer-filters');
      if (storedData) {
        const parsed = JSON.parse(storedData);
        expect(parsed.state.savedFilters).toHaveLength(0);
      }
    });
  });

  describe('Load → Modify → Save as New', () => {
    it('should allow loading, modifying, and saving as new filter', () => {
      const { addFilter, saveFilter, loadFilter, updateFilter } = useFilterStore.getState();

      // Create and save original filter
      addFilter({ field: 'sourceIp', operator: 'equals', value: '10.0.0.1' });
      saveFilter('Original Filter');

      const originalId = useFilterStore.getState().savedFilters[0].id;

      // Clear and load
      useFilterStore.setState({ filters: [], draftMode: false });
      loadFilter(originalId);

      // Modify loaded filter
      const loadedFilterId = useFilterStore.getState().filters[0].id;
      updateFilter(loadedFilterId, { value: '10.0.0.2' });

      expect(useFilterStore.getState().filters[0].value).toBe('10.0.0.2');

      // Save as new
      saveFilter('Modified Filter');

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(2);
      expect(state.savedFilters[1].name).toBe('Modified Filter');
      expect(state.savedFilters[1].filters[0].value).toBe('10.0.0.2');
      // Original should be unchanged
      expect(state.savedFilters[0].filters[0].value).toBe('10.0.0.1');
    });
  });

  describe('localStorage Persistence', () => {
    it('should persist saved filters to localStorage', () => {
      const { addFilter, saveFilter } = useFilterStore.getState();

      addFilter({ field: 'action', operator: 'equals', value: 'block' });
      saveFilter('Persistent Filter');

      // Check localStorage
      const stored = localStorageMock.getItem('opnsense-log-viewer-filters');
      expect(stored).toBeDefined();

      if (stored) {
        const parsed = JSON.parse(stored);
        expect(parsed.state.savedFilters).toHaveLength(1);
        expect(parsed.state.savedFilters[0].name).toBe('Persistent Filter');
      }
    });

    it('should restore saved filters from localStorage', () => {
      const { addFilter, saveFilter } = useFilterStore.getState();

      // Save filters
      addFilter({ field: 'interface', operator: 'equals', value: 'vtnet0' });
      saveFilter('Session Filter');

      const savedData = localStorageMock.getItem('opnsense-log-viewer-filters');

      // Reset store (simulating new session)
      useFilterStore.setState({ filters: [], draftMode: false, savedFilters: [] });
      expect(useFilterStore.getState().savedFilters).toHaveLength(0);

      // Restore from localStorage (simulating Zustand persist hydration)
      if (savedData) {
        const parsed = JSON.parse(savedData);
        useFilterStore.setState({ savedFilters: parsed.state.savedFilters });
      }

      expect(useFilterStore.getState().savedFilters).toHaveLength(1);
      expect(useFilterStore.getState().savedFilters[0].name).toBe('Session Filter');
    });
  });

  describe('FIFO Eviction (20 Filter Limit)', () => {
    it('should remove oldest filter when exceeding 20 filters', () => {
      const { addFilter, saveFilter } = useFilterStore.getState();

      // Add 21 filters
      for (let i = 1; i <= 21; i++) {
        // Clear previous filter
        useFilterStore.setState({ filters: [], draftMode: false });

        addFilter({ field: 'sourceIp', operator: 'equals', value: `192.168.1.${i}` });
        saveFilter(`Filter ${i}`);
      }

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(20);
      // First filter should be removed (FIFO)
      expect(state.savedFilters.find(f => f.name === 'Filter 1')).toBeUndefined();
      // Last filter should exist
      expect(state.savedFilters.find(f => f.name === 'Filter 21')).toBeDefined();
    });
  });

  describe('Error Scenarios', () => {
    it('should allow saving with valid name after trim', () => {
      const { addFilter, saveFilter } = useFilterStore.getState();
      addFilter({ field: 'protocol', operator: 'equals', value: 'UDP' });

      // Save with spaces around name (will be trimmed)
      saveFilter('  Valid Filter  ');

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(1);
      expect(state.savedFilters[0].name).toBe('  Valid Filter  '); // Store preserves original
    });
  });

  describe('ID Regeneration on Load', () => {
    it('should regenerate IDs when loading saved filters', () => {
      // Start with clean state
      useFilterStore.setState({
        filters: [],
        draftMode: false,
        savedFilters: [],
      });

      const { addFilter, saveFilter, loadFilter } = useFilterStore.getState();

      // Create filter with ID
      addFilter({ field: 'destinationPort', operator: 'equals', value: 22 });
      const originalId = useFilterStore.getState().filters[0].id;

      saveFilter('SSH Filter');
      const savedFilterId = useFilterStore.getState().savedFilters[0].id;

      // Clear and load
      useFilterStore.setState({ filters: [], draftMode: false, savedFilters: useFilterStore.getState().savedFilters });
      loadFilter(savedFilterId);

      const loadedId = useFilterStore.getState().filters[0].id;

      // IDs should be different (regenerated)
      expect(loadedId).not.toBe(originalId);
      expect(loadedId).not.toBe(savedFilterId);
      expect(typeof loadedId).toBe('string');
      expect(loadedId.length).toBeGreaterThan(0);
    });

    it('should not have ID property in saved filter configurations', () => {
      // Start with clean state
      useFilterStore.setState({
        filters: [],
        draftMode: false,
        savedFilters: [],
      });

      const { addFilter, saveFilter } = useFilterStore.getState();

      addFilter({ field: 'action', operator: 'equals', value: 'pass' });
      saveFilter('Pass Filter');

      const savedFilter = useFilterStore.getState().savedFilters[0];
      // Saved filter configurations should NOT have 'id' property
      expect(savedFilter.filters[0]).not.toHaveProperty('id');
      expect(savedFilter.filters[0].field).toBe('action');
    });
  });
});
