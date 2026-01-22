import { describe, it, expect, beforeEach } from 'vitest';
import { useFileStore } from './file-store';
import type { IndexProgress } from '@/types/file';

describe('useFileStore', () => {
  beforeEach(() => {
    // Reset store before each test
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

  describe('progressive loading state (Story 6.5)', () => {
    describe('setIndexProgress', () => {
      it('should update indexProgress with valid progress data', () => {
        const { setIndexProgress } = useFileStore.getState();
        const progress: IndexProgress = {
          percentage: 50,
          bytesProcessed: 1073741824, // 1GB
          totalBytes: 2147483648, // 2GB
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

        setIndexProgress(progress);

        const state = useFileStore.getState();
        expect(state.indexProgress).toEqual(progress);
        expect(state.indexProgress?.percentage).toBe(50);
        expect(state.indexProgress?.batchesCompleted).toBe(3);
        expect(state.indexProgress?.totalBatches).toBe(7);
      });

      it('should clear indexProgress when set to null', () => {
        const { setIndexProgress } = useFileStore.getState();
        const progress: IndexProgress = {
          percentage: 75,
          bytesProcessed: 1500000000,
          totalBytes: 2000000000,
          speedGbps: 2.0,
          etaSeconds: 30,
          entriesIndexed: 7500000,
          totalEntriesEstimated: 10000000,
          batchesCompleted: 5,
          totalBatches: 7,
          partialFilterAvailable: true,
          entriesPerSecond: 125000,
          elapsedSeconds: 60,
        };

        setIndexProgress(progress);
        expect(useFileStore.getState().indexProgress).not.toBeNull();

        setIndexProgress(null);
        expect(useFileStore.getState().indexProgress).toBeNull();
      });

      it('should update indexProgress independently of other state', () => {
        const { setIndexProgress, setError } = useFileStore.getState();

        // Note: setError sets isLoading to false by design
        setError('Some error');

        const progress: IndexProgress = {
          percentage: 25,
          bytesProcessed: 500000000,
          totalBytes: 2000000000,
          speedGbps: 1.0,
          etaSeconds: 120,
          entriesIndexed: 2500000,
          totalEntriesEstimated: 10000000,
          batchesCompleted: 2,
          totalBatches: 8,
          partialFilterAvailable: false,
          entriesPerSecond: 41667,
          elapsedSeconds: 60,
        };

        setIndexProgress(progress);

        const state = useFileStore.getState();
        expect(state.indexProgress).toEqual(progress);
        expect(state.error).toBe('Some error');
        // isLoading is false because setError clears it (expected behavior)
        expect(state.isLoading).toBe(false);
      });
    });

    describe('setPartialFilterAvailable', () => {
      it('should set partialFilterAvailable to true', () => {
        const { setPartialFilterAvailable } = useFileStore.getState();

        expect(useFileStore.getState().partialFilterAvailable).toBe(false);

        setPartialFilterAvailable(true);

        expect(useFileStore.getState().partialFilterAvailable).toBe(true);
      });

      it('should set partialFilterAvailable to false', () => {
        const { setPartialFilterAvailable } = useFileStore.getState();

        setPartialFilterAvailable(true);
        expect(useFileStore.getState().partialFilterAvailable).toBe(true);

        setPartialFilterAvailable(false);
        expect(useFileStore.getState().partialFilterAvailable).toBe(false);
      });
    });

    describe('setCacheHit', () => {
      it('should set cacheHit to true', () => {
        const { setCacheHit } = useFileStore.getState();

        expect(useFileStore.getState().cacheHit).toBe(false);

        setCacheHit(true);

        expect(useFileStore.getState().cacheHit).toBe(true);
      });

      it('should set cacheHit to false', () => {
        const { setCacheHit } = useFileStore.getState();

        setCacheHit(true);
        expect(useFileStore.getState().cacheHit).toBe(true);

        setCacheHit(false);
        expect(useFileStore.getState().cacheHit).toBe(false);
      });
    });

    describe('clearFile', () => {
      it('should reset all progressive loading fields', () => {
        const { setIndexProgress, setPartialFilterAvailable, setCacheHit, clearFile } =
          useFileStore.getState();

        const progress: IndexProgress = {
          percentage: 100,
          bytesProcessed: 2000000000,
          totalBytes: 2000000000,
          speedGbps: 2.5,
          etaSeconds: 0,
          entriesIndexed: 10000000,
          totalEntriesEstimated: 10000000,
          batchesCompleted: 7,
          totalBatches: 7,
          partialFilterAvailable: true,
          entriesPerSecond: 100000,
          elapsedSeconds: 100,
        };

        setIndexProgress(progress);
        setPartialFilterAvailable(true);
        setCacheHit(true);

        expect(useFileStore.getState().indexProgress).not.toBeNull();
        expect(useFileStore.getState().partialFilterAvailable).toBe(true);
        expect(useFileStore.getState().cacheHit).toBe(true);

        clearFile();

        const state = useFileStore.getState();
        expect(state.indexProgress).toBeNull();
        expect(state.partialFilterAvailable).toBe(false);
        expect(state.cacheHit).toBe(false);
        expect(state.currentFile).toBeNull();
        expect(state.indexMetadata).toBeNull();
        expect(state.error).toBeNull();
      });

      it('should reset isLoading to false', () => {
        const { setLoading, clearFile } = useFileStore.getState();

        setLoading(true);
        expect(useFileStore.getState().isLoading).toBe(true);

        clearFile();

        expect(useFileStore.getState().isLoading).toBe(false);
      });
    });

    describe('typed selectors', () => {
      it('should return indexProgress via selector', () => {
        const { setIndexProgress } = useFileStore.getState();
        const progress: IndexProgress = {
          percentage: 42,
          bytesProcessed: 840000000,
          totalBytes: 2000000000,
          speedGbps: 1.8,
          etaSeconds: 45,
          entriesIndexed: 4200000,
          totalEntriesEstimated: 10000000,
          batchesCompleted: 3,
          totalBatches: 7,
          partialFilterAvailable: false,
          entriesPerSecond: 70000,
          elapsedSeconds: 60,
        };

        setIndexProgress(progress);

        // Test selector returns current state
        const indexProgress = useFileStore.getState().indexProgress;
        expect(indexProgress?.percentage).toBe(42);
        expect(indexProgress?.batchesCompleted).toBe(3);
      });

      it('should return partialFilterAvailable via selector', () => {
        const { setPartialFilterAvailable } = useFileStore.getState();

        setPartialFilterAvailable(true);

        const isAvailable = useFileStore.getState().partialFilterAvailable;
        expect(isAvailable).toBe(true);
      });

      it('should return cacheHit via selector', () => {
        const { setCacheHit } = useFileStore.getState();

        setCacheHit(true);

        const isCacheHit = useFileStore.getState().cacheHit;
        expect(isCacheHit).toBe(true);
      });
    });
  });

  describe('existing functionality preservation', () => {
    it('should set current file', () => {
      const { setCurrentFile } = useFileStore.getState();
      const file = {
        path: '/path/to/file.log',
        size: 1000000,
        format: 'RFC3164' as const,
        selectedAt: new Date(),
      };

      setCurrentFile(file);

      const state = useFileStore.getState();
      expect(state.currentFile).toEqual(file);
      expect(state.error).toBeNull();
    });

    it('should set loading state', () => {
      const { setLoading } = useFileStore.getState();

      setLoading(true);
      expect(useFileStore.getState().isLoading).toBe(true);

      setLoading(false);
      expect(useFileStore.getState().isLoading).toBe(false);
    });

    it('should set error state', () => {
      const { setError } = useFileStore.getState();

      setError('Test error message');

      const state = useFileStore.getState();
      expect(state.error).toBe('Test error message');
      expect(state.isLoading).toBe(false);
    });

    it('should set index metadata', () => {
      const { setIndexMetadata } = useFileStore.getState();
      const metadata = {
        sourceFileHash: 'abc123',
        entryCount: 50000,
        format: 'RFC5424' as const,
        indexSizeBytes: 5000000,
        createdAt: '2026-01-22T10:00:00Z',
      };

      setIndexMetadata(metadata);

      const state = useFileStore.getState();
      expect(state.indexMetadata).toEqual(metadata);
      expect(state.isLoading).toBe(false);
    });
  });
});
