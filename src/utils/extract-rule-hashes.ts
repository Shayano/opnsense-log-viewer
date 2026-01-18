import { LogEntry } from '@/types/log-entry';

/**
 * Extract unique rule hashes from log entries
 *
 * @param entries - Array of log entries
 * @returns Set of unique rule hashes (filters out empty/null values)
 *
 * @example
 * const entries = [
 *   { ruleLabel: "abc123", ... },
 *   { ruleLabel: "def456", ... },
 *   { ruleLabel: "abc123", ... }, // duplicate
 * ];
 * const hashes = extractUniqueRuleHashes(entries);
 * // Returns: Set { "abc123", "def456" }
 */
export function extractUniqueRuleHashes(entries: LogEntry[]): Set<string> {
  const hashes = new Set<string>();

  for (const entry of entries) {
    // ruleLabel field contains the hash
    if (entry.ruleLabel && entry.ruleLabel.trim() !== '') {
      hashes.add(entry.ruleLabel);
    }
  }

  return hashes;
}
