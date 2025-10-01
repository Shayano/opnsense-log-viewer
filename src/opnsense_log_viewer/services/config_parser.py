"""
OPNsense XML configuration parser for interface mapping and firewall rules
"""
import xml.etree.ElementTree as ET
import os
from typing import Dict, Optional

from opnsense_log_viewer.exceptions import FileOperationError, ParseError
from opnsense_log_viewer.utils.logging_config import get_logger, log_exception

logger = get_logger(__name__)

class OPNsenseConfigParser:
    """OPNsense XML configuration parser"""
    
    def __init__(self):
        self.interface_mapping = {}
        self.ip_aliases = {}    # ip/network -> alias_name
        self.port_aliases = {}  # port -> alias_name
        
    def parse_interfaces_from_xml(self, xml_file_path: str) -> Dict[str, str]:
        """
        Parses the OPNsense XML configuration file and extracts interface mapping
        
        Args:
            xml_file_path: Path to the XML configuration file
            
        Returns:
            Dict mapping physical interface -> logical name
        """
        interface_mapping = {}
        
        try:
            tree = ET.parse(xml_file_path)
            root = tree.getroot()
            
            # Look for interfaces section
            interfaces_section = root.find('interfaces')
            if interfaces_section is None:
                print("Section 'interfaces' not found in XML file")
                return interface_mapping
                
            # Go through each defined interface
            for interface_elem in interfaces_section:
                interface_name = interface_elem.tag  # ex: 'lan', 'wan', 'opt1'
                
                # Get the physical interface
                if_elem = interface_elem.find('if')
                if if_elem is not None:
                    physical_if = if_elem.text
                    
                    # Use description only, fallback to interface name
                    descr_elem = interface_elem.find('descr')
                    if descr_elem is not None and descr_elem.text:
                        display_name = descr_elem.text  # Only use description
                    else:
                        display_name = interface_name.upper()  # Fallback to interface name
                    
                    interface_mapping[physical_if] = display_name
                    
        except ET.ParseError as e:
            print(f"XML parsing error: {e}")
        except FileNotFoundError:
            print(f"XML file not found: {xml_file_path}")
        except Exception as e:
            print(f"Error during XML file parsing: {e}")
            
        return interface_mapping
    
    
    
    def parse_aliases_from_xml(self, xml_file_path: str) -> tuple:
        """
        Parses OPNsense XML configuration file and extracts aliases
        
        Args:
            xml_file_path: Path to the XML configuration file
            
        Returns:
            Tuple (ip_aliases_dict, port_aliases_dict)
        """
        ip_aliases = {}
        port_aliases = {}
        
        try:
            tree = ET.parse(xml_file_path)
            root = tree.getroot()
            
            # Look for aliases section (real OPNsense structure)
            aliases_section = root.find('.//Firewall/Alias/aliases')
            if aliases_section is None:
                # Fallback: look directly for 'aliases' for backward compatibility
                aliases_section = root.find('aliases')
                if aliases_section is None:
                    print("Section 'aliases' not found in XML file")
                    return ip_aliases, port_aliases
                
            # Go through each alias
            for alias_elem in aliases_section.findall('alias'):
                # Check if alias is enabled
                enabled_elem = alias_elem.find('enabled')
                if enabled_elem is not None and enabled_elem.text != '1':
                    continue  # Skip disabled aliases
                
                alias_name = self._get_alias_name(alias_elem)
                alias_type = self._get_alias_type(alias_elem)
                alias_content = self._get_alias_content(alias_elem)
                
                if not alias_name or not alias_content:
                    continue
                
                # Classify by type
                if alias_type in ['host', 'network', 'urltable', 'geoip']:
                    # IP/Network alias (including geoip)
                    self._process_ip_alias(alias_name, alias_content, ip_aliases)
                elif alias_type == 'port':
                    # Port alias
                    self._process_port_alias(alias_name, alias_content, port_aliases)
            
            # Resolve alias references (aliases that reference other aliases)
            self._resolve_alias_references(aliases_section, ip_aliases, port_aliases)
                    
        except ET.ParseError as e:
            print(f"XML aliases parsing error: {e}")
        except FileNotFoundError:
            print(f"XML file not found: {xml_file_path}")
        except Exception as e:
            print(f"Error during XML aliases parsing: {e}")
            
        return ip_aliases, port_aliases
    
    
    def _get_alias_name(self, alias_elem) -> str:
        """Extracts the name of an alias"""
        name_elem = alias_elem.find('name')
        return name_elem.text if name_elem is not None and name_elem.text else ""
    
    def _get_alias_type(self, alias_elem) -> str:
        """Extracts the type of an alias"""
        type_elem = alias_elem.find('type')
        return type_elem.text if type_elem is not None and type_elem.text else ""
    
    def _get_alias_content(self, alias_elem) -> str:
        """Extracts the content of an alias"""
        content_elem = alias_elem.find('content')
        return content_elem.text if content_elem is not None and content_elem.text else ""
    
    def _process_ip_alias(self, alias_name: str, content: str, ip_aliases: dict):
        """Process IP/Network alias content"""
        # Content can contain multiple IPs/networks separated by newlines or commas
        items = content.replace('\n', ',').split(',')
        
        for item in items:
            item = item.strip()
            if item:
                # Clean the item (remove spaces, etc.)
                clean_item = item.strip()
                if clean_item:
                    ip_aliases[clean_item] = alias_name
    
    def _process_port_alias(self, alias_name: str, content: str, port_aliases: dict):
        """Process Port alias content"""
        # Content can contain multiple ports separated by newlines or commas
        items = content.replace('\n', ',').split(',')
        
        for item in items:
            item = item.strip()
            if item:
                # Support for ranges (ex: "8080:8090")
                if ':' in item and '-' not in item:
                    # Range format port:port
                    try:
                        start_port, end_port = item.split(':')
                        start_port = int(start_port.strip())
                        end_port = int(end_port.strip())
                        
                        # Add each port in the range
                        for port in range(start_port, end_port + 1):
                            port_aliases[str(port)] = alias_name
                    except ValueError:
                        # If conversion fails, treat as simple port
                        port_aliases[item] = alias_name
                elif '-' in item:
                    # Range format port-port
                    try:
                        start_port, end_port = item.split('-')
                        start_port = int(start_port.strip())
                        end_port = int(end_port.strip())
                        
                        # Add each port in the range
                        for port in range(start_port, end_port + 1):
                            port_aliases[str(port)] = alias_name
                    except ValueError:
                        # If conversion fails, treat as simple port
                        port_aliases[item] = alias_name
                else:
                    # Simple port
                    port_aliases[item] = alias_name
    
    def get_ip_alias(self, ip_or_network: str) -> Optional[str]:
        """Returns the alias of an IP or network"""
        return self.ip_aliases.get(ip_or_network)
    
    def get_port_alias(self, port: str) -> Optional[str]:
        """Returns the alias of a port"""
        return self.port_aliases.get(port)
    
    def _resolve_alias_references(self, aliases_section, ip_aliases: dict, port_aliases: dict):
        """Resolves alias references (aliases that reference other aliases)"""
        # Create a mapping alias_name -> resolved_content
        alias_definitions = {}
        
        # First pass: collect all aliases with their content
        for alias_elem in aliases_section.findall('alias'):
            enabled_elem = alias_elem.find('enabled')
            if enabled_elem is not None and enabled_elem.text != '1':
                continue
                
            alias_name = self._get_alias_name(alias_elem)
            alias_content = self._get_alias_content(alias_elem)
            alias_type = self._get_alias_type(alias_elem)
            
            if alias_name and alias_content:
                alias_definitions[alias_name] = {
                    'content': alias_content,
                    'type': alias_type
                }
        
        # Second pass: resolve references
        for alias_elem in aliases_section.findall('alias'):
            enabled_elem = alias_elem.find('enabled')
            if enabled_elem is not None and enabled_elem.text != '1':
                continue
                
            alias_name = self._get_alias_name(alias_elem)
            alias_content = self._get_alias_content(alias_elem)
            alias_type = self._get_alias_type(alias_elem)
            
            if not alias_name or not alias_content:
                continue
            
            # Check if content contains references to other aliases
            lines = alias_content.replace('\n', ',').split(',')
            resolved_content = []
            
            for line in lines:
                line = line.strip()
                if line:
                    # Check if it's a reference to another alias
                    if line in alias_definitions and line != alias_name:
                        # It's a reference to another alias
                        referenced_alias = alias_definitions[line]
                        referenced_content = referenced_alias['content']
                        
                        # Add the referenced alias content
                        ref_lines = referenced_content.replace('\n', ',').split(',')
                        for ref_line in ref_lines:
                            ref_line = ref_line.strip()
                            if ref_line:
                                resolved_content.append(ref_line)
                    else:
                        # Normal content
                        resolved_content.append(line)
            
            # If we have resolved content, process it
            if resolved_content:
                resolved_content_str = '\n'.join(resolved_content)
                
                # Reprocess with resolved content
                if alias_type in ['host', 'network', 'urltable', 'geoip']:
                    self._process_ip_alias(alias_name, resolved_content_str, ip_aliases)
                elif alias_type == 'port':
                    self._process_port_alias(alias_name, resolved_content_str, port_aliases)
    