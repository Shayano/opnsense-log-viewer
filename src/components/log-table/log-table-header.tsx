import React from 'react';
import { ArrowUp, ArrowDown } from 'lucide-react';
import type { ColumnVisibility } from './use-responsive-columns';

export type SortColumn = 'timestamp' | 'sourceIp' | 'destinationIp';
export type SortDirection = 'asc' | 'desc';

interface LogTableHeaderProps {
  sortColumn: SortColumn;
  sortDirection: SortDirection;
  onSort: (column: SortColumn) => void;
  columnVisibility: ColumnVisibility;
}

export function LogTableHeader({
  sortColumn,
  sortDirection,
  onSort,
  columnVisibility,
}: LogTableHeaderProps): React.JSX.Element {
  const getSortIndicator = (column: SortColumn): React.JSX.Element | null => {
    if (sortColumn !== column) return null;
    const Icon = sortDirection === 'asc' ? ArrowUp : ArrowDown;
    return <Icon className="w-3 h-3 ml-1 inline" aria-hidden="true" />;
  };

  const headerClass =
    'px-2 py-2 text-left text-xs font-semibold text-gray-700 dark:text-gray-300 border-b border-gray-200 dark:border-gray-700';
  const sortableClass = 'cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors';

  return (
    <div
      className="grid gap-2 px-4 bg-gray-50 dark:bg-gray-900"
      style={{
        gridTemplateColumns: `
          ${columnVisibility.timestamp ? 'minmax(140px, 160px)' : ''}
          ${columnVisibility.interface ? 'minmax(80px, 100px)' : ''}
          ${columnVisibility.sourceIp ? 'minmax(100px, 120px)' : ''}
          ${columnVisibility.sourcePort ? 'minmax(60px, 80px)' : ''}
          ${columnVisibility.destinationIp ? 'minmax(100px, 120px)' : ''}
          ${columnVisibility.destinationPort ? 'minmax(60px, 80px)' : ''}
          ${columnVisibility.protocol ? 'minmax(60px, 80px)' : ''}
          ${columnVisibility.action ? 'minmax(100px, 120px)' : ''}
          ${columnVisibility.ruleLabel ? 'minmax(120px, 1fr)' : ''}
        `
          .split('\n')
          .map((s) => s.trim())
          .filter(Boolean)
          .join(' '),
      }}
      role="row"
    >
      {columnVisibility.timestamp && (
        <div
          className={`${headerClass} ${sortableClass}`}
          onClick={() => onSort('timestamp')}
          role="columnheader"
          aria-sort={
            sortColumn === 'timestamp'
              ? sortDirection === 'asc'
                ? 'ascending'
                : 'descending'
              : 'none'
          }
        >
          Timestamp
          {getSortIndicator('timestamp')}
        </div>
      )}

      {columnVisibility.interface && (
        <div className={headerClass} role="columnheader">
          Interface
        </div>
      )}

      {columnVisibility.sourceIp && (
        <div
          className={`${headerClass} ${sortableClass}`}
          onClick={() => onSort('sourceIp')}
          role="columnheader"
          aria-sort={
            sortColumn === 'sourceIp'
              ? sortDirection === 'asc'
                ? 'ascending'
                : 'descending'
              : 'none'
          }
        >
          Source IP
          {getSortIndicator('sourceIp')}
        </div>
      )}

      {columnVisibility.sourcePort && (
        <div className={headerClass} role="columnheader">
          Src Port
        </div>
      )}

      {columnVisibility.destinationIp && (
        <div
          className={`${headerClass} ${sortableClass}`}
          onClick={() => onSort('destinationIp')}
          role="columnheader"
          aria-sort={
            sortColumn === 'destinationIp'
              ? sortDirection === 'asc'
                ? 'ascending'
                : 'descending'
              : 'none'
          }
        >
          Dest IP
          {getSortIndicator('destinationIp')}
        </div>
      )}

      {columnVisibility.destinationPort && (
        <div className={headerClass} role="columnheader">
          Dst Port
        </div>
      )}

      {columnVisibility.protocol && (
        <div className={headerClass} role="columnheader">
          Protocol
        </div>
      )}

      {columnVisibility.action && (
        <div className={headerClass} role="columnheader">
          Action
        </div>
      )}

      {columnVisibility.ruleLabel && (
        <div className={headerClass} role="columnheader">
          Rule Label
        </div>
      )}
    </div>
  );
}
