"""
OPNsense Log Parser with comprehensive error handling
"""
import re
import hashlib
import os
from datetime import datetime
from typing import Dict, List, Optional, Any

from opnsense_log_viewer.exceptions import FileOperationError, ParseError
from opnsense_log_viewer.utils.logging_config import get_logger, log_exception

logger = get_logger(__name__)


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
    """OPNsense log parser with comprehensive error handling"""

    def __init__(self):
        self.interface_mapping = {}
        logger.debug("OPNsenseLogParser initialized")

    def set_interface_mapping(self, mapping: Dict[str, str]):
        """Set physical to logical interface mapping"""
        try:
            if not isinstance(mapping, dict):
                raise TypeError("Interface mapping must be a dictionary")
            self.interface_mapping = mapping
            logger.info(f"Interface mapping set: {len(mapping)} interfaces configured")
        except Exception as e:
            log_exception(logger, e, "Failed to set interface mapping")
            raise

    def parse_log_line(self, line: str) -> Optional[LogEntry]:
        """Parse a log line and return a LogEntry"""
        try:
            if not line or 'filterlog' not in line:
                return None

            entry = LogEntry(line)
            parts = line.split()
            if len(parts) < 4:
                return None

            timestamp_str = parts[0]
            host = 'opnsense'

            filterlog_idx = -1
            for i, part in enumerate(parts):
                if 'filterlog' in part:
                    filterlog_idx = i
                    break

            if filterlog_idx < 0 or filterlog_idx + 1 >= len(parts):
                return None

            data_part = ' '.join(parts[filterlog_idx + 1:])
            fields = [f.strip() for f in data_part.split(',')]
            rule = self._parse_fields(fields)

            if not rule or 'action' not in rule:
                return None

            if 'interface' in rule and rule['interface'] in self.interface_mapping:
                rule['interface_display'] = self.interface_mapping[rule['interface']]
            else:
                rule['interface_display'] = rule.get('interface', '')

            rule['__timestamp__'] = timestamp_str
            rule['__host__'] = host
            try:
                rule['__digest__'] = hashlib.md5(line.encode()).hexdigest()
            except Exception:
                rule['__digest__'] = ''

            entry.parsed_data = rule

            try:
                if 'T' in timestamp_str:
                    entry.timestamp = datetime.fromisoformat(timestamp_str.replace('T', ' '))
                else:
                    entry.timestamp = datetime.strptime(timestamp_str, '%b %d %H:%M:%S')
            except Exception:
                entry.timestamp = datetime.now()

            entry.host = host
            entry.digest = rule['__digest__']
            return entry

        except Exception as e:
            logger.debug(f"Error parsing line: {e}")
            return None

    def _parse_fields(self, fields: List[str]) -> Dict[str, str]:
        """Parse fields according to OPNsense specification"""
        rule = {}
        try:
            rule['rulenr'] = fields[0] if len(fields) > 0 else ''
            rule['subrulenr'] = fields[1] if len(fields) > 1 else ''
            rule['anchorname'] = fields[2] if len(fields) > 2 else ''
            rule['rid'] = fields[3] if len(fields) > 3 else ''
            rule['interface'] = fields[4] if len(fields) > 4 else ''
            rule['reason'] = fields[5] if len(fields) > 5 else ''
            rule['action'] = fields[6] if len(fields) > 6 else ''
            rule['dir'] = fields[7] if len(fields) > 7 else ''
            rule['ipversion'] = fields[8] if len(fields) > 8 else ''

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

                if rule['protonum'] in ['6', '17'] and len(fields) > 20:
                    rule['srcport'] = fields[20] if len(fields) > 20 else ''
                    rule['dstport'] = fields[21] if len(fields) > 21 else ''
                    rule['datalen'] = fields[22] if len(fields) > 22 else ''

                    if rule['protonum'] == '6' and len(fields) > 23:
                        rule['tcpflags'] = fields[23] if len(fields) > 23 else ''
                        rule['seq'] = fields[24] if len(fields) > 24 else ''
                        rule['ack'] = fields[25] if len(fields) > 25 else ''
                        rule['urp'] = fields[26] if len(fields) > 26 else ''
                        rule['tcpopts'] = fields[27] if len(fields) > 27 else ''

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

        if not os.path.exists(filepath):
            error_msg = f"Log file not found: {filepath}"
            logger.error(error_msg)
            raise FileOperationError(error_msg, file_path=filepath, operation="read")

        if not os.access(filepath, os.R_OK):
            error_msg = f"Log file is not readable: {filepath}"
            logger.error(error_msg)
            raise FileOperationError(error_msg, file_path=filepath, operation="read")

        logger.info(f"Parsing log file: {filepath}")

        try:
            with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                for line_num, line in enumerate(f, 1):
                    if max_lines and lines_processed >= max_lines:
                        break
                    try:
                        entry = self.parse_log_line(line.strip())
                        if entry:
                            entries.append(entry)
                    except Exception as e:
                        logger.debug(f"Error parsing line {line_num}: {e}")
                    lines_processed += 1

        except PermissionError as e:
            error_msg = f"Permission denied reading file: {filepath}"
            logger.error(error_msg)
            raise FileOperationError(error_msg, file_path=filepath, operation="read", original_error=e)
        except IOError as e:
            error_msg = f"I/O error reading file: {filepath}"
            log_exception(logger, e, error_msg, filepath=filepath)
            raise FileOperationError(error_msg, file_path=filepath, operation="read", original_error=e)
        except Exception as e:
            error_msg = f"Unexpected error reading file: {filepath}"
            log_exception(logger, e, error_msg, filepath=filepath)
            raise FileOperationError(error_msg, file_path=filepath, operation="read", original_error=e)

        logger.info(f"Parsed {len(entries)} valid entries from {lines_processed} lines")
        return entries

    def parse_log_file_generator(self, filepath: str):
        """Generator to iterate through a large file line by line"""
        if not os.path.exists(filepath):
            error_msg = f"Log file not found: {filepath}"
            logger.error(error_msg)
            raise FileOperationError(error_msg, file_path=filepath, operation="read")

        if not os.access(filepath, os.R_OK):
            error_msg = f"Log file is not readable: {filepath}"
            logger.error(error_msg)
            raise FileOperationError(error_msg, file_path=filepath, operation="read")

        logger.info(f"Starting generator parse for file: {filepath}")

        try:
            with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                line_num = 0
                for line in f:
                    line_num += 1
                    try:
                        entry = self.parse_log_line(line.strip())
                        if entry:
                            yield entry
                    except Exception as e:
                        logger.debug(f"Error parsing line {line_num}: {e}")

        except PermissionError as e:
            error_msg = f"Permission denied reading file: {filepath}"
            logger.error(error_msg)
            raise FileOperationError(error_msg, file_path=filepath, operation="read", original_error=e)
        except IOError as e:
            error_msg = f"I/O error reading file: {filepath}"
            log_exception(logger, e, error_msg, filepath=filepath)
            raise FileOperationError(error_msg, file_path=filepath, operation="read", original_error=e)
        except Exception as e:
            error_msg = f"Unexpected error reading file: {filepath}"
            log_exception(logger, e, error_msg, filepath=filepath)
            raise FileOperationError(error_msg, file_path=filepath, operation="read", original_error=e)
