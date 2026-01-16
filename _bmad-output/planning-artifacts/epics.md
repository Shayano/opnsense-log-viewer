---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories']
inputDocuments:
  - "_bmad-output/planning-artifacts/prd.md"
  - "_bmad-output/planning-artifacts/architecture.md"
  - "_bmad-output/planning-artifacts/ux-design-specification.md"
---

# opnsense-log-viewer - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for opnsense-log-viewer, decomposing the requirements from the PRD, UX Design if it exists, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

**FR-001: Log File Management**
- FR-001.1: File Selection - User can select single log file via native OS file picker, supporting .log/.txt/.csv formats up to 50 GB with warnings
- FR-001.2: File Indexation - System creates persistent index (.idx file) with inverted index (IPs, ports), bitmap index (action, protocol, interface), and entry offset table. Progress indicator shows % completion, GB processed, estimated time remaining
- FR-001.3: Index Persistence & Reuse - System stores index with SHA-256 hash of source file, reuses existing index if hash matches, prompts for re-indexing if source modified. Users can manage existing indexes

**FR-002: Log Parsing & Display**
- FR-002.1: Format Detection - Auto-detect RFC3164, RFC5424, or CSV filterlog formats from first 100 lines with 95% accuracy
- FR-002.2: Log Entry Display - Display entries in table format with columns (Timestamp, Interface, Source IP, Source Port, Destination IP, Destination Port, Protocol, Action, Rule Label). Virtual scrolling for 100K+ entries at 60 FPS
- FR-002.3: Entry Detail View - Bottom pane shows full raw log line, parsed fields in key-value format, copy buttons for individual fields and full entry

**FR-003: Filtering & Search**
- FR-003.1: Filter Builder UI - Left sidebar collapsible filter panel with modal for "Select Field → Select Operator → Enter Value". Filter fields include Timestamp Range, Source/Dest IP/Port, Protocol, Action, Interface, Rule Label. Operators: equals, contains, startswith, endswith, regex, numeric comparisons, boolean logic (AND/OR/NOT)
- FR-003.2: Filter Execution - Filters execute against index (not raw log) with <500ms response time. Real-time result updates, result count display ("Showing X of Y entries"), empty result state handling
- FR-003.3: Filter Management - Clear All, Save Filter (to local storage), Load Filter dropdown, Delete Filter, Search history (last 10 searches with persistence)

**FR-004: OPNsense API Enrichment**
- FR-004.1: API Connection Setup - Settings panel with fields for Endpoint URL, API Key, API Secret. Test Connection button, credentials stored in OS keychain, connection status indicator (Green/Yellow/Red)
- FR-004.2: Interface Mapping - Call /api/diagnostics/interface/getInterfaceNames to map physical interfaces (vtnet0, em0) to logical names (LAN, WAN, DMZ). Display logical names with hover tooltips showing physical names
- FR-004.3: Rule Label Enrichment - Call /api/firewall/filter/searchRule for unique rule hashes. Replace hashes with human-readable descriptions, cache in memory, handle missing rules gracefully
- FR-004.4: Alias Resolution - Call /api/firewall/alias/searchItem for aliased IPs. Display alias names alongside IPs with hover tooltips for group members
- FR-004.5: Graceful Degradation - Continue investigation with raw data if API unavailable. Display banner offering backup enrichment option

**FR-005: Backup Enrichment System**
- FR-005.1: Enrichment Export - Export Enrichment Data button generates JSON file with interface mappings, rule labels, alias definitions, and metadata. Default filename: enrichment_<hostname>_<timestamp>.json
- FR-005.2: Enrichment Import - Load Backup Enrichment button with JSON validation. Display staleness warning, apply enrichment to current view, persist until API reconnected
- FR-005.3: Staleness Indicators - Visual indicator "⚠️ Using backup enrichment (X days old)" with yellow background and hover tooltip. Option to suppress warning for session

**FR-006: Export & Reporting**
- FR-006.1: Filtered Result Export - Export button with CSV/JSON format selection. Include metadata header (tool version, export date, source file SHA-256, filters applied, entry counts). Progress indicator for >10K rows
- FR-006.2: Full Dataset Export - Export All (Unfiltered) option with large export warning. Stream export to disk without loading entire dataset into memory
- FR-006.3: Export Integrity - Include SHA-256 checksum in metadata, row count verification, open export location button

### NonFunctional Requirements

**NFR-001: Performance Requirements**
- NFR-001.1: Indexing Performance - Index at ≥6 GB/min (≤10 sec/GB) on reference hardware (Intel i5-10400/AMD Ryzen 5 3600, 8GB RAM, SATA SSD). Mean ≤7 sec/GB, P95 ≤8 sec/GB
- NFR-001.2: Search & Filter Performance - <500ms for simple queries, <750ms for complex queries (5+ filters with AND/OR/NOT logic, regex, timestamp ranges). P95 latency ≤750ms across 1000 query executions
- NFR-001.3: UI Responsiveness - Maintain 60 FPS (16ms frame time) during scrolling and UI interactions. Virtual scrolling with 50-100 visible DOM rows, 300ms input debouncing
- NFR-001.4: Memory Efficiency - Peak ≤500 MB during indexing, ≤300 MB during search. Stream mode if exceeding 500 MB
- NFR-001.5: Startup Time - Cold start ≤3 sec, hot start ≤1 sec (P95: ≤4 sec cold, ≤1.5 sec hot)
- NFR-001.6: Index Load Time - Load existing index in ≤2 sec regardless of source file size (95% of loads for files up to 30 GB)

**NFR-002: Reliability Requirements**
- NFR-002.1: Crash Rate - <0.1% per session (<1 crash per 1000 sessions across all platforms). Zero crashes on valid log/index/config files
- NFR-002.2: Data Integrity - 100% parse accuracy for well-formed log entries. Gracefully handle malformed entries (log, skip, continue, report count). <0.1% discrepancy vs Python tool, 0% data loss
- NFR-002.3: Index Corruption Recovery - Detect corrupted indexes via checksum validation. Prompt for re-indexing, never crash on corruption
- NFR-002.4: Idempotency - Re-indexing same log file produces bit-identical index (excluding timestamps). 100% SHA-256 hash match across 10 re-index operations
- NFR-002.5: Graceful Degradation - Remain functional when API unavailable, enrichment missing, network disconnected, disk full, or low memory. Zero crashes in degraded modes

**NFR-003: Security Requirements**
- NFR-003.1: Credential Protection - API credentials encrypted at rest using OS keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service) or AES-256-GCM with Argon2id fallback. Never log plaintext credentials
- NFR-003.2: Local Data Processing - 100% local log processing, zero data transmission to external servers (except user's OPNsense API and optional GitHub version check)
- NFR-003.3: Input Validation - Sanitize all user inputs (file paths, filter values, API endpoints). Prevent path traversal, injection attacks. Tauri sandboxing with restricted file system access
- NFR-003.4: Secure Defaults - HTTPS enforced for API, TLS certificate validation enabled, Content Security Policy restricts inline scripts, no debug logging in production

**NFR-004: Usability Requirements**
- NFR-004.1: Learnability - First-time users complete basic investigation (open file, apply filter, export) within 10 minutes without documentation. 4 of 5 users succeed in user testing
- NFR-004.2: Efficiency - Expert users complete routine investigations 3x faster than Python tool. Target <5 minutes vs Python baseline 8-12 minutes
- NFR-004.3: Error Messages - Provide actionable guidance, specific failure reasons, recovery steps. 80%+ users recover from errors without external help
- NFR-004.4: Accessibility - Keyboard navigation for all core features, screen reader compatibility. ARIA labels, semantic HTML, alt text. Tab navigation, Enter, Esc, Cmd/Ctrl+F shortcuts
- NFR-004.5: Responsive Design - UI adapts from 1280x720 (13" laptop) to 2560x1440 (27" desktop). Collapsible sidebar, prioritized table columns, scaled fonts

**NFR-005: Maintainability Requirements**
- NFR-005.1: Code Quality - Rust code passes clippy with zero warnings, 80%+ test coverage. CI/CD fails on warnings or coverage drop below 75%
- NFR-005.2: Documentation - All public APIs documented with rustdoc, complex algorithms explained. Embedded user guide (HTML), README with quick start
- NFR-005.3: Logging & Diagnostics - Log errors, warnings, performance metrics to local file. Max 10 MB per file, keep last 5 files. Never log credentials, user data, or PII

**NFR-006: Portability Requirements**
- NFR-006.1: Cross-Platform Consistency - Identical feature set and UX across Windows/Linux/macOS. Native look per platform (system fonts, window chrome)
- NFR-006.2: Platform Compatibility - Support Windows 10 v1809+, Ubuntu 20.04+, macOS 10.15 Catalina+
- NFR-006.3: Portable Deployment - Single executable runs without installation, dependencies (except WebView2/WebKitGTK), or admin rights. USB drive deployment supported

**NFR-007: Scalability Requirements**
- NFR-007.1: File Size Limits - Support up to 30 GB without degradation. 30-50 GB acceptable with reduced performance. Reject >50 GB with error
- NFR-007.2: Result Set Size - Display up to 10M filtered results without UI freezing. Virtual scrolling with 50-100 DOM rows, 10K chunk loading
- NFR-007.3: Concurrent Sessions - Support 1 active indexing + unlimited searches. 3 concurrent search operations without memory overflow

### Additional Requirements

**From Architecture Document:**

**Starter Template & Project Setup:**
- Use create-tauri-app v4.6.0 with react-ts template for project initialization
- Project structure: src-tauri/ (Backend Rust) vs src/ (Frontend React)
- Story 0.2 MANDATORY: Test Infrastructure Setup (blocks Stories 1.x+)
  - Setup Vitest + React Testing Library for frontend
  - Setup cargo bench framework for performance regression tests
  - Create 30GB synthetic log fixture generator (RFC3164/5424/CSV formats)
  - Configure CI/CD performance gates: <7 sec/GB indexation, <750ms queries, <600 MB memory
  - Setup property-based testing (proptest crate) for parser robustness
  - Create integration test suite for Tauri IPC commands
- Story 0.3 Required: Tailwind CSS Integration for UX Design Spec compliance

**Technology Stack Decisions:**
- Index Persistence: bincode 2.0.1 for optimal serialization/deserialization
- Compression: Zstd 0.13.3 at level 1 (fast compression, ~2x ratio)
- Checksum & Integrity: SHA-256 via sha2 crate for corruption detection
- HTTP Client: reqwest 0.13.1 for OPNsense API calls with TLS
- OS Keychain: keyring 3.6.3 for secure credential storage
- Async Runtime: Tokio 1.x with optimized features
- Logging: tracing 0.1.x + tracing-subscriber
- Error Types: thiserror 2.x + anyhow 1.x
- State Management: Zustand 5.0.10 (frontend)
- Virtual Scrolling: TanStack Virtual 3.13.18
- Error Handling UX: react-hot-toast
- Form Handling: React Hook Form 7.x
- Date/Time: date-fns 3.x
- Icons: lucide-react

**Code Organization:**
- Frontend modules: components/ (FilterBuilder, LogResultsTable, SearchHistory), hooks/, types/
- Backend modules: indexer/ (inverted.rs, bitmap.rs, hybrid.rs), parser/ (rfc3164.rs, rfc5424.rs, csv_filterlog.rs), query/ (executor.rs, optimizer.rs), api_client/ (client.rs, enrichment.rs), storage/, export/
- Test suites: unit/, integration/, performance/, fixtures/

**From UX Design Specification:**

**User Experience Requirements:**
- Theme Switcher: Toggle between dark ↔ light modes with user preference stored locally and persisting across sessions. Default to system preference on first launch
- VS Code-Inspired Layout: Sidebar + Main Content pattern with collapsible sections for space optimization. Adapts to screen sizes while maintaining information density
- Postman-Style Explicit Control: Visual filter builder with explicit "Search" button. Draft mode prevents accidental expensive operations
- Progressive Complexity: Default to simple intuitive interactions while making advanced features discoverable. Toggle "Advanced" mode or expand inline
- Keyboard Shortcuts: Tab navigation, Enter to select, Esc to cancel, Cmd/Ctrl+F for filter. Power users get full keyboard navigation
- Accessibility: ARIA labels on all interactive components, keyboard navigation throughout, focus indicators visible in both themes, color never the only indicator

**Visual Design Requirements:**
- Professional Modern Aesthetic: Clean, modern design that signals quality and reliability. Not playful, not austere - balanced professional look
- Data-Dense Styling: Tighter line-height and spacing than consumer apps. Monospace fonts for technical data (timestamps, IPs, ports). Alternating row colors for scannability
- Color-Coding: Actions color-coded (block=red, pass=green, reject=orange) with icons for non-color-dependent identification
- Consistent Design Tokens: 4px or 8px border radius, subtle shadows for elevation, clear visual hierarchy through size/weight/color contrast
- Enriched Data Display: Interface names inline (LAN not vtnet0), rule descriptions instead of hashes, alias names alongside IPs. Clear visual distinction between enriched and raw data

**Interaction Patterns:**
- Filter Builder: Field + Operator + Value dropdowns eliminate syntax memorization. Add/remove filter controls. Filters accumulated in draft mode
- Search History: Automatic history of recent filter combinations with quick re-execution. History persists across sessions
- Connection Status: Clear visual indication (Green=Connected, Yellow=Degraded, Red=Disconnected) with hover tooltips
- Progress Indication: Clear progress bars with estimated completion for indexation (2-3 minutes). Transparent feedback builds trust
- Result Display: Virtual scrolling for performance, scannable layout with visual hierarchy, inline enrichment display, contextual actions (right-click menus, copy buttons)

**Critical Success Moments:**
- First-Time "Aha!": 20 GB file indexes successfully in 2-3 minutes, first filter returns instant results (<1 sec), proves tool is fundamentally different
- Enrichment Clarity: "LAN → WAN: BLOCK - Block RFC1918 Networks" instead of cryptic hashes. Immediate understandability
- Iterative Investigation Success: Build complex query, see instant results, refine filters without waiting or crashes
- Recurring Productivity: Common searches in history, API credentials remembered, find answers in <5 minutes vs giving up

### FR Coverage Map

**Epic 0: Development Foundation & Quality Infrastructure**
- Architecture: create-tauri-app v4.6.0 with react-ts template for project initialization
- Architecture: Test Infrastructure Setup (Vitest, cargo bench, 30GB fixtures, CI/CD performance gates, proptest, integration tests)
- Architecture: Tailwind CSS Integration for UX Design Spec compliance
- NFR-001: Performance gates in CI/CD
- NFR-005.1: Code quality (clippy, 80%+ coverage)
- NFR-005.2: Documentation standards

**Epic 1: Core Log Investigation Capability**
- FR-001.1: File Selection
- FR-001.2: File Indexation
- FR-001.3: Index Persistence & Reuse
- FR-002.1: Format Detection
- FR-002.2: Log Entry Display
- FR-002.3: Entry Detail View
- NFR-001.1: Indexing Performance (≥6 GB/min)
- NFR-001.3: UI Responsiveness (60 FPS)
- NFR-001.5: Startup Time (≤3 sec cold, ≤1 sec hot)
- NFR-001.6: Index Load Time (≤2 sec)
- NFR-002.2: Data Integrity (100% parse accuracy)
- NFR-002.3: Index Corruption Recovery
- NFR-002.4: Idempotency

**Epic 2: Advanced Filtering & Search**
- FR-003.1: Filter Builder UI
- FR-003.2: Filter Execution
- FR-003.3: Filter Management
- NFR-001.2: Search & Filter Performance (<500ms simple, <750ms complex)
- NFR-004.1: Learnability (10 min for first investigation)
- NFR-004.2: Efficiency (3x faster than Python tool)

**Epic 3: Enhanced Context with API Enrichment**
- FR-004.1: API Connection Setup
- FR-004.2: Interface Mapping
- FR-004.3: Rule Label Enrichment
- FR-004.4: Alias Resolution
- FR-004.5: Graceful Degradation
- NFR-003.1: Credential Protection (OS keychain, AES-256-GCM)
- NFR-003.2: Local Data Processing (100% local)
- NFR-003.3: Input Validation
- NFR-003.4: Secure Defaults (HTTPS, TLS)
- NFR-002.5: Graceful Degradation

**Epic 4: Offline Enrichment Capability**
- FR-005.1: Enrichment Export
- FR-005.2: Enrichment Import
- FR-005.3: Staleness Indicators
- NFR-002.5: Graceful Degradation
- NFR-006.3: Portable Deployment

**Epic 5: Export & Reporting**
- FR-006.1: Filtered Result Export
- FR-006.2: Full Dataset Export
- FR-006.3: Export Integrity
- NFR-001.4: Memory Efficiency (streaming export)
- NFR-002.2: Data Integrity (checksum, verification)

## Epic List

### Epic 0: Development Foundation & Quality Infrastructure

Establish the necessary infrastructure for building with quality, automated tests, and performance gates before any feature implementation.

**User Outcome (Developer Value):** Project scaffolded with Tauri + React, complete test infrastructure (unit, integration, performance benchmarks), Tailwind design system configured, and CI/CD with automated performance gates. Developers can build features with confidence knowing quality and performance are continuously validated.

**FRs Covered:** Architecture Requirements (Project Setup, Test Infrastructure, Tailwind CSS)
**NFRs Addressed:** NFR-001 (performance gates), NFR-005.1 (code quality), NFR-005.2 (documentation)

---

### Epic 1: Core Log Investigation Capability

Open OPNsense log files (up to 30 GB), index them quickly (2-3 minutes), and visualize all entries in a high-performance table.

**User Outcome:** Marc can open his 20 GB filter.log file, watch the indexing progress, and then instantly browse through all log entries with timestamps, IPs, ports, and actions in a smooth, responsive table. He experiences the first "Aha!" moment when the file indexes successfully without crashing and results display instantly.

**FRs Covered:** FR-001 (Log File Management), FR-002 (Log Parsing & Display)
**NFRs Addressed:** NFR-001.1 (indexing performance), NFR-001.3 (UI responsiveness), NFR-001.5 (startup time), NFR-001.6 (index load time), NFR-002.2 (data integrity), NFR-002.3 (corruption recovery), NFR-002.4 (idempotency)

---

### Epic 2: Advanced Filtering & Search

Build complex filters (time range + action + IP/port) visually, execute instant searches (<1 sec), and save frequent filters for reuse.

**User Outcome:** Marc can filter "blocked traffic on WAN between 2AM-3AM yesterday to port 443" using visual dropdowns (no syntax memorization), get 2,431 results in <1 second, and save this filter as "Nightly Port 443 Blocks" for future investigations. He completes investigations 3x faster than with the Python tool.

**FRs Covered:** FR-003 (Filtering & Search)
**NFRs Addressed:** NFR-001.2 (search performance), NFR-004.1 (learnability), NFR-004.2 (efficiency)

---

### Epic 3: Enhanced Context with API Enrichment

Connect to OPNsense API to transform cryptic technical data (vtnet0, rule hashes) into human-readable context (LAN, "Block RFC1918 Networks").

**User Outcome:** Marc sees "LAN → WAN: BLOCK - GeoIP Block Non-EU" instead of "vtnet0 → vtnet1: pass, rule abc123". He immediately understands what's happening without deciphering codes. This is the "Enrichment Clarity" success moment where technical data becomes instantly comprehensible.

**FRs Covered:** FR-004 (OPNsense API Enrichment)
**NFRs Addressed:** NFR-003.1 (credential protection), NFR-003.2 (local processing), NFR-003.3 (input validation), NFR-003.4 (secure defaults), NFR-002.5 (graceful degradation)

---

### Epic 4: Offline Enrichment Capability

Export enrichment mappings for offline use or environments without API access, with clear staleness indicators.

**User Outcome:** Marc exports the enrichment data from his office, then uses this file at 2AM from home to investigate with readable context even without VPN access to the API. The tool clearly indicates the enrichment is 6 days old, but he can still work effectively offline.

**FRs Covered:** FR-005 (Backup Enrichment System)
**NFRs Addressed:** NFR-002.5 (graceful degradation), NFR-006.3 (portable deployment)

---

### Epic 5: Export & Reporting

Export filtered results (or complete dataset) to CSV/JSON with comprehensive metadata for documentation, compliance, or external analysis.

**User Outcome:** Marc exports his 1,247 "blocked traffic" entries to CSV with complete metadata (filters applied, checksums, timestamps) for a report to his manager or a compliance audit. The export maintains data integrity and includes everything needed to reproduce the investigation.

**FRs Covered:** FR-006 (Export & Reporting)
**NFRs Addressed:** NFR-001.4 (memory efficiency via streaming), NFR-002.2 (data integrity)

---

## Epic 0: Development Foundation & Quality Infrastructure

Establish the necessary infrastructure for building with quality, automated tests, and performance gates before any feature implementation.

### Story 0.1: Project Scaffolding with Tauri + React

As a developer,
I want the project initialized with Tauri + React using the official create-tauri-app template,
So that I have a clean, production-ready foundation to build the log viewer application.

**Acceptance Criteria:**

**Given** create-tauri-app v4.6.0 is available
**When** I run `npm create tauri-app@latest opnsense-log-viewer -- --template react-ts`
**Then** the project structure is created with:
- `src-tauri/` directory for Rust backend with Cargo.toml
- `src/` directory for React frontend with TypeScript
- Vite configuration for frontend build
- Basic Tauri configuration in tauri.conf.json

**And** when I run `npm install && npm run tauri dev`
**Then** the application launches successfully showing a default window

**And** the project includes:
- TypeScript strict mode enabled
- ESLint and Prettier configurations
- Rust clippy and rustfmt configurations
- Git repository initialized with .gitignore

**And** the README contains:
- Project name: opnsense-log-viewer
- Quick start instructions
- Build commands for development and production

---

### Story 0.2: Comprehensive Test Infrastructure

As a developer,
I want complete test infrastructure with performance gates and quality automation,
So that I can build features with confidence knowing quality and performance are continuously validated.

**Acceptance Criteria:**

**Given** the project is scaffolded (Story 0.1 complete)
**When** I set up Vitest + React Testing Library
**Then** I can run `npm test` to execute frontend unit tests
**And** example component tests pass successfully

**When** I set up cargo bench with criterion.rs
**Then** I can run `cargo bench` to execute performance benchmarks
**And** baseline benchmarks are established for:
- Indexing performance target: <7 sec/GB
- Query performance target: <750ms for complex queries
- Memory usage target: <600 MB peak

**When** I create the 30GB synthetic log fixture generator
**Then** running the generator creates test files in `tests/fixtures/` with:
- RFC3164 format samples (10 GB)
- RFC5424 format samples (10 GB)
- CSV filterlog format samples (10 GB)
**And** generated logs contain realistic OPNsense firewall data patterns

**When** I configure CI/CD performance gates in GitHub Actions (or similar)
**Then** the CI pipeline includes:
- Automated performance regression tests on each PR
- Build fails if indexing exceeds 7 sec/GB (±15% tolerance)
- Build fails if queries exceed 750ms (±50% tolerance)
- Build fails if memory usage exceeds 600 MB (±20% tolerance)

**When** I set up property-based testing with proptest crate
**Then** parser robustness tests execute with:
- Random malformed log entries
- Edge cases (empty lines, special characters, truncated entries)
- Fuzzing for 1000+ iterations per test case

**When** I create integration test suite for Tauri IPC commands
**Then** I can run integration tests that:
- Mock Tauri command invocations from frontend
- Validate serialization/deserialization of Rust ↔ TypeScript
- Test error handling across IPC boundary

**And** all test commands are documented in README with:
- `npm test` - Run frontend unit tests
- `cargo test` - Run Rust unit tests
- `cargo bench` - Run performance benchmarks
- `npm run test:integration` - Run integration tests
- `./scripts/generate-fixtures.sh` - Generate 30GB test data

---

### Story 0.3: Tailwind CSS & Design System Foundation

As a developer,
I want Tailwind CSS integrated with design tokens and theme support,
So that I can build the UI following the UX Design Specification with consistent styling and dark/light mode support.

**Acceptance Criteria:**

**Given** the project is scaffolded (Story 0.1 complete)
**When** I install and configure Tailwind CSS
**Then** running `npm install -D tailwindcss postcss autoprefixer` succeeds
**And** `npx tailwindcss init -p` creates tailwind.config.js and postcss.config.js

**When** I configure design tokens in tailwind.config.js
**Then** the configuration includes custom tokens for:
- Colors: primary, success, warning, error, neutral scale (50-950)
- Typography: monospace for technical data (IPs, timestamps), sans-serif for UI
- Spacing: tight spacing values (2px, 4px, 8px) for data density
- Border radius: 4px and 8px for modern but professional aesthetic
- Shadows: subtle elevation shadows for visual hierarchy

**When** I configure dark/light theme support
**Then** Tailwind's `dark:` variant is enabled using class strategy
**And** CSS variables are defined for theme-aware colors:
- Background colors (light: white, dark: gray-900)
- Text colors (light: gray-900, dark: gray-50)
- Border colors with appropriate contrast
- Action color-coding: block=red, pass=green, reject=orange (both themes)

**When** I create base component library in `src/components/base/`
**Then** the following foundation components exist:
- Button (primary, secondary, ghost variants)
- Input (text, number types)
- Select/Dropdown
- Checkbox and Toggle
- Modal/Dialog
- ProgressBar (for indexation feedback)
- Toast (using react-hot-toast for errors/success messages)

**And** each component:
- Uses Tailwind utility classes
- Supports dark/light themes via `dark:` variants
- Includes TypeScript prop types
- Has ARIA labels for accessibility

**When** I add theme switcher utility
**Then** a `useTheme` hook exists that:
- Reads system preference on first launch (prefers-color-scheme)
- Stores user preference in localStorage
- Persists across sessions
- Provides `toggleTheme()` function

**And** applying a theme updates the document root class:
- Light theme: `<html class="light">`
- Dark theme: `<html class="dark">`

**And** the Vite build pipeline:
- Includes Tailwind CSS processing
- Purges unused CSS in production builds
- Tree-shakes for minimal bundle size

---

## Epic 1: Core Log Investigation Capability

Open OPNsense log files (up to 30 GB), index them quickly (2-3 minutes), and visualize all entries in a high-performance table.

### Story 1.1: File Selection with Native OS Picker

As a network administrator,
I want to select log files using my operating system's native file picker,
So that I can quickly open OPNsense log files from any location on my system.

**Acceptance Criteria:**

**Given** the application is launched
**When** I click File > Open or use Ctrl/Cmd+O shortcut
**Then** the native OS file picker dialog opens

**And** the file picker filters show:
- Log files (*.log)
- Text files (*.txt)
- CSV files (*.csv)
- All files (*.*)

**When** I select a file larger than 50 GB
**Then** a warning dialog displays: "Large file may take extended time to index. File size: XX GB. Continue?"
**And** I can choose [Continue] or [Cancel]

**When** I select a valid log file and click Open
**Then** the file path is loaded into the application
**And** the indexation process begins automatically (Story 1.3)

**And** if the file is unreadable (permissions, corrupt)
**Then** an error message displays: "Failed to open file: [specific error]. Please check file permissions and try again."

---

### Story 1.2: Multi-Format Log Parser (RFC3164, RFC5424, CSV)

As a network administrator,
I want the application to automatically detect and parse OPNsense log formats,
So that I don't need to manually specify the log format for each file.

**Acceptance Criteria:**

**Given** a log file is opened
**When** the parser analyzes the first 100 lines
**Then** it detects the format as one of: RFC3164, RFC5424, or CSV filterlog

**When** the format is RFC3164 (legacy syslog)
**Then** entries are parsed extracting:
- Priority (PRI)
- Timestamp
- Hostname
- Process name and PID
- Message content

**When** the format is RFC5424 (modern syslog)
**Then** entries are parsed extracting:
- Priority, Version, Timestamp
- Hostname, App-name, Process ID
- Message ID, Structured data
- Message content

**When** the format is CSV filterlog (OPNsense specific)
**Then** entries are parsed extracting all comma-separated fields:
- Timestamp, Interface, Action (pass/block/reject)
- Source IP, Source Port
- Destination IP, Destination Port
- Protocol, Rule hash/label

**When** the format is ambiguous or unrecognized
**Then** a dialog prompts: "Select log format: [RFC3164] [RFC5424] [CSV filterlog]"
**And** the user's selection is applied to parse the file

**When** malformed entries are encountered during parsing
**Then** the parser:
- Logs the line number and error to application log
- Skips the malformed entry
- Continues parsing subsequent entries
- Reports total skipped count after completion: "Parsed X entries, skipped Y malformed lines"

**And** parsing accuracy meets NFR-002.2:
- 100% accuracy for well-formed entries
- <0.1% discrepancy vs Python tool validation

---

### Story 1.3: Hybrid Index Creation (Inverted + Bitmap)

As a network administrator,
I want my large log files indexed with a hybrid inverted + bitmap index,
So that I can perform instant searches (<1 second) across 20-30 GB files after initial indexation.

**Acceptance Criteria:**

**Given** a log file is successfully parsed (Story 1.2 complete)
**When** indexation begins
**Then** a progress indicator displays showing:
- Percentage completion (0-100%)
- GB processed / Total GB
- Estimated time remaining
- Current indexation speed (GB/min)

**When** the indexer creates the inverted index
**Then** it builds HashMap structures for:
- Source IPs → List of entry IDs
- Destination IPs → List of entry IDs
- Source Ports → List of entry IDs
- Destination Ports → List of entry IDs

**When** the indexer creates the bitmap index
**Then** it builds compressed bitmaps (using roaring crate) for:
- Actions (pass, block, reject)
- Protocols (TCP, UDP, ICMP, etc.)
- Interfaces (physical interface names)

**And** the hybrid orchestrator combines both indexes
**Then** the complete index structure includes:
- Inverted index for high-cardinality fields (IPs, ports)
- Bitmap index for low-cardinality fields (actions, protocols)
- Entry offset table for random access to raw log lines
- Metadata: format type, entry count, timestamp

**When** indexation completes
**Then** performance meets NFR-001.1:
- Indexation speed: ≤7 sec/GB mean, ≤8 sec/GB P95
- On reference hardware: Intel i5-10400/AMD Ryzen 5 3600, 8GB RAM, SATA SSD

**And** memory usage meets NFR-001.4:
- Peak memory during indexation: ≤500 MB

**When** indexation is cancelled mid-process
**Then** temporary files (*.idx.tmp) are cleaned up
**And** the user is returned to the file selection state

---

### Story 1.4: Index Persistence & Reuse with SHA-256

As a network administrator,
I want my log file indexes saved and reused automatically,
So that I don't have to wait 2-3 minutes re-indexing the same file every time I open it.

**Acceptance Criteria:**

**Given** indexation completes successfully (Story 1.3)
**When** the index is persisted to disk
**Then** the index file is saved to application data directory as:
- Windows: `%APPDATA%\opnsense-log-viewer\indexes\`
- macOS: `~/Library/Application Support/opnsense-log-viewer/indexes/`
- Linux: `~/.local/share/opnsense-log-viewer/indexes/`

**And** the index filename includes the source file SHA-256 hash:
- Format: `{sha256_hash}.idx`

**And** the index file is compressed using Zstd (level 1)
**Then** the compressed index size is approximately:
- 10% of source log file size before compression
- ~5% of source log file size after compression (2x ratio)

**When** the index includes integrity metadata
**Then** the index header contains:
- Version number (for format evolution)
- SHA-256 checksum of index contents
- Source file SHA-256 hash
- Index creation timestamp
- Source file metadata (size, entry count, format)

**When** a user opens a previously indexed file
**Then** the application:
- Calculates SHA-256 hash of the source file
- Checks for existing index file matching the hash
- If found and valid, loads the index in ≤2 seconds (NFR-001.6)
- Skips re-indexation entirely

**When** the source file has been modified (hash mismatch)
**Then** a dialog prompts: "Source file changed since last index. Re-index? [Yes] [No]"
**And** selecting Yes triggers new indexation
**And** selecting No cancels the operation

**When** an index file is corrupted (checksum validation fails)
**Then** an error displays: "Index file corrupted. Re-index to continue? [Yes] [No]"
**And** the application never crashes on corrupted index (NFR-002.3)

**When** the user views existing indexes (Settings > Manage Indexes)
**Then** a list displays showing:
- Source filename (if still accessible)
- Index creation date
- Index file size
- Entry count
- [Delete] button for each index

**And** selecting Delete removes the .idx file and frees disk space

**And** index idempotency meets NFR-002.4:
- Re-indexing the same file produces bit-identical index (excluding timestamps)
- 100% SHA-256 hash match across 10 re-index operations

---

### Story 1.5: Log Entry Display Table with Virtual Scrolling

As a network administrator,
I want to view log entries in a high-performance table that smoothly handles 100K+ results,
So that I can quickly scan through large result sets without UI freezing or lag.

**Acceptance Criteria:**

**Given** a log file is indexed and loaded (Story 1.4 complete)
**When** the log entries are displayed
**Then** a table shows columns for:
- Timestamp (sortable, default descending)
- Interface (physical name, e.g., vtnet0)
- Source IP (sortable)
- Source Port
- Destination IP (sortable)
- Destination Port
- Protocol
- Action (pass/block/reject with color coding)
- Rule Label (hash or enriched name)

**And** action color-coding is applied:
- Block: red background/text
- Pass: green background/text
- Reject: orange background/text
**And** each action has an icon for non-color-dependent identification (accessibility)

**When** the result set contains 100K+ entries
**Then** virtual scrolling is implemented using TanStack Virtual
**And** only 50-100 visible rows are rendered in the DOM
**And** scrolling maintains 60 FPS (16ms frame time) per NFR-001.3

**When** I scroll through the table
**Then** rows render/unrender smoothly without frame drops
**And** the scrollbar accurately represents the total dataset size

**When** I click a column header (Timestamp, Source IP, Dest IP)
**Then** the table sorts by that column
**And** sort direction toggles between ascending/descending
**And** sort completes in <500ms for 100K entries

**When** the table displays data
**Then** styling follows UX Design Spec:
- Monospace fonts for technical data (IPs, ports, timestamps)
- Alternating row colors for scannability (subtle zebra striping)
- Tight line-height and spacing for data density
- Hover highlights without disrupting visual scanning

**When** I right-click on any cell
**Then** a context menu appears with:
- Copy Cell Value
- Copy Row
- Filter by This Value (launches filter builder with pre-filled value)

**And** the table is keyboard navigable:
- Arrow keys move selection
- Tab key navigates between table and other UI elements
- Enter on a row selects it for detail view (Story 1.6)

**And** the table adapts to screen sizes per NFR-004.5:
- On smaller screens (1280x720), less critical columns are hidden
- Column priority: Timestamp > Action > IPs > Ports > Interface > Protocol > Rule
- Horizontal scrolling available for full data access

---

### Story 1.6: Entry Detail View with Copy Functionality

As a network administrator,
I want to click on a log entry and see full details with easy copy functionality,
So that I can examine specific entries closely and extract data for reports or further analysis.

**Acceptance Criteria:**

**Given** log entries are displayed in the table (Story 1.5 complete)
**When** I single-click on a table row
**Then** the bottom pane expands showing the entry detail view

**When** the detail view displays
**Then** it shows two sections:

**Section 1 - Raw Log Line:**
- Full unparsed log entry exactly as it appears in the source file
- Monospace font for technical accuracy
- Read-only text area
- [Copy Full Entry] button copies entire raw line to clipboard

**Section 2 - Parsed Fields:**
- Key-value pairs for all parsed fields:
  - Timestamp: [value]
  - Interface: [value]
  - Source IP: [value] [Copy]
  - Source Port: [value] [Copy]
  - Destination IP: [value] [Copy]
  - Destination Port: [value] [Copy]
  - Protocol: [value] [Copy]
  - Action: [value] [Copy]
  - Rule Label: [value] [Copy]
- Each field has a [Copy] button for individual field copying

**When** I click any [Copy] button
**Then** the specific value is copied to clipboard
**And** a toast notification appears: "Copied to clipboard"

**When** I click [Copy Full Entry]
**Then** the entire raw log line is copied
**And** a toast notification confirms the action

**And** parsed field accuracy meets NFR-002.2:
- 100% of parsed fields match raw log data
- No data corruption or transformation errors

**When** I click outside the detail pane or press Esc
**Then** the detail view collapses back to minimal size

**And** the detail view is keyboard accessible:
- Tab navigates between Copy buttons
- Enter activates the focused Copy button
- Esc closes the detail view

**And** if enrichment is available (Epic 3), enriched values display alongside raw values:
- Interface: "LAN (vtnet0)"
- Rule Label: "Block RFC1918 Networks (rule hash abc123)"

---

## Epic 2: Advanced Filtering & Search

Build complex filters (time range + action + IP/port) visually, execute instant searches (<1 sec), and save frequent filters for reuse.

### Story 2.1: Visual Filter Builder UI

As a network administrator,
I want to build complex filters using visual dropdowns instead of syntax,
So that I can construct queries like "blocked traffic on WAN between 2AM-3AM to port 443" without memorizing filter syntax.

**Acceptance Criteria:**

**Given** log entries are loaded and displayed (Epic 1 complete)
**When** I click the "Add Filter" button in the left sidebar
**Then** a filter builder modal opens with three dropdown fields:

**Dropdown 1 - Field Selection:**
- Timestamp Range
- Source IP
- Destination IP
- Source Port
- Destination Port
- Protocol
- Action
- Interface
- Rule Label

**Dropdown 2 - Operator Selection (context-aware based on field):**

For text fields (IPs, Interface, Rule Label):
- equals
- contains
- starts with
- ends with
- regex

For numeric fields (Ports):
- equals
- greater than
- less than
- between

For select fields (Protocol, Action):
- equals
- not equals

For Timestamp:
- absolute range (date picker)
- relative (last 1h, 6h, 24h, 7d, 30d)

**Dropdown 3 - Value Input (context-aware based on field):**
- Text input for IPs, ports, regex
- Date/time picker for timestamps
- Dropdown for Protocol (TCP, UDP, ICMP, etc.)
- Dropdown for Action (pass, block, reject)

**When** I select a field and operator
**Then** the appropriate value input appears
**And** placeholder text provides guidance (e.g., "Enter IP address like 192.168.1.1")

**When** I complete all three fields and click "Add Filter"
**Then** the filter is added to the "Active Filters" list
**And** the modal closes
**And** the filter is in "draft mode" (not executed yet)

**When** I add multiple filters
**Then** each filter displays with:
- Field name
- Operator
- Value
- Boolean logic selector (AND/OR/NOT) for combining with next filter
- [Edit] button
- [Remove] button

**And** the filter UI follows UX Design Spec:
- VS Code-inspired sidebar layout
- Collapsible filter panel for space optimization
- Clear visual hierarchy
- Dark/light theme support

**When** filters are in draft mode
**Then** a prominent "Search" button is displayed
**And** the button is styled to signal it's the action trigger (Postman-style explicit control)
**And** no automatic query execution occurs when adding/removing filters

---

### Story 2.2: Query Execution Engine with Boolean Logic

As a network administrator,
I want to execute complex filter queries with AND/OR/NOT logic against the indexed data,
So that I can perform investigations like "blocked OR rejected traffic from 192.168.1.0/24 AND NOT to port 80".

**Acceptance Criteria:**

**Given** filters are added to the Active Filters list (Story 2.1 complete)
**When** I click the "Search" button
**Then** the query execution engine processes all active filters

**When** the query engine executes
**Then** it applies filters against the index structures (Story 1.3):
- IP filters query the inverted index (HashMap lookups)
- Port filters query the inverted index
- Action/Protocol/Interface filters query the bitmap index
- Timestamp filters apply range checks

**When** filters use AND logic
**Then** results must match ALL filter conditions
**And** bitmap intersection is used for efficiency

**When** filters use OR logic
**Then** results match ANY filter condition
**And** bitmap union is used for efficiency

**When** filters use NOT logic
**Then** results exclude matching entries
**And** bitmap negation/difference is used

**When** complex nested logic is present
**Then** the query optimizer determines the most efficient execution order:
- Bitmap filters first (fastest, most selective)
- Inverted index filters second
- Timestamp range filters last

**When** query execution completes
**Then** performance meets NFR-001.2:
- Simple queries (1 filter): <500ms
- Complex queries (5+ filters with AND/OR/NOT logic): <750ms
- P95 latency ≤750ms across 1000 query executions

**When** results are returned
**Then** the table updates showing matching entries
**And** a result count displays: "Showing X of Y entries"
**And** if X = 0, an empty state message displays: "No entries match current filters. Try adjusting your criteria."

**When** I modify filters after a search
**Then** the filters return to draft mode
**And** the "Search" button re-enables
**And** no automatic re-query occurs until I click "Search" again

**And** the query engine supports regex operators
**Then** regex patterns are compiled and applied efficiently
**And** invalid regex patterns show error: "Invalid regex pattern: [error details]"

---

### Story 2.3: Filter Management (Save, Load, Delete)

As a network administrator,
I want to save frequently-used filter combinations and reload them instantly,
So that I can reuse complex queries like "Nightly Port 443 Blocks" without manually reconstructing 5+ filters each time.

**Acceptance Criteria:**

**Given** I have active filters configured (Story 2.1 complete)
**When** I click the "Save Filter" button
**Then** a dialog prompts: "Enter filter name:"
**And** I provide a name like "Nightly Port 443 Blocks"

**When** I save the filter
**Then** the filter configuration is stored in browser localStorage including:
- Filter name
- All filter fields, operators, and values
- Boolean logic between filters (AND/OR/NOT)
- Creation timestamp

**When** I open the "Load Filter" dropdown
**Then** all saved filters are listed showing:
- Filter name
- Filter count (e.g., "5 filters")
- [Load] button
- [Delete] button

**When** I click [Load] on a saved filter
**Then** the Active Filters list is cleared
**And** all saved filters are loaded into the Active Filters list
**And** filters are in draft mode (not automatically executed)
**And** I can modify loaded filters before clicking "Search"

**When** I click [Delete] on a saved filter
**Then** a confirmation dialog appears: "Delete filter 'Nightly Port 443 Blocks'? This cannot be undone."
**And** confirming removes the filter from storage

**When** I click "Clear All Filters"
**Then** a confirmation dialog appears: "Clear all active filters?"
**And** confirming removes all filters from Active Filters list
**And** the table returns to showing all entries

**And** the filter management UI follows UX Design Spec:
- Saved filters organized in collapsible sidebar section
- Clear icons for Load/Delete actions
- Keyboard shortcuts: Ctrl/Cmd+S to save current filters

**And** I can save up to 20 filters in localStorage
**Then** loading any filter completes in <100ms per NFR-003.3

---

### Story 2.4: Search History with Persistence

As a network administrator,
I want automatic history of my recent searches with quick re-execution,
So that I can easily repeat yesterday's investigation or compare results across sessions.

**Acceptance Criteria:**

**Given** I execute a search query (Story 2.2 complete)
**When** the search completes successfully
**Then** the query is automatically added to search history

**When** search history is stored
**Then** each history entry captures:
- Timestamp of search execution
- All filter configurations (fields, operators, values, logic)
- Result count (X matches out of Y total)
- Source file name/hash

**And** history is stored in browser localStorage
**And** history persists across application sessions
**And** maximum 10 most recent searches are retained (FIFO)

**When** I open the "Search History" panel
**Then** history entries display showing:
- Time ago (e.g., "2 hours ago", "Yesterday at 3:42 PM")
- Summary of filters (e.g., "Action: block AND Dest Port: 443")
- Result count (e.g., "2,431 results")
- [Re-run] button
- [Delete] button

**When** I click [Re-run] on a history entry
**Then** the filters from that search are loaded into Active Filters
**And** the search executes automatically
**And** results update in the table

**When** I hover over a history entry
**Then** a tooltip displays the complete filter configuration:
- All filter fields with operators and values
- Boolean logic between filters
- Exact timestamp of original search

**When** I click [Delete] on a history entry
**Then** that entry is removed from history (no confirmation needed)

**And** search history follows UX Design Spec:
- Collapsible "Recent Searches" section in sidebar
- Recent items at the top
- Clear visual distinction from saved filters
- Automatic vs manual management (history is automatic, saved filters are manual)

**And** privacy consideration per NFR-003.2:
- Search history stored locally only
- No transmission to external servers
- Users can clear all history via Settings > Privacy > Clear Search History

---

## Epic 3: Enhanced Context with API Enrichment

Connect to OPNsense API to transform cryptic technical data (vtnet0, rule hashes) into human-readable context (LAN, "Block RFC1918 Networks").

### Story 3.1: API Connection Setup with Credential Storage

As a network administrator,
I want to configure my OPNsense API connection with secure credential storage,
So that I can connect once and have my credentials remembered securely across sessions.

**Acceptance Criteria:**

**Given** the application is running
**When** I open Settings > OPNsense API Configuration
**Then** I see a configuration form with fields:
- Endpoint URL (placeholder: "https://192.168.1.1")
- API Key (password field with show/hide toggle)
- API Secret (password field with show/hide toggle)
- [Test Connection] button
- [Save] button
- Connection status indicator (initially: Disconnected - Red)

**When** I enter API credentials
**Then** input validation ensures:
- Endpoint URL uses HTTPS protocol (HTTP rejected per NFR-003.4)
- Endpoint URL is a valid URL format
- API Key and Secret are non-empty strings

**When** I click [Test Connection]
**Then** the application attempts to connect to the OPNsense API
**And** makes a test call to `/api/diagnostics/interface/getInterfaceNames`
**And** a loading spinner appears during the test

**When** the connection succeeds
**Then** a success message displays: "Connected to OPNsense [version]. Found X interfaces."
**And** connection status indicator changes to: Connected - Green
**And** OPNsense version is displayed

**When** the connection fails
**Then** an error message displays with specific reason:
- "Connection refused: Unable to reach [URL]. Check firewall and network settings."
- "Authentication failed: Invalid API key or secret. Verify credentials in OPNsense."
- "TLS certificate invalid: [details]. Enable 'Accept Invalid Certificates' in Advanced settings (not recommended)."
**And** connection status indicator remains: Disconnected - Red

**When** I click [Save] with valid credentials
**Then** credentials are stored securely using OS keychain per NFR-003.1:
- Windows: Windows Credential Manager
- macOS: macOS Keychain
- Linux: Secret Service (libsecret)

**And** if OS keychain is unavailable
**Then** fallback to encrypted file storage:
- AES-256-GCM encryption
- Argon2id key derivation from device-specific seed
- Encrypted file stored in application data directory

**And** credentials never appear in:
- Application logs
- Error messages
- Debug output
- Network traffic logs (only sent via HTTPS to user's specified OPNsense endpoint)

**When** I reopen the application
**Then** saved credentials are automatically loaded from OS keychain
**And** connection status indicator shows: Attempting connection...
**And** automatic connection attempt is made in background
**And** if successful, status changes to: Connected - Green

**When** I want to manage multiple OPNsense devices
**Then** I can switch between saved device profiles:
- Profile name (e.g., "Office Firewall", "Home OPNsense")
- Each profile stores its own endpoint + credentials
- Active profile indicated in UI

---

### Story 3.2: Interface Name Mapping (Physical to Logical)

As a network administrator,
I want to see logical interface names like "LAN" and "WAN" instead of physical names like "vtnet0",
So that I can immediately understand which network segment the traffic belongs to without consulting documentation.

**Acceptance Criteria:**

**Given** API connection is established (Story 3.1 complete)
**When** the application connects successfully
**Then** it immediately calls `/api/diagnostics/interface/getInterfaceNames`

**When** the API response is received
**Then** the response is parsed to extract mappings:
- Physical interface: "vtnet0" → Logical name: "LAN"
- Physical interface: "vtnet1" → Logical name: "WAN"
- Physical interface: "vtnet2" → Logical name: "DMZ"
- (Continues for all configured interfaces)

**And** the mapping is cached in memory
**Then** the cache includes:
- Physical → Logical name mappings
- Last update timestamp
- OPNsense device identifier

**When** log entries are displayed in the table (Epic 1)
**Then** the Interface column shows:
- Logical name first: "LAN"
- Physical name on hover tooltip: "LAN (vtnet0)"

**When** a logical name is not available for a physical interface
**Then** the physical name is displayed: "vtnet3"
**And** hover tooltip shows: "vtnet3 (no logical name configured)"

**When** interface mappings are used in filter builder (Story 2.1)
**Then** the Interface field dropdown shows:
- Logical names as options: "LAN", "WAN", "DMZ"
- Physical names in parentheses: "LAN (vtnet0)"

**When** filtering by interface
**Then** queries work with either logical or physical names
**And** "Interface equals LAN" matches entries with physical interface "vtnet0"

**When** the API connection is lost
**Then** the application continues using cached interface mappings
**And** a warning indicator shows: "Using cached interface names (last updated: X minutes ago)"

**And** interface mapping meets NFR-003.2:
- API calls only to user's specified OPNsense endpoint
- Zero data transmission to external servers
- All processing happens locally

**And** mapping accuracy meets requirements:
- Correctly map all interfaces for firewalls with 2-10 interfaces per FR-004.2
- Handle edge cases (unconfigured interfaces, renamed interfaces)

---

### Story 3.3: Rule Label Enrichment (Hash to Description)

As a network administrator,
I want to see human-readable rule descriptions like "Block RFC1918 Networks" instead of cryptic rule hashes,
So that I can instantly understand why traffic was blocked or passed without looking up rule configurations.

**Acceptance Criteria:**

**Given** API connection is established (Story 3.1 complete)
**And** log entries are displayed with rule hashes (e.g., "rule abc123def")

**When** the application processes rule references
**Then** it identifies all unique rule hashes in the loaded log entries

**When** rule enrichment begins
**Then** for each unique rule hash:
- Call `/api/firewall/filter/searchRule` with the rule hash
- Parse the API response to extract rule description/label
- Cache the mapping in memory: hash → description

**And** rule enrichment happens asynchronously
**Then** the UI remains responsive during enrichment
**And** a progress indicator shows: "Enriching rules... X of Y"

**When** a rule label is successfully retrieved
**Then** the Rule Label column updates to show:
- Human-readable description: "Block RFC1918 Networks"
- Hash on hover tooltip: "Block RFC1918 Networks (rule abc123def)"

**When** a rule is not found via API
**Then** the display shows: "Rule abc123def (label unavailable)"
**And** the hash remains searchable/filterable

**When** the API returns an error for a specific rule
**Then** the error is logged but enrichment continues for other rules
**And** the problematic rule displays with hash only

**And** rule label caching is efficient
**Then** the cache avoids redundant API calls:
- Same rule hash queried only once per session
- Cache persists for the session duration
- Cache cleared when switching OPNsense devices

**When** rule labels are used in filter builder (Story 2.1)
**Then** the Rule Label field supports:
- Filtering by enriched description: "contains Block RFC1918"
- Filtering by hash: "equals abc123def"
- Autocomplete suggestions based on cached rule labels

**And** enrichment success rate meets FR-004.3:
- 95%+ of rule references successfully enriched
- Clear indication when enrichment unavailable
- No blocking of core functionality when API calls fail

**When** connection is degraded or offline
**Then** previously cached rule labels remain available
**And** new/unknown rules display with hashes only
**And** no application crashes or blocking errors occur (NFR-002.5)

---

### Story 3.4: Alias Resolution (IP Groups)

As a network administrator,
I want to see alias names like "Servers_Group" alongside IP addresses,
So that I can understand which predefined groups are involved in the traffic without memorizing IP ranges.

**Acceptance Criteria:**

**Given** API connection is established (Story 3.1 complete)
**And** log entries contain IP addresses that may be aliased

**When** the application processes IP addresses
**Then** it identifies all unique IPs in the loaded log entries

**When** alias resolution begins
**Then** for each unique IP address:
- Call `/api/firewall/alias/searchItem` with the IP address
- Parse the API response to extract alias name(s)
- Cache the mapping in memory: IP → alias name(s)

**And** alias resolution happens asynchronously
**Then** the UI remains responsive during resolution
**And** enrichment indicator updates: "Enriching aliases... X IPs processed"

**When** an alias is successfully retrieved for an IP
**Then** the Source IP or Destination IP column displays:
- IP address with alias: "192.168.1.100 (Servers_Group)"
- For multiple aliases: "192.168.1.100 (Servers_Group, DMZ_Hosts)"

**When** an IP is not aliased
**Then** only the IP address is displayed: "192.168.1.100"
**And** no hover tooltip or additional indicator

**When** an alias is a group (contains multiple IPs)
**Then** hovering over the alias name shows tooltip:
- "Servers_Group members: 192.168.1.100, 192.168.1.101, 192.168.1.102"

**When** the API returns an error for a specific IP
**Then** the error is logged but resolution continues for other IPs
**And** the problematic IP displays without alias

**And** alias caching is efficient
**Then** the cache avoids redundant API calls:
- Same IP queried only once per session
- Cache includes both single aliases and group expansions
- Cache cleared when switching OPNsense devices

**When** aliases are used in filter builder (Story 2.1)
**Then** the Source IP / Destination IP fields support:
- Filtering by alias name: "contains Servers_Group"
- Filtering by IP: "equals 192.168.1.100"
- Both match the same entries (expanded query)

**And** alias resolution success rate meets FR-004.4:
- 90%+ of aliased IPs successfully resolved
- Group alias members displayed in tooltips
- Graceful handling of unaliased IPs

**When** connection is degraded or offline
**Then** previously cached aliases remain available
**And** new/unknown IPs display without aliases
**And** no application crashes or blocking errors occur (NFR-002.5)

---

### Story 3.5: Graceful Degradation & Offline Mode

As a network administrator,
I want the application to continue working with raw data if the API is unavailable,
So that I can still investigate logs even without VPN access or when the firewall is unreachable.

**Acceptance Criteria:**

**Given** the application is running with log data loaded
**When** the API connection is lost or never established
**Then** the application continues to function with raw data:
- Physical interface names displayed (vtnet0, em0)
- Rule hashes displayed (abc123def)
- IP addresses displayed without aliases

**When** API connection fails during initial load
**Then** a banner displays at the top:
- "⚠️ API Offline - Showing raw data without enrichment."
- [Load Backup Enrichment] button (Epic 4)
- [Retry Connection] button
- [Dismiss] button

**And** the banner styling:
- Yellow/amber background (warning, not error)
- Icon indicating degraded mode
- Non-blocking (doesn't prevent usage)

**When** I click [Retry Connection]
**Then** the application attempts to reconnect using saved credentials
**And** connection status indicator updates accordingly
**And** if successful, enrichment begins automatically

**When** API connection is lost during active session
**Then** a toast notification appears: "API connection lost. Using cached enrichment data."
**And** previously enriched data remains visible
**And** new/un-cached data displays as raw values

**When** the API becomes available again
**Then** automatic reconnection happens in background
**And** toast notification: "API reconnected. Enrichment resumed."
**And** new queries receive fresh enrichment

**And** graceful degradation meets NFR-002.5:
- Zero crashes when API unavailable
- All core functionality accessible (file loading, indexing, filtering, search, export)
- Clear visual indicators of degraded mode
- No blocking errors or dialogs

**When** API call timeouts occur
**Then** timeout is set to 10 seconds per call
**And** after timeout, fall back to raw data for that specific enrichment
**And** other enrichment types continue attempting

**And** error messaging meets NFR-004.3:
- Actionable guidance: Suggests loading backup enrichment or reconnecting
- Specific failure reasons when possible
- Recovery steps clearly indicated
- 80%+ of users can recover without external help

**When** in offline mode
**Then** filter builder still works with:
- Raw interface names in dropdowns
- Rule hash filtering
- IP address filtering (no alias expansion)

**And** the application never blocks core workflows per anti-pattern:
- "If API fails, tool becomes useless" ❌
- "Tool continues working with raw data" ✅

---

## Epic 4: Offline Enrichment Capability

Export enrichment mappings for offline use or environments without API access, with clear staleness indicators.

### Story 4.1: Enrichment Data Export to JSON

As a network administrator,
I want to export my OPNsense enrichment data (interfaces, rules, aliases) to a JSON file,
So that I can use it later for offline investigations when I don't have API access.

**Acceptance Criteria:**

**Given** API connection is established and enrichment data is loaded (Epic 3 complete)
**When** I navigate to Settings > OPNsense API Configuration
**Then** I see an [Export Enrichment Data] button

**When** I click [Export Enrichment Data]
**Then** the application gathers all cached enrichment data:
- Interface mappings: `{"vtnet0": "LAN", "vtnet1": "WAN", "vtnet2": "DMZ"}`
- Rule labels: `{"abc123def": "Block RFC1918 Networks", "def456ghi": "GeoIP Block - Non-EU"}`
- Alias definitions: `{"Servers_Group": ["192.168.1.100", "192.168.1.101"], "DMZ_Hosts": ["10.0.1.5"]}`

**And** metadata is included:
- Export timestamp (ISO 8601 format)
- OPNsense hostname or endpoint URL
- OPNsense version (if available)
- Configuration hash (for change detection)
- Application version that created the export

**When** the export file is generated
**Then** it is saved as JSON with default filename:
- Format: `enrichment_{hostname}_{timestamp}.json`
- Example: `enrichment_firewall.local_2026-01-15T14-30-00.json`

**And** a save file dialog opens
**Then** I can choose the save location
**And** default location is user's Downloads folder

**When** the export completes successfully
**Then** a toast notification appears: "Enrichment data exported successfully"
**And** a [Open Folder] button appears to open the save location

**And** export file size meets FR-005.1:
- Typical firewall configs: <5 MB
- Larger configs (100+ rules, 50+ aliases): <15 MB
- JSON is human-readable (pretty-printed with 2-space indent)

**When** API connection is unavailable during export
**Then** the export uses cached enrichment data from current session
**And** metadata indicates: "Exported from cache (last API update: [timestamp])"

**And** security per NFR-003.1:
- API credentials are NOT included in export
- Only enrichment mappings are exported
- Export file can be safely shared without exposing credentials

**When** enrichment data is empty or minimal
**Then** a warning appears: "Limited enrichment data available. Connect to API first for complete export."
**And** export proceeds with available data

---

### Story 4.2: Enrichment Data Import with Validation

As a network administrator,
I want to import a previously exported enrichment JSON file,
So that I can investigate logs with readable context even when I'm offline or can't access the API.

**Acceptance Criteria:**

**Given** I have an enrichment JSON file (Story 4.1 complete)
**And** API connection is unavailable or I choose to use backup data

**When** I see the "API Offline" banner (Story 3.5)
**Then** the [Load Backup Enrichment] button is visible

**When** I click [Load Backup Enrichment]
**Then** a file picker dialog opens
**And** file type filter shows: "Enrichment Files (*.json)"

**When** I select an enrichment JSON file
**Then** the application validates the file:
- JSON syntax is valid
- Required fields present: interfaces, rules, aliases, metadata
- Metadata includes timestamp and source identifier

**When** validation succeeds
**Then** the enrichment data is loaded into memory
**And** a toast notification appears: "Backup enrichment loaded successfully"

**When** validation fails
**Then** an error dialog displays with specific reason:
- "Invalid JSON format: [parse error details]"
- "Missing required fields: [field names]"
- "File appears corrupted. Try exporting a new enrichment file."

**And** the application does not crash (NFR-002.1)
**And** current state is preserved (no partial load)

**When** the enrichment file is older than 7 days
**Then** a warning displays per FR-005.2:
- "⚠️ Enrichment data from [date] ([X] days old)."
- "Interface mappings and rule labels may be outdated."
- "Verify accuracy for critical investigations."
- [Continue] and [Cancel] buttons

**And** the warning is dismissible with:
- Checkbox: "Don't show this warning again for this file"
- Dismissed state saved for current session only

**When** I continue with outdated enrichment
**Then** the data is applied to the current log view
**And** all enriched fields update:
- Interface column shows logical names from backup
- Rule Label column shows descriptions from backup
- IP columns show aliases from backup

**When** backup enrichment is active
**Then** it persists until:
- API reconnects and user chooses to use live data
- New backup enrichment file is loaded
- Application is restarted

**And** the application clearly indicates backup mode per FR-005.3:
- Status indicator shows: "Using backup enrichment"
- Timestamp of enrichment data displayed
- Visual distinction from live API enrichment

---

### Story 4.3: Staleness Indicators & Warnings

As a network administrator,
I want clear visual indicators when using outdated backup enrichment,
So that I'm always aware if the context I'm seeing might not reflect current firewall configuration.

**Acceptance Criteria:**

**Given** backup enrichment is loaded (Story 4.2 complete)
**When** the enrichment data is active
**Then** a persistent indicator displays in the top bar:
- Icon: ⚠️ (warning triangle)
- Text: "Using backup enrichment ([X] days old)"
- Background: Yellow/amber (subtle but noticeable)
- Position: Top-right corner, non-blocking

**When** I hover over the staleness indicator
**Then** a tooltip displays with details:
- "Enrichment exported: [full date and time]"
- "Age: [X days, Y hours]"
- "Source: [hostname/endpoint]"
- "Interface mappings and rule labels may be outdated."
- "Reconnect to API for current data."

**When** I click the staleness indicator
**Then** a dialog opens with options:
- [Reconnect to API] - Attempts to connect and refresh enrichment
- [Load Different Backup] - Opens file picker for another enrichment file
- [Use Raw Data] - Removes backup enrichment, shows raw values
- [Dismiss for Session] - Hides indicator for current session

**When** backup enrichment is <24 hours old
**Then** indicator shows: "Using backup enrichment (1 day old)" with yellow background

**When** backup enrichment is 1-7 days old
**Then** indicator shows: "Using backup enrichment ([X] days old)" with amber background

**When** backup enrichment is >7 days old
**Then** indicator shows: "Using backup enrichment ([X] days old) ⚠️ Verify accuracy" with orange background

**And** on initial load of >7 day old enrichment
**Then** a more prominent warning dialog appears per Story 4.2

**When** I choose "Dismiss for Session"
**Then** the indicator is hidden for the remainder of the session
**And** a small icon remains in the status bar: ⚠️
**And** clicking the icon re-shows the full indicator

**When** API reconnects while backup enrichment is active
**Then** a notification appears: "API reconnected. Switch to live enrichment? [Yes] [Keep Backup]"
**And** selecting Yes:
- Clears backup enrichment
- Fetches fresh enrichment from API
- Removes staleness indicators
- Updates all enriched fields in the view

**And** visual distinction meets UX Design Spec per FR-005.3:
- Clear difference between live API mode and backup mode
- Never ambiguous which enrichment source is active
- Color-coding aligns with warning severity (yellow → amber → orange)

**And** the option to ignore warnings respects user choice:
- "Don't show again for this session" - Respects user's decision
- Warning re-appears on next session (safety consideration)
- User remains in control of their workflow

---

## Epic 5: Export & Reporting

Export filtered results (or complete dataset) to CSV/JSON with comprehensive metadata for documentation, compliance, or external analysis.

### Story 5.1: Filtered Result Export (CSV/JSON with Metadata)

As a network administrator,
I want to export my filtered search results to CSV or JSON with complete metadata,
So that I can create reports for my manager, document incidents, or analyze data in external tools like Excel.

**Acceptance Criteria:**

**Given** I have executed a search with filters (Epic 2 complete)
**And** results are displayed in the table (e.g., 1,247 entries)

**When** I click the "Export" button in the toolbar
**Then** an export dialog opens showing:
- Format selection: [CSV] or [JSON] radio buttons
- Export scope: "Filtered Results (1,247 entries)" (pre-selected)
- [Export] button
- [Cancel] button

**When** I select CSV format and click [Export]
**Then** a save file dialog opens
**And** default filename: `opnsense_filtered_export_{timestamp}.csv`
**And** default location: user's Downloads folder

**When** the CSV export is generated
**Then** the file includes:

**Metadata header (as CSV comments):**
```
# Exported by: opnsense-log-viewer v1.0.0
# Export Date: 2026-01-15T14:32:01Z
# Source File: /path/to/filter.log (SHA-256: abc123...)
# Filters Applied: action=block, interface=WAN, timestamp=2026-01-10 to 2026-01-15
# Total Entries: 1,247 of 2,450,000
```

**Data rows with headers:**
- Timestamp, Interface, Source IP, Source Port, Destination IP, Destination Port, Protocol, Action, Rule Label
- All filtered entries in CSV format
- Proper escaping of commas and quotes
- Compatible with Excel, LibreOffice, Google Sheets

**When** I select JSON format and click [Export]
**Then** a save file dialog opens
**And** default filename: `opnsense_filtered_export_{timestamp}.json`

**When** the JSON export is generated
**Then** the file includes:

**Metadata object:**
```json
{
  "metadata": {
    "exportedBy": "opnsense-log-viewer v1.0.0",
    "exportDate": "2026-01-15T14:32:01Z",
    "sourceFile": {
      "path": "/path/to/filter.log",
      "sha256": "abc123..."
    },
    "filtersApplied": [
      {"field": "action", "operator": "equals", "value": "block"},
      {"field": "interface", "operator": "equals", "value": "WAN"}
    ],
    "totalEntries": 1247,
    "totalInSource": 2450000
  },
  "entries": [
    {
      "timestamp": "2026-01-15T14:30:00Z",
      "interface": "WAN",
      "sourceIp": "203.0.113.5",
      ...
    }
  ]
}
```

**And** JSON is pretty-printed (2-space indent) for readability

**When** the export involves >10,000 rows
**Then** a progress indicator displays:
- "Exporting... X of Y entries"
- Progress bar showing percentage
- [Cancel] button to abort export

**And** the export completes successfully
**Then** a toast notification appears: "Export completed: 1,247 entries written"
**And** an [Open Folder] button opens the save location

**And** export performance meets FR-006.1:
- Export 100K entries to CSV in <10 seconds
- No UI blocking during export (async operation)

**When** enrichment is active (Epic 3)
**Then** exported data includes enriched values:
- Logical interface names (LAN, WAN)
- Rule descriptions instead of hashes
- Alias names alongside IPs

**And** if using backup enrichment
**Then** metadata notes: "Enrichment: Backup (exported 2026-01-10)"

---

### Story 5.2: Full Dataset Export with Streaming

As a network administrator,
I want to export the entire unfiltered dataset for archival or compliance purposes,
So that I can create complete backups of log data in portable formats.

**Acceptance Criteria:**

**Given** I have a log file loaded with 2.5M entries
**When** I open the export dialog
**Then** I see an additional option:
- Export scope: [Filtered Results (1,247 entries)] or [Full Dataset (2,500,000 entries)]

**When** I select "Full Dataset" and click [Export]
**Then** a warning dialog appears:
- "⚠️ Export 2,500,000 entries? This may take several minutes."
- Estimated time: "[X] minutes (approximate)"
- Estimated file size: "[Y] MB"
- [Continue] [Cancel] buttons

**When** I click [Continue] on the warning
**Then** the export begins using streaming mode per NFR-001.4:
- Data is written to disk in chunks (not loaded entirely into memory)
- Memory usage remains ≤600 MB throughout export
- No application freezing or unresponsiveness

**And** a progress dialog displays:
- "Exporting full dataset... X of Y entries"
- Progress bar (percentage complete)
- Elapsed time: "[X] seconds"
- Estimated remaining: "[Y] seconds"
- [Cancel] button to abort export

**When** I click [Cancel] during export
**Then** the export is aborted
**And** a confirmation appears: "Export cancelled. Partial file deleted."
**And** the incomplete export file is removed from disk

**When** the full dataset export completes
**Then** the file includes the same metadata format as filtered export (Story 5.1)
**And** metadata notes: "Export Type: Full Dataset (unfiltered)"

**And** export performance meets FR-006.2:
- Export 2M entries without exceeding 600 MB memory usage
- Streaming prevents memory overflow
- Large exports complete without crashes

**When** disk space is insufficient
**Then** the export fails gracefully:
- Error dialog: "Insufficient disk space. Required: [X] MB, Available: [Y] MB"
- Partial export file is deleted
- User is returned to export dialog

**And** export integrity is maintained:
- No data corruption during streaming
- All entries present in export match source data
- Row count verification after completion

---

### Story 5.3: Export Integrity & Verification

As a network administrator,
I want exported files to include integrity checksums and verification,
So that I can trust the export data for compliance audits and ensure no corruption occurred.

**Acceptance Criteria:**

**Given** an export is completed (Story 5.1 or 5.2)
**When** the export file is generated
**Then** a SHA-256 checksum is calculated for the export file contents

**And** for CSV exports
**Then** the checksum is added to the metadata header:
```
# Export SHA-256: abc123def456...
# Row Count Verification: 1,247 entries written
```

**And** for JSON exports
**Then** the checksum is added to the metadata object:
```json
{
  "metadata": {
    ...
    "exportChecksum": {
      "algorithm": "SHA-256",
      "hash": "abc123def456..."
    },
    "verification": {
      "entriesWritten": 1247,
      "exportComplete": true
    }
  }
}
```

**When** the export completes successfully
**Then** a verification summary is displayed:
- "✅ Export completed successfully"
- "Entries written: 1,247"
- "File size: 2.3 MB"
- "SHA-256: abc123def... [Copy]"
- [Open Folder] button

**When** I click [Copy] next to the checksum
**Then** the full SHA-256 hash is copied to clipboard
**And** a toast appears: "Checksum copied to clipboard"

**When** the export completes
**Then** a row count verification occurs:
- Count entries written to file
- Compare with expected count (filtered results or full dataset)
- Log any discrepancies

**And** verification meets FR-006.3:
- 100% of exported rows match source data
- No data loss or corruption during export
- Checksums allow independent verification

**When** an export fails mid-process (crash, power loss, disk full)
**Then** the partial file is marked incomplete:
- For JSON: `"exportComplete": false` in metadata
- For CSV: No checksum or verification comments added

**And** on next application launch
**Then** incomplete export files are detected
**And** a notification appears: "Incomplete export detected. Delete partial file? [Yes] [No]"

**When** I want to verify an export file later
**Then** Settings > Verify Export File option exists
**And** selecting a file:
- Recalculates SHA-256 checksum
- Compares with checksum in file metadata
- Displays result: "✅ Checksum valid" or "❌ Checksum mismatch - file may be corrupted"

**And** [Open Folder] functionality
**Then** clicking the button opens the file explorer:
- Windows: Explorer at export file location
- macOS: Finder at export file location
- Linux: Default file manager at export file location
**And** the exported file is highlighted/selected in the explorer

**And** export integrity supports compliance requirements:
- Checksums provide evidence integrity for audits
- Metadata documents filters and source for traceability
- Verification process is transparent and reproducible
