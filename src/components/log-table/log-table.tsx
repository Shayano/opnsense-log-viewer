import React, { useRef, useState, useMemo, useCallback, useEffect } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { LogEntry } from '@/types/log-entry';
import { LogTableHeader, type SortColumn, type SortDirection } from './log-table-header';
import { LogTableRow } from './log-table-row';
import { ContextMenu } from './context-menu';
import { useResponsiveColumns } from './use-responsive-columns';
import { EntryDetailView } from '@/components/entry-detail-view';
import { enrichRuleLabels, enrichAliases } from '@/services/enrichment-service';

interface LogTableProps {
  entries: LogEntry[];
  onFilterByValue?: (value: string) => void;
  onRowSelect?: (entry: LogEntry) => void;
}

interface ContextMenuState {
  visible: boolean;
  position: { x: number; y: number };
  entry: LogEntry | null;
  cellValue?: string;
}

// Helper: Convert IP to number for sorting
function ipToNumber(ip: string): number {
  const parts = ip.split('.').map(Number);
  if (parts.length !== 4 || parts.some((p) => isNaN(p))) {
    return 0;
  }
  return parts[0] * 16777216 + parts[1] * 65536 + parts[2] * 256 + parts[3];
}

export function LogTable({ entries, onFilterByValue, onRowSelect }: LogTableProps): React.JSX.Element {
  const parentRef = useRef<HTMLDivElement>(null);
  const [sortColumn, setSortColumn] = useState<SortColumn>('timestamp');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');
  const [selectedRowIndex, setSelectedRowIndex] = useState<number | null>(null);
  const [detailPaneOpen, setDetailPaneOpen] = useState(false);
  const [selectedEntry, setSelectedEntry] = useState<LogEntry | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    visible: false,
    position: { x: 0, y: 0 },
    entry: null,
  });

  const columnVisibility = useResponsiveColumns();

  // Sort entries based on current sort state
  const sortedEntries = useMemo(() => {
    const sorted = [...entries].sort((a, b) => {
      let compareResult = 0;

      switch (sortColumn) {
        case 'timestamp':
          compareResult = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
          break;
        case 'sourceIp':
          compareResult = ipToNumber(a.sourceIp) - ipToNumber(b.sourceIp);
          break;
        case 'destinationIp':
          compareResult = ipToNumber(a.destinationIp) - ipToNumber(b.destinationIp);
          break;
      }

      return sortDirection === 'asc' ? compareResult : -compareResult;
    });

    return sorted;
  }, [entries, sortColumn, sortDirection]);

  // TanStack Virtual setup
  const rowVirtualizer = useVirtualizer({
    count: sortedEntries.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 32, // Row height in pixels
    overscan: 10, // Render 10 extra rows above/below viewport
  });

  const handleSort = useCallback(
    (column: SortColumn): void => {
      if (sortColumn === column) {
        setSortDirection((prev) => (prev === 'asc' ? 'desc' : 'asc'));
      } else {
        setSortColumn(column);
        setSortDirection('desc');
      }
    },
    [sortColumn]
  );

  const handleRowSelect = useCallback(
    (index: number): void => {
      setSelectedRowIndex(index);
      setSelectedEntry(sortedEntries[index]);
      setDetailPaneOpen(true);
      if (onRowSelect) {
        onRowSelect(sortedEntries[index]);
      }
    },
    [onRowSelect, sortedEntries]
  );

  const handleCloseDetailPane = useCallback((): void => {
    setDetailPaneOpen(false);
  }, []);

  const handleContextMenu = useCallback((event: React.MouseEvent, entry: LogEntry): void => {
    event.preventDefault();
    const target = event.target as HTMLElement;
    const cellValue = target.textContent?.trim() || '';

    setContextMenu({
      visible: true,
      position: { x: event.clientX, y: event.clientY },
      entry,
      cellValue,
    });
  }, []);

  const closeContextMenu = useCallback((): void => {
    setContextMenu({
      visible: false,
      position: { x: 0, y: 0 },
      entry: null,
    });
  }, []);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (!parentRef.current) return;

      switch (event.key) {
        case 'ArrowUp':
          event.preventDefault();
          setSelectedRowIndex((prev) => {
            if (prev === null) return 0;
            return Math.max(0, prev - 1);
          });
          break;

        case 'ArrowDown':
          event.preventDefault();
          setSelectedRowIndex((prev) => {
            if (prev === null) return 0;
            return Math.min(sortedEntries.length - 1, (prev ?? -1) + 1);
          });
          break;

        case 'Enter':
          if (selectedRowIndex !== null && onRowSelect) {
            onRowSelect(sortedEntries[selectedRowIndex]);
          }
          break;

        case 'Escape':
          setSelectedRowIndex(null);
          break;
      }
    };

    const element = parentRef.current;
    if (element) {
      element.addEventListener('keydown', handleKeyDown);
      return () => {
        element.removeEventListener('keydown', handleKeyDown);
      };
    }
  }, [selectedRowIndex, sortedEntries, onRowSelect]);

  // Scroll to selected row when navigating with keyboard
  useEffect(() => {
    if (selectedRowIndex !== null) {
      rowVirtualizer.scrollToIndex(selectedRowIndex, {
        align: 'center',
      });
    }
  }, [selectedRowIndex, rowVirtualizer]);

  // Auto-enrich rule labels when entries change (Story 3.3)
  useEffect(() => {
    console.log('[MEM] LogTable: useEffect triggered for rule labels', {
      entriesLength: entries.length,
      hasEntries: entries.length > 0
    });

    if (entries.length > 0) {
      const DEBOUNCE_ENRICHMENT_MS = 500;
      const timeoutId = setTimeout(() => {
        console.log('[MEM] LogTable: enrichRuleLabels about to call', { entriesCount: entries.length });
        enrichRuleLabels(entries);
      }, DEBOUNCE_ENRICHMENT_MS);

      return () => clearTimeout(timeoutId);
    } else {
      console.log('[MEM] LogTable: no entries to enrich for rule labels');
    }
  }, [entries]);

  // Auto-enrich IP aliases when entries change (Story 3.4)
  useEffect(() => {
    console.log('[MEM] LogTable: useEffect triggered for aliases', {
      entriesLength: entries.length,
      hasEntries: entries.length > 0
    });

    if (entries.length > 0) {
      const DEBOUNCE_ENRICHMENT_MS = 500;
      const timeoutId = setTimeout(() => {
        console.log('[MEM] LogTable: enrichAliases about to call', { entriesCount: entries.length });
        enrichAliases(entries);
      }, DEBOUNCE_ENRICHMENT_MS);

      return () => clearTimeout(timeoutId);
    } else {
      console.log('[MEM] LogTable: no entries to enrich for aliases');
    }
  }, [entries]);

  return (
    <div className="flex flex-col h-full">
      {/* Table Header */}
      <LogTableHeader
        sortColumn={sortColumn}
        sortDirection={sortDirection}
        onSort={handleSort}
        columnVisibility={columnVisibility}
      />

      {/* Virtual Scrolling Container */}
      <div
        ref={parentRef}
        className="flex-1 overflow-auto"
        role="grid"
        aria-label="Log entries table"
        tabIndex={0}
      >
        {sortedEntries.length === 0 ? (
          <div className="flex items-center justify-center h-full text-gray-500 dark:text-gray-400">
            No log entries to display
          </div>
        ) : (
          <div
            style={{
              height: `${rowVirtualizer.getTotalSize()}px`,
              width: '100%',
              position: 'relative',
            }}
          >
            {rowVirtualizer.getVirtualItems().map((virtualRow) => (
              <LogTableRow
                key={virtualRow.key}
                virtualRow={virtualRow}
                entry={sortedEntries[virtualRow.index]}
                isSelected={selectedRowIndex === virtualRow.index}
                onSelect={() => handleRowSelect(virtualRow.index)}
                onContextMenu={handleContextMenu}
                columnVisibility={columnVisibility}
              />
            ))}
          </div>
        )}
      </div>

      {/* Footer: Total count */}
      <div className="border-t border-gray-200 dark:border-gray-700 px-4 py-2 text-sm text-gray-600 dark:text-gray-400">
        Showing {sortedEntries.length.toLocaleString()} entries
        {selectedRowIndex !== null && ` | Row ${selectedRowIndex + 1} selected`}
      </div>

      {/* Context Menu */}
      {contextMenu.visible && (
        <ContextMenu
          position={contextMenu.position}
          entry={contextMenu.entry}
          cellValue={contextMenu.cellValue}
          onClose={closeContextMenu}
          onFilterByValue={onFilterByValue}
        />
      )}

      {/* Entry Detail View */}
      <EntryDetailView
        entry={selectedEntry}
        isOpen={detailPaneOpen}
        onClose={handleCloseDetailPane}
      />
    </div>
  );
}
