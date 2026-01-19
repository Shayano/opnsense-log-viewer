import { describe, it, expect } from 'vitest';
import { extractUniqueIPs } from './extract-ips';
import { LogEntry, Protocol, Action } from '@/types/log-entry';

// Helper to create mock log entries
function createMockLogEntry(
  sourceIp: string,
  destinationIp: string,
  overrides: Partial<LogEntry> = {}
): LogEntry {
  return {
    id: 'test-id',
    timestamp: new Date().toISOString(),
    sourceIp,
    destinationIp,
    sourcePort: 12345,
    destinationPort: 80,
    protocol: Protocol.TCP,
    action: Action.PASS,
    interface: 'lan',
    ruleLabel: '',
    ...overrides,
  };
}

describe('extractUniqueIPs', () => {
  it('should extract unique IPs from log entries (source + destination)', () => {
    const entries: LogEntry[] = [
      createMockLogEntry('192.168.1.100', '10.0.0.1'),
      createMockLogEntry('192.168.1.101', '10.0.0.2'),
      createMockLogEntry('192.168.1.102', '10.0.0.3'),
    ];

    const ips = extractUniqueIPs(entries);

    expect(ips.size).toBe(6);
    expect(ips.has('192.168.1.100')).toBe(true);
    expect(ips.has('192.168.1.101')).toBe(true);
    expect(ips.has('192.168.1.102')).toBe(true);
    expect(ips.has('10.0.0.1')).toBe(true);
    expect(ips.has('10.0.0.2')).toBe(true);
    expect(ips.has('10.0.0.3')).toBe(true);
  });

  it('should return unique IPs (no duplicates)', () => {
    const entries: LogEntry[] = [
      createMockLogEntry('192.168.1.100', '10.0.0.1'),
      createMockLogEntry('192.168.1.100', '10.0.0.1'), // Duplicate
      createMockLogEntry('192.168.1.100', '10.0.0.2'),
      createMockLogEntry('192.168.1.101', '10.0.0.1'), // 10.0.0.1 again
    ];

    const ips = extractUniqueIPs(entries);

    // Should only have 3 unique IPs: 192.168.1.100, 192.168.1.101, 10.0.0.1, 10.0.0.2
    expect(ips.size).toBe(4);
    expect(ips.has('192.168.1.100')).toBe(true);
    expect(ips.has('192.168.1.101')).toBe(true);
    expect(ips.has('10.0.0.1')).toBe(true);
    expect(ips.has('10.0.0.2')).toBe(true);
  });

  it('should handle empty entries array', () => {
    const entries: LogEntry[] = [];

    const ips = extractUniqueIPs(entries);

    expect(ips.size).toBe(0);
  });

  it('should filter out null/undefined IPs', () => {
    const entries: LogEntry[] = [
      createMockLogEntry('192.168.1.100', ''),
      createMockLogEntry('', '10.0.0.1'),
      // @ts-expect-error - Testing runtime behavior with undefined
      createMockLogEntry(undefined, '10.0.0.2'),
      // @ts-expect-error - Testing runtime behavior with null
      createMockLogEntry('192.168.1.101', null),
    ];

    const ips = extractUniqueIPs(entries);

    // Should only have valid IPs: 192.168.1.100, 10.0.0.1, 10.0.0.2, 192.168.1.101
    expect(ips.size).toBe(4);
    expect(ips.has('192.168.1.100')).toBe(true);
    expect(ips.has('10.0.0.1')).toBe(true);
    expect(ips.has('10.0.0.2')).toBe(true);
    expect(ips.has('192.168.1.101')).toBe(true);
    expect(ips.has('')).toBe(false);
  });

  it('should filter out whitespace-only IPs', () => {
    const entries: LogEntry[] = [
      createMockLogEntry('192.168.1.100', '   '),
      createMockLogEntry('  ', '10.0.0.1'),
      createMockLogEntry('\t', '\n'),
    ];

    const ips = extractUniqueIPs(entries);

    expect(ips.size).toBe(2);
    expect(ips.has('192.168.1.100')).toBe(true);
    expect(ips.has('10.0.0.1')).toBe(true);
  });

  it('should handle large dataset efficiently (100 entries with 50 unique IPs)', () => {
    const entries: LogEntry[] = [];

    // Generate 100 entries with 50 unique IPs (each IP appears twice)
    for (let i = 0; i < 50; i++) {
      const sourceIp = `192.168.1.${i + 1}`;
      const destIp = `10.0.0.${i + 1}`;

      // Each IP appears twice
      entries.push(createMockLogEntry(sourceIp, destIp));
      entries.push(createMockLogEntry(sourceIp, destIp));
    }

    const ips = extractUniqueIPs(entries);

    // Should have 100 unique IPs total (50 source + 50 destination)
    expect(ips.size).toBe(100);
    expect(ips.has('192.168.1.1')).toBe(true);
    expect(ips.has('192.168.1.50')).toBe(true);
    expect(ips.has('10.0.0.1')).toBe(true);
    expect(ips.has('10.0.0.50')).toBe(true);
  });

  it('should extract both source and destination IPs when different', () => {
    const entries: LogEntry[] = [
      createMockLogEntry('192.168.1.100', '10.0.0.1'),
    ];

    const ips = extractUniqueIPs(entries);

    expect(ips.size).toBe(2);
    expect(ips.has('192.168.1.100')).toBe(true); // source
    expect(ips.has('10.0.0.1')).toBe(true); // destination
  });

  it('should handle entries where source and destination are the same', () => {
    const entries: LogEntry[] = [
      createMockLogEntry('192.168.1.100', '192.168.1.100'), // Same IP
      createMockLogEntry('192.168.1.101', '192.168.1.101'),
    ];

    const ips = extractUniqueIPs(entries);

    // Should deduplicate since source == destination
    expect(ips.size).toBe(2);
    expect(ips.has('192.168.1.100')).toBe(true);
    expect(ips.has('192.168.1.101')).toBe(true);
  });

  it('should return Set data structure', () => {
    const entries: LogEntry[] = [
      createMockLogEntry('192.168.1.100', '10.0.0.1'),
    ];

    const ips = extractUniqueIPs(entries);

    expect(ips).toBeInstanceOf(Set);
  });

  it('should handle IPv6 addresses (if present in logs)', () => {
    const entries: LogEntry[] = [
      createMockLogEntry('2001:db8::1', '2001:db8::2'),
      createMockLogEntry('fe80::1', '192.168.1.100'),
    ];

    const ips = extractUniqueIPs(entries);

    expect(ips.size).toBe(4);
    expect(ips.has('2001:db8::1')).toBe(true);
    expect(ips.has('2001:db8::2')).toBe(true);
    expect(ips.has('fe80::1')).toBe(true);
    expect(ips.has('192.168.1.100')).toBe(true);
  });

  it('should handle mixed valid and invalid entries gracefully', () => {
    const entries: LogEntry[] = [
      createMockLogEntry('192.168.1.100', '10.0.0.1'),
      createMockLogEntry('', ''),
      createMockLogEntry('192.168.1.101', '10.0.0.2'),
      createMockLogEntry('  ', '\t'),
      createMockLogEntry('192.168.1.102', '10.0.0.3'),
    ];

    const ips = extractUniqueIPs(entries);

    expect(ips.size).toBe(6); // Only valid IPs
    expect(ips.has('192.168.1.100')).toBe(true);
    expect(ips.has('192.168.1.101')).toBe(true);
    expect(ips.has('192.168.1.102')).toBe(true);
    expect(ips.has('10.0.0.1')).toBe(true);
    expect(ips.has('10.0.0.2')).toBe(true);
    expect(ips.has('10.0.0.3')).toBe(true);
  });
});
