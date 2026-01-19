import { create } from 'zustand';
import { persist, createJSONStorage, StateStorage } from 'zustand/middleware';
import { v4 as uuidv4 } from 'uuid';
import toast from 'react-hot-toast';
import type { Filter, FilterState, SavedFilter } from '@/types/filter';

const MAX_SAVED_FILTERS = 20;
const CURRENT_VERSION = 1;

// Persisted state structure with versioning
interface PersistedState {
  version?: number;
  savedFilters: SavedFilter[];
}

/**
 * Migrate saved filter data from older versions to current version
 * This ensures backward compatibility when the SavedFilter structure changes
 */
function migrateFilterData(persistedState: any): PersistedState {
  // If no persisted state, return empty state with current version
  if (!persistedState) {
    return {
      version: CURRENT_VERSION,
      savedFilters: [],
    };
  }

  // Get version (default to 0 if not present - initial format)
  const version = persistedState.version ?? 0;

  // If already at current version, return as-is
  if (version >= CURRENT_VERSION) {
    return persistedState as PersistedState;
  }

  // Migration chain: apply all migrations from old version to current
  let migrated = { ...persistedState };

  // Migration v0 -> v1: Add version field to each saved filter
  if (version < 1) {
    console.log('[Migration] Migrating filter data from v0 to v1');
    migrated = {
      version: 1,
      savedFilters: (migrated.savedFilters || []).map((sf: any) => ({
        ...sf,
        version: 1, // Add version field to each filter
      })),
    };
  }

  // Future migrations can be added here:
  // if (version < 2) { ... }
  // if (version < 3) { ... }

  console.log(`[Migration] Migration complete: v${version} -> v${CURRENT_VERSION}`);
  return migrated as PersistedState;
}

// Custom storage with error handling for quota exceeded
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
        toast.error('Storage full. Delete old filters to make room.');
        throw error; // Re-throw so caller knows it failed
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
            version: CURRENT_VERSION, // Add version for future migrations
          };

          let updatedSavedFilters = [...state.savedFilters, savedFilter];
          let removedFilter: SavedFilter | null = null;

          // Enforce 20 filter limit (FIFO eviction)
          if (updatedSavedFilters.length > MAX_SAVED_FILTERS) {
            // Remove oldest filter by timestamp
            updatedSavedFilters.sort((a, b) => a.timestamp - b.timestamp);
            removedFilter = updatedSavedFilters[0];
            updatedSavedFilters = updatedSavedFilters.slice(1);

            // Notify user about FIFO eviction (check if toast is available - tests may not have it)
            if (typeof toast === 'function') {
              toast(`Removed oldest filter "${removedFilter.name}" to make room`, {
                duration: 4000,
                icon: '⚠️',
              });
            }
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
      storage: createJSONStorage(() => createCustomStorage()),
      version: CURRENT_VERSION,
      migrate: (persistedState: any, _version: number) => {
        // Migrate old data format to current version
        const migrated = migrateFilterData(persistedState);
        // Return only the FilterState properties (Zustand expects this shape)
        return {
          savedFilters: migrated.savedFilters,
        };
      },
      partialize: (state) => ({
        // Only persist saved filters (not active filters or draft mode)
        // Include version for migration tracking
        version: CURRENT_VERSION,
        savedFilters: state.savedFilters,
      }),
    }
  )
);
