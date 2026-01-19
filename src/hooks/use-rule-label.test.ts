import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useRuleLabel } from './use-rule-label';
import { useEnrichmentStore } from '@/stores/enrichment-store';

describe('useRuleLabel', () => {
  beforeEach(() => {
    // Reset enrichment store
    useEnrichmentStore.setState({
      ruleLabels: new Map(),
    });
  });

  it('should return description when mapping exists', () => {
    // Setup rule label in store
    useEnrichmentStore.setState({
      ruleLabels: new Map([['abc123', 'Block RFC1918 Networks']]),
    });

    const { result } = renderHook(() => useRuleLabel('abc123'));

    expect(result.current.description).toBe('Block RFC1918 Networks');
    expect(result.current.hash).toBe('abc123');
    expect(result.current.displayText).toBe('Block RFC1918 Networks');
    expect(result.current.tooltipText).toBe('Block RFC1918 Networks (abc123)');
  });

  it('should return hash when no mapping exists', () => {
    const { result } = renderHook(() => useRuleLabel('unknown123'));

    expect(result.current.description).toBeNull();
    expect(result.current.hash).toBe('unknown123');
    expect(result.current.displayText).toBe('Rule unknown123 (label unavailable)');
    expect(result.current.tooltipText).toBe('unknown123');
  });

  it('should format displayText correctly with description', () => {
    useEnrichmentStore.setState({
      ruleLabels: new Map([['test456', 'Allow HTTPS Traffic']]),
    });

    const { result } = renderHook(() => useRuleLabel('test456'));

    expect(result.current.displayText).toBe('Allow HTTPS Traffic');
  });

  it('should format displayText correctly without description', () => {
    const { result } = renderHook(() => useRuleLabel('xyz789'));

    expect(result.current.displayText).toBe('Rule xyz789 (label unavailable)');
  });

  it('should read from store at render time', () => {
    // Pre-populate store with label
    useEnrichmentStore.setState({
      ruleLabels: new Map([['dynamic123', 'Dynamic Rule Label']]),
    });

    // Hook should read the label at render time
    const { result } = renderHook(() => useRuleLabel('dynamic123'));

    expect(result.current.description).toBe('Dynamic Rule Label');
    expect(result.current.displayText).toBe('Dynamic Rule Label');
    expect(result.current.tooltipText).toBe('Dynamic Rule Label (dynamic123)');
  });
});
