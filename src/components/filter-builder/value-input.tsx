import { useEnrichmentStore } from '@/stores/enrichment-store';
import type { FieldType, OperatorType, RelativeTimeRange } from '@/types/filter';

interface ValueInputProps {
  fieldType: FieldType;
  operatorType: OperatorType;
  value: string | number | [string, string] | RelativeTimeRange;
  onChange: (value: string | number | [string, string] | RelativeTimeRange) => void;
  error?: string;
}

const PROTOCOL_OPTIONS = ['TCP', 'UDP', 'ICMP', 'IGMP', 'ESP', 'AH', 'GRE'];
const ACTION_OPTIONS = ['pass', 'block', 'reject'];
const RELATIVE_TIME_OPTIONS: { value: RelativeTimeRange; label: string }[] = [
  { value: '1h', label: 'Last 1 hour' },
  { value: '6h', label: 'Last 6 hours' },
  { value: '24h', label: 'Last 24 hours' },
  { value: '7d', label: 'Last 7 days' },
  { value: '30d', label: 'Last 30 days' },
];

export function ValueInput({ fieldType, operatorType, value, onChange, error }: ValueInputProps): JSX.Element {
  // Get interface mappings from enrichment store
  const interfaceMappings = useEnrichmentStore((state) => state.interfaceMappings);

  // Interface dropdown with logical names
  if (fieldType === 'interface') {
    // Convert Map to array of options
    const interfaceOptions = Array.from(interfaceMappings.entries()).map(
      ([physical, logical]) => ({
        value: physical, // Store physical name (for backend query)
        label: `${logical} (${physical})`, // Display format: "LAN (vtnet0)"
      })
    );

    // If no mappings are available, show a text input instead
    if (interfaceOptions.length === 0) {
      return (
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            3. Enter Interface Name
          </label>
          <input
            type="text"
            value={(value as string) || ''}
            onChange={(e) => onChange(e.target.value)}
            placeholder="e.g., vtnet0 or LAN"
            className="w-full px-3 py-2 bg-white dark:bg-gray-800
              border border-gray-300 dark:border-gray-700 rounded
              text-gray-900 dark:text-gray-100 font-mono text-sm
              focus:ring-2 focus:ring-blue-500"
            aria-label="Interface name"
          />
          {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
          <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
            Note: Connect to OPNsense API to see interface names in dropdown
          </p>
        </div>
      );
    }

    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Select Interface
        </label>
        <select
          value={(value as string) || ''}
          onChange={(e) => onChange(e.target.value)}
          className="w-full px-3 py-2 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500"
          aria-label="Select interface"
        >
          <option value="">Choose interface...</option>
          {interfaceOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Protocol dropdown
  if (fieldType === 'protocol') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Select Value
        </label>
        <select
          value={(value as string) || ''}
          onChange={(e) => onChange(e.target.value)}
          className="w-full px-3 py-2 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500"
          aria-label="Select protocol"
        >
          <option value="">Choose protocol...</option>
          {PROTOCOL_OPTIONS.map((protocol) => (
            <option key={protocol} value={protocol}>
              {protocol}
            </option>
          ))}
        </select>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Action dropdown
  if (fieldType === 'action') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Select Value
        </label>
        <select
          value={(value as string) || ''}
          onChange={(e) => onChange(e.target.value)}
          className="w-full px-3 py-2 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500"
          aria-label="Select action"
        >
          <option value="">Choose action...</option>
          {ACTION_OPTIONS.map((action) => (
            <option key={action} value={action}>
              {action}
            </option>
          ))}
        </select>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Relative timestamp
  if (fieldType === 'timestamp' && operatorType === 'relative') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Select Time Range
        </label>
        <select
          value={(value as string) || ''}
          onChange={(e) => onChange(e.target.value as RelativeTimeRange)}
          className="w-full px-3 py-2 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500"
          aria-label="Select time range"
        >
          <option value="">Choose time range...</option>
          {RELATIVE_TIME_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Absolute timestamp (date picker)
  if (fieldType === 'timestamp' && operatorType === 'absoluteRange') {
    const arrayValue = Array.isArray(value) ? value : ['', ''];
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Enter Date Range
        </label>
        <div className="grid grid-cols-2 gap-2">
          <input
            type="datetime-local"
            value={arrayValue[0]}
            onChange={(e) => onChange([e.target.value, arrayValue[1]])}
            className="px-3 py-2 bg-white dark:bg-gray-800
              border border-gray-300 dark:border-gray-700 rounded
              text-gray-900 dark:text-gray-100
              focus:ring-2 focus:ring-blue-500"
            placeholder="Start date"
            aria-label="Start date"
          />
          <input
            type="datetime-local"
            value={arrayValue[1]}
            onChange={(e) => onChange([arrayValue[0], e.target.value])}
            className="px-3 py-2 bg-white dark:bg-gray-800
              border border-gray-300 dark:border-gray-700 rounded
              text-gray-900 dark:text-gray-100
              focus:ring-2 focus:ring-blue-500"
            placeholder="End date"
            aria-label="End date"
          />
        </div>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Number input for ports with "between" operator
  if ((fieldType === 'sourcePort' || fieldType === 'destinationPort') && operatorType === 'between') {
    const arrayValue = Array.isArray(value) ? value : ['', ''];
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Enter Port Range
        </label>
        <div className="grid grid-cols-2 gap-2">
          <input
            type="number"
            value={arrayValue[0]}
            onChange={(e) => onChange([e.target.value, arrayValue[1]])}
            placeholder="Min port"
            min="0"
            max="65535"
            className="px-3 py-2 bg-white dark:bg-gray-800
              border border-gray-300 dark:border-gray-700 rounded
              text-gray-900 dark:text-gray-100
              focus:ring-2 focus:ring-blue-500"
            aria-label="Minimum port"
          />
          <input
            type="number"
            value={arrayValue[1]}
            onChange={(e) => onChange([arrayValue[0], e.target.value])}
            placeholder="Max port"
            min="0"
            max="65535"
            className="px-3 py-2 bg-white dark:bg-gray-800
              border border-gray-300 dark:border-gray-700 rounded
              text-gray-900 dark:text-gray-100
              focus:ring-2 focus:ring-blue-500"
            aria-label="Maximum port"
          />
        </div>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Number input for ports
  if (fieldType === 'sourcePort' || fieldType === 'destinationPort') {
    return (
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          3. Enter Port Number
        </label>
        <input
          type="number"
          value={value as number || ''}
          onChange={(e) => onChange(Number(e.target.value))}
          placeholder="e.g., 443"
          min="0"
          max="65535"
          className="w-full px-3 py-2 bg-white dark:bg-gray-800
            border border-gray-300 dark:border-gray-700 rounded
            text-gray-900 dark:text-gray-100
            focus:ring-2 focus:ring-blue-500"
          aria-label="Port number"
        />
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
    );
  }

  // Text input for IPs, Rule Label (Interface is handled above)
  const getPlaceholder = (): string => {
    if (fieldType === 'sourceIp' || fieldType === 'destinationIp') {
      return 'e.g., 192.168.1.1 or 10.0.0.0/24';
    }
    if (fieldType === 'ruleLabel') {
      return 'e.g., Block RFC1918';
    }
    if (operatorType === 'regex') {
      return 'e.g., ^192\\.168\\.*';
    }
    return 'Enter value...';
  };

  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        3. Enter Value
      </label>
      <input
        type="text"
        value={(value as string) || ''}
        onChange={(e) => onChange(e.target.value)}
        placeholder={getPlaceholder()}
        className="w-full px-3 py-2 bg-white dark:bg-gray-800
          border border-gray-300 dark:border-gray-700 rounded
          text-gray-900 dark:text-gray-100 font-mono text-sm
          focus:ring-2 focus:ring-blue-500"
        aria-label="Value"
      />
      {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>}
    </div>
  );
}
