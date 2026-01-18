import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { formatDistance } from 'date-fns';
import { Button } from '@/components/base/button';
import { Trash2, AlertCircle, CheckCircle, RefreshCw } from 'lucide-react';
import { Modal } from '@/components/base/modal';
import { toast } from '@/components/base/toaster';
import type { IndexInfo } from '@/types/file';

/**
 * ManageIndexes component
 * Displays and manages saved index files
 */
export function ManageIndexes() {
  const [indexes, setIndexes] = useState<IndexInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [deleteHash, setDeleteHash] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    loadIndexes();
  }, []);

  /**
   * Load all saved indexes from backend
   */
  const loadIndexes = async () => {
    try {
      setLoading(true);
      const result = await invoke<IndexInfo[]>('list_all_indexes');
      setIndexes(result);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to load indexes';
      toast.error(`Failed to load indexes: ${errorMessage}`);
    } finally {
      setLoading(false);
    }
  };

  /**
   * Delete an index by hash
   */
  const handleDelete = async (hash: string) => {
    try {
      setDeleting(true);
      await invoke('delete_index_by_hash', { hash });
      toast.success('Index deleted successfully');
      await loadIndexes(); // Refresh list
      setDeleteHash(null);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to delete index';
      toast.error(`Failed to delete index: ${errorMessage}`);
    } finally {
      setDeleting(false);
    }
  };

  /**
   * Format bytes to human-readable string
   */
  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  /**
   * Format date to relative time (e.g., "2 hours ago")
   */
  const formatDate = (dateStr: string): string => {
    const date = new Date(dateStr);
    return formatDistance(date, new Date(), { addSuffix: true });
  };

  /**
   * Extract filename from full path
   */
  const getFileName = (path: string): string => {
    return path.split(/[/\\]/).pop() || path;
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="text-gray-500 dark:text-gray-400">Loading indexes...</div>
      </div>
    );
  }

  if (indexes.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-center">
        <div className="text-gray-500 dark:text-gray-400 mb-2">No saved indexes found</div>
        <div className="text-sm text-gray-400 dark:text-gray-500">
          Indexes are created automatically when you open log files
        </div>
      </div>
    );
  }

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">Manage Indexes</h2>
        <Button
          onClick={loadIndexes}
          variant="ghost"
          size="sm"
          disabled={loading}
          className="inline-flex items-center gap-2"
          aria-label="Refresh index list"
        >
          <RefreshCw size={16} className={loading ? 'animate-spin' : ''} />
          Refresh
        </Button>
      </div>

      <div className="space-y-2">
        {indexes.map((index) => (
          <div
            key={index.hash}
            className="border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:bg-gray-50 dark:hover:bg-gray-800 transition"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1 min-w-0">
                {/* Source File Path */}
                <div className="flex items-center gap-2 mb-1">
                  {index.sourceFileExists ? (
                    <CheckCircle className="w-4 h-4 text-green-500 flex-shrink-0" />
                  ) : (
                    <AlertCircle className="w-4 h-4 text-orange-500 flex-shrink-0" />
                  )}
                  <span
                    className="font-mono text-sm truncate text-gray-900 dark:text-gray-100"
                    title={index.sourceFilePath}
                  >
                    {getFileName(index.sourceFilePath)}
                  </span>
                </div>

                {/* Full path (truncated) */}
                <div className="text-xs text-gray-500 dark:text-gray-400 truncate mb-2">
                  {index.sourceFilePath}
                </div>

                {/* Metadata */}
                <div className="text-xs text-gray-500 dark:text-gray-400 space-y-0.5">
                  <div>Created {formatDate(index.createdAt)}</div>
                  <div className="flex gap-4">
                    <span>Source: {formatBytes(index.fileSize)}</span>
                    <span>Index: {formatBytes(index.indexFileSize)}</span>
                    <span>{index.entryCount.toLocaleString()} entries</span>
                  </div>
                  {!index.sourceFileExists && (
                    <div className="text-orange-500 mt-1">⚠ Source file not found</div>
                  )}
                </div>
              </div>

              {/* Delete Button */}
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setDeleteHash(index.hash)}
                className="text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                aria-label="Delete index"
              >
                <Trash2 className="w-4 h-4" />
              </Button>
            </div>
          </div>
        ))}
      </div>

      {/* Confirmation Modal */}
      {deleteHash && (
        <Modal
          isOpen={!!deleteHash}
          onClose={() => !deleting && setDeleteHash(null)}
          title="Delete Index?"
        >
          <div className="space-y-4">
            <p className="text-gray-600 dark:text-gray-400">
              This will delete the index file. The source log file will not be affected.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setDeleteHash(null)} disabled={deleting}>
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={() => deleteHash && handleDelete(deleteHash)}
                disabled={deleting}
                className="bg-red-500 hover:bg-red-600"
              >
                {deleting ? 'Deleting...' : 'Delete'}
              </Button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
