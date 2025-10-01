#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
OPNsense SSH Client for Rule Label Extraction
Connects to OPNsense via SSH and retrieves firewall rule labels
"""

import paramiko
import time
import re
import socket
from typing import Dict, Optional, Tuple
import threading

from opnsense_log_viewer.exceptions import SSHConnectionError
from opnsense_log_viewer.utils.logging_config import get_logger, log_exception

logger = get_logger(__name__)


class OPNsenseSSHClient:
    """SSH client for OPNsense with rule label extraction"""
    
    def __init__(self):
        self.ssh_client = None
        self.shell_channel = None
        self.connected = False
        
    def connect(self, hostname: str, username: str, password: str, port: int = 22, timeout: int = 10) -> Tuple[bool, str]:
        """
        Connects to OPNsense via SSH
        
        Args:
            hostname: OPNsense IP address or hostname
            username: SSH username
            password: SSH password
            port: SSH port (default 22)
            timeout: Connection timeout in seconds
            
        Returns:
            Tuple (success, message)
        """
        try:
            # Create SSH client
            self.ssh_client = paramiko.SSHClient()
            self.ssh_client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
            
            # SSH connection
            self.ssh_client.connect(
                hostname=hostname,
                port=port,
                username=username,
                password=password,
                timeout=timeout,
                allow_agent=False,
                look_for_keys=False
            )
            
            # Open an interactive shell
            self.shell_channel = self.ssh_client.invoke_shell()
            self.shell_channel.settimeout(timeout)
            
            # Wait for OPNsense menu prompt
            self._wait_for_prompt("Enter an option:", timeout=5)
            
            self.connected = True
            return True, "SSH connection established successfully"
            
        except socket.timeout as e:
            logger.error(f"SSH connection timeout to {hostname}:{port}")
            raise SSHConnectionError(f"Connection timeout ({timeout}s)", hostname=hostname, username=username, error_type="timeout", original_error=e)
        except paramiko.AuthenticationException as e:
            logger.error(f"SSH authentication failed for {username}@{hostname}")
            raise SSHConnectionError("SSH authentication failed (incorrect credentials)", hostname=hostname, username=username, error_type="authentication", original_error=e)
        except paramiko.SSHException as e:
            return False, f"SSH error: {str(e)}"
        except Exception as e:
            return False, f"Connection error: {str(e)}"
    
    def _wait_for_prompt(self, expected_prompt: str, timeout: int = 5) -> str:
        """Waits for a specific prompt and returns the output"""
        output = ""
        start_time = time.time()
        
        while time.time() - start_time < timeout:
            if self.shell_channel.recv_ready():
                chunk = self.shell_channel.recv(1024).decode('utf-8', errors='ignore')
                output += chunk
                
                if expected_prompt in output:
                    return output
            
            time.sleep(0.1)
        
        raise TimeoutError(f"Timeout waiting for prompt: {expected_prompt}")
    
    def _send_command(self, command: str) -> None:
        """Sends a command via the shell"""
        self.shell_channel.send(command + '\n')
        time.sleep(0.5)  # Short delay for transmission
    
    def extract_rule_labels(self, timeout: int = 10) -> Tuple[bool, str, Dict[str, str]]:
        """
        Extracts rule labels via /tmp/rules.debug
        
        Args:
            timeout: Timeout for the complete operation
            
        Returns:
            Tuple (success, message, label_descriptions_dict)
        """
        if not self.connected or not self.shell_channel:
            return False, "No active SSH connection", {}
        
        try:
            # Step 1: Go to shell (option 8)
            self._send_command("8")
            
            # Wait for shell prompt
            self._wait_for_prompt("# ", timeout=5)
            
            # Step 2: Read rules.debug file which contains labels with descriptions
            self._send_command("cat /tmp/rules.debug")
            
            # Wait for execution to finish (file may be large)
            start_time = time.time()
            output = ""
            
            while time.time() - start_time < timeout:
                if self.shell_channel.recv_ready():
                    chunk = self.shell_channel.recv(8192).decode('utf-8', errors='ignore')
                    output += chunk
                    
                    # Detect end of command (prompt return)
                    if "# " in chunk and len(chunk) < 100:  # Short prompt = end
                        break
                
                time.sleep(0.1)
            
            # Parse output to extract labels and descriptions
            label_descriptions = self._parse_rules_debug_output(output)
            
            return True, f"Extraction successful: {len(label_descriptions)} rules found", label_descriptions
            
        except TimeoutError as e:
            return False, f"Timeout: {str(e)}", {}
        except Exception as e:
            return False, f"Error during extraction: {str(e)}", {}
    
    def _parse_rules_debug_output(self, output: str) -> Dict[str, str]:
        """
        Parses /tmp/rules.debug output to extract labels and descriptions
        
        Expected format:
        block in log quick inet from {<crowdsec_blocklists>} to {any} label "031d9d1edc75c3c8c634a8aee47134ef" # CrowdSec (IPv4) in
        
        Args:
            output: Content of /tmp/rules.debug file
            
        Returns:
            Dictionary {label_hash: description}
        """
        label_descriptions = {}
        
        # Pattern to match rules with labels and comments
        # label "hash" # description
        pattern = r'label\s+"([^"]+)"\s*#\s*(.+?)(?:\n|$)'
        
        matches = re.findall(pattern, output, re.MULTILINE)
        
        for label_hash, description in matches:
            # Clean description (remove extra spaces)
            clean_description = description.strip()
            label_descriptions[label_hash] = clean_description
        
        return label_descriptions
    
    
    def disconnect(self) -> None:
        """Closes SSH connection"""
        try:
            if self.shell_channel:
                self.shell_channel.close()
            if self.ssh_client:
                self.ssh_client.close()
        except:
            pass
        finally:
            self.connected = False
            self.shell_channel = None
            self.ssh_client = None
    
    
    def __del__(self):
        """Destructor - properly closes the connection"""
        self.disconnect()


class RuleLabelMapper:
    """Class to map rule labels with their descriptions directly"""
    
    def __init__(self):
        self.label_descriptions = {}  # label_hash -> description (from /tmp/rules.debug)
        
    def set_label_descriptions(self, label_descriptions: Dict[str, str]) -> None:
        """Sets descriptions extracted via SSH /tmp/rules.debug (label_hash -> description)"""
        self.label_descriptions = label_descriptions.copy()
    
    def get_rule_description_by_hash(self, label_hash: str) -> Optional[str]:
        """Returns the description of a rule by its label hash"""
        return self.label_descriptions.get(label_hash)
    
    def get_mapping_stats(self) -> Dict[str, int]:
        """Returns mapping statistics"""
        return {
            "total_labels": len(self.label_descriptions)
        }


