export enum Action {
  PASS = 'pass',
  BLOCK = 'block',
  REJECT = 'reject',
}

export enum Protocol {
  TCP = 'tcp',
  UDP = 'udp',
  ICMP = 'icmp',
  IGMP = 'igmp',
  ESP = 'esp',
  AH = 'ah',
  GRE = 'gre',
  UNKNOWN = 'unknown',
}

export interface LogEntry {
  id: string;
  timestamp: string; // ISO 8601 format
  interface: string;
  sourceIp: string;
  sourcePort: number;
  destinationIp: string;
  destinationPort: number;
  protocol: Protocol;
  action: Action;
  ruleLabel: string;
  rawLine?: string; // Optional: full raw log line
}
