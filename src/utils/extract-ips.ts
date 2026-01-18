import { LogEntry } from '@/types/log-entry';

/**
 * Extract unique IP addresses from log entries (both source and destination)
 *
 * @param entries - Array of log entries
 * @returns Set of unique IP addresses
 */
export function extractUniqueIPs(entries: LogEntry[]): Set<string> {
  const ips = new Set<string>();

  for (const entry of entries) {
    if (entry.sourceIp && entry.sourceIp.trim() !== '') {
      ips.add(entry.sourceIp);
    }
    if (entry.destinationIp && entry.destinationIp.trim() !== '') {
      ips.add(entry.destinationIp);
    }
  }

  return ips;
}
