import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SearchHistorySection } from './search-history-section';
import { useSearchHistoryStore } from '@/stores/search-history-store';
import { useFilterStore } from '@/stores/filter-store';
import { useQueryStore } from '@/stores/query-store';
import toast from 'react-hot-toast';

// Mock stores
vi.mock('@/stores/search-history-store');
vi.mock('@/stores/filter-store');
vi.mock('@/stores/query-store');
vi.mock('react-hot-toast');
vi.mock('@/utils/query-client', () => ({
  executeQuery: vi.fn().mockResolvedValue({
    entryIds: [1, 2, 3],
    totalCount: 1000,
    matchedCount: 100,
    executionTimeMs: 250,
  }),
}));

describe('SearchHistorySection', () => {
  const mockSearchHistory = [
    {
      id: 'history-1',
      timestamp: Date.now() - 2 * 60 * 60 * 1000, // 2 hours ago
      filters: [
        { field: 'action' as const, operator: 'equals' as const, value: 'block' },
      ],
      resultCount: 100,
      totalCount: 1000,
      executionTimeMs: 250,
    },
    {
      id: 'history-2',
      timestamp: Date.now() - 24 * 60 * 60 * 1000, // 1 day ago
      filters: [
        { field: 'destinationPort' as const, operator: 'equals' as const, value: 443 },
      ],
      resultCount: 50,
      totalCount: 500,
      executionTimeMs: 150,
    },
  ];

  const mockDeleteHistoryEntry = vi.fn();
  const mockClearAllHistory = vi.fn();
  const mockClearFilters = vi.fn();
  const mockSetDraftMode = vi.fn();
  const mockSetResult = vi.fn();
  const mockSetExecuting = vi.fn();
  const mockSetError = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Setup default mocks
    (useSearchHistoryStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      searchHistory: mockSearchHistory,
      deleteHistoryEntry: mockDeleteHistoryEntry,
      clearAllHistory: mockClearAllHistory,
    });

    (useFilterStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      clearFilters: mockClearFilters,
      filters: [],
      setDraftMode: mockSetDraftMode,
    });

    (useQueryStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      setResult: mockSetResult,
      setExecuting: mockSetExecuting,
      setError: mockSetError,
    });

    (toast.success as unknown as ReturnType<typeof vi.fn>) = vi.fn();
    (toast.error as unknown as ReturnType<typeof vi.fn>) = vi.fn();
  });

  describe('rendering', () => {
    it('should render section header with history count', () => {
      render(<SearchHistorySection />);
      expect(screen.getByText(/Recent Searches \(2\)/i)).toBeInTheDocument();
    });

    it('should render collapsed by default (chevron icon visible)', () => {
      render(<SearchHistorySection />);
      const button = screen.getByRole('button', { name: /Toggle search history section/i });
      expect(button).toHaveAttribute('aria-expanded', 'true');
    });

    it('should render empty state when no history', () => {
      (useSearchHistoryStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        searchHistory: [],
        deleteHistoryEntry: mockDeleteHistoryEntry,
        clearAllHistory: mockClearAllHistory,
      });

      render(<SearchHistorySection />);
      expect(screen.getByText(/No search history yet/i)).toBeInTheDocument();
      expect(screen.getByText(/Execute a search to start tracking history/i)).toBeInTheDocument();
    });

    it('should render history entries with filter summaries', () => {
      render(<SearchHistorySection />);
      expect(screen.getByText(/Action : block/i)).toBeInTheDocument();
      expect(screen.getByText(/Dest Port : 443/i)).toBeInTheDocument();
    });

    it('should render result counts', () => {
      render(<SearchHistorySection />);
      expect(screen.getByText(/100 results/i)).toBeInTheDocument();
      expect(screen.getByText(/50 results/i)).toBeInTheDocument();
    });

    it('should render relative timestamps', () => {
      render(<SearchHistorySection />);
      // date-fns formatDistanceToNow should show "2 hours ago"
      expect(screen.getByText(/hours ago/i)).toBeInTheDocument();
    });

    it('should render Clear All button when history exists', () => {
      render(<SearchHistorySection />);
      expect(screen.getByText(/Clear All/i)).toBeInTheDocument();
    });

    it('should not render Clear All button when history is empty', () => {
      (useSearchHistoryStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        searchHistory: [],
        deleteHistoryEntry: mockDeleteHistoryEntry,
        clearAllHistory: mockClearAllHistory,
      });

      render(<SearchHistorySection />);
      expect(screen.queryByText(/Clear All/i)).not.toBeInTheDocument();
    });

    it('should render Re-run and Delete buttons for each entry', () => {
      render(<SearchHistorySection />);
      const rerunButtons = screen.getAllByLabelText(/Re-run search/i);
      const deleteButtons = screen.getAllByLabelText(/Delete from history/i);

      expect(rerunButtons).toHaveLength(2);
      expect(deleteButtons).toHaveLength(2);
    });
  });

  describe('collapsible section', () => {
    it('should toggle section collapse on header click', () => {
      render(<SearchHistorySection />);
      const toggleButton = screen.getByRole('button', { name: /Toggle search history section/i });

      // Initially expanded
      expect(toggleButton).toHaveAttribute('aria-expanded', 'true');
      expect(screen.getByText(/Action : block/i)).toBeInTheDocument();

      // Click to collapse
      fireEvent.click(toggleButton);

      // Should be collapsed (content not visible)
      expect(toggleButton).toHaveAttribute('aria-expanded', 'false');
      expect(screen.queryByText(/Action : block/i)).not.toBeInTheDocument();

      // Click to expand again
      fireEvent.click(toggleButton);
      expect(toggleButton).toHaveAttribute('aria-expanded', 'true');
      expect(screen.getByText(/Action : block/i)).toBeInTheDocument();
    });
  });

  describe('re-run functionality', () => {
    it('should call executeQuery when Re-run button clicked', async () => {
      const { executeQuery } = await import('@/utils/query-client');

      render(<SearchHistorySection />);
      const rerunButtons = screen.getAllByLabelText(/Re-run search/i);

      fireEvent.click(rerunButtons[0]);

      await waitFor(() => {
        expect(mockClearFilters).toHaveBeenCalled();
        expect(mockSetExecuting).toHaveBeenCalledWith(true);
        expect(executeQuery).toHaveBeenCalled();
      });
    });

    it('should show success toast on successful re-run', async () => {
      render(<SearchHistorySection />);
      const rerunButtons = screen.getAllByLabelText(/Re-run search/i);

      fireEvent.click(rerunButtons[0]);

      await waitFor(() => {
        expect(toast.success).toHaveBeenCalledWith(expect.stringContaining('Re-ran search from'));
      });
    });

    it('should show error toast on failed re-run', async () => {
      const { executeQuery } = await import('@/utils/query-client');
      (executeQuery as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('Query failed'));

      render(<SearchHistorySection />);
      const rerunButtons = screen.getAllByLabelText(/Re-run search/i);

      fireEvent.click(rerunButtons[0]);

      await waitFor(() => {
        expect(mockSetError).toHaveBeenCalledWith('Error: Query failed');
        expect(toast.error).toHaveBeenCalledWith(expect.stringContaining('Failed to re-run search'));
      });
    });

    it('should set result and update state on successful re-run', async () => {
      const mockResult = {
        entryIds: [1, 2, 3],
        totalCount: 1000,
        matchedCount: 100,
        executionTimeMs: 250,
      };

      const { executeQuery } = await import('@/utils/query-client');
      (executeQuery as ReturnType<typeof vi.fn>).mockResolvedValueOnce(mockResult);

      render(<SearchHistorySection />);
      const rerunButtons = screen.getAllByLabelText(/Re-run search/i);

      fireEvent.click(rerunButtons[0]);

      await waitFor(() => {
        expect(mockSetResult).toHaveBeenCalledWith(mockResult);
        expect(mockSetExecuting).toHaveBeenCalledWith(false);
      });
    });
  });

  describe('delete functionality', () => {
    it('should call deleteHistoryEntry when Delete button clicked', () => {
      render(<SearchHistorySection />);
      const deleteButtons = screen.getAllByLabelText(/Delete from history/i);

      fireEvent.click(deleteButtons[0]);

      expect(mockDeleteHistoryEntry).toHaveBeenCalledWith('history-1');
    });

    it('should show success toast on delete', () => {
      render(<SearchHistorySection />);
      const deleteButtons = screen.getAllByLabelText(/Delete from history/i);

      fireEvent.click(deleteButtons[0]);

      expect(toast.success).toHaveBeenCalledWith('Search removed from history');
    });

    it('should delete correct entry by ID', () => {
      render(<SearchHistorySection />);
      const deleteButtons = screen.getAllByLabelText(/Delete from history/i);

      // Delete second entry
      fireEvent.click(deleteButtons[1]);

      expect(mockDeleteHistoryEntry).toHaveBeenCalledWith('history-2');
    });
  });

  describe('clear all functionality', () => {
    it('should show confirmation dialog when Clear All clicked', () => {
      render(<SearchHistorySection />);
      const clearAllButton = screen.getByText(/Clear All/i);

      fireEvent.click(clearAllButton);

      expect(screen.getByText(/Clear Search History/i)).toBeInTheDocument();
      expect(screen.getByText(/Clear all search history\? This cannot be undone/i)).toBeInTheDocument();
    });

    it('should close dialog when Cancel clicked', () => {
      render(<SearchHistorySection />);
      const clearAllButton = screen.getByText(/Clear All/i);

      fireEvent.click(clearAllButton);
      expect(screen.getByText(/Clear Search History/i)).toBeInTheDocument();

      const cancelButton = screen.getByText(/Cancel/i);
      fireEvent.click(cancelButton);

      expect(screen.queryByText(/Clear Search History/i)).not.toBeInTheDocument();
    });

    it('should call clearAllHistory when confirmed', () => {
      render(<SearchHistorySection />);
      const clearAllButton = screen.getByText(/Clear All/i);

      fireEvent.click(clearAllButton);

      // Get all buttons with "Clear All" text - dialog button is last
      const confirmButtons = screen.getAllByRole('button', { name: /Clear All/i });
      const confirmButton = confirmButtons[confirmButtons.length - 1];
      fireEvent.click(confirmButton);

      expect(mockClearAllHistory).toHaveBeenCalled();
    });

    it('should show success toast on clear all', () => {
      render(<SearchHistorySection />);
      const clearAllButton = screen.getByText(/Clear All/i);

      fireEvent.click(clearAllButton);

      // Get all buttons with "Clear All" text - dialog button is last
      const confirmButtons = screen.getAllByRole('button', { name: /Clear All/i });
      const confirmButton = confirmButtons[confirmButtons.length - 1];
      fireEvent.click(confirmButton);

      expect(toast.success).toHaveBeenCalledWith('Search history cleared');
    });

    it('should close dialog after clear all confirmed', async () => {
      render(<SearchHistorySection />);
      const clearAllButton = screen.getByText(/Clear All/i);

      fireEvent.click(clearAllButton);
      expect(screen.getByText(/Clear Search History/i)).toBeInTheDocument();

      // Get all buttons with "Clear All" text - dialog button is last
      const confirmButtons = screen.getAllByRole('button', { name: /Clear All/i });
      const confirmButton = confirmButtons[confirmButtons.length - 1];
      fireEvent.click(confirmButton);

      await waitFor(() => {
        expect(screen.queryByText(/Clear Search History/i)).not.toBeInTheDocument();
      });
    });
  });

  describe('timestamp formatting', () => {
    it('should format recent timestamps with relative time', () => {
      const recentHistory = [
        {
          id: 'recent-1',
          timestamp: Date.now() - 30 * 60 * 1000, // 30 minutes ago
          filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'pass' }],
          resultCount: 10,
          totalCount: 100,
          executionTimeMs: 50,
        },
      ];

      (useSearchHistoryStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        searchHistory: recentHistory,
        deleteHistoryEntry: mockDeleteHistoryEntry,
        clearAllHistory: mockClearAllHistory,
      });

      render(<SearchHistorySection />);
      expect(screen.getByText(/minutes ago/i)).toBeInTheDocument();
    });

    it('should format old timestamps (>7 days) with full date', () => {
      const oldHistory = [
        {
          id: 'old-1',
          timestamp: Date.now() - 10 * 24 * 60 * 60 * 1000, // 10 days ago
          filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'pass' }],
          resultCount: 10,
          totalCount: 100,
          executionTimeMs: 50,
        },
      ];

      (useSearchHistoryStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        searchHistory: oldHistory,
        deleteHistoryEntry: mockDeleteHistoryEntry,
        clearAllHistory: mockClearAllHistory,
      });

      render(<SearchHistorySection />);
      // Should contain "at" for full date format
      expect(screen.getByText(/at \d{1,2}:\d{2}/i)).toBeInTheDocument();
    });
  });

  describe('sorting', () => {
    it('should display history in reverse chronological order (most recent first)', () => {
      const unsortedHistory = [
        {
          id: 'old',
          timestamp: Date.now() - 48 * 60 * 60 * 1000,
          filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'old' }],
          resultCount: 10,
          totalCount: 100,
          executionTimeMs: 50,
        },
        {
          id: 'recent',
          timestamp: Date.now() - 1 * 60 * 60 * 1000,
          filters: [{ field: 'action' as const, operator: 'equals' as const, value: 'recent' }],
          resultCount: 20,
          totalCount: 200,
          executionTimeMs: 75,
        },
      ];

      (useSearchHistoryStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        searchHistory: unsortedHistory,
        deleteHistoryEntry: mockDeleteHistoryEntry,
        clearAllHistory: mockClearAllHistory,
      });

      render(<SearchHistorySection />);

      const filterSummaries = screen.getAllByText(/Action : (old|recent)/i);
      // Most recent should be first
      expect(filterSummaries[0]).toHaveTextContent('recent');
      expect(filterSummaries[1]).toHaveTextContent('old');
    });
  });
});
