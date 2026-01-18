import { create } from 'zustand';
import { v4 as uuidv4 } from 'uuid';
import type { Filter, FilterState } from '@/types/filter';

export const useFilterStore = create<FilterState>((set) => ({
  filters: [],
  draftMode: false,

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
}));
