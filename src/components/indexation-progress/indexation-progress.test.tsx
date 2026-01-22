import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { IndexationProgress } from './indexation-progress';
import { useFileStore } from '@/stores/file-store';
import { listen } from '@tauri-apps/api/event';

// Mock Tauri APIs
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

describe('IndexationProgress', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset file store before each test
    useFileStore.setState({
      currentFile: null,
      indexMetadata: null,
      isLoading: false,
      error: null,
      indexProgress: null,
      partialFilterAvailable: false,
      cacheHit: false,
    });
  });

  it('should not render when not indexing', () => {
    const { container } = render(
      <IndexationProgress isIndexing={false} onComplete={() => {}} onError={() => {}} />
    );
    expect(container.firstChild).toBeNull();
  });

  it('should render progress UI when indexing', () => {
    render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

    // Story 6.3: Updated UI - Speed/ETA only shown when progress values > 0
    expect(screen.getByText('Indexing Log File')).toBeInTheDocument();
    expect(screen.getByText('Data processed:')).toBeInTheDocument();
    expect(screen.getByText('Cancel')).toBeInTheDocument();
    // Speed and ETA are conditionally rendered when values > 0
    expect(screen.queryByText('Speed:')).not.toBeInTheDocument(); // Hidden at 0
  });

  describe('progressive loading display (Story 6.5)', () => {
    it('should display percentage prominently', () => {
      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      // Initial state shows 0%
      expect(screen.getByText('0%')).toBeInTheDocument();
    });

    it('should set up event listeners for cache events', () => {
      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      const listenMock = listen as Mock;
      const eventNames = listenMock.mock.calls.map((call) => call[0]);

      expect(eventNames).toContain('index-cache-hit');
      expect(eventNames).toContain('index-cache-miss');
      expect(eventNames).toContain('indexation-progress');
      expect(eventNames).toContain('indexation-complete');
      expect(eventNames).toContain('indexation-error');
    });

    it('should render cache hit display when cacheHit is true', () => {
      // Set cache hit state before rendering
      useFileStore.setState({ cacheHit: true });

      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      expect(screen.getByText('Loaded from cache')).toBeInTheDocument();
      expect(screen.getByText('Instant load')).toBeInTheDocument();
    });

    it('should update store on cache hit event', async () => {
      const onComplete = vi.fn();
      let cacheHitHandler: ((event: { payload: unknown }) => void) | null = null;

      (listen as Mock).mockImplementation((eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'index-cache-hit') {
          cacheHitHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      render(<IndexationProgress isIndexing={true} onComplete={onComplete} onError={() => {}} />);

      // Simulate cache hit event
      await act(async () => {
        cacheHitHandler?.({ payload: { filePath: '/test.log', cacheHit: true, cacheAgeSeconds: 60 } });
      });

      const state = useFileStore.getState();
      expect(state.cacheHit).toBe(true);
      expect(state.partialFilterAvailable).toBe(true);
      expect(onComplete).toHaveBeenCalled();
    });

    it('should update store on cache miss event', async () => {
      let cacheMissHandler: ((event: { payload: unknown }) => void) | null = null;

      (listen as Mock).mockImplementation((eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'index-cache-miss') {
          cacheMissHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      // Start with cacheHit true to verify it gets reset
      useFileStore.setState({ cacheHit: true });

      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      // Simulate cache miss event
      await act(async () => {
        cacheMissHandler?.({ payload: { filePath: '/test.log', cacheHit: false, reason: 'not_found' } });
      });

      expect(useFileStore.getState().cacheHit).toBe(false);
    });

    it('should update progress and store on progress event', async () => {
      let progressHandler: ((event: { payload: unknown }) => void) | null = null;

      (listen as Mock).mockImplementation((eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'indexation-progress') {
          progressHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      const progressData = {
        percentage: 50,
        bytesProcessed: 1073741824,
        totalBytes: 2147483648,
        speedGbps: 1.5,
        etaSeconds: 60,
        entriesIndexed: 5000000,
        totalEntriesEstimated: 10000000,
        batchesCompleted: 3,
        totalBatches: 7,
        partialFilterAvailable: true,
        entriesPerSecond: 83333,
        elapsedSeconds: 60,
      };

      // Simulate progress event
      await act(async () => {
        progressHandler?.({ payload: progressData });
      });

      const state = useFileStore.getState();
      expect(state.indexProgress).toEqual(progressData);
      expect(state.partialFilterAvailable).toBe(true);
    });

    it('should clean up event listeners on unmount', async () => {
      const unlistenMock = vi.fn();
      (listen as Mock).mockReturnValue(Promise.resolve(unlistenMock));

      const { unmount } = render(
        <IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />
      );

      // 5 listeners: cache-hit, cache-miss, progress, complete, error
      expect(listen).toHaveBeenCalledTimes(5);

      unmount();

      // Wait for async cleanup to complete
      await act(async () => {
        await Promise.resolve();
      });

      // Each of the 5 listeners should have called its unlisten function
      expect(unlistenMock).toHaveBeenCalledTimes(5);
    });
  });

  describe('entry formatting', () => {
    it('should format millions of entries correctly', async () => {
      let progressHandler: ((event: { payload: unknown }) => void) | null = null;

      (listen as Mock).mockImplementation((eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'indexation-progress') {
          progressHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      await act(async () => {
        progressHandler?.({
          payload: {
            percentage: 45,
            bytesProcessed: 1000000000,
            totalBytes: 2000000000,
            speedGbps: 1.0,
            etaSeconds: 60,
            entriesIndexed: 32000000,
            totalEntriesEstimated: 71000000,
            batchesCompleted: 6,
            totalBatches: 14,
            partialFilterAvailable: false,
            entriesPerSecond: 533333,
            elapsedSeconds: 60,
          },
        });
      });

      // Check formatted display (Story 6.3 - new field names)
      expect(screen.getByText(/32\.0M \/ 71\.0M/)).toBeInTheDocument();
    });

    it('should format thousands of entries correctly', async () => {
      let progressHandler: ((event: { payload: unknown }) => void) | null = null;

      (listen as Mock).mockImplementation((eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'indexation-progress') {
          progressHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      await act(async () => {
        progressHandler?.({
          payload: {
            percentage: 10,
            bytesProcessed: 100000000,
            totalBytes: 1000000000,
            speedGbps: 0.5,
            etaSeconds: 120,
            entriesIndexed: 50000,
            totalEntriesEstimated: 500000,
            batchesCompleted: 1,
            totalBatches: 10,
            partialFilterAvailable: false,
            entriesPerSecond: 833,
            elapsedSeconds: 60,
          },
        });
      });

      expect(screen.getByText(/50\.0K \/ 500\.0K/)).toBeInTheDocument();
    });
  });

  describe('partial filtering badge (Story 6.5 - Task 17.4)', () => {
    it('should show partial filtering badge when available', async () => {
      let progressHandler: ((event: { payload: unknown }) => void) | null = null;

      (listen as Mock).mockImplementation((eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'indexation-progress') {
          progressHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      await act(async () => {
        progressHandler?.({
          payload: {
            percentage: 30,
            bytesProcessed: 600000000,
            totalBytes: 2000000000,
            speedGbps: 1.2,
            etaSeconds: 90,
            entriesIndexed: 3000000,
            totalEntriesEstimated: 10000000,
            batchesCompleted: 2,
            totalBatches: 7,
            partialFilterAvailable: true,
            entriesPerSecond: 50000,
            elapsedSeconds: 60,
          },
        });
      });

      expect(screen.getByText('Partial filtering available')).toBeInTheDocument();
    });

    it('should not show partial filtering badge when not available', async () => {
      let progressHandler: ((event: { payload: unknown }) => void) | null = null;

      (listen as Mock).mockImplementation((eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'indexation-progress') {
          progressHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      await act(async () => {
        progressHandler?.({
          payload: {
            percentage: 10,
            bytesProcessed: 200000000,
            totalBytes: 2000000000,
            speedGbps: 1.0,
            etaSeconds: 120,
            entriesIndexed: 1000000,
            totalEntriesEstimated: 10000000,
            batchesCompleted: 1,
            totalBatches: 7,
            partialFilterAvailable: false,
            entriesPerSecond: 16667,
            elapsedSeconds: 60,
          },
        });
      });

      expect(screen.queryByText('Partial filtering available')).not.toBeInTheDocument();
    });

    it('should have tooltip on partial filtering badge per AC2', async () => {
      let progressHandler: ((event: { payload: unknown }) => void) | null = null;

      (listen as Mock).mockImplementation((eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'indexation-progress') {
          progressHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      render(<IndexationProgress isIndexing={true} onComplete={() => {}} onError={() => {}} />);

      await act(async () => {
        progressHandler?.({
          payload: {
            percentage: 30,
            bytesProcessed: 600000000,
            totalBytes: 2000000000,
            speedGbps: 1.2,
            etaSeconds: 90,
            entriesIndexed: 3000000,
            totalEntriesEstimated: 10000000,
            batchesCompleted: 2,
            totalBatches: 7,
            partialFilterAvailable: true,
            entriesPerSecond: 50000,
            elapsedSeconds: 60,
          },
        });
      });

      const badge = screen.getByText('Partial filtering available').closest('div');
      expect(badge).toHaveAttribute('title', 'You can start filtering now with partial results');
    });
  });

  describe('completion notification (Story 6.3 AC6)', () => {
    it('should show completion toast with statistics', async () => {
      const toast = await import('react-hot-toast');
      let completeHandler: ((event: { payload: unknown }) => void) | null = null;

      (listen as Mock).mockImplementation((eventName: string, handler: (event: { payload: unknown }) => void) => {
        if (eventName === 'indexation-complete') {
          completeHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      const onComplete = vi.fn();
      render(<IndexationProgress isIndexing={true} onComplete={onComplete} onError={() => {}} />);

      // Simulate completion event with final statistics
      await act(async () => {
        completeHandler?.({
          payload: {
            percentage: 100,
            bytesProcessed: 2000000000,
            totalBytes: 2000000000,
            speedGbps: 1.5,
            etaSeconds: 0,
            entriesIndexed: 71930527,
            totalEntriesEstimated: 71930527,
            batchesCompleted: 1,
            totalBatches: 1,
            partialFilterAvailable: true,
            entriesPerSecond: 301245,
            elapsedSeconds: 238, // 3:58
          },
        });
      });

      // Should show toast with statistics (AC6)
      expect(toast.default.success).toHaveBeenCalledWith(
        expect.stringContaining('Import complete!')
      );
      expect(toast.default.success).toHaveBeenCalledWith(
        expect.stringMatching(/71\.9M entries/)
      );
      expect(onComplete).toHaveBeenCalled();
    });
  });
});
