import React, { useRef, useState, useMemo, useCallback, useEffect } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from 'lucide-react';
import type { LogEntry } from '@/types/log-entry';
import { LogTableHeader, type SortColumn, type SortDirection } from './log-table-header';
import { LogTableRow } from './log-table-row';
import { ContextMenu } from './context-menu';
import { useResponsiveColumns } from './use-responsive-columns';
import { EntryDetailView } from '@/components/entry-detail-view';
import { enrichRuleLabels, enrichAliases } from '@/services/enrichment-service';

// Page size options (descending order for dropdown)
const PAGE_SIZE_OPTIONS = [5000, 2000, 1000, 500, 100] as const;
const DEFAULT_PAGE_SIZE = 1000;

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

  // Pagination state
  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);

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

  // Pagination calculations
  const totalPages = Math.ceil(sortedEntries.length / pageSize);
  const startIndex = (currentPage - 1) * pageSize;
  const endIndex = Math.min(startIndex + pageSize, sortedEntries.length);

  // Get paginated entries
  const paginatedEntries = useMemo(() => {
    return sortedEntries.slice(startIndex, endIndex);
  }, [sortedEntries, startIndex, endIndex]);

  // Reset to page 1 when entries change (new filter applied)
  useEffect(() => {
    setCurrentPage(1);
  }, [entries]);

  // Reset to page 1 when page size changes
  useEffect(() => {
    setCurrentPage(1);
  }, [pageSize]);

  // Pagination handlers
  const goToFirstPage = useCallback(() => setCurrentPage(1), []);
  const goToLastPage = useCallback(() => setCurrentPage(totalPages), [totalPages]);
  const goToPreviousPage = useCallback(() => setCurrentPage((p) => Math.max(1, p - 1)), []);
  const goToNextPage = useCallback(() => setCurrentPage((p) => Math.min(totalPages, p + 1)), [totalPages]);

  const handlePageSizeChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    setPageSize(Number(e.target.value));
  }, []);

  // TanStack Virtual setup - uses paginated entries
  const rowVirtualizer = useVirtualizer({
    count: paginatedEntries.length,
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
      setSelectedEntry(paginatedEntries[index]);
      setDetailPaneOpen(true);
      if (onRowSelect) {
        onRowSelect(paginatedEntries[index]);
      }
    },
    [onRowSelect, paginatedEntries]
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
            return Math.min(paginatedEntries.length - 1, (prev ?? -1) + 1);
          });
          break;

        case 'Enter':
          if (selectedRowIndex !== null && onRowSelect) {
            onRowSelect(paginatedEntries[selectedRowIndex]);
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
  }, [selectedRowIndex, paginatedEntries, onRowSelect]);

  // Scroll to selected row when navigating with keyboard
  useEffect(() => {
    if (selectedRowIndex !== null) {
      rowVirtualizer.scrollToIndex(selectedRowIndex, {
        align: 'center',
      });
    }
  }, [selectedRowIndex, rowVirtualizer]);

  // Auto-enrich rule labels when paginated entries change (Story 3.3)
  // Only enriches the visible page for performance
  useEffect(() => {
    if (paginatedEntries.length > 0) {
      const DEBOUNCE_ENRICHMENT_MS = 500;
      const timeoutId = setTimeout(() => {
        enrichRuleLabels(paginatedEntries);
      }, DEBOUNCE_ENRICHMENT_MS);

      return () => clearTimeout(timeoutId);
    }
  }, [paginatedEntries]);

  // Auto-enrich IP aliases when paginated entries change (Story 3.4)
  // Only enriches the visible page for performance
  useEffect(() => {
    if (paginatedEntries.length > 0) {
      const DEBOUNCE_ENRICHMENT_MS = 500;
      const timeoutId = setTimeout(() => {
        enrichAliases(paginatedEntries);
      }, DEBOUNCE_ENRICHMENT_MS);

      return () => clearTimeout(timeoutId);
    }
  }, [paginatedEntries]);

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
        {paginatedEntries.length === 0 ? (
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
                entry={paginatedEntries[virtualRow.index]}
                isSelected={selectedRowIndex === virtualRow.index}
                onSelect={() => handleRowSelect(virtualRow.index)}
                onContextMenu={handleContextMenu}
                columnVisibility={columnVisibility}
              />
            ))}
          </div>
        )}
      </div>

      {/* Footer: Pagination Controls */}
      <div className="border-t border-gray-200 dark:border-gray-700 px-4 py-2 flex items-center justify-between text-sm">
        {/* Left: Entry count info */}
        <div className="text-gray-600 dark:text-gray-400">
          Showing {startIndex + 1}-{endIndex} of {sortedEntries.length.toLocaleString()} entries
          {selectedRowIndex !== null && ` | Row ${selectedRowIndex + 1} selected`}
        </div>

        {/* Right: Pagination controls */}
        <div className="flex items-center gap-4">
          {/* Page size selector */}
          <div className="flex items-center gap-2">
            <label htmlFor="pageSize" className="text-gray-600 dark:text-gray-400">
              Per page:
            </label>
            <select
              id="pageSize"
              value={pageSize}
              onChange={handlePageSizeChange}
              className="px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded
                bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100
                focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {PAGE_SIZE_OPTIONS.map((size) => (
                <option key={size} value={size}>
                  {size.toLocaleString()}
                </option>
              ))}
            </select>
          </div>

          {/* Page navigation */}
          <div className="flex items-center gap-1">
            <button
              onClick={goToFirstPage}
              disabled={currentPage === 1}
              className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
              title="First page"
              aria-label="Go to first page"
            >
              <ChevronsLeft className="w-4 h-4 text-gray-600 dark:text-gray-400" />
            </button>
            <button
              onClick={goToPreviousPage}
              disabled={currentPage === 1}
              className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
              title="Previous page"
              aria-label="Go to previous page"
            >
              <ChevronLeft className="w-4 h-4 text-gray-600 dark:text-gray-400" />
            </button>

            <span className="px-3 text-gray-600 dark:text-gray-400">
              Page {currentPage} of {totalPages || 1}
            </span>

            <button
              onClick={goToNextPage}
              disabled={currentPage >= totalPages}
              className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
              title="Next page"
              aria-label="Go to next page"
            >
              <ChevronRight className="w-4 h-4 text-gray-600 dark:text-gray-400" />
            </button>
            <button
              onClick={goToLastPage}
              disabled={currentPage >= totalPages}
              className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
              title="Last page"
              aria-label="Go to last page"
            >
              <ChevronsRight className="w-4 h-4 text-gray-600 dark:text-gray-400" />
            </button>
          </div>
        </div>
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
