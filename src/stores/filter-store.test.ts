import { describe, it, expect, beforeEach } from 'vitest';
import { useFilterStore } from './filter-store';

describe('useFilterStore', () => {
  beforeEach(() => {
    // Reset store before each test
    useFilterStore.setState({
      filters: [],
      draftMode: false,
      savedFilters: [],
    });

    // Clear localStorage
    localStorage.clear();
  });

  describe('addFilter', () => {
    it('should add a filter with generated ID', () => {
      const { addFilter } = useFilterStore.getState();

      addFilter({
        field: 'sourceIp',
        operator: 'equals',
        value: '192.168.1.1',
      });

      const state = useFilterStore.getState();
      expect(state.filters).toHaveLength(1);
      expect(state.filters[0]).toMatchObject({
        field: 'sourceIp',
        operator: 'equals',
        value: '192.168.1.1',
        logic: 'AND',
      });
      expect(state.filters[0].id).toBeDefined();
      expect(typeof state.filters[0].id).toBe('string');
    });

    it('should set draftMode to true when adding a filter', () => {
      const { addFilter } = useFilterStore.getState();

      addFilter({
        field: 'action',
        operator: 'equals',
        value: 'block',
      });

      const state = useFilterStore.getState();
      expect(state.draftMode).toBe(true);
    });

    it('should use provided logic operator if specified', () => {
      const { addFilter } = useFilterStore.getState();

      addFilter({
        field: 'protocol',
        operator: 'equals',
        value: 'TCP',
        logic: 'OR',
      });

      const state = useFilterStore.getState();
      expect(state.filters[0].logic).toBe('OR');
    });

    it('should default to AND logic if not specified', () => {
      const { addFilter } = useFilterStore.getState();

      addFilter({
        field: 'protocol',
        operator: 'equals',
        value: 'TCP',
      });

      const state = useFilterStore.getState();
      expect(state.filters[0].logic).toBe('AND');
    });

    it('should add multiple filters in order', () => {
      const { addFilter } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
      addFilter({ field: 'destinationPort', operator: 'equals', value: 443 });
      addFilter({ field: 'action', operator: 'equals', value: 'block' });

      const state = useFilterStore.getState();
      expect(state.filters).toHaveLength(3);
      expect(state.filters[0].field).toBe('sourceIp');
      expect(state.filters[1].field).toBe('destinationPort');
      expect(state.filters[2].field).toBe('action');
    });
  });

  describe('removeFilter', () => {
    it('should remove filter by ID', () => {
      const { addFilter, removeFilter } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
      addFilter({ field: 'destinationPort', operator: 'equals', value: 443 });

      const state = useFilterStore.getState();
      const firstFilterId = state.filters[0].id;

      removeFilter(firstFilterId);

      const newState = useFilterStore.getState();
      expect(newState.filters).toHaveLength(1);
      expect(newState.filters[0].field).toBe('destinationPort');
    });

    it('should set draftMode to false when removing last filter', () => {
      const { addFilter, removeFilter } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });

      const state = useFilterStore.getState();
      const filterId = state.filters[0].id;

      removeFilter(filterId);

      const newState = useFilterStore.getState();
      expect(newState.filters).toHaveLength(0);
      expect(newState.draftMode).toBe(false);
    });

    it('should keep draftMode true when filters remain', () => {
      const { addFilter, removeFilter } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
      addFilter({ field: 'destinationPort', operator: 'equals', value: 443 });

      const state = useFilterStore.getState();
      const firstFilterId = state.filters[0].id;

      removeFilter(firstFilterId);

      const newState = useFilterStore.getState();
      expect(newState.filters).toHaveLength(1);
      expect(newState.draftMode).toBe(true);
    });
  });

  describe('updateFilter', () => {
    it('should update filter by ID', () => {
      const { addFilter, updateFilter } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });

      const state = useFilterStore.getState();
      const filterId = state.filters[0].id;

      updateFilter(filterId, { value: '10.0.0.1' });

      const newState = useFilterStore.getState();
      expect(newState.filters[0].value).toBe('10.0.0.1');
      expect(newState.filters[0].field).toBe('sourceIp'); // Other fields unchanged
    });

    it('should update multiple fields at once', () => {
      const { addFilter, updateFilter } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });

      const state = useFilterStore.getState();
      const filterId = state.filters[0].id;

      updateFilter(filterId, {
        operator: 'contains',
        value: '192.168',
        logic: 'OR',
      });

      const newState = useFilterStore.getState();
      expect(newState.filters[0].operator).toBe('contains');
      expect(newState.filters[0].value).toBe('192.168');
      expect(newState.filters[0].logic).toBe('OR');
    });

    it('should set draftMode to true when updating', () => {
      const { addFilter, updateFilter, setDraftMode } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
      setDraftMode(false); // Simulate active mode

      const state = useFilterStore.getState();
      const filterId = state.filters[0].id;

      updateFilter(filterId, { value: '10.0.0.1' });

      const newState = useFilterStore.getState();
      expect(newState.draftMode).toBe(true);
    });

    it('should not modify other filters', () => {
      const { addFilter, updateFilter } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
      addFilter({ field: 'destinationPort', operator: 'equals', value: 443 });

      const state = useFilterStore.getState();
      const firstFilterId = state.filters[0].id;

      updateFilter(firstFilterId, { value: '10.0.0.1' });

      const newState = useFilterStore.getState();
      expect(newState.filters[0].value).toBe('10.0.0.1');
      expect(newState.filters[1].value).toBe(443); // Unchanged
    });
  });

  describe('clearFilters', () => {
    it('should remove all filters', () => {
      const { addFilter, clearFilters } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
      addFilter({ field: 'destinationPort', operator: 'equals', value: 443 });
      addFilter({ field: 'action', operator: 'equals', value: 'block' });

      clearFilters();

      const state = useFilterStore.getState();
      expect(state.filters).toHaveLength(0);
    });

    it('should set draftMode to false', () => {
      const { addFilter, clearFilters } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });

      clearFilters();

      const state = useFilterStore.getState();
      expect(state.draftMode).toBe(false);
    });
  });

  describe('setDraftMode', () => {
    it('should set draftMode to true', () => {
      const { setDraftMode } = useFilterStore.getState();

      setDraftMode(true);

      const state = useFilterStore.getState();
      expect(state.draftMode).toBe(true);
    });

    it('should set draftMode to false', () => {
      const { addFilter, setDraftMode } = useFilterStore.getState();

      addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
      // draftMode is now true

      setDraftMode(false);

      const state = useFilterStore.getState();
      expect(state.draftMode).toBe(false);
    });
  });

  // NEW TESTS FOR STORY 2.3 - Saved Filters
  describe('saveFilter (Story 2.3)', () => {
    it('should save current filters with a name', () => {
      const { addFilter, saveFilter } = useFilterStore.getState();

      addFilter({
        field: 'sourceIp',
        operator: 'equals',
        value: '192.168.1.1',
        logic: 'AND',
      });

      saveFilter('Test Filter');

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(1);
      expect(state.savedFilters[0].name).toBe('Test Filter');
      expect(state.savedFilters[0].filters).toHaveLength(1);
      expect(state.savedFilters[0].filters[0].field).toBe('sourceIp');
      expect(state.savedFilters[0].filters[0].value).toBe('192.168.1.1');
      expect(state.savedFilters[0].timestamp).toBeDefined();
    });

    it('should strip runtime IDs when saving filters', () => {
      const { addFilter, saveFilter } = useFilterStore.getState();

      addFilter({
        field: 'protocol',
        operator: 'equals',
        value: 'TCP',
        logic: 'AND',
      });

      saveFilter('Protocol Filter');

      const state = useFilterStore.getState();
      const savedFilter = state.savedFilters[0];

      // Saved filters should not have runtime IDs
      expect(savedFilter.filters[0]).not.toHaveProperty('id');
    });

    it('should enforce 20 filter limit with FIFO eviction', () => {
      const { saveFilter, addFilter } = useFilterStore.getState();

      addFilter({
        field: 'sourceIp',
        operator: 'equals',
        value: '10.0.0.1',
        logic: 'AND',
      });

      // Save 21 filters
      for (let i = 0; i < 21; i++) {
        saveFilter(`Filter ${i}`);
      }

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(20);

      // First filter (Filter 0) should be removed
      const filterNames = state.savedFilters.map((f) => f.name);
      expect(filterNames).not.toContain('Filter 0');
      expect(filterNames).toContain('Filter 20');
    });
  });

  describe('loadFilter (Story 2.3)', () => {
    it('should load saved filter into active filters', () => {
      const { addFilter, saveFilter, loadFilter, clearFilters } = useFilterStore.getState();

      addFilter({
        field: 'action',
        operator: 'equals',
        value: 'block',
        logic: 'AND',
      });

      saveFilter('Blocked Actions');
      const savedFilterId = useFilterStore.getState().savedFilters[0].id;

      clearFilters();
      expect(useFilterStore.getState().filters).toHaveLength(0);

      loadFilter(savedFilterId);

      const state = useFilterStore.getState();
      expect(state.filters).toHaveLength(1);
      expect(state.filters[0].field).toBe('action');
      expect(state.filters[0].value).toBe('block');
      expect(state.draftMode).toBe(true);
    });

    it('should generate new runtime IDs when loading', () => {
      const { addFilter, saveFilter, loadFilter, clearFilters } = useFilterStore.getState();

      addFilter({
        field: 'protocol',
        operator: 'equals',
        value: 'UDP',
        logic: 'AND',
      });

      const originalId = useFilterStore.getState().filters[0].id;

      saveFilter('UDP Filter');
      const savedFilterId = useFilterStore.getState().savedFilters[0].id;

      clearFilters();
      loadFilter(savedFilterId);

      const loadedFilter = useFilterStore.getState().filters[0];
      expect(loadedFilter.id).toBeDefined();
      expect(loadedFilter.id).not.toBe(originalId);
    });

    it('should handle loading non-existent filter gracefully', () => {
      const { loadFilter } = useFilterStore.getState();

      const initialState = useFilterStore.getState();
      loadFilter('non-existent-id');

      const finalState = useFilterStore.getState();
      expect(finalState.filters).toEqual(initialState.filters);
    });
  });

  describe('deleteSavedFilter (Story 2.3)', () => {
    it('should delete a saved filter by ID', () => {
      const { addFilter, saveFilter, deleteSavedFilter } = useFilterStore.getState();

      addFilter({
        field: 'sourcePort',
        operator: 'equals',
        value: '8080',
        logic: 'AND',
      });

      saveFilter('Port 8080 Filter');
      const savedFilterId = useFilterStore.getState().savedFilters[0].id;

      expect(useFilterStore.getState().savedFilters).toHaveLength(1);

      deleteSavedFilter(savedFilterId);

      expect(useFilterStore.getState().savedFilters).toHaveLength(0);
    });

    it('should delete correct filter when multiple exist', () => {
      const { addFilter, saveFilter, deleteSavedFilter } = useFilterStore.getState();

      addFilter({
        field: 'action',
        operator: 'equals',
        value: 'pass',
        logic: 'AND',
      });

      saveFilter('Filter A');
      saveFilter('Filter B');
      saveFilter('Filter C');

      const filters = useFilterStore.getState().savedFilters;
      const filterBId = filters.find((f) => f.name === 'Filter B')?.id;

      deleteSavedFilter(filterBId!);

      const remaining = useFilterStore.getState().savedFilters;
      expect(remaining).toHaveLength(2);
      expect(remaining.map((f) => f.name)).toEqual(['Filter A', 'Filter C']);
    });
  });

  describe('localStorage persistence (Story 2.3)', () => {
    it('should persist saved filters to localStorage', () => {
      const { addFilter, saveFilter } = useFilterStore.getState();

      addFilter({
        field: 'timestamp',
        operator: 'relative',
        value: '24h',
        logic: 'AND',
      });

      saveFilter('Last 24h');

      const stored = localStorage.getItem('opnsense-log-viewer-filters');
      expect(stored).toBeDefined();

      const parsed = JSON.parse(stored!);
      expect(parsed.state.savedFilters).toHaveLength(1);
      expect(parsed.state.savedFilters[0].name).toBe('Last 24h');
    });

    it('should not persist active filters or draft mode', () => {
      const { addFilter, setDraftMode } = useFilterStore.getState();

      addFilter({
        field: 'sourceIp',
        operator: 'equals',
        value: '10.0.0.1',
        logic: 'AND',
      });

      setDraftMode(true);

      const stored = localStorage.getItem('opnsense-log-viewer-filters');
      const parsed = JSON.parse(stored!);

      expect(parsed.state).toHaveProperty('savedFilters');
      expect(parsed.state).not.toHaveProperty('filters');
      expect(parsed.state).not.toHaveProperty('draftMode');
    });
  });

  describe('performance requirements (NFR-003.3)', () => {
    it('should save filter in <100ms', () => {
      const { addFilter, saveFilter } = useFilterStore.getState();

      for (let i = 0; i < 5; i++) {
        addFilter({
          field: 'sourceIp',
          operator: 'equals',
          value: `192.168.1.${i}`,
          logic: 'AND',
        });
      }

      const start = performance.now();
      saveFilter('Performance Test Filter');
      const duration = performance.now() - start;

      expect(duration).toBeLessThan(100);
    });

    it('should load filter in <100ms', () => {
      const { addFilter, saveFilter, loadFilter } = useFilterStore.getState();

      for (let i = 0; i < 5; i++) {
        addFilter({
          field: 'destinationIp',
          operator: 'equals',
          value: `10.0.0.${i}`,
          logic: 'OR',
        });
      }

      saveFilter('Complex Filter');
      const savedFilterId = useFilterStore.getState().savedFilters[0].id;

      const start = performance.now();
      loadFilter(savedFilterId);
      const duration = performance.now() - start;

      expect(duration).toBeLessThan(100);
    });

    it('should delete filter in <50ms', () => {
      const { addFilter, saveFilter, deleteSavedFilter } = useFilterStore.getState();

      addFilter({
        field: 'protocol',
        operator: 'equals',
        value: 'TCP',
        logic: 'AND',
      });

      saveFilter('Delete Test');
      const savedFilterId = useFilterStore.getState().savedFilters[0].id;

      const start = performance.now();
      deleteSavedFilter(savedFilterId);
      const duration = performance.now() - start;

      expect(duration).toBeLessThan(50);
    });
  });
});
