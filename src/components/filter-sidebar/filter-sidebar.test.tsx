import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { FilterSidebar } from './filter-sidebar';
import { useFilterStore } from '@/stores/filter-store';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => { store[key] = value; },
    clear: () => { store = {}; },
  };
})();
Object.defineProperty(window, 'localStorage', { value: localStorageMock });

describe('FilterSidebar', () => {
  beforeEach(() => {
    // Reset store before each test
    useFilterStore.setState({
      filters: [],
      draftMode: false,
    });
    localStorageMock.clear();
  });

  it('should render expanded by default', () => {
    render(<FilterSidebar />);
    expect(screen.getByText('Filters')).toBeInTheDocument();
    expect(screen.getByText('Add Filter')).toBeInTheDocument();
  });

  it('should display empty state when no filters', () => {
    render(<FilterSidebar />);
    expect(screen.getByText('No filters added yet.')).toBeInTheDocument();
    expect(screen.getByText('Click "Add Filter" to start.')).toBeInTheDocument();
  });

  it('should collapse sidebar when collapse button is clicked', () => {
    render(<FilterSidebar />);

    const collapseButton = screen.getByLabelText('Collapse sidebar');
    fireEvent.click(collapseButton);

    expect(screen.queryByText('Filters')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Expand sidebar')).toBeInTheDocument();
  });

  it('should expand sidebar when expand button is clicked', () => {
    render(<FilterSidebar />);

    // Collapse first
    const collapseButton = screen.getByLabelText('Collapse sidebar');
    fireEvent.click(collapseButton);

    // Then expand
    const expandButton = screen.getByLabelText('Expand sidebar');
    fireEvent.click(expandButton);

    expect(screen.getByText('Filters')).toBeInTheDocument();
  });

  it('should persist collapsed state in localStorage', () => {
    render(<FilterSidebar />);

    const collapseButton = screen.getByLabelText('Collapse sidebar');
    fireEvent.click(collapseButton);

    expect(localStorageMock.getItem('filter-sidebar-collapsed')).toBe('true');

    const expandButton = screen.getByLabelText('Expand sidebar');
    fireEvent.click(expandButton);

    expect(localStorageMock.getItem('filter-sidebar-collapsed')).toBe('false');
  });

  it('should show filter count badge when collapsed', () => {
    const { addFilter } = useFilterStore.getState();
    addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
    addFilter({ field: 'destinationPort', operator: 'equals', value: 443 });

    render(<FilterSidebar />);

    // Collapse sidebar
    const collapseButton = screen.getByLabelText('Collapse sidebar');
    fireEvent.click(collapseButton);

    // Should show badge with count
    expect(screen.getByText('2')).toBeInTheDocument();
  });

  it('should display filters when present', () => {
    const { addFilter } = useFilterStore.getState();
    addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });

    render(<FilterSidebar />);

    expect(screen.getByText('Active Filters (1)')).toBeInTheDocument();
    expect(screen.getByText('192.168.1.1')).toBeInTheDocument();
  });

  it('should show draft mode indicator when in draft mode', () => {
    const { addFilter } = useFilterStore.getState();
    addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });

    render(<FilterSidebar />);

    expect(screen.getByText('Draft')).toBeInTheDocument();
  });

  it('should show Search button when filters exist in draft mode', () => {
    const { addFilter } = useFilterStore.getState();
    addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
    addFilter({ field: 'destinationPort', operator: 'equals', value: 443 });

    render(<FilterSidebar />);

    const searchButton = screen.getByLabelText('Execute search with 2 filters');
    expect(searchButton).toBeInTheDocument();
    expect(searchButton).toHaveTextContent('Search (2 filters)');
  });

  it('should transition to active mode when Search button is clicked', () => {
    const { addFilter } = useFilterStore.getState();
    addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });

    render(<FilterSidebar />);

    const searchButton = screen.getByLabelText('Execute search with 1 filter');
    fireEvent.click(searchButton);

    const state = useFilterStore.getState();
    expect(state.draftMode).toBe(false);
  });

  it('should not show Search button when not in draft mode', () => {
    const { addFilter, setDraftMode } = useFilterStore.getState();
    addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
    setDraftMode(false);

    render(<FilterSidebar />);

    expect(screen.queryByText(/Search/)).not.toBeInTheDocument();
  });

  it('should open FilterBuilder modal when Add Filter is clicked', () => {
    render(<FilterSidebar />);

    const addButton = screen.getByText('Add Filter');
    fireEvent.click(addButton);

    // Modal should open (FilterBuilder component)
    expect(screen.getByLabelText('Select field')).toBeInTheDocument();
  });

  it('should display Clear All button when filters exist', () => {
    const { addFilter } = useFilterStore.getState();
    addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });

    render(<FilterSidebar />);

    expect(screen.getByLabelText('Clear all filters')).toBeInTheDocument();
  });

  it('should clear all filters when Clear All is clicked', () => {
    const { addFilter } = useFilterStore.getState();
    addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });
    addFilter({ field: 'destinationPort', operator: 'equals', value: 443 });

    render(<FilterSidebar />);

    const clearButton = screen.getByLabelText('Clear all filters');
    fireEvent.click(clearButton);

    const state = useFilterStore.getState();
    expect(state.filters).toHaveLength(0);
  });
});
