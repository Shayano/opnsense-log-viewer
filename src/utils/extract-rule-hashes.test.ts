import { describe, it, expect } from 'vitest';
import { extractUniqueRuleHashes } from './extract-rule-hashes';
import type { LogEntry } from '@/types/log-entry';

describe('extractUniqueRuleHashes', () => {
  it('should extract unique rule hashes from entries', () => {
    const entries: LogEntry[] = [
      { ruleLabel: 'abc123', timestamp: '', sourceIp: '', destinationIp: '', protocol: '', action: '' } as unknown as LogEntry,
      { ruleLabel: 'def456', timestamp: '', sourceIp: '', destinationIp: '', protocol: '', action: '' } as unknown as LogEntry,
      { ruleLabel: 'abc123', timestamp: '', sourceIp: '', destinationIp: '', protocol: '', action: '' } as unknown as LogEntry, // duplicate
      { ruleLabel: 'xyz789', timestamp: '', sourceIp: '', destinationIp: '', protocol: '', action: '' } as unknown as LogEntry,
    ];

    const result = extractUniqueRuleHashes(entries);

    expect(result.size).toBe(3);
    expect(result.has('abc123')).toBe(true);
    expect(result.has('def456')).toBe(true);
    expect(result.has('xyz789')).toBe(true);
  });

  it('should handle empty array', () => {
    const result = extractUniqueRuleHashes([]);
    expect(result.size).toBe(0);
  });

  it('should filter out null/undefined/empty values', () => {
    const entries: LogEntry[] = [
      { ruleLabel: 'abc123', timestamp: '', sourceIp: '', destinationIp: '', protocol: '', action: '' } as unknown as LogEntry,
      { ruleLabel: null as any, timestamp: '', sourceIp: '', destinationIp: '', protocol: '', action: '' } as unknown as LogEntry,
      { ruleLabel: undefined as any, timestamp: '', sourceIp: '', destinationIp: '', protocol: '', action: '' } as unknown as LogEntry,
      { ruleLabel: '', timestamp: '', sourceIp: '', destinationIp: '', protocol: '', action: '' } as unknown as LogEntry,
      { ruleLabel: '   ', timestamp: '', sourceIp: '', destinationIp: '', protocol: '', action: '' } as unknown as LogEntry, // whitespace
    ];

    const result = extractUniqueRuleHashes(entries);

    expect(result.size).toBe(1);
    expect(result.has('abc123')).toBe(true);
  });

  it('should return Set (no duplicates)', () => {
    const entries: LogEntry[] = Array(100).fill(null).map((_, i) => ({
      ruleLabel: `rule${i % 15}`, // 15 unique hashes repeated
      timestamp: '',
      sourceIp: '',
      destinationIp: '',
      protocol: '',
      action: '',
    })) as unknown as LogEntry[];

    const result = extractUniqueRuleHashes(entries);

    expect(result.size).toBe(15);
    expect(result).toBeInstanceOf(Set);
  });
});
