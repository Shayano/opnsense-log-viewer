"""
Improved OPNsense Log Viewer Application
Enhanced multithreading, unlimited file size support, English interface
"""
import tkinter as tk
from tkinter import ttk, filedialog, messagebox, scrolledtext
import os
import threading
from datetime import datetime
from typing import List, Optional, Dict, Any
import json

from log_parser import OPNsenseLogParser, LogEntry
from config_parser import OPNsenseConfigParser
from log_filter import LogFilter
from virtual_log_manager import VirtualLogManager
from ssh_client import OPNsenseSSHClient, RuleLabelMapper

class ProgressDialog:
    """Progress dialog for long operations"""
    def __init__(self, parent, title="Loading..."):
        self.parent = parent
        self.dialog = tk.Toplevel(parent)
        self.dialog.title(title)
        self.dialog.geometry("400x100")
        self.dialog.resizable(False, False)
        self.dialog.transient(parent)
        self.dialog.grab_set()
        
        # Center the dialog
        self.dialog.update_idletasks()
        x = (self.dialog.winfo_screenwidth() // 2) - (400 // 2)
        y = (self.dialog.winfo_screenheight() // 2) - (100 // 2)
        self.dialog.geometry(f"400x100+{x}+{y}")
        
        self.label = ttk.Label(self.dialog, text="Loading...")
        self.label.pack(pady=10)
        
        self.progress = ttk.Progressbar(self.dialog, mode='indeterminate')
        self.progress.pack(pady=10, padx=20, fill='x')
        self.progress.start()
        
        self.cancel_button = ttk.Button(self.dialog, text="Cancel", command=self.cancel)
        self.cancel_button.pack(pady=5)
        
        self.cancelled = False
        
    def update_text(self, text):
        self.label.config(text=text)
        
    def cancel(self):
        self.cancelled = True
        self.close()
        
    def close(self):
        self.progress.stop()
        self.dialog.destroy()

class LogViewerApp:
    """Main OPNsense Log Viewer Application"""
    
    def __init__(self, root):
        self.root = root
        self.root.title("OPNsense Log Viewer")
        self.root.geometry("1500x1000")
        
        # Set icon if available
        try:
            self.root.iconbitmap("icon/icon32.ico")
        except:
            pass
            
        # Initialize components
        self.log_parser = OPNsenseLogParser()
        self.config_parser = OPNsenseConfigParser()
        self.log_filter = LogFilter()
        self.virtual_log_manager = VirtualLogManager(chunk_size=1000, cache_size=50, log_parser=self.log_parser)
        
        # Inject threaded methods immediately to avoid runtime issues
        self._inject_threaded_methods()
        
        # State variables
        self.displayed_entries = []  # Currently displayed entries
        self.current_log_file = None
        self.current_config_file = None  # Current XML config file
        self.is_loading = False
        self.loading_thread = None
        self.progress_dialog = None
        self.menu_collapsed = False  # Track menu state
        self.total_entries_count = 0  # Total entries in current file/filter
        
        # Pagination
        self.page_size = 1000
        self.current_page = 0
        self.total_pages = 0
        
        # Filter management
        self.active_filters = []  # List of active filter conditions
        self.using_fast_tail = False  # Flag to track if we're using fast tail mode
        
        # Time filter variables
        self.time_filter_enabled = tk.BooleanVar()
        self.start_time_var = tk.StringVar()
        self.end_time_var = tk.StringVar()
        
        # SSH and rule label variables
        self.ssh_client = OPNsenseSSHClient()
        self.rule_mapper = RuleLabelMapper()
        self.ssh_address = tk.StringVar()
        self.ssh_port = tk.StringVar(value="22")  # Default SSH port
        self.ssh_username = tk.StringVar()
        self.ssh_password = tk.StringVar()
        self.ssh_connected = False
        self.rule_labels_loaded = False
        
        self.setup_ui()
    
    def _update_pagination_buttons(self):
        """Update pagination button states based on current page"""
        try:
            # Get real total entries to determine last accessible page
            if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
                total_entries = self.virtual_log_manager.get_total_entries()
            else:
                total_entries = self.total_entries_count
            
            # Calculate last accessible page
            if total_entries > 0:
                last_page = max(0, ((total_entries - 1) // self.page_size))
            else:
                last_page = 0
            
            # Enable/disable buttons based on current position
            # First and Previous buttons
            if self.current_page <= 0:
                self.first_button.config(state='disabled')
                self.prev_button.config(state='disabled')
            else:
                self.first_button.config(state='normal')
                self.prev_button.config(state='normal')
            
            # Next and Last buttons
            if self.current_page >= last_page:
                self.next_button.config(state='disabled')
                self.last_button.config(state='disabled')
            else:
                self.next_button.config(state='normal')
                self.last_button.config(state='normal')
                
        except Exception as e:
            print(f"Error updating pagination buttons: {e}")
    
    def _get_total_pages(self):
        """Calculate total pages based on current data"""
        try:
            if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
                total_entries = self.virtual_log_manager.get_total_entries()
            else:
                total_entries = self.total_entries_count
            
            # Ensure total_entries is valid
            if not isinstance(total_entries, int) or total_entries < 0:
                total_entries = 0
            
            if total_entries > 0:
                # Correct calculation: ceil(total_entries / page_size)
                total_pages = (total_entries + self.page_size - 1) // self.page_size
                return max(1, total_pages)
            else:
                return 1
        except Exception as e:
            print(f"Error in _get_total_pages: {e}")
            return 1
    
    def _inject_threaded_methods(self):
        """Inject threaded methods into VirtualLogManager"""
        import types
        from virtual_log_manager import VirtualLogManager
        
        if not hasattr(self.virtual_log_manager, '_apply_threaded_filter'):
            self.virtual_log_manager._apply_threaded_filter = types.MethodType(
                VirtualLogManager._apply_threaded_filter, self.virtual_log_manager
            )
        
        if not hasattr(self.virtual_log_manager, '_apply_sequential_filter'):
            self.virtual_log_manager._apply_sequential_filter = types.MethodType(
                VirtualLogManager._apply_sequential_filter, self.virtual_log_manager
            )
        
    def setup_ui(self):
        """Setup the user interface"""
        # Main menu
        self.setup_menu()
        
        # Main frame with panels
        main_frame = ttk.Frame(self.root)
        main_frame.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        # Frame for menu collapse button
        collapse_frame = ttk.Frame(main_frame)
        collapse_frame.pack(fill=tk.X, pady=(0, 2))
        
        self.collapse_button = ttk.Button(collapse_frame, text="▼ Hide Menu", command=self.toggle_menu)
        self.collapse_button.pack(side=tk.LEFT)
        
        # Control panel (collapsible)
        self.control_frame = ttk.LabelFrame(main_frame, text="Controls")
        self.control_frame.pack(fill=tk.X, pady=(0, 5))
        
        # File controls
        file_frame = ttk.Frame(self.control_frame)
        file_frame.pack(fill=tk.X, padx=5, pady=5)
        
        ttk.Button(file_frame, text="Load Log File", command=self.load_log_file).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(file_frame, text="Load XML Config", command=self.load_config_xml).pack(side=tk.LEFT, padx=(0, 5))
        
        # File info frame (vertical layout for both files)
        file_info_frame = ttk.Frame(file_frame)
        file_info_frame.pack(side=tk.LEFT, padx=(10, 0))
        
        self.log_file_label = ttk.Label(file_info_frame, text="Log file: None", font=("TkDefaultFont", 8))
        self.log_file_label.pack(anchor=tk.W)
        
        self.config_file_label = ttk.Label(file_info_frame, text="Config file: None", font=("TkDefaultFont", 8))
        self.config_file_label.pack(anchor=tk.W)
        
        # SSH connection panel (inside Controls, after file buttons)
        self.setup_ssh_panel_inline()
        
        # Quick filters
        preset_frame = ttk.LabelFrame(self.control_frame, text="Quick Filters")
        preset_frame.pack(fill=tk.X, padx=5, pady=5)
        
        ttk.Button(preset_frame, text="All Traffic", command=lambda: self.apply_preset_filter('all')).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(preset_frame, text="Blocked Traffic", command=lambda: self.apply_preset_filter('blocked')).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(preset_frame, text="Allowed Traffic", command=lambda: self.apply_preset_filter('allowed')).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(preset_frame, text="TCP", command=lambda: self.apply_preset_filter('tcp')).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(preset_frame, text="UDP", command=lambda: self.apply_preset_filter('udp')).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(preset_frame, text="Reset Filters", command=self.clear_filters).pack(side=tk.LEFT, padx=(10, 0))
        
        # Time filter panel
        self.time_filter_frame = self.setup_time_filter_panel(main_frame)
        
        # Advanced filtering panel (moved out of Controls)
        self.advanced_filter_frame = self.setup_filter_panel(main_frame)
        
        # Main notebook for tabs
        notebook = ttk.Notebook(main_frame)
        notebook.pack(fill=tk.BOTH, expand=True)
        
        # Log table tab
        self.setup_log_table_tab(notebook)
        
        # Details tab
        self.setup_details_tab(notebook)
        
        # Status bar
        self.status_bar = ttk.Label(main_frame, text="Ready", relief=tk.SUNKEN)
        self.status_bar.pack(side=tk.BOTTOM, fill=tk.X)
        
    def setup_menu(self):
        """Setup main menu"""
        menubar = tk.Menu(self.root)
        self.root.config(menu=menubar)
        
        # File menu
        file_menu = tk.Menu(menubar, tearoff=0)
        menubar.add_cascade(label="File", menu=file_menu)
        file_menu.add_command(label="Open Logs...", command=self.load_log_file)
        file_menu.add_command(label="Open XML Config...", command=self.load_config_xml)
        file_menu.add_separator()
        file_menu.add_command(label="Export Results...", command=self.export_results)
        file_menu.add_separator()
        file_menu.add_command(label="Exit", command=self.root.quit)
        
        # Tools menu
        tools_menu = tk.Menu(menubar, tearoff=0)
        menubar.add_cascade(label="Tools", menu=tools_menu)
        tools_menu.add_command(label="Interfaces", command=self.show_interfaces)
        
        # Help menu
        help_menu = tk.Menu(menubar, tearoff=0)
        menubar.add_cascade(label="Help", menu=help_menu)
        help_menu.add_command(label="About", command=self.show_about)
        
    def toggle_menu(self):
        """Toggle controls menu display"""
        if self.menu_collapsed:
            # Show menu
            self.control_frame.pack(fill=tk.X, pady=(0, 5), after=self.collapse_button.master)
            if hasattr(self, 'time_filter_frame'):
                self.time_filter_frame.pack(fill=tk.X, padx=5, pady=5, after=self.control_frame)
            if hasattr(self, 'advanced_filter_frame'):
                self.advanced_filter_frame.pack(fill=tk.X, padx=5, pady=5, after=self.time_filter_frame)
            self.collapse_button.config(text="▼ Hide Menu")
            self.menu_collapsed = False
        else:
            # Hide menu
            self.control_frame.pack_forget()
            if hasattr(self, 'time_filter_frame'):
                self.time_filter_frame.pack_forget()
            if hasattr(self, 'advanced_filter_frame'):
                self.advanced_filter_frame.pack_forget()
            self.collapse_button.config(text="▲ Show Menu")
            self.menu_collapsed = True
            
    def setup_time_filter_panel(self, parent):
        """Setup time filter panel"""
        time_frame = ttk.LabelFrame(parent, text="Time Filter")
        time_frame.pack(fill=tk.X, padx=5, pady=5)
        
        # Row 1: Enable
        enable_frame = ttk.Frame(time_frame)
        enable_frame.pack(fill=tk.X, padx=5, pady=2)
        
        ttk.Checkbutton(enable_frame, text="Enable time filter", 
                       variable=self.time_filter_enabled, 
                       command=self.on_time_filter_toggle).pack(side=tk.LEFT)
        
        # Row 2: Time range
        range_frame = ttk.Frame(time_frame)
        range_frame.pack(fill=tk.X, padx=5, pady=2)
        
        ttk.Label(range_frame, text="From:").pack(side=tk.LEFT)
        self.start_time_entry = ttk.Entry(range_frame, textvariable=self.start_time_var, width=20)
        self.start_time_entry.pack(side=tk.LEFT, padx=(5, 10))
        
        ttk.Label(range_frame, text="To:").pack(side=tk.LEFT)
        self.end_time_entry = ttk.Entry(range_frame, textvariable=self.end_time_var, width=20)
        self.end_time_entry.pack(side=tk.LEFT, padx=(5, 10))
        
        ttk.Label(range_frame, text="(Format: YYYY-MM-DD HH:MM:SS)").pack(side=tk.LEFT, padx=(10, 0))
        
        return time_frame
        
    def on_time_filter_toggle(self):
        """Handle time filter enable/disable"""
        enabled = self.time_filter_enabled.get()
        state = tk.NORMAL if enabled else tk.DISABLED
        self.start_time_entry.config(state=state)
        self.end_time_entry.config(state=state)
    
    def setup_ssh_panel_inline(self):
        """Setup SSH connection panel inline within Controls"""
        ssh_frame = ttk.LabelFrame(self.control_frame, text="SSH Connection (Rule Labels)")
        ssh_frame.pack(fill=tk.X, padx=5, pady=5)
        
        # SSH connection fields
        ssh_fields_frame = ttk.Frame(ssh_frame)
        ssh_fields_frame.pack(fill=tk.X, padx=5, pady=5)
        
        # Address field
        ttk.Label(ssh_fields_frame, text="Address:").grid(row=0, column=0, sticky=tk.W, padx=(0, 5))
        address_entry = ttk.Entry(ssh_fields_frame, textvariable=self.ssh_address, width=15)
        address_entry.grid(row=0, column=1, padx=(0, 10))
        
        # Port field
        ttk.Label(ssh_fields_frame, text="Port:").grid(row=0, column=2, sticky=tk.W, padx=(0, 5))
        port_entry = ttk.Entry(ssh_fields_frame, textvariable=self.ssh_port, width=6)
        port_entry.grid(row=0, column=3, padx=(0, 10))
        
        # Username field
        ttk.Label(ssh_fields_frame, text="User:").grid(row=0, column=4, sticky=tk.W, padx=(0, 5))
        user_entry = ttk.Entry(ssh_fields_frame, textvariable=self.ssh_username, width=12)
        user_entry.grid(row=0, column=5, padx=(0, 10))
        
        # Password field
        ttk.Label(ssh_fields_frame, text="Password:").grid(row=0, column=6, sticky=tk.W, padx=(0, 5))
        password_entry = ttk.Entry(ssh_fields_frame, textvariable=self.ssh_password, show="*", width=12)
        password_entry.grid(row=0, column=7, padx=(0, 10))
        
        # Connect button
        self.ssh_connect_button = ttk.Button(ssh_fields_frame, text="Connect & Get Labels", 
                                           command=self.connect_ssh_and_get_labels)
        self.ssh_connect_button.grid(row=0, column=8, padx=(10, 0))
        
        # Status label
        self.ssh_status_label = ttk.Label(ssh_fields_frame, text="Not connected", foreground="red")
        self.ssh_status_label.grid(row=0, column=9, padx=(10, 0))
    
        
    def setup_filter_panel(self, parent):
        """Setup advanced filtering panel"""
        filter_frame = ttk.LabelFrame(parent, text="Advanced Filters")
        filter_frame.pack(fill=tk.X, padx=5, pady=5)
        
        # Add filter frame
        add_filter_frame = ttk.Frame(filter_frame)
        add_filter_frame.pack(fill=tk.X, padx=5, pady=5)
        
        # Field
        ttk.Label(add_filter_frame, text="Field:").pack(side=tk.LEFT)
        self.field_var = tk.StringVar()
        self.field_combo = ttk.Combobox(add_filter_frame, textvariable=self.field_var, 
                                       values=['Action', 'Interface', 'Source', 'Source port', 
                                             'Destination', 'Destination port', 'Protocol', 'Label'], width=15)
        self.field_combo.pack(side=tk.LEFT, padx=(5, 10))
        
        # Operator
        ttk.Label(add_filter_frame, text="Operator:").pack(side=tk.LEFT)
        self.operator_var = tk.StringVar(value='==')
        operator_combo = ttk.Combobox(add_filter_frame, textvariable=self.operator_var,
                                    values=['==', '!=', 'contains', 'startswith', 'endswith', 'regex'], width=10)
        operator_combo.pack(side=tk.LEFT, padx=(5, 10))
        
        # Value
        ttk.Label(add_filter_frame, text="Value:").pack(side=tk.LEFT)
        self.value_var = tk.StringVar()
        value_entry = ttk.Entry(add_filter_frame, textvariable=self.value_var, width=20)
        value_entry.pack(side=tk.LEFT, padx=(5, 10))
        
        # Logic operator
        ttk.Label(add_filter_frame, text="Logic:").pack(side=tk.LEFT)
        self.logic_var = tk.StringVar(value='AND')
        logic_combo = ttk.Combobox(add_filter_frame, textvariable=self.logic_var,
                                 values=['AND', 'OR'], width=8)
        logic_combo.pack(side=tk.LEFT, padx=(5, 10))
        
        # NOT
        self.negate_var = tk.BooleanVar()
        ttk.Checkbutton(add_filter_frame, text="NOT", variable=self.negate_var).pack(side=tk.LEFT, padx=(5, 10))
        
        # Buttons
        ttk.Button(add_filter_frame, text="Add Filter", command=self.add_filter).pack(side=tk.LEFT, padx=(10, 5))
        ttk.Button(add_filter_frame, text="Apply Filters", command=self.apply_filters).pack(side=tk.LEFT)
        
        # Active filters display
        self.setup_active_filters_display(filter_frame)
        
        return filter_frame
        
    def setup_active_filters_display(self, parent):
        """Setup display for active filters with individual removal"""
        self.active_filters_frame = ttk.LabelFrame(parent, text="Active Filters")
        self.active_filters_frame.pack(fill=tk.X, padx=5, pady=5)
        
        # Scrollable frame for filters
        self.filters_canvas = tk.Canvas(self.active_filters_frame, height=60)
        self.filters_scrollbar = ttk.Scrollbar(self.active_filters_frame, orient="horizontal", command=self.filters_canvas.xview)
        self.filters_scrollable_frame = ttk.Frame(self.filters_canvas)
        
        self.filters_scrollable_frame.bind(
            "<Configure>",
            lambda e: self.filters_canvas.configure(scrollregion=self.filters_canvas.bbox("all"))
        )
        
        self.filters_canvas.create_window((0, 0), window=self.filters_scrollable_frame, anchor="nw")
        self.filters_canvas.configure(xscrollcommand=self.filters_scrollbar.set)
        
        self.filters_canvas.pack(side="top", fill="both", expand=True)
        self.filters_scrollbar.pack(side="bottom", fill="x")
        
        self.update_active_filters_display()
        
    def setup_log_table_tab(self, notebook):
        """Setup log table tab with pagination"""
        table_frame = ttk.Frame(notebook)
        notebook.add(table_frame, text="Logs")
        
        # Pagination controls
        pagination_frame = ttk.Frame(table_frame)
        pagination_frame.pack(fill=tk.X, padx=5, pady=5)
        
        self.first_button = ttk.Button(pagination_frame, text="First", command=self.first_page)
        self.first_button.pack(side=tk.LEFT, padx=2)
        
        self.prev_button = ttk.Button(pagination_frame, text="Previous", command=self.prev_page)
        self.prev_button.pack(side=tk.LEFT, padx=2)
        
        self.page_label = ttk.Label(pagination_frame, text="Page 0 of 0")
        self.page_label.pack(side=tk.LEFT, padx=10)
        
        self.next_button = ttk.Button(pagination_frame, text="Next", command=self.next_page)
        self.next_button.pack(side=tk.LEFT, padx=2)
        
        self.last_button = ttk.Button(pagination_frame, text="Last", command=self.last_page)
        self.last_button.pack(side=tk.LEFT, padx=2)
        
        # Page size selector
        ttk.Label(pagination_frame, text="Rows per page:").pack(side=tk.LEFT, padx=(20, 5))
        self.page_size_var = tk.StringVar(value="1000")
        page_size_combo = ttk.Combobox(pagination_frame, textvariable=self.page_size_var,
                                      values=['100', '500', '1000', '2500', '5000'], width=8)
        page_size_combo.pack(side=tk.LEFT)
        page_size_combo.bind('<<ComboboxSelected>>', self.on_page_size_change)
        
        # Tree view with scrollbars
        tree_frame = ttk.Frame(table_frame)
        tree_frame.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        # Columns to display (new order: Timestamp Action Interface Source Srcport Destination Dstport Protocol)
        columns = ('timestamp', 'action', 'interface', 'src', 'srcport', 'dst', 'dstport', 'proto', 'label')
        self.log_tree = ttk.Treeview(tree_frame, columns=columns, show='headings', height=20)
        
        # Column configuration with custom names
        col_widths = {'timestamp': 150, 'action': 80, 'interface': 120, 'src': 120, 
                     'srcport': 80, 'dst': 120, 'dstport': 80, 'proto': 80, 'label': 200}
        
        # Custom column headers
        col_headers = {
            'timestamp': 'Timestamp',
            'action': 'Action',
            'interface': 'Interface',
            'src': 'Source',
            'srcport': 'Source port',
            'dst': 'Destination',
            'dstport': 'Destination port',
            'proto': 'Protocol',
            'label': 'Label'
        }
        
        for col in columns:
            self.log_tree.heading(col, text=col_headers[col], command=lambda c=col: self.sort_column(c))
            self.log_tree.column(col, width=col_widths.get(col, 100))
        
        # Scrollbars
        v_scrollbar = ttk.Scrollbar(tree_frame, orient=tk.VERTICAL, command=self.log_tree.yview)
        h_scrollbar = ttk.Scrollbar(tree_frame, orient=tk.HORIZONTAL, command=self.log_tree.xview)
        self.log_tree.configure(yscrollcommand=v_scrollbar.set, xscrollcommand=h_scrollbar.set)
        
        # Grid placement
        self.log_tree.grid(row=0, column=0, sticky='nsew')
        v_scrollbar.grid(row=0, column=1, sticky='ns')
        h_scrollbar.grid(row=1, column=0, sticky='ew')
        
        tree_frame.grid_columnconfigure(0, weight=1)
        tree_frame.grid_rowconfigure(0, weight=1)
        
        # Bindings
        self.log_tree.bind('<<TreeviewSelect>>', self.on_log_select)
        self.log_tree.bind('<Double-1>', self.on_cell_double_click)  # Double-click to copy cell content
        
    def setup_details_tab(self, notebook):
        """Setup details tab"""
        details_frame = ttk.Frame(notebook)
        notebook.add(details_frame, text="Details")
        
        # Details text area
        self.details_text = scrolledtext.ScrolledText(details_frame, wrap=tk.WORD, height=25)
        self.details_text.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
    def load_log_file(self):
        """Load a log file"""
        file_path = filedialog.askopenfilename(
            title="Select Log File",
            filetypes=[("Log files", "*.log *.txt"), ("All files", "*.*")]
        )
        
        if file_path:
            self.current_log_file = file_path
            self.update_file_labels()
            self.load_logs_threaded()
    
    def update_file_labels(self):
        """Update the file labels display"""
        # Update log file label
        if self.current_log_file:
            log_filename = os.path.basename(self.current_log_file)
            self.log_file_label.config(text=f"Log file: {log_filename}")
        else:
            self.log_file_label.config(text="Log file: None")
        
        # Update config file label
        if self.current_config_file:
            config_filename = os.path.basename(self.current_config_file)
            self.config_file_label.config(text=f"Config file: {config_filename}")
        else:
            self.config_file_label.config(text="Config file: None")
            
    def load_config_xml(self):
        """Load XML configuration file"""
        file_path = filedialog.askopenfilename(
            title="Select XML Configuration File",
            filetypes=[("XML files", "*.xml"), ("All files", "*.*")]
        )
        
        if file_path:
            try:
                # Load interface mapping
                interface_mapping = self.config_parser.parse_interfaces_from_xml(file_path)
                self.log_parser.set_interface_mapping(interface_mapping)
                
                # Load aliases
                ip_aliases, port_aliases = self.config_parser.parse_aliases_from_xml(file_path)
                self.config_parser.ip_aliases = ip_aliases
                self.config_parser.port_aliases = port_aliases
                
                # Store config file path and update labels
                self.current_config_file = file_path
                self.update_file_labels()
                
                messagebox.showinfo("Success", 
                                  f"Configuration loaded:\n"
                                  f"• {len(interface_mapping)} interfaces\n"
                                  f"• {len(ip_aliases)} IP aliases\n"
                                  f"• {len(port_aliases)} port aliases")
                
                # Reload logs if already loaded to apply new mapping
                if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
                    self.apply_interface_mapping()
                    self.refresh_display()
                    
            except Exception as e:
                messagebox.showerror("Error", f"Error loading configuration: {e}")
                
    def apply_interface_mapping(self):
        """Apply interface mapping to virtual manager"""
        if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
            # Update virtual manager with new interface mapping
            self.virtual_log_manager.set_interface_mapping(self.log_parser.interface_mapping)
                
    def load_logs_threaded(self):
        """Load logs using virtual manager (memory efficient)"""
        if self.is_loading:
            return
            
        self.is_loading = True
        self.progress_dialog = ProgressDialog(self.root, "Loading Log File")
        
        def load_worker():
            try:
                def progress_callback(message):
                    if not self.progress_dialog.cancelled:
                        self.progress_dialog.update_text(message)
                
                # Use virtual log manager for memory-efficient loading
                self.virtual_log_manager.load_file(self.current_log_file, progress_callback)
                
                if self.progress_dialog.cancelled:
                    return
                
                # Update UI in main thread
                total_entries = self.virtual_log_manager.get_total_entries()
                self.root.after(0, lambda: self.on_logs_loaded_virtual(total_entries))
                
            except Exception as e:
                self.root.after(0, lambda: self.on_load_error(str(e)))
                
        self.loading_thread = threading.Thread(target=load_worker)
        self.loading_thread.daemon = True
        self.loading_thread.start()
        
    def on_logs_loaded_virtual(self, total_entries):
        """Called when logs are loaded using virtual manager"""
        self.is_loading = False
        if self.progress_dialog:
            self.progress_dialog.close()
            self.progress_dialog = None
        
        self.total_entries_count = total_entries
        self.current_page = 0
        self.refresh_display()
        
        # Show memory usage info
        memory_info = self.virtual_log_manager.get_memory_info()
        self.status_bar.config(text=f"Loaded {total_entries:,} entries (Memory-efficient mode: ~{memory_info['estimated_total_memory_mb']:.1f}MB)")
        
        
    def on_load_error(self, error_message):
        """Called on loading error"""
        self.is_loading = False
        if self.progress_dialog:
            self.progress_dialog.close()
            self.progress_dialog = None
        self.status_bar.config(text="Loading error")
        messagebox.showerror("Error", f"Error loading file: {error_message}")
        
    def refresh_display(self):
        """Refresh the table display with pagination (memory-efficient)"""
        # If we're in fast tail mode and trying to navigate, exit fast tail mode
        if getattr(self, 'using_fast_tail', False):
            self.using_fast_tail = False
        
        # Use virtual manager for total count
        if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
            total_entries = self.virtual_log_manager.get_total_entries()
        else:
            # Fallback when no file is loaded
            total_entries = self.total_entries_count
        
        # Calculate pagination - use correct formula
        self.total_pages = max(1, (total_entries + self.page_size - 1) // self.page_size)
        
        if self.current_page >= self.total_pages:
            self.current_page = max(0, self.total_pages - 1)
            
        # Get entries for current page using virtual manager
        start_idx = self.current_page * self.page_size
        
        try:
            if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
                # Use virtual manager (memory efficient)
                # Calculate how many entries we can actually get
                max_count = min(self.page_size, max(0, total_entries - start_idx))
                
                if max_count > 0:
                    self.displayed_entries = self.virtual_log_manager.get_entries(start_idx, max_count)
                else:
                    self.displayed_entries = []
            else:
                # No file loaded
                self.displayed_entries = []
        except Exception as e:
            print(f"Error getting entries: {e}")
            import traceback
            traceback.print_exc()
            self.displayed_entries = []
        
        # Clear table
        for item in self.log_tree.get_children():
            self.log_tree.delete(item)
            
        # Add entries for current page (new order: timestamp, action, interface, src, srcport, dst, dstport, proto, label)
        for entry in self.displayed_entries:
            # Get rule label for this entry
            rule_label = self.get_rule_label_for_entry(entry)
            
            # Enrich with aliases
            src_display = self.get_enriched_ip(entry.get('src', ''))
            srcport_display = self.get_enriched_port(entry.get('srcport', ''))
            dst_display = self.get_enriched_ip(entry.get('dst', ''))
            dstport_display = self.get_enriched_port(entry.get('dstport', ''))
            
            values = (
                entry.timestamp.strftime('%Y-%m-%d %H:%M:%S') if entry.timestamp else '',
                entry.get('action', ''),
                entry.get('interface_display', ''),
                src_display,
                srcport_display,
                dst_display,
                dstport_display,
                entry.get('protoname', ''),
                rule_label
            )
            
            # Color coding based on action
            tags = []
            if entry.get('action') == 'block':
                tags.append('blocked')
            elif entry.get('action') == 'pass':
                tags.append('passed')
                
            self.log_tree.insert('', 'end', values=values, tags=tags)
            
        # Configure colors
        self.log_tree.tag_configure('blocked', background='#ffcccc')
        self.log_tree.tag_configure('passed', background='#ccffcc')
        
        # Update pagination info
        self.page_label.config(text=f"Page {self.current_page + 1} of {self.total_pages}")
        
        # Update button states
        self._update_pagination_buttons()
        
        # Show memory info if using virtual manager
        if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
            memory_info = self.virtual_log_manager.get_memory_info()
            from parallel_filter import get_cpu_count, get_max_parallel_workers
            cpu_count = get_cpu_count()
            max_workers = get_max_parallel_workers()
            
            filter_status = ""
            if self.virtual_log_manager.is_filtered:
                filter_status = f" | Filtered with {max_workers} cores"
            
            self.status_bar.config(text=f"Showing {len(self.displayed_entries):,} entries (Page {self.current_page + 1}/{self.total_pages}) - {total_entries:,} total (~{memory_info['estimated_total_memory_mb']:.1f}MB, {cpu_count} CPU cores){filter_status}")
        else:
            self.status_bar.config(text=f"Showing {len(self.displayed_entries):,} entries (Page {self.current_page + 1}/{self.total_pages}) - {total_entries:,} total")
        
    def first_page(self):
        """Go to first page"""
        self.current_page = 0
        self.using_fast_tail = False  # Return to normal mode
        self.refresh_display()
        
    def prev_page(self):
        """Go to previous page"""
        if self.current_page > 0:
            self.current_page -= 1
            self.using_fast_tail = False  # Always return to normal mode
            self.refresh_display()
            
    def next_page(self):
        """Go to next page - SAFE VERSION"""
        # RADICAL FIX: Never try to go beyond a safe threshold
        if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
            total_entries = self.virtual_log_manager.get_total_entries()
        else:
            total_entries = self.total_entries_count
        
        # Calculate safe last page (avoid the problematic very last pages)
        safe_last_page = max(0, ((total_entries - 1000) // self.page_size))  # Stay 1000 entries before end
        
        if self.current_page < safe_last_page:
            self.current_page += 1
            self.using_fast_tail = False
            self.refresh_display()
        else:
            # We're near the end - use Last button for final pages
            if hasattr(self, 'status_bar'):
                self.status_bar.config(text=f"Near end of file - use 'Last' button to see final entries")
            
    def last_page(self):
        """Go to end of file - ALWAYS use fast tail for safety"""
        try:
            self._show_file_tail()
        except Exception as e:
            print(f"Error in last_page: {e}")
            import traceback
            traceback.print_exc()
            # Fallback: go to a safe page
            if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
                total_entries = self.virtual_log_manager.get_total_entries()
                safe_page = max(0, ((total_entries - 2000) // self.page_size))  # Go to a safe page
                self.current_page = safe_page
            else:
                self.current_page = 0
            self.refresh_display()
    
    def _show_file_tail(self):
        """Show the last entries of the file using fast tail approach"""
        try:
            if not (hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file):
                return
            
            file_path = self.virtual_log_manager.current_file
            
            # Read last 2000 lines directly from file (fast)
            last_lines = self._read_file_tail(file_path, 2000)
            
            # Parse only these lines
            self.displayed_entries = []
            
            for line in last_lines[-self.page_size:]:  # Take only last page_size entries
                entry = self.virtual_log_manager.log_parser.parse_log_line(line.strip())
                if entry:
                    self.displayed_entries.append(entry)
            
            # Update display without using virtual_log_manager
            self._update_table_directly()
            
            # Update status and pagination info
            total_pages = self._get_total_pages()
            self.current_page = max(0, total_pages - 1)
            self.using_fast_tail = True  # Mark that we're using fast tail
            
            # Update pagination display
            if hasattr(self, 'page_label'):
                self.page_label.config(text=f"Page {self.current_page + 1} of {total_pages}")
            
            # Update button states
            self._update_pagination_buttons()
            
            # Update status bar
            if hasattr(self, 'status_bar'):
                self.status_bar.config(text=f"Showing last {len(self.displayed_entries)} entries (fast tail view) - Page {self.current_page + 1}/{total_pages}")
            
            
        except Exception as e:
            print(f"Error in _show_file_tail: {e}")
            import traceback
            traceback.print_exc()
    
    def _read_file_tail(self, file_path, num_lines):
        """Read last N lines from file efficiently"""
        try:
            with open(file_path, 'rb') as f:
                # Go to end of file
                f.seek(0, 2)  # Seek to end
                file_size = f.tell()
                
                # Read chunks from end until we have enough lines
                lines = []
                chunk_size = 8192  # 8KB chunks
                pos = file_size
                
                while len(lines) < num_lines and pos > 0:
                    # Calculate chunk position
                    chunk_start = max(0, pos - chunk_size)
                    chunk_size_actual = pos - chunk_start
                    
                    # Read chunk
                    f.seek(chunk_start)
                    chunk = f.read(chunk_size_actual).decode('utf-8', errors='ignore')
                    
                    # Split into lines and prepend to our list
                    chunk_lines = chunk.split('\n')
                    lines = chunk_lines + lines
                    
                    pos = chunk_start
                
                # Return last num_lines
                return lines[-num_lines:] if len(lines) > num_lines else lines
                
        except Exception as e:
            print(f"Error reading file tail: {e}")
            return []
    
    def _update_table_directly(self):
        """Update the table display directly without refresh_display"""
        try:
            # Clear table
            for item in self.log_tree.get_children():
                self.log_tree.delete(item)
                
            # Add entries for current page
            for entry in self.displayed_entries:
                # Get rule label for this entry
                rule_label = self.get_rule_label_for_entry(entry)
                
                # Enrich with aliases
                src_display = self.get_enriched_ip(entry.get('src', ''))
                srcport_display = self.get_enriched_port(entry.get('srcport', ''))
                dst_display = self.get_enriched_ip(entry.get('dst', ''))
                dstport_display = self.get_enriched_port(entry.get('dstport', ''))
                
                values = (
                    entry.timestamp.strftime('%Y-%m-%d %H:%M:%S') if entry.timestamp else '',
                    entry.get('action', ''),
                    entry.get('interface_display', ''),
                    src_display,
                    srcport_display,
                    dst_display,
                    dstport_display,
                    entry.get('protoname', ''),
                    rule_label
                )
                
                # Color coding based on action
                tags = []
                if entry.get('action') == 'block':
                    tags.append('blocked')
                elif entry.get('action') == 'pass':
                    tags.append('passed')
                    
                self.log_tree.insert('', 'end', values=values, tags=tags)
                
            # Configure colors
            self.log_tree.tag_configure('blocked', background='#ffcccc')
            self.log_tree.tag_configure('passed', background='#ccffcc')
            
        except Exception as e:
            print(f"Error updating table: {e}")
            import traceback
            traceback.print_exc()
        
    def on_page_size_change(self, event=None):
        """Handle page size change"""
        try:
            self.page_size = int(self.page_size_var.get())
            self.current_page = 0
            self.refresh_display()
        except ValueError:
            pass
            
    def on_cell_double_click(self, event):
        """Handle double-click on any cell to copy its content to clipboard"""
        # Identify the clicked region
        region = self.log_tree.identify_region(event.x, event.y)
        if region != "cell":
            return
            
        # Identify the column and element
        column = self.log_tree.identify_column(event.x)
        item_id = self.log_tree.identify_row(event.y)
        
        if not item_id:
            return
            
        # Get the element values
        item = self.log_tree.item(item_id)
        values = item.get('values', [])
        
        if not values:
            return
            
        # Convert column to index (column #1 = index 0, etc.)
        try:
            col_index = int(column.replace('#', '')) - 1
            if 0 <= col_index < len(values):
                cell_value = str(values[col_index])
                
                if cell_value:
                    # Copy to clipboard
                    self.root.clipboard_clear()
                    self.root.clipboard_append(cell_value)
                    
                    # Get column name for display
                    columns = ('timestamp', 'action', 'interface', 'src', 'srcport', 'dst', 'dstport', 'proto', 'label')
                    col_headers = {
                        'timestamp': 'Timestamp',
                        'action': 'Action',
                        'interface': 'Interface',
                        'src': 'Source',
                        'srcport': 'Source port',
                        'dst': 'Destination',
                        'dstport': 'Destination port',
                        'proto': 'Protocol',
                        'label': 'Label'
                    }
                    
                    if col_index < len(columns):
                        col_name = col_headers.get(columns[col_index], columns[col_index])
                        self.status_bar.config(text=f"Copied {col_name}: {cell_value}")
                    else:
                        self.status_bar.config(text=f"Copied: {cell_value}")
                        
        except (ValueError, IndexError):
            return
            
    def add_filter(self):
        """Add a filter condition"""
        field_display = self.field_var.get()
        operator = self.operator_var.get()
        value = self.value_var.get()
        logic = self.logic_var.get()
        negate = self.negate_var.get()
        
        if not field_display or not value:
            messagebox.showwarning("Warning", "Please specify a field and value")
            return
        
        # Mapping from display names to technical names
        field_mapping = {
            'Action': 'action',
            'Interface': 'interface',
            'Source': 'src',
            'Source port': 'srcport',
            'Destination': 'dst',
            'Destination port': 'dstport',
            'Protocol': 'protoname',
            'Label': 'label'
        }
        
        # Convert display name to technical name
        field_technical = field_mapping.get(field_display, field_display.lower())
            
        # Create filter info for display (keep display name)
        filter_info = {
            'field': field_display,  # Keep display name for UI
            'operator': operator,
            'value': value,
            'logic': logic,
            'negate': negate
        }
        
        self.active_filters.append(filter_info)
        
        # Special handling for Label filtering
        if field_technical == 'label':
            # For label, add a special condition that will be evaluated dynamically
            self.log_filter.add_filter_condition('__label__', operator, value, logic, negate)
        else:
            # Use technical name for actual filtering
            self.log_filter.add_filter_condition(field_technical, operator, value, logic, negate)
        self.update_active_filters_display()
        
        # Reset input fields
        self.value_var.set("")
        
    def update_active_filters_display(self):
        """Update the display of active filters"""
        # Clear existing filter widgets
        for widget in self.filters_scrollable_frame.winfo_children():
            widget.destroy()
            
        # Add filter widgets
        for i, filter_info in enumerate(self.active_filters):
            filter_frame = ttk.Frame(self.filters_scrollable_frame)
            filter_frame.pack(side=tk.LEFT, padx=2, pady=2)
            
            # Filter description
            logic_text = f"{filter_info['logic']} " if i > 0 else ""
            negate_text = "NOT " if filter_info['negate'] else ""
            filter_text = f"{logic_text}{negate_text}{filter_info['field']} {filter_info['operator']} '{filter_info['value']}'"
            
            filter_label = ttk.Label(filter_frame, text=filter_text, relief=tk.RAISED, padding=2)
            filter_label.pack(side=tk.LEFT)
            
            # Remove button
            remove_btn = ttk.Button(filter_frame, text="×", width=3, 
                                  command=lambda idx=i: self.remove_filter(idx))
            remove_btn.pack(side=tk.RIGHT)
            
    def remove_filter(self, index):
        """Remove a specific filter"""
        if 0 <= index < len(self.active_filters):
            self.active_filters.pop(index)
            
            # Rebuild the filter object
            self.log_filter.clear_filters()
            
            # Mapping from display names to technical names (same as in add_filter)
            field_mapping = {
                'Action': 'action',
                'Interface': 'interface',
                'Source': 'src',
                'Source port': 'srcport',
                'Destination': 'dst',
                'Destination port': 'dstport',
                'Protocol': 'protoname',
                'Label': 'label'  # Add Label mapping
            }
            
            for filter_info in self.active_filters:
                # Convert display name to technical name
                field_technical = field_mapping.get(filter_info['field'], filter_info['field'].lower())
                
                # Special handling for Label filtering
                if field_technical == 'label':
                    # For label, add a special condition that will be evaluated dynamically
                    self.log_filter.add_filter_condition('__label__', filter_info['operator'], filter_info['value'],
                                                       filter_info['logic'], filter_info['negate'])
                else:
                    # Use technical name for actual filtering
                    self.log_filter.add_filter_condition(
                        field_technical, filter_info['operator'], filter_info['value'],
                        filter_info['logic'], filter_info['negate']
                    )
            
            self.update_active_filters_display()
            self.apply_filters()
            
    def apply_filters(self):
        """Apply all filters using virtual manager (memory efficient)"""
        if not hasattr(self, 'virtual_log_manager') or not self.virtual_log_manager.current_file:
            return
        
        # Apply time filter if enabled
        if self.time_filter_enabled.get():
            try:
                start_time = None
                end_time = None
                
                if self.start_time_var.get():
                    start_time = datetime.strptime(self.start_time_var.get(), '%Y-%m-%d %H:%M:%S')
                    
                if self.end_time_var.get():
                    end_time = datetime.strptime(self.end_time_var.get(), '%Y-%m-%d %H:%M:%S')
                    
                self.log_filter.set_time_range(start_time, end_time)
                
            except ValueError as e:
                messagebox.showerror("Error", f"Invalid time format: {e}")
                return
        else:
            # Clear time filter
            self.log_filter.set_time_range(None, None)
        
        # Create optimized filter function
        from parallel_filter import OptimizedFilterFunction
        
        # Check if Label filters are present
        has_label_filters = any(
            hasattr(condition, 'field') and condition.field == '__label__' 
            for condition in self.log_filter.expression.conditions
        )
        
        # Check filter types
        filter_fields = [condition.field if hasattr(condition, 'field') else 'unknown' 
                        for condition in self.log_filter.expression.conditions]
        
        if has_label_filters:
            # Use standard optimized function with pre-calculated labels
            combined_filter = OptimizedFilterFunction(
                self.log_filter,
                time_filter_enabled=self.time_filter_enabled.get(),
                time_range_start=self.log_filter.time_range_start,
                time_range_end=self.log_filter.time_range_end
            )
            
            # Enable multiprocessing with pre-calculated labels
            use_parallel = True
        else:
            # Use standard optimized function for multiprocessing
            combined_filter = OptimizedFilterFunction(
                self.log_filter,
                time_filter_enabled=self.time_filter_enabled.get(),
                time_range_start=self.log_filter.time_range_start,
                time_range_end=self.log_filter.time_range_end
            )
            
            # Allow multiprocessing for filters without labels
            use_parallel = True
        
        # Show progress dialog for filtering
        self.progress_dialog = ProgressDialog(self.root, "Applying Filters")
        
        def filter_worker():
            try:
                def progress_callback(message):
                    if not self.progress_dialog.cancelled:
                        self.progress_dialog.update_text(message)
                
                # Prepare label mapping for multiprocessing
                rule_labels_mapping = None
                if has_label_filters and self.rule_labels_loaded:
                    # Create a simple dictionary {rid: description} for serialization
                    rule_labels_mapping = {}
                    if hasattr(self.rule_mapper, 'label_descriptions'):
                        for label_hash, description in self.rule_mapper.label_descriptions.items():
                            rule_labels_mapping[label_hash] = description
                
                # Apply filter using virtual manager with optimized label handling
                if use_parallel and has_label_filters:
                    # Use multiprocessing with label mapping
                    from parallel_filter import ParallelLogFilter
                    parallel_filter = ParallelLogFilter()
                    filtered_indices = parallel_filter.apply_filter_parallel(
                        self.virtual_log_manager, combined_filter, progress_callback, rule_labels_mapping
                    )
                    self.virtual_log_manager.filtered_indices = filtered_indices
                    self.virtual_log_manager.is_filtered = True
                else:
                    # Use standard method
                    self.virtual_log_manager.apply_filter(combined_filter, progress_callback, use_parallel)
                
                if not self.progress_dialog.cancelled:
                    # Update UI in main thread
                    self.root.after(0, self.on_filter_applied)
                
            except Exception as e:
                self.root.after(0, lambda: self.on_filter_error(str(e)))
        
        filter_thread = threading.Thread(target=filter_worker)
        filter_thread.daemon = True
        filter_thread.start()
        
    def on_filter_applied(self):
        """Called when filters are applied"""
        if self.progress_dialog:
            self.progress_dialog.close()
            self.progress_dialog = None
        
        self.current_page = 0
        self.refresh_display()
        
    def on_filter_error(self, error_message):
        """Called when filter application fails"""
        if self.progress_dialog:
            self.progress_dialog.close()
            self.progress_dialog = None
        messagebox.showerror("Error", f"Filter error: {error_message}")
        
    def clear_filters(self):
        """Clear all filters"""
        self.log_filter.clear_filters()
        self.active_filters = []
        self.time_filter_enabled.set(False)
        self.start_time_var.set("")
        self.end_time_var.set("")
        self.on_time_filter_toggle()
        self.update_active_filters_display()
        
        # Clear virtual manager filters
        if hasattr(self, 'virtual_log_manager') and self.virtual_log_manager.current_file:
            self.virtual_log_manager.clear_filter()
            self.current_page = 0
            self.refresh_display()
            
    def apply_preset_filter(self, preset_name):
        """Apply a preset filter"""
        self.clear_filters()
        
        if preset_name == 'all':
            # No filter - show all
            pass
        elif preset_name == 'blocked':
            self.log_filter.add_filter_condition('action', '==', 'block')
        elif preset_name == 'allowed':
            self.log_filter.add_filter_condition('action', '==', 'pass')
        elif preset_name == 'tcp':
            self.log_filter.add_filter_condition('protoname', '==', 'tcp')
        elif preset_name == 'udp':
            self.log_filter.add_filter_condition('protoname', '==', 'udp')
            
        self.apply_filters()
        
    def on_log_select(self, event):
        """Handle log entry selection"""
        selection = self.log_tree.selection()
        if not selection:
            return
            
        row_index = self.log_tree.index(selection[0])
        
        if row_index < len(self.displayed_entries):
            entry = self.displayed_entries[row_index]
            self.show_log_details(entry)
            
    def show_log_details(self, entry: LogEntry):
        """Show detailed information for a log entry"""
        self.details_text.delete(1.0, tk.END)
        
        details = f"=== LOG ENTRY DETAILS ===\n\n"
        details += f"Raw line:\n{entry.raw_line}\n\n"
        details += f"=== PARSED FIELDS ===\n\n"
        
        for key, value in entry.parsed_data.items():
            if not key.startswith('__'):
                details += f"{key}: {value}\n"
                
        details += f"\n=== METADATA ===\n\n"
        details += f"Timestamp: {entry.timestamp}\n"
        details += f"Host: {entry.host}\n"
        details += f"Digest: {entry.digest}\n"
        
        self.details_text.insert(1.0, details)
        
    def sort_column(self, col):
        """Sort column (placeholder for future implementation)"""
        pass
        
    def export_results(self):
        """Export filtered results with progress dialog"""
        # Check if there's data to export (use virtual manager)
        if not hasattr(self, 'virtual_log_manager') or not self.virtual_log_manager.current_file:
            messagebox.showwarning("Warning", "No log file loaded")
            return
        
        # Check if there's data to export
        if self.virtual_log_manager.is_filtered:
            total_entries = len(self.virtual_log_manager.filtered_indices)
            if total_entries == 0:
                messagebox.showwarning("Warning", "No filtered data to export")
                return
        else:
            total_entries = self.virtual_log_manager.get_total_entries()
            if total_entries == 0:
                messagebox.showwarning("Warning", "No data to export")
                return
            
        # Choose export file
        file_path = filedialog.asksaveasfilename(
            title="Export Results",
            defaultextension=".json",
            filetypes=[("JSON files", "*.json"), ("CSV files", "*.csv")]
        )
        
        if not file_path:
            return
            
        # Start export with progress dialog
        self.export_with_progress(file_path, total_entries)
    
    def export_with_progress(self, file_path, total_entries):
        """Export data with progress dialog in a separate thread"""
        # Show progress dialog
        self.progress_dialog = ProgressDialog(self.root, "Exporting Data")
        
        def export_worker():
            """Export worker thread"""
            try:
                # Get all current filtered entries with progress updates
                all_filtered_entries = []
                
                if self.virtual_log_manager.is_filtered:
                    # Export filtered entries
                    total_filtered = len(self.virtual_log_manager.filtered_indices)
                    
                    # Process in chunks with progress updates
                    chunk_size = 1000
                    for start_idx in range(0, total_filtered, chunk_size):
                        if self.progress_dialog.cancelled:
                            return
                            
                        # Update progress
                        progress_pct = (start_idx / total_filtered) * 50  # First 50% for data retrieval
                        self.progress_dialog.update_text(f"Retrieving data: {start_idx:,}/{total_filtered:,} entries")
                        
                        chunk_entries = self.virtual_log_manager._get_filtered_entries(start_idx, chunk_size)
                        all_filtered_entries.extend(chunk_entries)
                        
                else:
                    # Export all entries
                    total_chunks = (total_entries + 999) // 1000
                    
                    for chunk_id in range(total_chunks):
                        if self.progress_dialog.cancelled:
                            return
                            
                        # Update progress
                        progress_pct = (chunk_id / total_chunks) * 50  # First 50% for data retrieval
                        current_entries = chunk_id * 1000
                        self.progress_dialog.update_text(f"Retrieving data: {current_entries:,}/{total_entries:,} entries")
                        
                        chunk_entries = self.virtual_log_manager.get_chunk(chunk_id)
                        all_filtered_entries.extend(chunk_entries)
                
                if self.progress_dialog.cancelled:
                    return
                
                # Write to file with progress
                self.progress_dialog.update_text("Writing to file...")
                
                if file_path.endswith('.json'):
                    with open(file_path, 'w', encoding='utf-8') as f:
                        data = []
                        for i, entry in enumerate(all_filtered_entries):
                            if self.progress_dialog.cancelled:
                                return
                            
                            # Update progress for data processing (50-75%)
                            if i % 100 == 0:
                                progress_pct = 50 + (i / len(all_filtered_entries)) * 25
                                self.progress_dialog.update_text(f"Processing data: {i:,}/{len(all_filtered_entries):,} entries")
                            
                            data.append(entry.parsed_data)
                        
                        if self.progress_dialog.cancelled:
                            return
                            
                        # Update progress for JSON serialization (75-100%)
                        self.progress_dialog.update_text("Writing JSON file...")
                        json.dump(data, f, indent=2, default=str, ensure_ascii=False)
                        
                elif file_path.endswith('.csv'):
                    import csv
                    with open(file_path, 'w', newline='', encoding='utf-8') as f:
                        if all_filtered_entries:
                            # Use keys from first element for headers
                            fieldnames = all_filtered_entries[0].parsed_data.keys()
                            writer = csv.DictWriter(f, fieldnames=fieldnames)
                            writer.writeheader()
                            
                            for i, entry in enumerate(all_filtered_entries):
                                if self.progress_dialog.cancelled:
                                    return
                                
                                # Update progress for CSV writing (50-100%)
                                if i % 100 == 0:
                                    progress_pct = 50 + (i / len(all_filtered_entries)) * 50
                                    self.progress_dialog.update_text(f"Writing CSV: {i:,}/{len(all_filtered_entries):,} entries")
                                
                                writer.writerow(entry.parsed_data)
                
                if not self.progress_dialog.cancelled:
                    # Success - update UI in main thread
                    filter_status = "with filters applied" if self.virtual_log_manager.is_filtered else "all entries"
                    self.root.after(0, lambda: self.on_export_success(file_path, len(all_filtered_entries), filter_status))
                
            except Exception as e:
                # Error - update UI in main thread
                self.root.after(0, lambda: self.on_export_error(str(e)))
        
        # Start export thread
        export_thread = threading.Thread(target=export_worker, daemon=True)
        export_thread.start()
    
    def on_export_success(self, file_path, entry_count, filter_status):
        """Called when export completes successfully"""
        if self.progress_dialog:
            self.progress_dialog.close()
            self.progress_dialog = None
        
        messagebox.showinfo("Export Complete", 
                          f"Data exported successfully to:\n{file_path}\n\n"
                          f"{entry_count:,} entries exported ({filter_status})")
    
    def on_export_error(self, error_message):
        """Called when export fails"""
        if self.progress_dialog:
            self.progress_dialog.close()
            self.progress_dialog = None
        
        messagebox.showerror("Export Error", f"Export failed: {error_message}")
                
    def show_interfaces(self):
        """Show configured interfaces"""
        if not self.log_parser.interface_mapping:
            messagebox.showinfo("Information", "No interface configuration loaded")
            return
            
        interfaces_window = tk.Toplevel(self.root)
        interfaces_window.title("Configured Interfaces")
        interfaces_window.geometry("500x300")
        
        text_widget = scrolledtext.ScrolledText(interfaces_window, wrap=tk.WORD)
        text_widget.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        
        interfaces_text = "=== INTERFACE MAPPING ===\n\n"
        for physical, logical in self.log_parser.interface_mapping.items():
            interfaces_text += f"{physical} -> {logical}\n"
            
        text_widget.insert(1.0, interfaces_text)
        
    def show_about(self):
        """Show about dialog"""
        messagebox.showinfo("About", 
                          "OPNsense Log Viewer v1.0\n\n"
                          "Advanced log viewer for OPNsense firewall logs\n"
                          "https://github.com/Shayano/opnsense-log-viewer\n\n"
                          "Built with Python/tkinter")
    
    def connect_ssh_and_get_labels(self):
        """Connect to OPNsense via SSH and extract rule labels"""
        # Validate input fields
        address = self.ssh_address.get().strip()
        port_str = self.ssh_port.get().strip()
        username = self.ssh_username.get().strip()
        password = self.ssh_password.get()
        
        if not all([address, port_str, username, password]):
            messagebox.showerror("Error", "Please fill in all SSH connection fields")
            return
        
        # Validate port number
        try:
            port = int(port_str)
            if port < 1 or port > 65535:
                raise ValueError("Port must be between 1 and 65535")
        except ValueError as e:
            messagebox.showerror("Error", f"Invalid port number: {e}")
            return
        
        # Disable button during connection
        self.ssh_connect_button.config(state='disabled', text="Connecting...")
        self.ssh_status_label.config(text="Connecting...", foreground="orange")
        
        def ssh_worker():
            """SSH connection worker thread"""
            try:
                # Step 1: Connect to SSH
                success, message = self.ssh_client.connect(address, username, password, port=port, timeout=10)
                
                if not success:
                    self.root.after(0, lambda: self.ssh_connection_failed(message))
                    return
                
                # Step 2: Extract rule labels and descriptions
                success, message, label_descriptions = self.ssh_client.extract_rule_labels(timeout=10)
                
                if not success:
                    self.root.after(0, lambda: self.ssh_extraction_failed(message))
                    return
                
                # Build the mapping directly with descriptions
                self.rule_mapper.set_label_descriptions(label_descriptions)
                
                # Success callback
                self.root.after(0, lambda: self.ssh_connection_success(len(label_descriptions)))
                
            except Exception as e:
                self.root.after(0, lambda: self.ssh_connection_failed(f"Unexpected error: {str(e)}"))
        
        # Start SSH connection in background thread
        ssh_thread = threading.Thread(target=ssh_worker, daemon=True)
        ssh_thread.start()
    
    def ssh_connection_success(self, rule_count):
        """Handle successful SSH connection and label extraction"""
        self.ssh_connected = True
        self.rule_labels_loaded = True
        
        # Update UI
        self.ssh_connect_button.config(state='normal', text="Reconnect")
        self.ssh_status_label.config(text=f"Connected ({rule_count} labels)", foreground="green")
        
        # Show success message
        stats = self.rule_mapper.get_mapping_stats()
        message = (f"SSH connection successful!\n\n"
                  f"Labels with descriptions extracted: {stats['total_labels']}\n\n"
                  f"The 'Label' column will now show rule descriptions.")
        
        messagebox.showinfo("SSH Success", message)
        
        # Refresh display to show labels
        if self.virtual_log_manager.current_file:
            self.refresh_display()
    
    def ssh_connection_failed(self, error_message):
        """Handle SSH connection failure"""
        self.ssh_connected = False
        self.rule_labels_loaded = False
        
        # Update UI
        self.ssh_connect_button.config(state='normal', text="Connect & Get Labels")
        self.ssh_status_label.config(text="Connection failed", foreground="red")
        
        # Show error message
        messagebox.showerror("SSH Connection Error", error_message)
        
        # Disconnect if partially connected
        self.ssh_client.disconnect()
    
    def ssh_extraction_failed(self, error_message):
        """Handle SSH rule extraction failure"""
        self.ssh_connected = False
        self.rule_labels_loaded = False
        
        # Update UI
        self.ssh_connect_button.config(state='normal', text="Connect & Get Labels")
        self.ssh_status_label.config(text="Extraction failed", foreground="red")
        
        # Show error message
        messagebox.showerror("Rule Extraction Error", 
                           f"Connected to SSH but failed to extract rules:\n{error_message}")
        
        # Disconnect
        self.ssh_client.disconnect()
    
    def get_rule_label_for_entry(self, entry: LogEntry) -> str:
        """Get the rule label for a log entry"""
        if not self.rule_labels_loaded:
            return ""
        
        # Extract the label hash (rid) from the log entry
        label_hash = self.extract_label_hash_from_entry(entry)
        
        if label_hash:
            description = self.rule_mapper.get_rule_description_by_hash(label_hash)
            return description if description else ""
        
        return ""
    
    def extract_label_hash_from_entry(self, entry: LogEntry) -> Optional[str]:
        """Extract label hash (rid) from log entry"""
        # OPNsense logs contain the 'rid' field which corresponds to the label hash
        
        # Check if rid is in parsed data (most common case)
        if hasattr(entry, 'parsed_data') and 'rid' in entry.parsed_data:
            return str(entry.parsed_data['rid'])
        
        # Fallback: try to extract from the raw line using regex
        import re
        if hasattr(entry, 'raw_line'):
            # Look for rid pattern in the log line
            # Example: "Sep 10 20:48:47 fw01 filterlog: 25,,,1000000103,vtnet0,match,block,in,4,0x0,,64,1,0,UE,17,96,10.10.10.1,8.8.8.8,12345,53,76"
            # The rid is typically one of the early comma-separated fields
            
            # Split by comma and look for potential rid (32-char hex string)
            parts = entry.raw_line.split(',')
            for part in parts:
                part = part.strip()
                # Check if it's a 32-character hexadecimal string (MD5 hash format)
                if len(part) == 32 and all(c in '0123456789abcdef' for c in part.lower()):
                    return part
        
        return None
    
    def get_enriched_ip(self, ip: str) -> str:
        """Enriches an IP with its alias if available"""
        if not ip or not hasattr(self.config_parser, 'ip_aliases'):
            return ip
        
        # Search for exact alias
        alias = self.config_parser.get_ip_alias(ip)
        if alias:
            return f"{ip} ({alias})"
        
        # If no exact alias, search in networks
        # Format: "ip (ALIAS)" if found, otherwise just "ip"
        for network, alias_name in self.config_parser.ip_aliases.items():
            if '/' in network:
                # It's a network, check if IP belongs to the network
                try:
                    import ipaddress
                    ip_obj = ipaddress.ip_address(ip)
                    network_obj = ipaddress.ip_network(network, strict=False)
                    if ip_obj in network_obj:
                        return f"{ip} ({alias_name})"
                except:
                    continue
        
        return ip
    
    def get_enriched_port(self, port: str) -> str:
        """Enriches a port with its alias if available"""
        if not port or not hasattr(self.config_parser, 'port_aliases'):
            return port
        
        # Search for alias
        alias = self.config_parser.get_port_alias(port)
        if alias:
            return f"{port} ({alias})"
        
        return port

def main():
    """Main entry point"""
    root = tk.Tk()
    app = LogViewerApp(root)
    root.mainloop()

if __name__ == "__main__":
    # Fix for PyInstaller multiprocessing issues
    import multiprocessing
    multiprocessing.freeze_support()
    main()