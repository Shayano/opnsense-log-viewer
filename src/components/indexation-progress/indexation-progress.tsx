import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { CheckCircle, Zap } from 'lucide-react';
import { ProgressBar } from '@/components/base/progress-bar';
import { Button } from '@/components/base/button';
import { useFileStore } from '@/stores/file-store';
import toast from 'react-hot-toast';
import type { IndexProgress, IndexCacheEvent } from '@/types/file';

interface IndexationProgressProps {
  isIndexing: boolean;
  onComplete: () => void;
  onError: (error: string) => void;
}

export function IndexationProgress({ isIndexing, onComplete, onError }: IndexationProgressProps) {
  const [progress, setProgress] = useState<IndexProgress>({
    percentage: 0,
    bytesProcessed: 0,
    totalBytes: 0,
    speedGbps: 0,
    etaSeconds: 0,
    entriesProcessed: 0,
    totalEntriesEstimate: 0,
    currentBatch: 0,
    totalBatches: 0,
    partialFilterAvailable: false,
  });

  // Store actions for progressive loading state
  const setIndexProgress = useFileStore((state) => state.setIndexProgress);
  const setPartialFilterAvailable = useFileStore((state) => state.setPartialFilterAvailable);
  const setCacheHit = useFileStore((state) => state.setCacheHit);
  const cacheHit = useFileStore((state) => state.cacheHit);

  useEffect(() => {
    if (!isIndexing) return;

    // Listen to cache hit event (Story 6.4)
    const cacheHitUnlisten = listen<IndexCacheEvent>('index-cache-hit', () => {
      setCacheHit(true);
      setPartialFilterAvailable(true);
      toast.success('Index loaded from cache', { icon: '⚡' });
      // Cache hit means instant load - trigger completion
      onComplete();
    });

    // Listen to cache miss event (Story 6.4)
    const cacheMissUnlisten = listen<IndexCacheEvent>('index-cache-miss', () => {
      setCacheHit(false);
      // Normal progressive indexation will begin
    });

    // Listen to progress events
    const progressUnlisten = listen<IndexProgress>('indexation-progress', (event) => {
      const progressData = event.payload;
      setProgress(progressData);
      setIndexProgress(progressData);

      // Update partial filter availability in store
      if (progressData.partialFilterAvailable) {
        setPartialFilterAvailable(true);
      }
    });

    // Listen to completion event
    const completeUnlisten = listen('indexation-complete', () => {
      toast.success('Indexation completed successfully');
      setIndexProgress(null);
      onComplete();
    });

    // Listen to error event
    const errorUnlisten = listen<string>('indexation-error', (event) => {
      toast.error(`Indexation failed: ${event.payload}`);
      setIndexProgress(null);
      onError(event.payload);
    });

    return () => {
      cacheHitUnlisten.then((fn) => fn());
      cacheMissUnlisten.then((fn) => fn());
      progressUnlisten.then((fn) => fn());
      completeUnlisten.then((fn) => fn());
      errorUnlisten.then((fn) => fn());
    };
  }, [isIndexing, onComplete, onError, setIndexProgress, setPartialFilterAvailable, setCacheHit]);

  const handleCancel = async () => {
    try {
      await invoke('cancel_indexation');
      toast.success('Indexation cancelled');
    } catch (error) {
      toast.error(`Failed to cancel: ${error}`);
    }
  };

  const formatBytes = (bytes: number): string => {
    return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
  };

  const formatEta = (seconds: number): string => {
    if (seconds < 60) return `${Math.round(seconds)} seconds`;
    const minutes = Math.floor(seconds / 60);
    return `${minutes} minute${minutes !== 1 ? 's' : ''}`;
  };

  const formatEntries = (count: number): string => {
    if (count >= 1_000_000) {
      return `${(count / 1_000_000).toFixed(1)}M`;
    }
    if (count >= 1_000) {
      return `${(count / 1_000).toFixed(1)}K`;
    }
    return count.toString();
  };

  if (!isIndexing) return null;

  // Cache hit display - instant load success (parent dismisses via onComplete callback)
  if (cacheHit) {
    return (
      <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
        <div className="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-md">
          <div className="text-center py-4">
            <Zap className="w-12 h-12 text-green-500 mx-auto mb-3" />
            <p className="text-lg font-medium text-gray-900 dark:text-gray-100">
              Loaded from cache
            </p>
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">Instant load</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-md">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Indexing Log File
          </h2>
          <span className="text-2xl font-bold text-blue-600 dark:text-blue-400">
            {progress.percentage}%
          </span>
        </div>

        <ProgressBar value={progress.percentage} />

        {/* Partial filtering available badge */}
        {progress.partialFilterAvailable && (
          <div
            className="mt-3 flex items-center gap-2 px-3 py-1.5 bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300 rounded-full text-sm font-medium w-fit cursor-help"
            title="You can start filtering now with partial results"
          >
            <CheckCircle className="w-4 h-4" />
            Partial filtering available
          </div>
        )}

        <div className="mt-4 space-y-2 text-sm text-gray-600 dark:text-gray-400">
          {/* Entries progress (Story 6.5 - Task 17.2) */}
          {progress.totalEntriesEstimate > 0 && (
            <div className="flex justify-between">
              <span>Entries:</span>
              <span className="font-mono">
                {formatEntries(progress.entriesProcessed)} / {formatEntries(progress.totalEntriesEstimate)} entries
              </span>
            </div>
          )}

          {/* Batch progress (Story 6.5 - Task 17.3) */}
          {progress.totalBatches > 0 && (
            <div className="flex justify-between">
              <span>Batch:</span>
              <span className="font-mono">
                Batch {progress.currentBatch} / {progress.totalBatches}
              </span>
            </div>
          )}

          <div className="flex justify-between">
            <span>Data processed:</span>
            <span className="font-mono">
              {formatBytes(progress.bytesProcessed)} / {formatBytes(progress.totalBytes)}
            </span>
          </div>

          <div className="flex justify-between">
            <span>Speed:</span>
            <span className="font-mono">{progress.speedGbps.toFixed(2)} GB/min</span>
          </div>

          <div className="flex justify-between">
            <span>Estimated time remaining:</span>
            <span className="font-mono">{formatEta(progress.etaSeconds)}</span>
          </div>
        </div>

        <div className="mt-6 flex justify-end">
          <Button variant="ghost" onClick={handleCancel}>
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
}
