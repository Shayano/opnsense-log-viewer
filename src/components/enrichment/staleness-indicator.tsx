import { AlertTriangle } from 'lucide-react';
import { useState } from 'react';
import { useEnrichmentStore } from '@/stores/enrichment-store';
import { StalenessSeverity, calculateStalenessSeverity } from '@/utils/staleness-utils';
import { StalenessTooltip } from './staleness-tooltip';
import { StalenessOptionsDialog } from '@/components/dialogs/staleness-options-dialog';

export function StalenessIndicator() {
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const backupEnrichmentActive = useEnrichmentStore((state) => state.backupEnrichmentActive);
  const backupMetadata = useEnrichmentStore((state) => state.backupMetadata);
  const stalenessIndicatorDismissed = useEnrichmentStore((state) => state.stalenessIndicatorDismissed);

  // Don't show if not in backup mode or if dismissed
  if (!backupEnrichmentActive || !backupMetadata || stalenessIndicatorDismissed) {
    return null;
  }

  // Calculate staleness severity
  const severity = calculateStalenessSeverity(
    new Date(backupMetadata.importedAt),
    new Date(backupMetadata.exportTimestamp)
  );

  // Calculate age text
  const ageText = `${backupMetadata.ageDays} ${backupMetadata.ageDays === 1 ? 'day' : 'days'} old`;

  // Get severity-based styling
  const backgroundClass = getSeverityBackgroundClass(severity);
  const textClass = getSeverityTextClass(severity);

  return (
    <>
      <StalenessTooltip metadata={backupMetadata}>
        <div
          onClick={() => setIsDialogOpen(true)}
          className={`
            fixed top-16 right-4 z-40
            flex items-center gap-2 px-3 py-2 rounded-md shadow-sm
            cursor-pointer transition-all duration-200
            hover:shadow-md hover:scale-105
            ${backgroundClass} ${textClass}
          `}
          role="button"
          tabIndex={0}
          aria-label="Backup enrichment staleness indicator"
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              setIsDialogOpen(true);
            }
          }}
        >
          <AlertTriangle className="h-4 w-4" aria-hidden="true" />
          <span className="text-sm font-medium hidden sm:inline">
            Using backup enrichment ({ageText})
          </span>
          <span className="text-sm font-medium sm:hidden">
            Backup ({ageText})
          </span>
          {severity === StalenessSeverity.High && (
            <span className="ml-1" aria-label="Verify accuracy">
              ⚠️
            </span>
          )}
        </div>
      </StalenessTooltip>

      <StalenessOptionsDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
        metadata={backupMetadata}
      />
    </>
  );
}

function getSeverityBackgroundClass(severity: StalenessSeverity): string {
  switch (severity) {
    case StalenessSeverity.Fresh:
      return 'bg-yellow-100 dark:bg-yellow-900/20';
    case StalenessSeverity.Moderate:
      return 'bg-amber-100 dark:bg-amber-900/30';
    case StalenessSeverity.High:
      return 'bg-orange-100 dark:bg-orange-900/40';
  }
}

function getSeverityTextClass(severity: StalenessSeverity): string {
  switch (severity) {
    case StalenessSeverity.Fresh:
      return 'text-yellow-800 dark:text-yellow-200';
    case StalenessSeverity.Moderate:
      return 'text-amber-800 dark:text-amber-200';
    case StalenessSeverity.High:
      return 'text-orange-800 dark:text-orange-200';
  }
}
