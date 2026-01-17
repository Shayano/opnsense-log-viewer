import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { IndexationProgress } from './indexation-progress';

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
  it('should not render when not indexing', () => {
    const { container } = render(
      <IndexationProgress
        isIndexing={false}
        onComplete={() => {}}
        onError={() => {}}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it('should render progress UI when indexing', () => {
    render(
      <IndexationProgress
        isIndexing={true}
        onComplete={() => {}}
        onError={() => {}}
      />
    );

    expect(screen.getByText('Indexing Log File')).toBeInTheDocument();
    expect(screen.getByText('Progress:')).toBeInTheDocument();
    expect(screen.getByText('Speed:')).toBeInTheDocument();
    expect(screen.getByText('Estimated time remaining:')).toBeInTheDocument();
    expect(screen.getByText('Cancel')).toBeInTheDocument();
  });
});
