# OPNsense Log Viewer

Advanced GUI application for analyzing OPNsense firewall logs with filtering capabilities.  
Load log files exported from **OPNsense → Firewall → Log Files → Plain View** for advanced filtering and analysis.

<img width="1079" height="816" alt="image" src="https://github.com/user-attachments/assets/ed929b0d-105e-4ba8-b43b-17d9571b08f4" />



## Features

- **Modern GUI**: Clean interface with sortable columns and pagination
- **Large file support**: Handles multi-GB log files. Filtering uses DuckDB's compiled, multi-threaded engine to scan the raw file in place, with no index to build, so even a 14 GB log filters in well under a minute and every later page is instant.
- **Fast filtering**: DuckDB-backed queries with logical operators (AND/OR/NOT), regex, time ranges, and reusable saved presets
- **IPv4 and IPv6**: Decodes both IPv4 and IPv6 filterlog entries
- **Interface mapping**: Automatic renaming from physical (vtnet0) to logical names (LAN)
- **SSH integration**: Direct rule label extraction from OPNsense
- **Export capabilities**: Save filtered results to JSON/CSV

## Quick Start

### Option 1: Portable Executable (Recommended)
1. Download `OPNsense_Log_Viewer.exe` from the release
2. Double-click to run - no installation required

### Option 2: Run from Source
```bash
# Install dependencies
pip install -r requirements.txt

# Run application (from the project root)
cd src
python -m opnsense_log_viewer
```

### Option 3: Build Executable
```bash
# Windows
build_complete.bat

# Or manually
python build_exe.py
```

## Usage

1. **Load logs**: File → Open log file
2. **Load config** (optional): File → Open XML config for interface mapping
3. **Apply filters**: Use quick filters or advanced filtering
4. **Analyze**: View results in logs tab, details in details tab
5. **Export**: File → Export results

## Supported Log Formats

OPNsense filterlog formats:
- RFC3164 with `filterlog[pid]:`
- RFC5424 structured logs
- Custom CSV format

### Example log line:
```
2025-09-08T20:31:34 filterlog 30,,,cd4617bd680a0a5aa4c5694f2eefa56e,vtnet0,match,pass,out,4,0x0,,62,35294,0,DF,6,tcp,60,10.13.37.2,191.101.31.14,29397,29376,0,S,1162654291,,64240,,mss;sackOK;TS;nop;wscale
```

## Interface Configuration

For automatic interface renaming, provide XML configuration:

```xml
<interfaces>
  <lan>
    <if>vtnet0</if>
    <descr>LAN</descr>
  </lan>
  <wan>
    <if>vtnet1</if>
    <descr>WAN</descr>
  </wan>
</interfaces>
```

## Troubleshooting

**Missing interfaces**: Load XML configuration file  
**Missing Labels**: Connect with the SSH Connection

## License

MIT License
