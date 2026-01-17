import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { ProgressBar } from '@/components/base/progress-bar';
import { Button } from '@/components/base/button';
import toast from 'react-hot-toast';

interface IndexProgress {
  percentage: number;
  bytesProcessed: number;
  totalBytes: number;
  speedGbps: number;
  etaSeconds: number;
}

interface IndexationProgressProps {
  isIndexing: boolean;
  onComplete: () => void;
  onError: (error: string) => void;
}

export function IndexationProgress({
  isIndexing,
  onComplete,
  onError,
}: IndexationProgressProps) {
  const [progress, setProgress] = useState<IndexProgress>({
    percentage: 0,
    bytesProcessed: 0,
    totalBytes: 0,
    speedGbps: 0,
    etaSeconds: 0,
  });

  useEffect(() => {
    if (!isIndexing) return;

    // Listen to progress events
    const progressUnlisten = listen<IndexProgress>('indexation-progress', (event) => {
      setProgress(event.payload);
    });

    // Listen to completion event
    const completeUnlisten = listen('indexation-complete', () => {
      toast.success('Indexation completed successfully');
      onComplete();
    });

    // Listen to error event
    const errorUnlisten = listen<string>('indexation-error', (event) => {
      toast.error(`Indexation failed: ${event.payload}`);
      onError(event.payload);
    });

    return () => {
      progressUnlisten.then(fn => fn());
      completeUnlisten.then(fn => fn());
      errorUnlisten.then(fn => fn());
    };
  }, [isIndexing, onComplete, onError]);

  const handleCancel = async () => {
    try {
      await invoke('cancel_indexation');
      toast.info('Indexation cancelled');
    } catch (error) {
      toast.error(`Failed to cancel: ${error}`);
    }
  };

  const formatBytes = (bytes: number): string => {
    return `${(bytes / (1024 ** 3)).toFixed(2)} GB`;
  };

  const formatEta = (seconds: number): string => {
    if (seconds < 60) return `${Math.round(seconds)} seconds`;
    const minutes = Math.floor(seconds / 60);
    return `${minutes} minute${minutes !== 1 ? 's' : ''}`;
  };

  if (!isIndexing) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-md">
        <h2 className="text-lg font-semibold mb-4">Indexing Log File</h2>

        <ProgressBar value={progress.percentage} max={100} />

        <div className="mt-4 space-y-2 text-sm text-gray-600 dark:text-gray-400">
          <div className="flex justify-between">
            <span>Progress:</span>
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
