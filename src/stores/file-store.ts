import { create } from 'zustand';
import type { FileInfo, IndexMetadata, IndexProgress } from '@/types/file';

/**
 * File store state interface
 */
interface FileState {
  /** Currently selected file */
  currentFile: FileInfo | null;
  /** Index metadata for the current file */
  indexMetadata: IndexMetadata | null;
  /** Loading state */
  isLoading: boolean;
  /** Error message */
  error: string | null;
  /** Progressive loading: current indexation progress */
  indexProgress: IndexProgress | null;
  /** Progressive loading: true when first batch complete, filtering can begin */
  partialFilterAvailable: boolean;
  /** Progressive loading: true when index loaded from cache (instant load) */
  cacheHit: boolean;

  // Actions
  /** Set the currently selected file */
  setCurrentFile: (file: FileInfo) => void;
  /** Set the index metadata */
  setIndexMetadata: (metadata: IndexMetadata) => void;
  /** Set loading state */
  setLoading: (loading: boolean) => void;
  /** Set error message */
  setError: (error: string | null) => void;
  /** Clear all file data */
  clearFile: () => void;
  /** Set indexation progress (progressive loading) */
  setIndexProgress: (progress: IndexProgress | null) => void;
  /** Set partial filter availability (progressive loading) */
  setPartialFilterAvailable: (isAvailable: boolean) => void;
  /** Set cache hit status (progressive loading) */
  setCacheHit: (isCacheHit: boolean) => void;
}

/**
 * File store (ephemeral, not persisted to localStorage)
 */
export const useFileStore = create<FileState>((set) => ({
  currentFile: null,
  indexMetadata: null,
  isLoading: false,
  error: null,
  indexProgress: null,
  partialFilterAvailable: false,
  cacheHit: false,

  setCurrentFile: (file) => set({ currentFile: file, error: null }),
  setIndexMetadata: (metadata) => set({ indexMetadata: metadata, isLoading: false }),
  setLoading: (loading) => set({ isLoading: loading, error: null }),
  setError: (error) => set({ error, isLoading: false }),
  clearFile: () =>
    set({
      currentFile: null,
      indexMetadata: null,
      isLoading: false,
      error: null,
      indexProgress: null,
      partialFilterAvailable: false,
      cacheHit: false,
    }),
  setIndexProgress: (progress) => set({ indexProgress: progress }),
  setPartialFilterAvailable: (isAvailable) => set({ partialFilterAvailable: isAvailable }),
  setCacheHit: (isCacheHit) => set({ cacheHit: isCacheHit }),
}));

// Typed selectors for components (prevents unnecessary re-renders)
export const useCurrentFile = () => useFileStore((state) => state.currentFile);
export const useIndexMetadata = () => useFileStore((state) => state.indexMetadata);
export const useIsLoading = () => useFileStore((state) => state.isLoading);
export const useFileError = () => useFileStore((state) => state.error);
export const useIndexProgress = () => useFileStore((state) => state.indexProgress);
export const usePartialFilterAvailable = () => useFileStore((state) => state.partialFilterAvailable);
export const useCacheHit = () => useFileStore((state) => state.cacheHit);
