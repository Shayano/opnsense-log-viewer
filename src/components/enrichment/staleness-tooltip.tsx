import * as Tooltip from '@radix-ui/react-tooltip';
import { formatEnrichmentAge, formatExportTimestamp } from '@/utils/staleness-utils';
import { ReactNode } from 'react';

interface BackupMetadata {
  importedAt: string;
  ageDays: number;
  sourceFile: string;
  exportTimestamp: string;
  deviceId: string;
}

interface StalenessTooltipProps {
  metadata: BackupMetadata;
  children: ReactNode;
}

export function StalenessTooltip({ metadata, children }: StalenessTooltipProps) {
  const exportDate = new Date(metadata.exportTimestamp);
  const formattedTimestamp = formatExportTimestamp(exportDate);
  const ageBreakdown = formatEnrichmentAge(exportDate);

  return (
    <Tooltip.Provider>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          {children}
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            className="z-50 max-w-sm rounded-md bg-gray-900 dark:bg-gray-800 px-4 py-3 text-sm text-white shadow-lg"
            sideOffset={5}
          >
            <div className="space-y-2">
              <div>
                <span className="font-semibold">Exported:</span>{' '}
                <span className="text-gray-300">{formattedTimestamp}</span>
              </div>
              <div>
                <span className="font-semibold">Age:</span>{' '}
                <span className="text-gray-300">{ageBreakdown}</span>
              </div>
              <div>
                <span className="font-semibold">Source:</span>{' '}
                <span className="text-gray-300">{metadata.deviceId}</span>
              </div>
              <div className="pt-2 border-t border-gray-700">
                <p className="text-yellow-300 text-xs">
                  ⚠️ Interface mappings and rule labels may be outdated.
                </p>
                <p className="text-gray-400 text-xs mt-1">
                  Reconnect to API for current data.
                </p>
              </div>
            </div>
            <Tooltip.Arrow className="fill-gray-900 dark:fill-gray-800" />
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}
