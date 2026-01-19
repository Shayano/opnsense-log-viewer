import { intervalToDuration, format } from 'date-fns';

export enum StalenessSeverity {
  Fresh = 'fresh',
  Moderate = 'moderate',
  High = 'high',
}

const STALENESS_THRESHOLD_DAYS = 7;

/**
 * Calculate staleness severity based on enrichment age
 */
export function calculateStalenessSeverity(
  _importedAt: Date,
  exportTimestamp: Date
): StalenessSeverity {
  const ageDays = calculateAgeDays(exportTimestamp);

  if (ageDays < 1) {
    return StalenessSeverity.Fresh;
  } else if (ageDays <= STALENESS_THRESHOLD_DAYS) {
    return StalenessSeverity.Moderate;
  } else {
    return StalenessSeverity.High;
  }
}

/**
 * Calculate age in days from export timestamp
 */
export function calculateAgeDays(exportTimestamp: Date): number {
  const now = new Date();
  const diffMs = now.getTime() - exportTimestamp.getTime();
  return Math.floor(diffMs / (1000 * 60 * 60 * 24));
}

/**
 * Format age as "X days, Y hours"
 */
export function formatEnrichmentAge(exportTimestamp: Date): string {
  const now = new Date();
  const duration = intervalToDuration({ start: exportTimestamp, end: now });

  if (duration.days && duration.days > 0) {
    if (duration.hours && duration.hours > 0) {
      return `${duration.days} ${duration.days === 1 ? 'day' : 'days'}, ${duration.hours} ${duration.hours === 1 ? 'hour' : 'hours'}`;
    }
    return `${duration.days} ${duration.days === 1 ? 'day' : 'days'}`;
  } else if (duration.hours && duration.hours > 0) {
    return `${duration.hours} ${duration.hours === 1 ? 'hour' : 'hours'}`;
  } else {
    return 'less than 1 hour';
  }
}

/**
 * Format export timestamp for display
 */
export function formatExportTimestamp(timestamp: Date): string {
  return format(timestamp, 'PPpp'); // e.g., "Apr 29, 2021, 11:21:17 AM"
}
