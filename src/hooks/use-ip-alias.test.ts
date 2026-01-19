import { renderHook } from '@testing-library/react';
import { describe, it, expect, beforeEach } from 'vitest';
import { useIPAlias } from './use-ip-alias';
import { useEnrichmentStore } from '@/stores/enrichment-store';

describe('useIPAlias', () => {
  beforeEach(() => {
    // Clear store before each test
    useEnrichmentStore.getState().clearAliases();
  });

  it('should return IP only when no alias mapping exists', () => {
    const { result } = renderHook(() => useIPAlias('192.168.1.100'));

    expect(result.current.aliases).toEqual([]);
    expect(result.current.displayText).toBe('192.168.1.100');
    expect(result.current.tooltipText).toBe('192.168.1.100');
  });

  it('should return alias when mapping exists (single alias)', () => {
    // Set up alias mapping
    useEnrichmentStore.getState().setAliases({
      '192.168.1.100': [
        {
          aliasName: 'Servers_Group',
          groupMembers: ['192.168.1.100', '192.168.1.101'],
          description: 'Server subnet',
          aliasType: 'network',
        },
      ],
    });

    const { result } = renderHook(() => useIPAlias('192.168.1.100'));

    expect(result.current.aliases).toEqual(['Servers_Group']);
    expect(result.current.displayText).toBe('192.168.1.100 (Servers_Group)');
    expect(result.current.tooltipText).toBe(
      'Servers_Group: 192.168.1.100, 192.168.1.101'
    );
  });

  it('should return multiple aliases when IP has multiple alias mappings', () => {
    // Set up multiple alias mappings
    useEnrichmentStore.getState().setAliases({
      '192.168.1.100': [
        {
          aliasName: 'Servers_Group',
          groupMembers: ['192.168.1.100'],
          description: undefined,
          aliasType: 'host',
        },
        {
          aliasName: 'DMZ_Hosts',
          groupMembers: ['192.168.1.100', '192.168.1.101'],
          description: 'DMZ hosts',
          aliasType: 'network',
        },
      ],
    });

    const { result } = renderHook(() => useIPAlias('192.168.1.100'));

    expect(result.current.aliases).toEqual(['Servers_Group', 'DMZ_Hosts']);
    expect(result.current.displayText).toBe(
      '192.168.1.100 (Servers_Group, DMZ_Hosts)'
    );
    expect(result.current.tooltipText).toBe(
      'Servers_Group: 192.168.1.100\nDMZ_Hosts: 192.168.1.100, 192.168.1.101'
    );
  });

  it('should handle alias with empty group members', () => {
    useEnrichmentStore.getState().setAliases({
      '192.168.1.100': [
        {
          aliasName: 'Empty_Alias',
          groupMembers: [],
          description: undefined,
          aliasType: undefined,
        },
      ],
    });

    const { result } = renderHook(() => useIPAlias('192.168.1.100'));

    expect(result.current.aliases).toEqual(['Empty_Alias']);
    expect(result.current.displayText).toBe('192.168.1.100 (Empty_Alias)');
    expect(result.current.tooltipText).toBe('Empty_Alias: ');
  });

  it('should handle alias with many group members', () => {
    useEnrichmentStore.getState().setAliases({
      '192.168.1.100': [
        {
          aliasName: 'DMZ_Servers',
          groupMembers: [
            '192.168.1.100',
            '192.168.1.101',
            '192.168.1.102',
            '192.168.1.103',
            '192.168.1.104',
          ],
          description: 'DMZ server subnet',
          aliasType: 'network',
        },
      ],
    });

    const { result } = renderHook(() => useIPAlias('192.168.1.100'));

    expect(result.current.aliases).toEqual(['DMZ_Servers']);
    expect(result.current.displayText).toBe('192.168.1.100 (DMZ_Servers)');
    expect(result.current.tooltipText).toBe(
      'DMZ_Servers: 192.168.1.100, 192.168.1.101, 192.168.1.102, 192.168.1.103, 192.168.1.104'
    );
  });

  it('should handle different IP addresses independently', () => {
    useEnrichmentStore.getState().setAliases({
      '192.168.1.100': [
        {
          aliasName: 'Servers',
          groupMembers: ['192.168.1.100'],
          description: undefined,
          aliasType: 'host',
        },
      ],
      '192.168.1.200': [
        {
          aliasName: 'Workstations',
          groupMembers: ['192.168.1.200'],
          description: undefined,
          aliasType: 'host',
        },
      ],
    });

    const { result: result1 } = renderHook(() => useIPAlias('192.168.1.100'));
    const { result: result2 } = renderHook(() => useIPAlias('192.168.1.200'));
    const { result: result3 } = renderHook(() => useIPAlias('192.168.1.300'));

    expect(result1.current.displayText).toBe('192.168.1.100 (Servers)');
    expect(result2.current.displayText).toBe('192.168.1.200 (Workstations)');
    expect(result3.current.displayText).toBe('192.168.1.300'); // No alias
  });

  it('should update when store changes', () => {
    const { result, rerender } = renderHook(() => useIPAlias('192.168.1.100'));

    // Initially no alias
    expect(result.current.aliases).toEqual([]);
    expect(result.current.displayText).toBe('192.168.1.100');

    // Add alias mapping
    useEnrichmentStore.getState().setAliases({
      '192.168.1.100': [
        {
          aliasName: 'NewServer',
          groupMembers: ['192.168.1.100'],
          description: undefined,
          aliasType: 'host',
        },
      ],
    });

    // Rerender to pick up store changes
    rerender();

    expect(result.current.aliases).toEqual(['NewServer']);
    expect(result.current.displayText).toBe('192.168.1.100 (NewServer)');
  });

  it('should handle empty IP string', () => {
    const { result } = renderHook(() => useIPAlias(''));

    expect(result.current.aliases).toEqual([]);
    expect(result.current.displayText).toBe('');
    expect(result.current.tooltipText).toBe('');
  });

  it('should format tooltip text correctly with newlines for multiple aliases', () => {
    useEnrichmentStore.getState().setAliases({
      '192.168.1.100': [
        {
          aliasName: 'Alias1',
          groupMembers: ['192.168.1.100', '192.168.1.101'],
          description: undefined,
          aliasType: undefined,
        },
        {
          aliasName: 'Alias2',
          groupMembers: ['192.168.1.100', '192.168.1.102'],
          description: undefined,
          aliasType: undefined,
        },
        {
          aliasName: 'Alias3',
          groupMembers: ['192.168.1.100'],
          description: undefined,
          aliasType: undefined,
        },
      ],
    });

    const { result } = renderHook(() => useIPAlias('192.168.1.100'));

    expect(result.current.tooltipText).toBe(
      'Alias1: 192.168.1.100, 192.168.1.101\nAlias2: 192.168.1.100, 192.168.1.102\nAlias3: 192.168.1.100'
    );
  });
});
