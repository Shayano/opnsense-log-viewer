import { useEnrichmentStore } from '@/stores/enrichment-store';

interface RuleLabelResult {
  description: string | null;
  hash: string;
  displayText: string;
  tooltipText: string;
}

/**
 * Hook to resolve rule labels (hash → description)
 *
 * @param hash - Rule hash (e.g., "abc123def")
 * @returns RuleLabelResult with description, display text, and tooltip
 *
 * @example
 * const { displayText, tooltipText } = useRuleLabel("abc123");
 * // displayText: "Block RFC1918 Networks"
 * // tooltipText: "Block RFC1918 Networks (abc123)"
 *
 * // If no label found:
 * // displayText: "Rule abc123 (label unavailable)"
 * // tooltipText: "abc123"
 */
export function useRuleLabel(hash: string): RuleLabelResult {
  const getRuleLabel = useEnrichmentStore((state) => state.getRuleLabel);

  const description = getRuleLabel(hash);

  // Display text: Description if available, otherwise hash with "(label unavailable)"
  const displayText = description || `Rule ${hash} (label unavailable)`;

  // Tooltip text: Show hash alongside description
  const tooltipText = description
    ? `${description} (${hash})`
    : hash;

  return {
    description,
    hash,
    displayText,
    tooltipText,
  };
}
