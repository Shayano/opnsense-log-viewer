import React, { memo, useMemo } from 'react';
import type { VirtualItem } from '@tanstack/react-virtual';
import { format } from 'date-fns';
import type { LogEntry } from '@/types/log-entry';
import { getActionStyle } from '@/utils/action-colors';
import { useInterfaceName } from '@/hooks/use-interface-name';
import { useRuleLabel } from '@/hooks/use-rule-label';
import { useIPAlias } from '@/hooks/use-ip-alias';
import type { ColumnVisibility } from './use-responsive-columns';

// Helper to compute grid template columns from visibility
function computeGridTemplateColumns(visibility: ColumnVisibility): string {
  const columns: string[] = [];
  if (visibility.timestamp) columns.push('minmax(140px, 160px)');
  if (visibility.interface) columns.push('minmax(80px, 100px)');
  if (visibility.sourceIp) columns.push('minmax(100px, 120px)');
  if (visibility.sourcePort) columns.push('minmax(60px, 80px)');
  if (visibility.destinationIp) columns.push('minmax(100px, 120px)');
  if (visibility.destinationPort) columns.push('minmax(60px, 80px)');
  if (visibility.protocol) columns.push('minmax(60px, 80px)');
  if (visibility.action) columns.push('minmax(100px, 120px)');
  if (visibility.ruleLabel) columns.push('minmax(120px, 1fr)');
  return columns.join(' ');
}

interface LogTableRowProps {
  virtualRow: VirtualItem;
  entry: LogEntry;
  isSelected: boolean;
  onSelect: () => void;
  onContextMenu: (event: React.MouseEvent, entry: LogEntry) => void;
  columnVisibility: ColumnVisibility;
}

export const LogTableRow = memo(function LogTableRow({
  virtualRow,
  entry,
  isSelected,
  onSelect,
  onContextMenu,
  columnVisibility,
}: LogTableRowProps): React.JSX.Element {
  const actionStyle = getActionStyle(entry.action);
  const ActionIcon = actionStyle.icon;

  // Resolve interface name (physical → logical)
  const { displayName: interfaceDisplayName, tooltipText: interfaceTooltip } = useInterfaceName(
    entry.interface
  );

  // Resolve rule label (hash → description)
  const { displayText: ruleLabelDisplayText, tooltipText: ruleLabelTooltip } = useRuleLabel(
    entry.ruleLabel
  );

  // Resolve IP aliases (source and destination)
  const { displayText: sourceIpDisplay, tooltipText: sourceIpTooltip } = useIPAlias(entry.sourceIp);
  const { displayText: destIpDisplay, tooltipText: destIpTooltip } = useIPAlias(entry.destinationIp);

  // Memoize grid template columns to avoid recalculating on every render
  const gridTemplateColumns = useMemo(
    () => computeGridTemplateColumns(columnVisibility),
    [columnVisibility]
  );

  const handleContextMenu = (event: React.MouseEvent): void => {
    event.preventDefault();
    onContextMenu(event, entry);
  };

  const formatTimestamp = (timestamp: string): string => {
    try {
      return format(new Date(timestamp), 'yyyy-MM-dd HH:mm:ss');
    } catch {
      return timestamp;
    }
  };

  return (
    <div
      className={`
        grid gap-2 px-4 py-1 text-sm
        even:bg-gray-50 dark:even:bg-gray-800/50
        hover:bg-gray-100 dark:hover:bg-gray-700
        ${isSelected ? 'bg-blue-100 dark:bg-blue-900/30' : ''}
        cursor-pointer transition-colors
      `}
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        width: '100%',
        height: `${virtualRow.size}px`,
        transform: `translateY(${virtualRow.start}px)`,
        gridTemplateColumns,
      }}
      onClick={onSelect}
      onContextMenu={handleContextMenu}
      role="row"
      aria-selected={isSelected}
    >
      {columnVisibility.timestamp && (
        <div
          className="font-mono text-xs truncate leading-tight"
          title={entry.timestamp}
          role="gridcell"
        >
          {formatTimestamp(entry.timestamp)}
        </div>
      )}

      {columnVisibility.interface && (
        <div className="truncate leading-tight" title={interfaceTooltip} role="gridcell">
          {interfaceDisplayName}
        </div>
      )}

      {columnVisibility.sourceIp && (
        <div className="font-mono truncate leading-tight" title={sourceIpTooltip} role="gridcell">
          {sourceIpDisplay}
        </div>
      )}

      {columnVisibility.sourcePort && (
        <div
          className="font-mono truncate leading-tight"
          title={entry.sourcePort.toString()}
          role="gridcell"
        >
          {entry.sourcePort}
        </div>
      )}

      {columnVisibility.destinationIp && (
        <div
          className="font-mono truncate leading-tight"
          title={destIpTooltip}
          role="gridcell"
        >
          {destIpDisplay}
        </div>
      )}

      {columnVisibility.destinationPort && (
        <div
          className="font-mono truncate leading-tight"
          title={entry.destinationPort.toString()}
          role="gridcell"
        >
          {entry.destinationPort}
        </div>
      )}

      {columnVisibility.protocol && (
        <div className="truncate uppercase leading-tight" title={entry.protocol} role="gridcell">
          {entry.protocol}
        </div>
      )}

      {columnVisibility.action && (
        <div
          className={`flex items-center gap-1 px-2 py-0.5 rounded leading-tight ${actionStyle.bgClass} ${actionStyle.textClass}`}
          role="gridcell"
        >
          <ActionIcon className={`w-3 h-3 ${actionStyle.iconColor}`} aria-hidden="true" />
          <span className="uppercase">{entry.action}</span>
        </div>
      )}

      {columnVisibility.ruleLabel && (
        <div className="truncate leading-tight" title={ruleLabelTooltip} role="gridcell">
          {ruleLabelDisplayText}
        </div>
      )}
    </div>
  );
});
