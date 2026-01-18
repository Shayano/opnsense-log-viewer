import { describe, it, expect, beforeEach } from 'vitest';
import { useFilterStore } from './filter-store';
import type { Filter } from '@/types/filter';

describe('useFilterStore', () => {
  beforeEach(() => {
    // Reset store before each test
    useFilterStore.setState({
      filters: [],
      draftMode: false,
    });
  });

  describe('addFilter', () => {
    it('should add a filter with generated ID', () => {
      const { addFilter, filters } = useFilterStore.getState();

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
});
