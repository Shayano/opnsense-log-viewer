import { create } from 'zustand';
import type { FileInfo, IndexMetadata } from '@/types/file';

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
}

/**
 * File store (ephemeral, not persisted to localStorage)
 */
export const useFileStore = create<FileState>((set) => ({
  currentFile: null,
  indexMetadata: null,
  isLoading: false,
  error: null,

  setCurrentFile: (file) => set({ currentFile: file, error: null }),
  setIndexMetadata: (metadata) => set({ indexMetadata: metadata, isLoading: false }),
  setLoading: (loading) => set({ isLoading: loading, error: null }),
  setError: (error) => set({ error, isLoading: false }),
  clearFile: () => set({ currentFile: null, indexMetadata: null, error: null }),
}));

// Typed selectors for components (prevents unnecessary re-renders)
export const useCurrentFile = () => useFileStore((state) => state.currentFile);
export const useIndexMetadata = () => useFileStore((state) => state.indexMetadata);
export const useIsLoading = () => useFileStore((state) => state.isLoading);
export const useFileError = () => useFileStore((state) => state.error);
