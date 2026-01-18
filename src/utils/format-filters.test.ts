import { describe, it, expect } from 'vitest';
import { formatFilterSummary, formatFullFilterDetails } from './format-filters';
import type { Filter } from '@/types/filter';

describe('formatFilterSummary', () => {
  it('returns "No filters" for empty array', () => {
    const result = formatFilterSummary([]);
    expect(result).toBe('No filters');
  });

  it('formats single filter correctly', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'action', operator: 'equals', value: 'block' },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Action : block');
  });

  it('formats multiple filters with AND logic', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'action', operator: 'equals', value: 'block', logic: 'AND' },
      { field: 'destinationPort', operator: 'equals', value: '443' },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Action : block AND Dest Port : 443');
  });

  it('formats multiple filters with mixed logic operators', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'action', operator: 'equals', value: 'block', logic: 'AND' },
      { field: 'destinationPort', operator: 'equals', value: '443', logic: 'OR' },
      { field: 'protocol', operator: 'equals', value: 'tcp' },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Action : block AND Dest Port : 443 OR Protocol : tcp');
  });

  it('formats contains operator with tilde', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'sourceIp', operator: 'contains', value: '192.168' },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Source IP ~ 192.168');
  });

  it('formats startsWith operator with caret', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'destinationIp', operator: 'startsWith', value: '10.' },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Dest IP ^ 10.');
  });

  it('formats endsWith operator with dollar sign', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'ruleLabel', operator: 'endsWith', value: '-allow' },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Rule $ -allow');
  });

  it('formats greaterThan and lessThan operators', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'sourcePort', operator: 'greaterThan', value: 1024, logic: 'AND' },
      { field: 'destinationPort', operator: 'lessThan', value: 65535 },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Source Port > 1024 AND Dest Port < 65535');
  });

  it('formats notEquals operator', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'protocol', operator: 'notEquals', value: 'udp' },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Protocol ≠ udp');
  });

  it('formats between operator with array values', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'sourcePort', operator: 'between', value: ['1024', '65535'] },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Source Port between [1024 - 65535]');
  });

  it('formats relative time operator', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'timestamp', operator: 'relative', value: '24h' },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Time last 24h');
  });

  it('formats absoluteRange operator with array values', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'timestamp', operator: 'absoluteRange', value: ['2024-01-01', '2024-01-31'] },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Time range [2024-01-01 - 2024-01-31]');
  });

  it('formats regex operator', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'sourceIp', operator: 'regex', value: '^192\\.168\\.' },
    ];
    const result = formatFilterSummary(filters);
    expect(result).toBe('Source IP ~regex ^192\\.168\\.');
  });

  it('truncates long summary with ellipsis', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'action', operator: 'equals', value: 'block', logic: 'AND' },
      { field: 'destinationPort', operator: 'equals', value: '443', logic: 'AND' },
      { field: 'sourceIp', operator: 'contains', value: '192.168.1', logic: 'OR' },
      { field: 'protocol', operator: 'equals', value: 'tcp' },
    ];
    const result = formatFilterSummary(filters, 40);
    expect(result).toContain('...');
    expect(result.length).toBe(40);
  });

  it('does not truncate if within maxLength', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'action', operator: 'equals', value: 'block' },
    ];
    const result = formatFilterSummary(filters, 50);
    expect(result).toBe('Action : block');
    expect(result.length).toBeLessThan(50);
  });

  it('uses default maxLength of 60', () => {
    const longValue = 'a'.repeat(100);
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'ruleLabel', operator: 'contains', value: longValue },
    ];
    const result = formatFilterSummary(filters);
    expect(result.length).toBeLessThanOrEqual(60);
    expect(result).toContain('...');
  });

  it('handles all field types correctly', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'timestamp', operator: 'relative', value: '1h' },
      { field: 'sourceIp', operator: 'equals', value: '192.168.1.1' },
      { field: 'destinationIp', operator: 'equals', value: '10.0.0.1' },
      { field: 'sourcePort', operator: 'equals', value: 12345 },
      { field: 'destinationPort', operator: 'equals', value: 443 },
      { field: 'protocol', operator: 'equals', value: 'tcp' },
      { field: 'action', operator: 'equals', value: 'pass' },
      { field: 'interface', operator: 'equals', value: 'WAN' },
      { field: 'ruleLabel', operator: 'contains', value: 'allow' },
    ];

    const summary = formatFilterSummary(filters, 500);
    expect(summary).toContain('Time');
    expect(summary).toContain('Source IP');
    expect(summary).toContain('Dest IP');
    expect(summary).toContain('Source Port');
    expect(summary).toContain('Dest Port');
    expect(summary).toContain('Protocol');
    expect(summary).toContain('Action');
    expect(summary).toContain('Interface');
    expect(summary).toContain('Rule');
  });
});

describe('formatFullFilterDetails', () => {
  it('returns "No filters" for empty array', () => {
    const result = formatFullFilterDetails([]);
    expect(result).toBe('No filters');
  });

  it('formats single filter with numbered list', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'action', operator: 'equals', value: 'block' },
    ];
    const result = formatFullFilterDetails(filters);
    expect(result).toBe('1. Action : block');
  });

  it('formats multiple filters with numbered list', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'action', operator: 'equals', value: 'block', logic: 'AND' },
      { field: 'destinationPort', operator: 'equals', value: '443' },
    ];
    const result = formatFullFilterDetails(filters);
    expect(result).toBe('1. Action : block\n2. Dest Port : 443');
  });

  it('formats all filters without truncation', () => {
    const longValue = 'a'.repeat(200);
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'ruleLabel', operator: 'contains', value: longValue },
    ];
    const result = formatFullFilterDetails(filters);
    expect(result).toContain(longValue);
    expect(result).not.toContain('...');
  });

  it('separates filters with newlines', () => {
    const filters: Omit<Filter, 'id'>[] = [
      { field: 'action', operator: 'equals', value: 'block', logic: 'AND' },
      { field: 'protocol', operator: 'equals', value: 'tcp', logic: 'OR' },
      { field: 'sourcePort', operator: 'greaterThan', value: 1024 },
    ];
    const result = formatFullFilterDetails(filters);
    const lines = result.split('\n');
    expect(lines).toHaveLength(3);
    expect(lines[0]).toBe('1. Action : block');
    expect(lines[1]).toBe('2. Protocol : tcp');
    expect(lines[2]).toBe('3. Source Port > 1024');
  });
});
