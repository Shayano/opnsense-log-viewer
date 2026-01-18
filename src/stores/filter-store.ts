import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { v4 as uuidv4 } from 'uuid';
import type { Filter, FilterState, SavedFilter } from '@/types/filter';

const MAX_SAVED_FILTERS = 20;

export const useFilterStore = create<FilterState>()(
  persist(
    (set) => ({
      // Active filters (from Story 2.1)
      filters: [],
      draftMode: false,

      // Saved filters (NEW for Story 2.3)
      savedFilters: [],

      // Active filter actions (from Story 2.1)
      addFilter: (filter) =>
        set((state) => ({
          filters: [
            ...state.filters,
            { ...filter, id: uuidv4(), logic: filter.logic || 'AND' },
          ],
          draftMode: true, // Adding filter enters draft mode
        })),

      removeFilter: (id) =>
        set((state) => {
          const newFilters = state.filters.filter((f) => f.id !== id);
          return {
            filters: newFilters,
            draftMode: newFilters.length > 0, // Stay in draft if filters remain
          };
        }),

      updateFilter: (id, updates) =>
        set((state) => ({
          filters: state.filters.map((f) => (f.id === id ? { ...f, ...updates } : f)),
          draftMode: true,
        })),

      clearFilters: () =>
        set({
          filters: [],
          draftMode: false,
        }),

      setDraftMode: (draft) =>
        set({
          draftMode: draft,
        }),

      // Saved filter actions (NEW for Story 2.3)
      saveFilter: (name) =>
        set((state) => {
          // Create saved filter from current filters (strip runtime IDs)
          const savedFilter: SavedFilter = {
            id: uuidv4(),
            name,
            filters: state.filters.map(({ id, ...filter }) => filter),
            timestamp: Date.now(),
          };

          let updatedSavedFilters = [...state.savedFilters, savedFilter];

          // Enforce 20 filter limit (FIFO eviction)
          if (updatedSavedFilters.length > MAX_SAVED_FILTERS) {
            // Remove oldest filter by timestamp
            updatedSavedFilters.sort((a, b) => a.timestamp - b.timestamp);
            updatedSavedFilters = updatedSavedFilters.slice(1);
          }

          return {
            savedFilters: updatedSavedFilters,
          };
        }),

      loadFilter: (savedFilterId) =>
        set((state) => {
          const savedFilter = state.savedFilters.find((sf) => sf.id === savedFilterId);
          if (!savedFilter) return {};

          // Load filters with new runtime IDs
          const loadedFilters: Filter[] = savedFilter.filters.map((filter) => ({
            ...filter,
            id: uuidv4(),
          }));

          return {
            filters: loadedFilters,
            draftMode: true, // Loaded filters start in draft mode
          };
        }),

      deleteSavedFilter: (savedFilterId) =>
        set((state) => ({
          savedFilters: state.savedFilters.filter((sf) => sf.id !== savedFilterId),
        })),
    }),
    {
      name: 'opnsense-log-viewer-filters', // localStorage key
      partialize: (state) => ({
        // Only persist saved filters (not active filters or draft mode)
        savedFilters: state.savedFilters,
      }),
    }
  )
);
