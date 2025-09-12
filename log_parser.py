"""
OPNsense Log Parser
"""
import re
import hashlib
from datetime import datetime
from typing import Dict, List, Optional, Any

class LogEntry:
    """Log entry class"""
    def __init__(self, raw_line: str):
        self.raw_line = raw_line
        self.parsed_data = {}
        self.timestamp = None
        self.host = None
        self.digest = None
        
    def __getitem__(self, key):
        return self.parsed_data.get(key, '')
        
    def __contains__(self, key):
        return key in self.parsed_data
        
    def get(self, key, default=''):
        return self.parsed_data.get(key, default)

class OPNsenseLogParser:
    """OPNsense log parser"""
    
    def __init__(self):
        self.interface_mapping = {}
        
    def set_interface_mapping(self, mapping: Dict[str, str]):
        """Set physical to logical interface mapping"""
        self.interface_mapping = mapping

    def parse_log_line(self, line: str) -> Optional[LogEntry]:
        """Parse a log line and return a LogEntry"""
        if 'filterlog' not in line:
            return None
            
        entry = LogEntry(line)
        
        # Format detection (rfc5424)
        parts = line.split()
        if len(parts) < 4:
            return None
            
        # Extract timestamp and host
        timestamp_str = parts[0]
        host = 'opnsense'  # Default
        
        # Look for the part after "filterlog"
        filterlog_idx = -1
        for i, part in enumerate(parts):
            if 'filterlog' in part:
                filterlog_idx = i
                break
        
        if filterlog_idx < 0 or filterlog_idx + 1 >= len(parts):
            return None
            
        # Extract CSV data
        data_part = ' '.join(parts[filterlog_idx + 1:])
        fields = [f.strip() for f in data_part.split(',')]
        
        # Manual parsing according to OPNsense specification
        rule = self._parse_fields(fields)
        
        if not rule or 'action' not in rule:
            return None
        
        # Apply interface mapping
        if 'interface' in rule and rule['interface'] in self.interface_mapping:
            rule['interface_display'] = self.interface_mapping[rule['interface']]
        else:
            rule['interface_display'] = rule.get('interface', '')
        
        # Metadata
        rule['__timestamp__'] = timestamp_str
        rule['__host__'] = host
        rule['__digest__'] = hashlib.md5(line.encode()).hexdigest()
        
        entry.parsed_data = rule
        
        # Timestamp conversion
        try:
            if 'T' in timestamp_str:
                entry.timestamp = datetime.fromisoformat(timestamp_str.replace('T', ' '))
            else:
                entry.timestamp = datetime.strptime(timestamp_str, '%b %d %H:%M:%S')
        except:
            entry.timestamp = datetime.now()
            
        entry.host = host
        entry.digest = rule['__digest__']
        
        return entry
    
    def _parse_fields(self, fields: List[str]) -> Dict[str, str]:
        """Parse fields according to OPNsense specification"""
        rule = {}
        
        try:
            # Basic fields (always present)
            rule['rulenr'] = fields[0] if len(fields) > 0 else ''
            rule['subrulenr'] = fields[1] if len(fields) > 1 else ''
            rule['anchorname'] = fields[2] if len(fields) > 2 else ''
            rule['rid'] = fields[3] if len(fields) > 3 else ''
            rule['interface'] = fields[4] if len(fields) > 4 else ''
            rule['reason'] = fields[5] if len(fields) > 5 else ''
            rule['action'] = fields[6] if len(fields) > 6 else ''
            rule['dir'] = fields[7] if len(fields) > 7 else ''
            rule['ipversion'] = fields[8] if len(fields) > 8 else ''
            
            # IPv4 fields
            if rule['ipversion'] == '4' and len(fields) > 9:
                rule['tos'] = fields[9] if len(fields) > 9 else ''
                rule['ecn'] = fields[10] if len(fields) > 10 else ''
                rule['ttl'] = fields[11] if len(fields) > 11 else ''
                rule['id'] = fields[12] if len(fields) > 12 else ''
                rule['offset'] = fields[13] if len(fields) > 13 else ''
                rule['ipflags'] = fields[14] if len(fields) > 14 else ''
                rule['protonum'] = fields[15] if len(fields) > 15 else ''
                rule['protoname'] = fields[16] if len(fields) > 16 else ''
                rule['length'] = fields[17] if len(fields) > 17 else ''
                rule['src'] = fields[18] if len(fields) > 18 else ''
                rule['dst'] = fields[19] if len(fields) > 19 else ''
                
                # TCP/UDP fields
                if rule['protonum'] in ['6', '17'] and len(fields) > 20:  # TCP or UDP
                    rule['srcport'] = fields[20] if len(fields) > 20 else ''
                    rule['dstport'] = fields[21] if len(fields) > 21 else ''
                    rule['datalen'] = fields[22] if len(fields) > 22 else ''
                    
                    # Additional TCP fields
                    if rule['protonum'] == '6' and len(fields) > 23:  # TCP
                        rule['tcpflags'] = fields[23] if len(fields) > 23 else ''
                        rule['seq'] = fields[24] if len(fields) > 24 else ''
                        rule['ack'] = fields[25] if len(fields) > 25 else ''
                        rule['urp'] = fields[26] if len(fields) > 26 else ''
                        rule['tcpopts'] = fields[27] if len(fields) > 27 else ''
            
            # Convert protocol number to name
            if 'protonum' in rule:
                proto_map = {'6': 'tcp', '17': 'udp', '1': 'icmp', '112': 'carp'}
                rule['protoname'] = proto_map.get(rule['protonum'], rule['protonum'])
                
        except (IndexError, ValueError):
            pass
        
        return rule

    def parse_log_file(self, filepath: str, max_lines: int = None) -> List[LogEntry]:
        """Parse a complete log file"""
        entries = []
        lines_processed = 0
        
        try:
            with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                for line in f:
                    if max_lines and lines_processed >= max_lines:
                        break
                        
                    entry = self.parse_log_line(line.strip())
                    if entry:
                        entries.append(entry)
                        
                    lines_processed += 1
                    
        except Exception as e:
            print(f"Error reading file {filepath}: {e}")
            
        return entries

    def parse_log_file_generator(self, filepath: str):
        """Generator to iterate through a large file line by line"""
        try:
            with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                for line in f:
                    entry = self.parse_log_line(line.strip())
                    if entry:
                        yield entry
        except Exception as e:
            print(f"Error reading file {filepath}: {e}")