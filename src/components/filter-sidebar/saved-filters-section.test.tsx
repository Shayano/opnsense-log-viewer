import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SavedFiltersSection } from './saved-filters-section';
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

describe('SavedFiltersSection', () => {
  beforeEach(() => {
    // Reset store before each test
    useFilterStore.setState({
      filters: [],
      draftMode: false,
      savedFilters: [],
    });
    localStorageMock.clear();
    vi.clearAllMocks();
  });

  describe('Empty State', () => {
    it('should render empty state when no saved filters', () => {
      render(<SavedFiltersSection />);

      expect(screen.getByText('Saved Filters (0)')).toBeInTheDocument();
      expect(screen.getByText(/No saved filters yet\./)).toBeInTheDocument();
      expect(screen.getByText(/Save your current filters to reuse them later\./)).toBeInTheDocument();
    });

    it('should be expanded by default', () => {
      render(<SavedFiltersSection />);

      const toggleButton = screen.getByLabelText('Toggle saved filters section');
      expect(toggleButton).toHaveAttribute('aria-expanded', 'true');
    });
  });

  describe('With Saved Filters', () => {
    beforeEach(() => {
      // Add some saved filters to the store
      useFilterStore.setState({
        filters: [],
        draftMode: false,
        savedFilters: [
          {
            id: 'filter-1',
            name: 'Nightly Port 443 Blocks',
            filters: [
              { field: 'destinationPort', operator: 'equals', value: 443, logic: 'AND' },
              { field: 'action', operator: 'equals', value: 'block' },
            ],
            timestamp: Date.now() - 1000,
            version: 1,
          },
          {
            id: 'filter-2',
            name: 'SSH Connections',
            filters: [{ field: 'destinationPort', operator: 'equals', value: 22 }],
            timestamp: Date.now(),
            version: 1,
          },
        ],
      });
    });

    it('should display saved filter count', () => {
      render(<SavedFiltersSection />);
      expect(screen.getByText('Saved Filters (2)')).toBeInTheDocument();
    });

    it('should display saved filter names', () => {
      render(<SavedFiltersSection />);
      expect(screen.getByText('Nightly Port 443 Blocks')).toBeInTheDocument();
      expect(screen.getByText('SSH Connections')).toBeInTheDocument();
    });

    it('should display filter count for each saved filter', () => {
      render(<SavedFiltersSection />);
      expect(screen.getByText('2 filters')).toBeInTheDocument(); // First filter has 2 filters
      expect(screen.getByText('1 filters')).toBeInTheDocument(); // Second filter has 1 filter
    });

    it('should have load and delete buttons for each saved filter', () => {
      render(<SavedFiltersSection />);

      const loadButtons = screen.getAllByLabelText(/Load filter/);
      const deleteButtons = screen.getAllByLabelText(/Delete filter/);

      expect(loadButtons).toHaveLength(2);
      expect(deleteButtons).toHaveLength(2);
    });
  });

  describe('Expand/Collapse', () => {
    it('should collapse when toggle button is clicked', () => {
      render(<SavedFiltersSection />);

      const toggleButton = screen.getByLabelText('Toggle saved filters section');
      fireEvent.click(toggleButton);

      expect(toggleButton).toHaveAttribute('aria-expanded', 'false');
      expect(screen.queryByText('No saved filters yet.')).not.toBeInTheDocument();
    });

    it('should expand when toggle button is clicked again', () => {
      render(<SavedFiltersSection />);

      const toggleButton = screen.getByLabelText('Toggle saved filters section');

      // Collapse
      fireEvent.click(toggleButton);
      expect(toggleButton).toHaveAttribute('aria-expanded', 'false');

      // Expand
      fireEvent.click(toggleButton);
      expect(toggleButton).toHaveAttribute('aria-expanded', 'true');
      expect(screen.getByText(/No saved filters yet\./)).toBeInTheDocument();
    });
  });

  describe('Load Filter', () => {
    beforeEach(() => {
      useFilterStore.setState({
        filters: [],
        draftMode: false,
        savedFilters: [
          {
            id: 'filter-1',
            name: 'Test Filter',
            filters: [
              { field: 'sourceIp', operator: 'equals', value: '192.168.1.1', logic: 'AND' },
              { field: 'destinationPort', operator: 'equals', value: 443 },
            ],
            timestamp: Date.now(),
            version: 1,
          },
        ],
      });
    });

    it('should load filter when load button is clicked', () => {
      render(<SavedFiltersSection />);

      const loadButton = screen.getByLabelText('Load filter Test Filter');
      fireEvent.click(loadButton);

      const state = useFilterStore.getState();
      expect(state.filters).toHaveLength(2);
      expect(state.filters[0].field).toBe('sourceIp');
      expect(state.filters[1].field).toBe('destinationPort');
      expect(state.draftMode).toBe(true);
    });

    it('should show success toast when filter is loaded', () => {
      render(<SavedFiltersSection />);

      const loadButton = screen.getByLabelText('Load filter Test Filter');
      fireEvent.click(loadButton);

      expect(toast.success).toHaveBeenCalledWith('Filter "Test Filter" loaded');
    });

    it('should regenerate IDs when loading filter', () => {
      render(<SavedFiltersSection />);

      const loadButton = screen.getByLabelText('Load filter Test Filter');
      fireEvent.click(loadButton);

      const state = useFilterStore.getState();
      // Each loaded filter should have a new UUID
      expect(state.filters[0].id).toBeDefined();
      expect(state.filters[1].id).toBeDefined();
      expect(state.filters[0].id).not.toBe('filter-1'); // Not the saved filter ID
      expect(typeof state.filters[0].id).toBe('string');
    });
  });

  describe('Delete Filter', () => {
    beforeEach(() => {
      useFilterStore.setState({
        filters: [],
        draftMode: false,
        savedFilters: [
          {
            id: 'filter-1',
            name: 'Filter to Delete',
            filters: [{ field: 'sourceIp', operator: 'equals', value: '10.0.0.1' }],
            timestamp: Date.now(),
            version: 1,
          },
        ],
      });
    });

    it('should open delete confirmation dialog when delete button is clicked', () => {
      render(<SavedFiltersSection />);

      const deleteButton = screen.getByLabelText('Delete filter Filter to Delete');
      fireEvent.click(deleteButton);

      expect(screen.getByText('Delete Filter')).toBeInTheDocument();
      expect(screen.getByText(/Delete filter/)).toBeInTheDocument();
      expect(screen.getByText('"Filter to Delete"')).toBeInTheDocument();
      expect(screen.getByText('⚠️ This cannot be undone.')).toBeInTheDocument();
    });

    it('should delete filter when confirmed', async () => {
      render(<SavedFiltersSection />);

      // Open delete dialog
      const deleteButton = screen.getByLabelText('Delete filter Filter to Delete');
      fireEvent.click(deleteButton);

      // Confirm deletion - get all Delete buttons and click the one in the modal (last one)
      const deleteButtons = screen.getAllByRole('button', { name: /Delete/i });
      const confirmButton = deleteButtons[deleteButtons.length - 1]; // Last one is the confirm button
      fireEvent.click(confirmButton);

      await waitFor(() => {
        const state = useFilterStore.getState();
        expect(state.savedFilters).toHaveLength(0);
      });

      expect(toast.success).toHaveBeenCalledWith('Filter "Filter to Delete" deleted');
    });

    it('should close dialog when cancel is clicked', () => {
      render(<SavedFiltersSection />);

      // Open delete dialog
      const deleteButton = screen.getByLabelText('Delete filter Filter to Delete');
      fireEvent.click(deleteButton);

      // Cancel
      const cancelButton = screen.getByRole('button', { name: /Cancel/i });
      fireEvent.click(cancelButton);

      // Dialog should be closed (check by checking if confirm button is gone)
      expect(screen.queryByRole('button', { name: /^Delete$/i })).not.toBeInTheDocument();

      // Filter should still exist
      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(1);
    });

    it('should not delete filter when cancelled', () => {
      render(<SavedFiltersSection />);

      // Open delete dialog
      const deleteButton = screen.getByLabelText('Delete filter Filter to Delete');
      fireEvent.click(deleteButton);

      // Cancel
      const cancelButton = screen.getByRole('button', { name: /Cancel/i });
      fireEvent.click(cancelButton);

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(1);
      expect(toast.success).not.toHaveBeenCalled();
    });
  });

  describe('Accessibility', () => {
    it('should have aria-live region for dynamic updates', () => {
      useFilterStore.setState({
        filters: [],
        draftMode: false,
        savedFilters: [
          {
            id: 'filter-1',
            name: 'Test Filter',
            filters: [{ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' }],
            timestamp: Date.now(),
            version: 1,
          },
        ],
      });

      const { container } = render(<SavedFiltersSection />);
      const liveRegion = container.querySelector('[aria-live="polite"]');

      expect(liveRegion).toBeInTheDocument();
    });

    it('should have proper button labels', () => {
      useFilterStore.setState({
        filters: [],
        draftMode: false,
        savedFilters: [
          {
            id: 'filter-1',
            name: 'My Filter',
            filters: [{ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' }],
            timestamp: Date.now(),
            version: 1,
          },
        ],
      });

      render(<SavedFiltersSection />);

      expect(screen.getByLabelText('Load filter My Filter')).toBeInTheDocument();
      expect(screen.getByLabelText('Delete filter My Filter')).toBeInTheDocument();
      expect(screen.getByLabelText('Toggle saved filters section')).toBeInTheDocument();
    });
  });
});
