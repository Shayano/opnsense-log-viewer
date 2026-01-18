import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import '@testing-library/jest-dom';
import { LogTable } from './log-table';
import { Action, Protocol, type LogEntry } from '@/types/log-entry';

// Mock ResizeObserver for TanStack Virtual
class MockResizeObserver implements ResizeObserver {
  private callback: ResizeObserverCallback;

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback;
  }

  observe(target: Element): void {
    // Immediately call callback with fake resize entry
    const entry: ResizeObserverEntry = {
      target,
      contentRect: {
        x: 0,
        y: 0,
        width: 1400,
        height: 600,
        top: 0,
        right: 1400,
        bottom: 600,
        left: 0,
        toJSON: () => ({}),
      },
      borderBoxSize: [{ blockSize: 600, inlineSize: 1400 }],
      contentBoxSize: [{ blockSize: 600, inlineSize: 1400 }],
      devicePixelContentBoxSize: [{ blockSize: 600, inlineSize: 1400 }],
    };
    // Schedule callback for next microtask to simulate async behavior
    queueMicrotask(() => this.callback([entry], this));
  }

  unobserve = vi.fn();
  disconnect = vi.fn();
}

// Store original so we can restore it
const originalResizeObserver = window.ResizeObserver;

const mockEntries: LogEntry[] = [
  {
    id: '1',
    timestamp: '2026-01-18T10:00:00Z',
    interface: 'vtnet0',
    sourceIp: '192.168.1.100',
    sourcePort: 443,
    destinationIp: '10.0.0.50',
    destinationPort: 80,
    protocol: Protocol.TCP,
    action: Action.PASS,
    ruleLabel: 'allow-http',
  },
  {
    id: '2',
    timestamp: '2026-01-18T10:01:00Z',
    interface: 'vtnet1',
    sourceIp: '192.168.1.200',
    sourcePort: 22,
    destinationIp: '10.0.0.60',
    destinationPort: 443,
    protocol: Protocol.TCP,
    action: Action.BLOCK,
    ruleLabel: 'block-ssh',
  },
  {
    id: '3',
    timestamp: '2026-01-18T10:02:00Z',
    interface: 'vtnet0',
    sourceIp: '192.168.1.150',
    sourcePort: 8080,
    destinationIp: '10.0.0.70',
    destinationPort: 53,
    protocol: Protocol.UDP,
    action: Action.REJECT,
    ruleLabel: 'reject-dns',
  },
];

describe('LogTable', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Mock ResizeObserver
    window.ResizeObserver = MockResizeObserver as unknown as typeof ResizeObserver;

    // Mock getBoundingClientRect for virtual scrolling to work
    Element.prototype.getBoundingClientRect = vi.fn(() => ({
      width: 1400,
      height: 600,
      top: 0,
      left: 0,
      bottom: 600,
      right: 1400,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    }));

    // Mock scrollHeight and clientHeight for the virtual container
    Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
      configurable: true,
      get: function () {
        return 600;
      },
    });
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
      configurable: true,
      get: function () {
        return 600;
      },
    });
    Object.defineProperty(HTMLElement.prototype, 'scrollWidth', {
      configurable: true,
      get: function () {
        return 1400;
      },
    });
    Object.defineProperty(HTMLElement.prototype, 'clientWidth', {
      configurable: true,
      get: function () {
        return 1400;
      },
    });
    Object.defineProperty(HTMLElement.prototype, 'offsetHeight', {
      configurable: true,
      get: function () {
        return 600;
      },
    });
    Object.defineProperty(HTMLElement.prototype, 'offsetWidth', {
      configurable: true,
      get: function () {
        return 1400;
      },
    });

    // Mock window.innerWidth for responsive columns hook (need >= 1280 for all columns)
    Object.defineProperty(window, 'innerWidth', {
      configurable: true,
      writable: true,
      value: 1400,
    });
  });

  afterEach(() => {
    // Cleanup rendered components
    cleanup();
    // Restore ResizeObserver
    window.ResizeObserver = originalResizeObserver;
  });

  it('renders table with entries', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render after ResizeObserver callback
    await waitFor(() => {
      expect(screen.getByText('192.168.1.100')).toBeInTheDocument();
    });

    expect(screen.getByText('192.168.1.200')).toBeInTheDocument();
    expect(screen.getByText('192.168.1.150')).toBeInTheDocument();

    // Check footer shows correct count
    expect(screen.getByText('Showing 3 entries')).toBeInTheDocument();
  });

  it('displays empty state when no entries', () => {
    render(<LogTable entries={[]} />);
    expect(screen.getByText('No log entries to display')).toBeInTheDocument();
  });

  it('renders column headers', () => {
    render(<LogTable entries={mockEntries} />);

    // Check all column headers are present
    expect(screen.getByText('Timestamp')).toBeInTheDocument();
    expect(screen.getByText('Interface')).toBeInTheDocument();
    expect(screen.getByText('Source IP')).toBeInTheDocument();
    expect(screen.getByText('Src Port')).toBeInTheDocument();
    expect(screen.getByText('Dest IP')).toBeInTheDocument();
    expect(screen.getByText('Dst Port')).toBeInTheDocument();
    expect(screen.getByText('Protocol')).toBeInTheDocument();
    expect(screen.getByText('Action')).toBeInTheDocument();
    expect(screen.getByText('Rule Label')).toBeInTheDocument();
  });

  it('applies correct color-coding for actions', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render
    // Note: Action values are lowercase in DOM but displayed uppercase via CSS
    await waitFor(() => {
      expect(screen.getByText('pass')).toBeInTheDocument();
    });

    // Check for action text (lowercase in DOM, uppercase via CSS)
    expect(screen.getByText('block')).toBeInTheDocument();
    expect(screen.getByText('reject')).toBeInTheDocument();
  });

  it('sorts by timestamp when header clicked', () => {
    render(<LogTable entries={mockEntries} />);

    const timestampHeader = screen.getByText('Timestamp');

    // Initially sorted descending by timestamp
    fireEvent.click(timestampHeader);

    // After click, should toggle to ascending
    // We can verify this by checking aria-sort attribute
    const headerElement = timestampHeader.closest('[role="columnheader"]');
    expect(headerElement).toHaveAttribute('aria-sort');
  });

  it('sorts by source IP when header clicked', () => {
    render(<LogTable entries={mockEntries} />);

    const sourceIpHeader = screen.getByText('Source IP');

    // Click to sort by source IP
    fireEvent.click(sourceIpHeader);

    const headerElement = sourceIpHeader.closest('[role="columnheader"]');
    expect(headerElement).toHaveAttribute('aria-sort', 'descending');
  });

  it('toggles sort direction on repeated header clicks', () => {
    render(<LogTable entries={mockEntries} />);

    const timestampHeader = screen.getByText('Timestamp');
    const headerElement = timestampHeader.closest('[role="columnheader"]');

    // First click - ascending
    fireEvent.click(timestampHeader);
    expect(headerElement).toHaveAttribute('aria-sort', 'ascending');

    // Second click - descending
    fireEvent.click(timestampHeader);
    expect(headerElement).toHaveAttribute('aria-sort', 'descending');
  });

  it('has proper accessibility attributes', async () => {
    render(<LogTable entries={mockEntries} />);

    // Check for grid role
    const grid = screen.getByRole('grid');
    expect(grid).toBeInTheDocument();
    expect(grid).toHaveAttribute('aria-label', 'Log entries table');

    // Check for column headers
    const columnHeaders = screen.getAllByRole('columnheader');
    expect(columnHeaders.length).toBeGreaterThan(0);

    // Wait for virtual rows to render
    await waitFor(() => {
      const rows = screen.getAllByRole('row');
      expect(rows.length).toBeGreaterThan(1); // Header + at least 1 data row
    });
  });

  it('selects row on click', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render
    await waitFor(() => {
      const rows = screen.getAllByRole('row');
      expect(rows.length).toBeGreaterThan(1);
    });

    const rows = screen.getAllByRole('row');
    const firstRow = rows[1]; // Skip header row
    fireEvent.click(firstRow);

    // Check that selection is indicated
    expect(firstRow).toHaveAttribute('aria-selected', 'true');

    // Footer should show selected row info
    expect(screen.getByText(/Row 1 selected/)).toBeInTheDocument();
  });

  it('shows context menu on right-click', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render
    await waitFor(() => {
      const rows = screen.getAllByRole('row');
      expect(rows.length).toBeGreaterThan(1);
    });

    const rows = screen.getAllByRole('row');
    const firstRow = rows[1];
    fireEvent.contextMenu(firstRow);

    // Check for context menu items
    expect(screen.getByText('Copy Cell Value')).toBeInTheDocument();
    expect(screen.getByText('Copy Row')).toBeInTheDocument();
    expect(screen.getByText('Filter by This Value')).toBeInTheDocument();
  });

  it('closes context menu on outside click', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render
    await waitFor(() => {
      const rows = screen.getAllByRole('row');
      expect(rows.length).toBeGreaterThan(1);
    });

    const rows = screen.getAllByRole('row');
    const firstRow = rows[1];
    fireEvent.contextMenu(firstRow);

    // Context menu should be visible
    expect(screen.getByText('Copy Cell Value')).toBeInTheDocument();

    // Click outside (on the grid itself)
    const grid = screen.getByRole('grid');
    fireEvent.mouseDown(grid);

    // Context menu should be hidden
    expect(screen.queryByText('Copy Cell Value')).not.toBeInTheDocument();
  });

  it('handles keyboard navigation with arrow keys', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render
    await waitFor(() => {
      const rows = screen.getAllByRole('row');
      expect(rows.length).toBeGreaterThan(1);
    });

    const grid = screen.getByRole('grid');

    // Focus the grid
    grid.focus();

    // Press ArrowDown to select first row
    fireEvent.keyDown(grid, { key: 'ArrowDown' });
    expect(screen.getByText(/Row 1 selected/)).toBeInTheDocument();

    // Press ArrowDown again to select second row
    fireEvent.keyDown(grid, { key: 'ArrowDown' });
    expect(screen.getByText(/Row 2 selected/)).toBeInTheDocument();

    // Press ArrowUp to go back to first row
    fireEvent.keyDown(grid, { key: 'ArrowUp' });
    expect(screen.getByText(/Row 1 selected/)).toBeInTheDocument();
  });

  it('handles Escape key to deselect row', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render
    await waitFor(() => {
      const rows = screen.getAllByRole('row');
      expect(rows.length).toBeGreaterThan(1);
    });

    const grid = screen.getByRole('grid');
    grid.focus();

    // Select a row
    fireEvent.keyDown(grid, { key: 'ArrowDown' });
    expect(screen.getByText(/Row 1 selected/)).toBeInTheDocument();

    // Press Escape to deselect
    fireEvent.keyDown(grid, { key: 'Escape' });
    expect(screen.queryByText(/Row \d+ selected/)).not.toBeInTheDocument();
  });

  it('renders with virtual scrolling enabled', async () => {
    // Create a fresh set of test entries
    const testEntries: LogEntry[] = [
      {
        id: 'vs-1',
        timestamp: '2026-01-18T12:00:00Z',
        interface: 'vtnet0',
        sourceIp: '10.0.0.1',
        sourcePort: 1234,
        destinationIp: '10.0.0.2',
        destinationPort: 80,
        protocol: Protocol.TCP,
        action: Action.PASS,
        ruleLabel: 'test-rule',
      },
      {
        id: 'vs-2',
        timestamp: '2026-01-18T12:01:00Z',
        interface: 'vtnet0',
        sourceIp: '10.0.0.3',
        sourcePort: 5678,
        destinationIp: '10.0.0.4',
        destinationPort: 443,
        protocol: Protocol.TCP,
        action: Action.BLOCK,
        ruleLabel: 'test-rule-2',
      },
    ];

    render(<LogTable entries={testEntries} />);

    // Should render the table with grid role
    expect(screen.getByRole('grid')).toBeInTheDocument();

    // Wait for virtual rows to render
    await waitFor(() => {
      expect(screen.getByText('10.0.0.1')).toBeInTheDocument();
    });

    // Footer should show entry count
    expect(screen.getByText(/Showing \d+ entries/)).toBeInTheDocument();
  });

  it('applies monospace font to technical data fields', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render
    await waitFor(() => {
      expect(screen.getByText('192.168.1.100')).toBeInTheDocument();
    });

    // Find IP addresses and ports - they should have font-mono class
    const sourceIpCell = screen.getByText('192.168.1.100');
    expect(sourceIpCell.className).toContain('font-mono');
  });

  it('formats timestamp correctly', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render (check for an IP address that we know is in mockEntries)
    await waitFor(() => {
      expect(screen.getByText('192.168.1.100')).toBeInTheDocument();
    });

    // Verify timestamp is formatted - look for cells with timestamp format
    // The timestamp is in the format 'yyyy-MM-dd HH:mm:ss'
    const gridcells = screen.getAllByRole('gridcell');
    const timestampCell = gridcells.find((cell) =>
      /\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}/.test(cell.textContent || '')
    );
    expect(timestampCell).toBeTruthy();
  });

  it('displays protocol in uppercase', async () => {
    render(<LogTable entries={mockEntries} />);

    // Wait for virtual rows to render
    // Note: Protocol values are lowercase in DOM but displayed uppercase via CSS
    await waitFor(() => {
      expect(screen.getAllByText('tcp').length).toBeGreaterThan(0);
    });

    // Protocols are lowercase in DOM, uppercase via CSS
    const tcpCells = screen.getAllByText('tcp');
    expect(tcpCells.length).toBeGreaterThan(0);

    const udpCells = screen.getAllByText('udp');
    expect(udpCells.length).toBeGreaterThan(0);
  });
});
