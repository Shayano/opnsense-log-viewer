---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-03-success', 'step-04-journeys', 'step-05-domain', 'step-06-innovation', 'step-07-project-type', 'step-08-scoping', 'step-09-functional', 'step-10-nonfunctional', 'step-11-complete']
inputDocuments:
  - "_bmad-output/planning-artifacts/product-brief-opnsense-log-viewer-2026-01-14.md"
  - "_bmad-output/planning-artifacts/ux-design-specification.md"
  - "_bmad-output/analysis/brainstorming-session-2026-01-14.md"
  - "docs/DEVELOPER_GUIDE.md"
  - "README.md"
  - "docs/Introduction_API.html"
  - "docs/API_Reference.html"
  - "docs/Firewall_API.html"
  - "docs/Interfaces_API.html"
  - "docs/Diagnostics_logs_API.html"
workflowType: 'prd'
documentCounts:
  briefCount: 1
  researchCount: 0
  brainstormingCount: 1
  projectDocsCount: 2
classification:
  projectType: 'Desktop Application'
  domain: 'Network Administration / Security Operations / Log Analysis'
  complexity: 'Medium-High'
  projectContext: 'brownfield'
---

# Product Requirements Document - opnsense-log-viewer

**Author:** Shay
**Date:** 2026-01-15

## Success Criteria

### User Success

**Performance Achievement:**
- **Indexation Performance:** Successfully index 20-30 GB log files in 2-3 minutes on standard hardware (compared to 30+ minutes with current solution that crashes)
- **Search Performance:** Return filtered results in <1 second for any query combination across indexed files
- **Zero Crashes:** 0% crash rate when handling files up to 30 GB (compared to 100% failure rate with current Python application beyond 2-3 GB)

**Productivity Impact:**
- **Investigation Time Reduction:** Users complete typical log investigations in <10 minutes (compared to hours with current workarounds or abandoned investigations)
- **First-Time Success:** Users successfully complete their first investigation without consulting documentation or support
- **Workflow Integration:** Users abandon SSH/grep workarounds within first week of adoption

**User Satisfaction:**
- **Ease of Use:** Users rate the tool as "intuitive" without requiring training
- **Reliability Confidence:** Users trust the tool for critical incident investigations
- **Feature Adoption:** 80%+ of users utilize core features (indexing, filtering, interface mapping, API integration)

### Business Success

**3-Month Goals:**
- 100+ GitHub stars indicating community interest
- 500+ downloads from network admin community
- 5+ community contributions (bug reports, feature requests, PRs)

**12-Month Goals:**
- 1000+ active users (measured by unique download IPs or optional telemetry)
- Recognition in OPNsense community forums as recommended tool
- 20+ community contributions demonstrating active engagement

**Technical Excellence:**
- Maintain clean, well-documented codebase enabling community contributions
- Verified functionality on Windows/Linux/macOS with consistent UX
- Documented performance characteristics across hardware profiles

**Market Position:**
- Become the go-to desktop application for OPNsense log analysis
- Maintain unique positioning vs generic log analyzers (Elasticsearch/Splunk) and CLI tools

### Technical Success

**Performance Requirements:**
- Maximum file size successfully processed: 30 GB without degradation
- Average indexation time per GB: <6 seconds per GB
- Average search response time: <500ms for complex queries
- RAM usage during operations: <500 MB

**Quality Requirements:**
- Crash rate per session: <0.1%
- User-reported bugs per release: <3 critical bugs
- Time to fix critical issues: <48 hours
- Zero data corruption or parsing errors on production log files

**Adoption Metrics:**
- Downloads per month (target: 50+ by month 3, 200+ by month 12)
- Active users with successful investigations
- GitHub watch/star/fork activity indicating community interest
- Feature utilization rate (% of users using advanced filtering, API integration)
- Export feature usage (indicating users create reports/documentation)

**Community Health:**
- Response time to community issues: <24 hours for initial response
- Active community discussion threads
- Community contribution acceptance rate

### Measurable Outcomes

**MVP Validation (3-4 months):**
- First 10 users complete investigations faster than with current Python tool
- Users successfully enrich logs via API without configuration difficulties
- No critical bugs requiring workarounds in core filtering functionality
- Users report the tool as "reliable enough for production investigations"
- 50+ downloads in first month indicating community interest
- 5+ positive testimonials from network administrators
- GitHub issues demonstrate real-world usage patterns
- Community requests align with deferred feature roadmap

**Go/No-Go Decision Point:**
If MVP meets these criteria after 3-4 months of development, proceed with post-MVP enhancements. If technical performance targets aren't met, revisit architecture before adding features.

## Product Scope

### MVP - Minimum Viable Product

**1. High-Performance Log Indexation**
- Index 20-30 GB log files in 2-3 minutes on standard hardware
- Hybrid index architecture combining inverted multi-column index (for IPs, ports) and bitmap index (for action, protocol, interface)
- Adaptive multi-threading that scales automatically based on available CPU cores
- Zero crashes when handling large files (eliminate the `f.readlines()` bottleneck)

**2. Advanced Filtering System**
- Complete filtering capabilities: AND/OR/NOT boolean logic
- Operator support: equals, contains, startswith, endswith, regex
- Field filtering across all log attributes: source/destination IPs, ports, action, protocol, interface, timestamps
- Sub-second search response time across indexed files
- Time range filtering for historical analysis

**3. Data Enrichment via OPNsense API**
- **Interface Mapping:** Translate physical interfaces to logical names (vtnet0 → LAN, vtnet1 → WAN)
- **Firewall Rule Labels:** Display human-readable rule descriptions instead of cryptic hashes
- **Alias Resolution:** Show alias names for IP groups and network objects
- API-first integration replacing fragile SSH-based enrichment
- Graceful degradation when API unavailable (show raw data without enrichment)

**4. Reliable Core Operations**
- OPNsense log format parsing: RFC3164, RFC5424, CSV filterlog
- Memory-efficient operation that doesn't require loading entire files
- Result export to JSON/CSV for documentation and reporting
- Persistent index storage for fast re-opening of previously indexed files

**5. Multiplatform Desktop Application**
- Tauri + Rust architecture for Windows/Linux/macOS
- Portable executable with reasonable size
- Modern GUI with intuitive interface
- No installation, no dependencies, no server infrastructure required

### Growth Features (Post-MVP)

**Deferred from 54 Brainstorming Ideas:**

**Advanced Analytics (Phase 2: Months 5-8):**
- Automatic anomaly detection highlighting unusual traffic patterns
- Saved query library for common investigation scenarios
- Basic visualization: timeline charts, top talkers, protocol distribution
- Query result caching for repeated investigations

**Multi-Instance Support (Phase 3: Months 9-12):**
- Aggregate logs from multiple OPNsense firewalls
- Cross-firewall correlation for complex network environments
- Centralized query execution across distributed deployments
- Unified interface mapping and rule label management

**Enterprise Features (Post-MVP):**
- Team collaboration features (shared queries, annotations)
- RBAC and audit logging
- Centralized management for multiple OPNsense instances

**Advanced Integrations (Post-MVP):**
- SIEM integration (Splunk, ELK, etc.)
- Webhook notifications for alerts
- Third-party threat intelligence enrichment
- Automated response actions

**Performance Enhancements (Post-MVP):**
- Hot reloading for live log updates
- Incremental indexing for append-only logs
- Distributed indexing for 100+ GB files
- Cloud storage integration

**Enhanced Visualization (Post-MVP):**
- Interactive dashboards and charts
- Geographic IP mapping
- Traffic flow diagrams
- Real-time monitoring displays

### Vision (Future)

**Phase 4: Ecosystem Integration (Year 2+)**
- SIEM export capabilities for enterprise workflows
- Plugin architecture for custom enrichment sources
- Webhook alerts for automated monitoring
- Advanced ML-based threat detection

**Long-Term Vision:**
Become the definitive desktop application for OPNsense log analysis, balancing power-user capabilities with simplicity. Expand cautiously into multi-firewall scenarios while maintaining the portable, zero-infrastructure philosophy that differentiates it from enterprise log management platforms.

**Rationale for Deferred Features:**
The MVP focuses exclusively on solving the core problem - reliable, fast analysis of large log files with proper context enrichment. Advanced features risk delaying delivery and introducing complexity that conflicts with the "Simplicity > Feature Creep" principle established in the Product Brief.

## User Journeys

### Journey 1: Marc - Routine Investigation (Happy Path)

**Opening Scene:**
Monday morning, 9:15 AM. Marc's coffee is still hot when the first ticket arrives: "VPN users can't connect since Friday night." He's the solo network admin for a 50-person marketing agency, and this is his typical Monday morning firefight. The OPNsense firewall has been logging everything all weekend - the filter.log file is 18 GB. With the old Python tool, he'd spend 10 minutes just waiting for it to crash. Now he opens the new Tauri application.

**Rising Action:**
Marc drags the 18 GB filter.log file into the application. Indexation begins - the progress bar shows "Indexing... 6.2 GB/s" and completes in 2 minutes 54 seconds. The main interface appears with all log entries visible. He starts filtering systematically:

1. **Time range:** Filters to Friday 6 PM - Monday 9 AM (weekend window)
2. **Interface filter:** Selects "WAN" from the enriched dropdown (not vtnet1 - the tool shows "WAN" automatically)
3. **Action filter:** Selects "block" to see rejected connections
4. **Port filter:** Adds "1194" (OpenVPN port)

Each filter applies in under 500ms. Results show 3,847 blocked connection attempts. The enriched interface shows source IPs with their countries, rule labels that explain *why* they were blocked (not just cryptic hashes), and timestamps with clear patterns.

**Climax:**
Marc notices something odd in the rule label column: most blocks show "GeoIP Block - Non-EU", but 15 attempts from his CEO's home IP show "Certificate Validation Failed". That's the smoking gun. He clicks on one entry, exports the filtered view to CSV for documentation, then cross-references the certificate expiration date in the OPNsense UI. The SSL certificate for the VPN expired Friday at 11:47 PM.

**Resolution:**
By 9:21 AM - just 6 minutes after opening the ticket - Marc has identified the root cause, renewed the certificate, and sent a detailed report to the CEO with exported log evidence. The investigation that would have taken 30+ minutes (and multiple crashes) with the Python tool was completed in under 10 minutes with the new application. Marc adds a calendar reminder to monitor certificate expiration dates and closes the ticket.

**Emotional Journey:** Frustration → Confidence → Relief → Professional Satisfaction

---

### Journey 2: Marc - Offline Investigation (API Unavailable)

**Opening Scene:**
It's 2 AM on a Tuesday. Marc is woken by a monitoring alert: the company's main web server is unreachable. He grabs his laptop and VPNs in from home. The OPNsense firewall is accessible, but it's in maintenance mode - someone from the MSP is upgrading firmware, and the API is temporarily unavailable. Marc needs to investigate *now*, but he can't get enriched data from the API.

**Rising Action:**
Marc downloads the filter.log file (12 GB) to his laptop. He opens the Tauri application and sees the API connection indicator showing "API Offline - Using Backup Enrichment". Fortunately, last week he exported the enrichment data (interface mappings, rule labels, aliases) to a JSON file as a backup. The application prompts: "API unavailable. Load backup enrichment data? (Last export: 2026-01-10)". Marc loads the backup file.

The application indexes the 12 GB file using the backup enrichment data. Interfaces show as "LAN", "WAN", "DMZ" (not vtnet0/vtnet1/vtnet2), and rule labels display human-readable descriptions from the backup snapshot. The enrichment is 6 days old, but it's accurate enough for this emergency investigation.

**Climax:**
Marc filters to the web server's IP (192.168.100.50) and the last 2 hours. He sees the traffic pattern clearly: the server was receiving normal traffic until 1:47 AM, then suddenly all traffic shows "state mismatch" errors. The issue isn't firewall blocking - it's the server itself that stopped responding, causing the firewall to drop the stale connections.

**Resolution:**
Marc SSH's into the web server and finds the Apache service crashed due to a memory leak. He restarts Apache, verifies traffic is flowing again by checking the live logs in OPNsense, and documents the incident with exported CSV evidence from the Tauri application. By 2:31 AM, the issue is resolved. Even without live API enrichment, Marc completed a critical investigation offline using backup enrichment data.

**Emotional Journey:** Anxiety → Resourcefulness → Control → Exhausted Relief

---

### Journey 3: Sarah - First Time User Learning

**Opening Scene:**
Sarah is a junior network admin, just 3 months into her first IT job. Her manager Marc is out sick, and she's been asked to investigate reports of slow guest WiFi performance. She's heard Marc mention the "new log viewer tool" but has never used it herself. She finds the application icon on the shared admin desktop and double-clicks nervously.

**Rising Action:**
The application opens to a clean interface with a prominent "Open Log File" button. Sarah navigates to the firewall logs folder and selects filter.log (4.2 GB). A progress bar appears - "Indexing..." - and completes in 41 seconds. She's surprised it worked so fast; Marc mentioned the old Python tool would crash with large files.

Now she sees a table filled with log entries, but it's overwhelming - thousands of rows. She notices a collapsible filter panel on the left with dropdown menus. The interface looks familiar, almost like filtering a spreadsheet. She clicks on "Interface" and sees a dropdown with options: "LAN", "WAN", "Guest_WiFi", "DMZ". She selects "Guest_WiFi" - her target network.

The results update instantly: 8,392 entries. Still too many. She notices other filter options: "Action" (block/pass), "Protocol" (TCP/UDP/ICMP), "Source IP", "Destination IP". She selects "Action: block" to see what's being rejected. Results narrow to 1,240 entries.

**Climax:**
Sarah scans the blocked entries and notices a pattern: dozens of guest devices are being blocked with the rule label "Rate Limit Exceeded - Guest Network". She doesn't fully understand rate limiting yet, but the human-readable label tells her exactly what's happening. She exports the filtered results to CSV and emails it to Marc with a note: "I think the guest WiFi slowness might be related to rate limiting - see attached filtered logs."

**Resolution:**
Marc replies 20 minutes later (from his phone, still sick at home): "Perfect diagnosis! The rate limit is set too low for the current guest load. I'll adjust it remotely. Great work using the tool!" Sarah feels a surge of confidence - she successfully completed her first log investigation without extensive training. The visual interface with enriched, human-readable data made the tool approachable even for a beginner.

**Emotional Journey:** Nervousness → Curiosity → Discovery → Pride

---

### Journey 4: IT Manager - Compliance Audit Report

**Opening Scene:**
David is the IT manager for a financial services company subject to quarterly compliance audits. It's the last week of Q4, and the auditor has requested evidence of firewall traffic analysis for specific date ranges and source IPs related to a security incident in October. David needs to provide detailed reports showing blocked traffic, rule enforcement, and investigative actions taken.

With the old Python-based tool, generating these reports was painful: he'd manually filter logs, export sections, and compile evidence over 4+ hours. The logs for Q4 are massive - over 85 GB across multiple files covering October through December.

**Rising Action:**
David opens the Tauri application and begins loading the October log files one by one. He indexes filter.log.0 (22 GB), filter.log.1 (19 GB), and filter.log.2 (15 GB) sequentially. Each file indexes in 2-3 minutes. The application maintains all indexed files in memory for cross-file querying.

He builds a complex filter using the auditor's requirements:
- **Time Range:** October 12-18, 2025 (incident window)
- **Source IP:** 203.0.113.0/24 (suspicious IP range identified during incident)
- **Action:** "block" (show rejected traffic)
- **Protocol:** "TCP"
- **Destination Port:** "22, 3389, 5900" (SSH, RDP, VNC - common attack vectors)

The query executes across all three indexed files in 1.8 seconds, returning 4,267 matching entries.

**Climax:**
The enriched results show exactly what the auditor needs: source IPs with geographic context, destination services with port labels ("SSH", "RDP", "VNC"), rule labels explaining why traffic was blocked ("GeoIP Block - High Risk Country", "Port Scan Detection", "Brute Force Protection"), and precise timestamps.

David applies additional filters to create sub-reports:
1. **By Rule Type:** Separate exports for GeoIP blocks vs. brute force detection
2. **By Time of Day:** Hourly breakdown to show attack patterns
3. **Top Talkers:** Sort by source IP frequency to identify most aggressive attackers

He exports each view to CSV with descriptive filenames: "Q4_Audit_GeoIP_Blocks_Oct12-18.csv", "Q4_Audit_BruteForce_Oct12-18.csv", "Q4_Audit_TopAttackers_Oct12-18.csv".

**Resolution:**
In 22 minutes, David has generated comprehensive, audit-quality reports with proper evidence chain. He compiles the CSV exports into a compliance report package and submits it to the auditor. The auditor responds within hours: "Excellent documentation. This is exactly what we need. No further questions on firewall traffic analysis."

David reflects on the efficiency gain: what used to take 4+ hours of manual log parsing, crashes, and CSV stitching now takes under 30 minutes with a reliable, repeatable process. The tool's ability to handle massive multi-file datasets with complex filtering has transformed compliance reporting from a quarterly nightmare into a manageable task.

**Emotional Journey:** Dread → Focus → Satisfaction → Professional Validation

---

### Journey 5: Marc - Urgent Late-Night Troubleshooting

**Opening Scene:**
It's 2:47 AM on a Saturday. Marc's phone buzzes with a critical alert: the company's production server has lost connectivity. He's at home, exhausted, working from a 13-inch laptop. He needs to diagnose fast and get back to sleep. He downloads the filter.log file (8.7 GB) over his home internet - it takes 90 seconds.

**Rising Action:**
Marc opens the Tauri application on his small laptop screen. The interface adapts responsively - the filter panel is collapsible, giving him maximum space for the results table. He indexes the 8.7 GB file in 1 minute 24 seconds.

He's working with a small screen and limited patience at 3 AM. He needs efficiency:
1. **Time Range:** Last 30 minutes (2:15 AM - 2:45 AM)
2. **Destination IP:** 10.0.50.10 (production server)
3. **Action:** "block" (something is being rejected)

Results: 847 blocked connections. The enriched interface shows source IPs, protocols, and rule labels. Marc collapses the sidebar filter panel to see more columns.

**Climax:**
Marc notices all blocked traffic has the same source IP: 203.0.113.45 (a new monitoring service the dev team set up yesterday). The rule label shows "Rate Limit Exceeded - Production Network". The monitoring service is making health check requests every 100ms - way too aggressive. The firewall rate limiter is blocking it, cutting off the server's monitoring.

**Resolution:**
Marc adjusts the rate limit threshold in the OPNsense firewall UI, whitelists the monitoring service IP, and verifies traffic is flowing again. He exports the filtered logs to CSV as evidence for tomorrow's postmortem, closes his laptop, and goes back to sleep. Total time from alert to resolution: 8 minutes. Even on a small laptop screen at 3 AM, the tool's responsive UI and fast search kept him focused and efficient.

**Emotional Journey:** Exhaustion → Urgency → Focus → Relief

---

### Journey Requirements Summary

These journeys reveal the following capability areas that must be supported:

**Core Investigation Capabilities:**
- **Multi-filter builder** with AND/OR/NOT boolean logic (Journey 1, 4)
- **API-based enrichment** for interface mapping, rule labels, alias resolution (Journey 1, 3)
- **Sub-second search** across indexed files (Journey 1, 4, 5)
- **Result export** to CSV/JSON for documentation and reporting (Journey 1, 4, 5)
- **Search history** and saved filters for repeated investigations (Journey 4)

**Fallback & Resilience:**
- **Backup enrichment data support** via JSON export/import when API unavailable (Journey 2)
- **Graceful degradation** showing raw data when enrichment unavailable (Journey 2)
- **Offline capability** for emergency investigations (Journey 2)

**Learnability & Discoverability:**
- **Visual dropdown filters** with human-readable options (Journey 3)
- **Enriched value suggestions** in filter fields (Journey 3)
- **Helpful guidance** for first-time users without extensive training (Journey 3)

**Reporting & Compliance:**
- **Bulk export** for compliance and audit reporting (Journey 4)
- **CSV export compatibility** with spreadsheet tools (Journey 1, 4, 5)
- **Saved filters** for repeatable compliance queries (Journey 4)
- **Evidence-quality exports** with complete context (Journey 4)

**Emergency & Constraints:**
- **Responsive UI** adapting to small laptop screens (Journey 5)
- **Fast investigation** under time pressure (Journey 5)
- **Remembered credentials** for API connections (Journey 5)
- **Collapsible sidebar** for maximizing results view (Journey 5)

## Domain-Specific Requirements

### Compliance & Regulatory

**Evidence Chain of Custody:**
- Exported logs must maintain integrity for legal/compliance use (Journey 4: audit reporting)
- Timestamps must be preserved accurately (no timezone corruption)
- CSV/JSON exports must be tamper-evident for forensic investigations
- Data retention policies respected (no unauthorized data persistence)
- **Export Attribution & Metadata**: Every CSV/JSON export must include:
  - Export timestamp in ISO 8601 format with timezone
  - Tool version (e.g., "opnsense-log-viewer v1.2.3")
  - Original log file path and SHA-256 hash
  - Filter criteria applied (documenting what was excluded from export)
  - Export operator identification (username from OS, no separate login required)

**Audit Trail Requirements:**
- Log parsing operations must not modify original log files
- Export operations should be logged (who exported what, when)
- API credential handling must follow security best practices (no plaintext storage)

**Data Privacy Considerations:**
- **GDPR IP Anonymization**: Optional IP anonymization/pseudonymization for exports destined for compliance reporting in privacy-sensitive jurisdictions
- Network admins need real IPs for investigation (no anonymization during analysis)
- Export feature should offer "Anonymize IPs for compliance reporting" option

**Data Retention Policy Enforcement:**
- Persisted indexes should have configurable auto-expiration (e.g., delete indexes older than 90 days)
- Clear indicators showing when indexed data was created
- Warning when investigating "stale" indexes (e.g., "This index is 45 days old - verify log file hasn't been rotated")
- Tool must never delete original log files (user's responsibility)

### Technical Constraints

**Log Format Standards:**
- **RFC3164** (Legacy Syslog) - Must parse correctly with no data loss
- **RFC5424** (Modern Syslog) - Full structured data support
- **CSV filterlog** (OPNsense proprietary) - Accurate field mapping
- Graceful handling of malformed log entries (don't crash on corrupt data)
- **Reference Documentation**: `docs/Opnsense_doc/` for OPNsense log format specifications

**Index Persistence & Integrity:**
- Persisted indexes must be read-only once created
- Index files must include checksums to detect tampering
- Index-source mismatch detection: warn if original log file modified after indexing
- **Atomic Index Creation**: Use temp file + rename pattern (`.idx.tmp` → `.idx`)
- Automatic cleanup of orphaned `.idx.tmp` files from crashed sessions
- **Concurrent Access**: Specify file locking strategy for multi-user scenarios

**Cross-Platform Compatibility:**
- File path handling must work across Windows (backslashes) and Linux/macOS (forward slashes)
- Log exports must preserve original file paths in metadata without breaking cross-platform compatibility
- Portable executable with no platform-specific dependencies

**Security Requirements:**
- API credentials encrypted at rest (OS keychain: Windows Credential Manager, macOS Keychain, Linux Secret Service)
- No log data sent to external services (100% local processing)
- Memory-safe parsing (Rust advantage - no buffer overflows)
- Backup enrichment files validated before import (Journey 2 - prevent malicious JSON injection)
- **Sandboxed Log Parsing**: Parser runs with minimal privileges, no network access, no file system writes outside designated cache directory
- **Reference Documentation**: `docs/Rust_doc/` for Rust security patterns

**Performance Requirements:**
- **Real-time criticality**: Emergency investigations cannot wait (Journey 5: 3 AM troubleshooting)
- **Large dataset handling**: Must scale to 30+ GB without performance degradation
- **Search latency**: Sub-second response critical for investigation flow (Journey 1)
- **Memory efficiency**: <500 MB RAM usage to run on laptops during emergencies (Journey 5)
- **Hard Performance Gates** (CI/CD enforcement):
  - Indexing speed: <7 seconds per GB (±15% tolerance)
  - Search latency: <750ms for complex queries (±50% tolerance)
  - Memory usage: <600 MB peak (±20% tolerance)
  - Any regression beyond tolerance = build fails

**Reliability Requirements:**
- **Zero crashes**: Investigations cannot restart mid-analysis
- **Graceful degradation**: API unavailable? Use backup enrichment (Journey 2)
- **Data integrity**: No silent parsing failures - every log entry accounted for
- **Idempotent operations**: Re-indexing same file produces identical results
- **Index Corruption Recovery**: Partial indexes from failed builds must be automatically cleaned up
- **Optimistic Error Handling Anti-Pattern**: Every API call, file operation, and user input must have explicit error handling with actionable error messages

### Integration Requirements

**OPNsense API Integration:**
- **Interface mapping**: `/api/diagnostics/interface/getInterfaceNames`
- **Firewall rules**: `/api/firewall/filter/searchRule` for rule label enrichment
- **Alias resolution**: `/api/firewall/alias/searchItem` for IP group names
- **Authentication**: API key + secret with secure storage
- **Error handling**: API timeout, rate limiting, maintenance mode (Journey 2)
- **Version Compatibility**: Detect OPNsense version via `/api/core/firmware/status` and adapt API calls accordingly
- Graceful fallback for older OPNsense versions (pre-API era firewalls)
- **Reference Documentation**: `docs/Opnsense_doc/` for API endpoint specifications and `docs/Tauriv2_doc/` for Tauri API integration patterns

**Backup Enrichment Format:**
- JSON export format compatible with offline investigation (Journey 2)
- Include snapshot timestamp to indicate data freshness
- Include OPNsense configuration version/hash
- Validate schema on import (prevent corrupted backups from breaking investigations)
- **Enrichment Data Staleness Indicators**:
  - Visual indicator in UI showing enrichment data age (e.g., "Enrichment data is 6 days old")
  - Warning for critical investigations: "Verify accuracy - interface mappings may have changed"
  - Option to export fresh enrichment data on-demand during API maintenance
- **Validation Requirements**:
  - Reject oversized JSON files (>100 MB)
  - Handle truncated JSON gracefully (incomplete export scenarios)
  - Detect and reject JSON injection attacks (escape sequences, code execution attempts)
  - Schema validation for missing required fields and wrong data types

### Risk Mitigations

**Critical Failure Scenarios:**

1. **Risk**: Incorrect log parsing leads to false conclusions in security investigations
   - **Mitigation**: Unit tests against known OPNsense log samples; validation against Python tool output
   - **Cross-Tool Validation**: Compare parsing results between Tauri tool and Python tool (<0.1% acceptable discrepancy)

2. **Risk**: Performance degradation during emergency investigations (Journey 5)
   - **Mitigation**: Performance benchmarks as part of CI/CD; regression testing with 30 GB test files
   - **Hard Gates**: CI must fail builds exceeding performance tolerance thresholds

3. **Risk**: API credential compromise
   - **Mitigation**: OS-native credential storage (Windows Credential Manager, macOS Keychain, Linux Secret Service); never log credentials

4. **Risk**: Exported evidence files corrupted or incomplete
   - **Mitigation**: Export validation (row count verification, hash checksums); include metadata header (export time, filter criteria, tool version)

5. **Risk**: Tool unavailable during critical incidents (offline scenarios)
   - **Mitigation**: Portable executable with no external dependencies; backup enrichment system (Journey 2)

6. **Risk**: Enrichment Data Staleness
   - **Scenario**: Journey 2 shows Marc using 6-day-old backup enrichment where interface mappings may have changed
   - **Mitigation**: Backup enrichment files must include timestamp, configuration version/hash, and visual staleness indicators in UI

7. **Risk**: Index Corruption During Build
   - **Scenario**: Indexing 28 GB fails at 94% due to disk full, out of memory, or user process termination
   - **Mitigation**: Atomic index creation (`.idx.tmp` → `.idx` only on success); automatic cleanup of orphaned temp files on startup

8. **Risk**: Performance Regression Between Versions
   - **Scenario**: Version 1.3.0 regresses indexing speed from 6 seconds/GB to 9 seconds/GB
   - **Mitigation**: CI/CD hard performance gates with tolerance thresholds; builds fail on unacceptable regression

### Validation & Testing Standards

**Golden Log File Test Suite:**
- Maintain curated set of real OPNsense log samples covering:
  - RFC3164, RFC5424, CSV filterlog formats
  - Edge cases: malformed entries, truncated lines, non-ASCII characters
  - Different OPNsense versions (21.x, 22.x, 23.x, 24.x)
  - Size range: 100 KB, 10 MB, 1 GB, 15 GB, 30 GB

**Cross-Tool Validation:**
- For each golden log file, compare parsing results between Tauri tool and Python tool
- Acceptable discrepancy: <0.1% of entries (accounting for parsing improvement fixes)

**Stress Testing Protocol:**
- CI must run nightly stress tests:
  - 30 GB file indexing on minimum spec hardware (4 GB RAM, dual-core CPU)
  - 100 consecutive filter operations without memory leak
  - API timeout/retry behavior under simulated network failure

**Backup Enrichment Validation:**
- Test suite must include corrupted/malicious JSON files:
  - Truncated JSON (simulates incomplete export)
  - Oversized JSON (>100 MB - should reject with clear error)
  - JSON injection attacks (escape sequences, code execution attempts)
  - Schema violations (missing required fields, wrong data types)

### Domain-Specific Anti-Patterns

- ❌ **Cloud-based log analysis** (violates zero-infrastructure philosophy and introduces latency)
- ❌ **Modifying original log files during indexing** (breaks forensic integrity)
- ❌ **Assuming API is always available** (Journey 2 shows maintenance windows)
- ❌ **Requiring installation/admin rights** (hinders emergency troubleshooting)
- ❌ **Batch-only processing** (users need interactive filtering, not scheduled reports)
- ❌ **Optimistic error handling** ("Failed to load enrichment data" is useless; provide actionable error messages with byte offsets, specific failure reasons, and recovery steps)

## Desktop Application Specific Requirements

### Project-Type Overview

**Desktop Application Architecture:**
- **Framework**: Tauri v2 + Rust backend with system WebView frontend
- **UI Rendering**: Platform-native web engine (WebView2 on Windows, WebKit on macOS/Linux)
- **Distribution Model**: Portable executable with zero installation requirements
- **Philosophy**: Minimal OS integration, maximum portability

**Reference Documentation**:
- `docs/Tauriv2_doc/` for Tauri v2 architecture patterns and API specifications
- `docs/Rust_doc/` for Rust backend implementation and crate ecosystem

**Tauri v2 API Verification Requirements:**
- Verify `tauri-plugin-keyring` availability and OS support (Windows/macOS/Linux)
- Confirm `tauri::api::process::Command` for spawning heavy indexing threads
- Check `tauri::window::ProgressBarState` for native progress bar integration
- Validate Tauri v2 IPC streaming capabilities for large dataset handling

### Platform Support

**Build Targets:**
- **Windows**: x64 (Windows 10+ version 1809 or later)
- **Linux**: x64 (multiple distributions - see below)
- **macOS**: x86_64 (Intel, macOS 10.15+ Catalina) **AND** Apple Silicon (ARM64, macOS 11+ Big Sur)
  - Build universal binary via `lipo` to support both Intel and ARM architectures in single `.app` bundle

**Platform-Specific Considerations:**

**Windows:**
- **WebView Engine**: WebView2 (Edge Chromium engine)
- **WebView2 Runtime Requirement**:
  - **NOT bundled** in portable executable (reduces download size)
  - Application detects missing WebView2 on launch and displays error with download link: https://developer.microsoft.com/microsoft-edge/webview2/
  - Windows 11 and Windows 10 (post-May 2021) include WebView2 by default
  - Pre-2021 Windows 10 installations require manual WebView2 runtime installation (~100 MB download)
- **Executable Format**: `.exe` portable executable (no installer, run directly)
- **Binary Size**: ~15-20 MB (Rust binary + embedded web assets, excluding WebView2 runtime)

**Linux:**
- **WebView Engine**: WebKitGTK
- **Supported Distributions**:
  - Debian-based: Ubuntu 20.04+, Debian 11+, Linux Mint 20+
  - RHEL-based: Fedora 34+, CentOS Stream 8+, RHEL 8+, Rocky Linux 8+
  - Arch-based: Rolling release (current)
  - openSUSE: Leap 15.3+, Tumbleweed
- **System Dependencies**:
  ```
  libwebkit2gtk-4.0-37 (or webkit2gtk-4.0)
  libgtk-3-0
  libayatana-appindicator3-1 (for system tray, if added later)
  ```
- **Package Distribution**: Provide `.tar.gz` archive with binary and `.desktop` file for manual installation
- **Executable Format**: ELF binary (dynamically linked against system WebKitGTK)

**macOS:**
- **WebView Engine**: WKWebView (Safari engine)
- **Universal Binary**: Single `.app` bundle supporting both Intel (x86_64) and Apple Silicon (ARM64)
- **Executable Format**: `.app` bundle distributed as `.dmg` disk image
- **Gatekeeper Handling**: Unsigned app requires user override (see Code Signing section)

**No Platform-Specific Features:**
- Consistent UX across all platforms (no Windows-only or macOS-only functionality)
- Cross-platform UI testing required to ensure visual parity
- Feature parity maintained across all supported platforms

**Minimum Hardware Requirements:**
- **CPU**: Dual-core processor (quad-core recommended for 30+ GB files)
  - Example minimum: Intel i5-8250U, AMD Ryzen 5 2500U, Apple M1
- **RAM**: 4 GB minimum (8 GB recommended for emergency laptop scenarios - Journey 5)
- **Disk**: 20 MB for application executable + space for index files (~10% of log file size)
  - Example: 30 GB log file requires ~3 GB for index storage
- **Display**: 1280x720 minimum resolution (responsive UI adapts to 13" laptop screens - Journey 5)
- **Disk Type**: SSD recommended for indexing performance; HDD supported but slower indexing (10-15 seconds/GB vs 6 seconds/GB)

**Performance Benchmark Reference Hardware:**
- **Primary Benchmark**: Intel i5-10400 (6-core) or AMD Ryzen 5 3600, 8 GB RAM, SATA SSD
- **Minimum Spec Testing**: Dual-core CPU, 4 GB RAM, SATA SSD (validate performance gates hold)
- **CI/CD Runners**: GitHub Actions (2-core, 7 GB RAM) for smoke tests only; self-hosted runners for performance validation

### System Integration

**Credential Storage:**

**Primary: OS-Native Secure Storage**
- **Windows**: Windows Credential Manager via `winapi` crate (`CredWriteW`/`CredReadW` APIs)
- **macOS**: Keychain Services via `security-framework` crate
- **Linux**: Secret Service API via `secret-service` crate (libsecret)

**Implementation:**
- Use `tauri-plugin-keyring` if available in Tauri v2 stable
- If plugin unavailable, implement directly using platform-specific crates above
- Credential format: Store API key and secret as separate credential entries
- Credential identifier: `opnsense-log-viewer::<endpoint-hostname>`

**Fallback: Encrypted Local Storage**

When OS keychain unavailable (e.g., headless Linux, WSL, restricted environments):

- **Key Derivation**: Argon2id (memory-hard, GPU-resistant)
  - Parameters: 64 MB memory, 3 iterations, 4 parallelism
  - Salt: 16 bytes cryptographically random, stored alongside encrypted credentials
- **Encryption Algorithm**: AES-256-GCM (authenticated encryption)
- **Master Password**: User creates master password on first credential save when keychain unavailable
  - Password strength requirements: minimum 12 characters
  - Master password never stored, only derived key hash for verification
- **Storage Location**: `<config_dir>/credentials.enc` (encrypted blob)
- **Security Warning**: Display prominent warning to user:
  > "OS keychain unavailable. Credentials will be encrypted with master password. Store master password securely - it cannot be recovered if lost."

**Config & Data Storage:**

**Application Data Directories:**
- **Windows**: `%APPDATA%\opnsense-log-viewer\`
  - Config: `%APPDATA%\opnsense-log-viewer\config.toml`
  - Indexes: `%APPDATA%\opnsense-log-viewer\indexes\`
  - Logs: `%APPDATA%\opnsense-log-viewer\logs\`
- **macOS**: `~/Library/Application Support/opnsense-log-viewer/`
  - Config: `~/Library/Application Support/opnsense-log-viewer/config.toml`
  - Indexes: `~/Library/Application Support/opnsense-log-viewer/indexes/`
  - Logs: `~/Library/Application Support/opnsense-log-viewer/logs/`
- **Linux**: `~/.config/opnsense-log-viewer/` (config), `~/.local/share/opnsense-log-viewer/` (data)
  - Config: `~/.config/opnsense-log-viewer/config.toml`
  - Indexes: `~/.local/share/opnsense-log-viewer/indexes/`
  - Logs: `~/.local/share/opnsense-log-viewer/logs/`

**Config File Format (TOML):**
```toml
[app]
version = "1.0.0"
first_run = false

[ui]
theme = "dark" # "dark" | "light" | "system"
result_page_size = 1000
virtual_scroll_buffer = 100
sidebar_collapsed = false

[index]
auto_cleanup_enabled = true
auto_cleanup_days = 90
max_index_size_mb = 5000

[opnsense]
# API credentials stored via OS keychain or encrypted file, NOT in config.toml
last_used_endpoint = "https://192.168.1.1"
last_used_auth_method = "keychain" # "keychain" | "encrypted_file" | "none"

[performance]
indexing_threads = 0 # 0 = auto-detect CPU cores
memory_limit_mb = 500
```

**Index File Format:**

**Index File Structure (`.idx` binary format):**

```
┌─────────────────────────────────────────────────┐
│ HEADER (256 bytes fixed)                        │
├─────────────────────────────────────────────────┤
│ - Magic Number: 0x4F504E53 ("OPNS") [4 bytes]  │
│ - Format Version: u32 [4 bytes]                 │
│ - Source File SHA-256: [32 bytes]               │
│ - Index Timestamp: u64 (Unix epoch) [8 bytes]   │
│ - Total Entry Count: u64 [8 bytes]              │
│ - Compression: u8 (0=none, 1=zstd) [1 byte]     │
│ - Reserved: [197 bytes for future use]          │
├─────────────────────────────────────────────────┤
│ METADATA SECTION (variable length)              │
├─────────────────────────────────────────────────┤
│ - Field definitions (bincode-serialized)        │
│ - Index statistics (min/max timestamps, IP      │
│   ranges, port ranges, action counts, etc.)     │
│ - OPNsense version detected from logs           │
├─────────────────────────────────────────────────┤
│ INVERTED INDEX SECTION (variable length)        │
├─────────────────────────────────────────────────┤
│ - IP addresses → Vec<entry_id> (bincode)        │
│ - Ports → Vec<entry_id> (bincode)               │
│ - Timestamps → Vec<entry_id> (bincode)          │
│ - Hashed data structure for O(1) lookups        │
├─────────────────────────────────────────────────┤
│ BITMAP INDEX SECTION (variable length)          │
├─────────────────────────────────────────────────┤
│ - Action (block/pass): RoaringBitmap            │
│ - Protocol (TCP/UDP/ICMP): RoaringBitmap        │
│ - Interface IDs: RoaringBitmap per interface    │
│ - Using `roaring` crate for compressed bitmaps  │
├─────────────────────────────────────────────────┤
│ ENTRY OFFSET TABLE (variable length)            │
├─────────────────────────────────────────────────┤
│ - entry_id → byte_offset in source file [u64]   │
│ - Allows fast line retrieval without re-parsing │
└─────────────────────────────────────────────────┘
```

**Serialization:**
- Use `bincode` crate (fast, space-efficient Rust-native serialization)
- Alternative: `postcard` crate (smaller binary, slightly slower)
- **NOT JSON** (too slow and bloated for 30 GB datasets)

**Index Versioning:**
- Format version increments on breaking changes
- Application detects incompatible index version and prompts re-indexing
- Forward compatibility: v1.1 reads v1.0 indexes
- Backward compatibility: v1.0 cannot read v1.1 indexes (graceful error message)

**File System Access:**
- **Log File Access**: Read-only access to user-specified log files (no special permissions required)
- **Index Storage**: Write access to application data directory (user-owned, no elevation needed)
- **Export Directory**: User-specified location via file picker dialog (no restrictions)
- **Temporary Files**: Use system temp directory for `.idx.tmp` during indexing
  - Windows: `%TEMP%\opnsense-log-viewer\`
  - macOS/Linux: `/tmp/opnsense-log-viewer/` or `$TMPDIR`

**Cross-Platform File Locking:**

**Implementation (using `fs2` crate):**
```rust
use fs2::FileExt;

// Cross-platform file locking
// - Windows: LockFileEx (exclusive lock, enforced by OS)
// - Linux/macOS: flock (advisory lock, not enforced by OS)

let index_file = File::open("log.idx")?;
index_file.lock_exclusive()?; // Blocks until lock acquired
// ... perform operations ...
index_file.unlock()?;
```

**Linux Advisory Lock Caveat:**
- `flock()` on Linux/macOS is advisory-only (not enforced by kernel)
- Malicious/buggy process can ignore lock and corrupt `.idx` file
- **Mitigation Strategy**:
  - Atomic writes: `.idx.tmp` → `.idx` via `fs::rename()` (atomic on POSIX)
  - Checksum validation: Verify header checksum on load, fail if corrupted
  - Lock file pattern: Create `.idx.lock` alongside `.idx` as visual indicator

**No Additional OS Integration:**
- ❌ No file associations (`.log` files don't auto-open with tool)
- ❌ No context menu integration (no right-click "Open with" registry modifications)
- ❌ No system tray icon (application runs in taskbar/dock only)
- ❌ No Windows services or macOS daemons (no background processes)
- ❌ No shell integration or PATH modifications (no CLI commands installed globally)
- ❌ No URL protocol handlers (no `opnsense://` custom protocol registration)

### Update Strategy

**Distribution & Updates:**

**Primary Distribution Channel:**
- **GitHub Releases**: https://github.com/[org]/opnsense-log-viewer/releases
- **Semantic Versioning**: v1.0.0 (major.minor.patch)
  - Major: Breaking changes (API, config format, index format)
  - Minor: New features, backward-compatible changes
  - Patch: Bug fixes, security patches

**Release Artifacts (per version):**
- `opnsense-log-viewer-v1.0.0-windows-x64.exe` (Windows portable executable)
- `opnsense-log-viewer-v1.0.0-linux-x64.tar.gz` (Linux binary + .desktop file)
- `opnsense-log-viewer-v1.0.0-macos-universal.dmg` (macOS universal binary, Intel + ARM)
- `checksums.txt` (SHA-256 checksums for all artifacts)
- `CHANGELOG.md` (release notes with upgrade instructions)

**Update Mechanism:**
- **Manual Download & Replace**: Users download new version and replace old executable
- **Optional Version Check**: Application checks GitHub API for newer version on launch
  - Non-blocking: Does not prevent application startup if check fails
  - Frequency: Once per day maximum (cached locally)
  - Display notification in UI: "New version v1.2.0 available - Download"
  - Click notification opens GitHub Releases page in browser
- **No Auto-Update**: No automatic downloads or background updates
- **No Forced Updates**: Users can continue using older versions indefinitely

**Rollback Strategy:**
- **Manual Rollback**: Download previous version from GitHub Releases and replace executable
- **Config Migration**: Application detects older config format and migrates forward
- **Index Compatibility**: If new version creates incompatible indexes, keep old version executable to access old indexes

**Version Management:**

**Config File Migration:**
- Application detects `config.toml` version mismatch
- Automatic migration for minor/patch versions (v1.0 → v1.1)
- Manual migration prompt for major versions (v1.x → v2.x)
- Backup original config before migration: `config.toml.v1.0.0.bak`

**Index Format Compatibility:**
- Index files embed format version in header
- Application checks index version on load:
  - **Compatible**: Load and use normally
  - **Older but compatible**: Load with migration (if applicable)
  - **Incompatible**: Display error: "Index created with v2.x, please re-index with current version (v1.x)"
- Warning for stale indexes: "This index is 45 days old - verify log file hasn't been rotated"

**Breaking Changes Protocol:**
- Major version bumps (v1.x → v2.x) allowed for:
  - Index format changes requiring re-indexing
  - Config structure changes requiring manual migration
  - API changes requiring user intervention
- Release notes include clear upgrade path and migration instructions

### Code Signing & Security

**Code Signing:**

**MVP Approach (No Code Signing):**
- **Rationale**: Reduces complexity, eliminates certificate costs ($100-400/year), faster iteration
- **Target Audience**: Network administrators (tech-savvy, comfortable with security warnings)
- **Risk**: 30-40% user drop-off due to OS security warnings

**Security Warnings (Unsigned Binaries):**

**Windows SmartScreen:**
- Warning: "Windows protected your PC - Windows Defender SmartScreen prevented an unrecognized app from starting"
- User Action Required: Click "More info" → "Run anyway"
- **Mitigation**:
  - Detailed README section with screenshots showing bypass steps
  - 2-minute YouTube video walkthrough
  - FAQ: "Why does Windows block the app?"

**macOS Gatekeeper:**
- Warning: "[App] cannot be opened because the developer cannot be verified"
- User Action Required: Right-click app → "Open" → Confirm in dialog, OR terminal command:
  ```bash
  xattr -d com.apple.quarantine /Applications/opnsense-log-viewer.app
  ```
- **Mitigation**:
  - README with step-by-step instructions for both methods
  - Video tutorial for non-technical users
  - Self-signed certificate option (reduces friction slightly)

**Linux:**
- No security warnings (binary not signed by default on Linux)
- Users download via `wget`/`curl`, verify SHA-256 checksum manually

**Future Code Signing (Post-MVP):**
- **Windows**: EV Code Signing Certificate (~$300/year, requires hardware token)
  - Benefit: No SmartScreen warning after reputation builds (~1000 downloads)
- **macOS**: Apple Developer ID Certificate ($99/year)
  - Benefit: No Gatekeeper warning, app can be distributed outside App Store
  - Requirement: Notarization via `xcrun notarytool` (automated in CI/CD)
- **Decision Point**: Implement code signing when adoption reaches 500+ users or enterprise requests

**Risk Mitigation Strategies:**

**Community Trust Building:**
- **Early Adopters**: First 50 users (network admin community) accept warnings, create social proof
- **GitHub Transparency**: Full source code available, reproducible builds
- **Security Contact**: `SECURITY.md` file with vulnerability disclosure process
- **Checksum Verification**: Provide SHA-256 checksums for all releases, document verification process

**Self-Signed Certificate (Interim Option):**
- Generate self-signed certificate for macOS (free, reduces Gatekeeper friction)
- Does NOT eliminate warning but makes bypass one-click instead of two-step
- Cost: $0, Setup time: 30 minutes
- Recommendation: Implement for macOS in MVP, skip for Windows (no benefit without EV cert)

### Offline Capabilities

**100% Offline Operation:**

**No Internet Required for Core Functionality:**
- Log parsing, indexing, filtering, enrichment: All local processing
- Application launches and functions fully without network connectivity
- Exception: OPNsense API calls (optional, Journey 2 fallback to backup enrichment)

**Network Connectivity Requirements (Optional):**
- **OPNsense API**: User's firewall API (local network or VPN, not cloud)
- **Version Check**: GitHub API for update notifications (once per day, non-blocking)
- **No Other Dependencies**: No cloud services, no remote APIs, no analytics

**Embedded Documentation:**
- **User Guide**: HTML documentation embedded in application bundle
  - Accessible via Help menu → User Guide
  - Searchable, includes screenshots and examples
- **Troubleshooting**: Common issues and solutions embedded
- **API Setup Guide**: Step-by-step OPNsense API configuration instructions
- **Offline Access**: No external links required to understand core functionality

**Privacy-First Design:**
- **No License Validation**: No phone-home, no activation, no registration
- **No Usage Telemetry**: No analytics, no crash reporting, no data collection
- **No User Tracking**: No UUID generation, no session tracking, no fingerprinting
- **Local-Only Data**: All log data, indexes, and exports remain on user's device

**Portable Deployment:**
- **USB Drive Installation**: Copy executable to USB, run on any compatible machine
- **Air-Gapped Networks**: Fully functional on networks without Internet access
- **Compliance Environments**: Suitable for environments with strict data egress controls
- **Network Admin Workflow**: Tool can be carried on USB drive for on-site troubleshooting

**Data Locality:**
- **All Processing Local**: Zero data sent to external servers (except user's OPNsense API)
- **No Cloud Dependencies**: No cloud storage, no SaaS backend, no remote indexing
- **Export Portability**: CSV/JSON exports self-contained with embedded metadata (Journey 4)
- **Backup Enrichment**: JSON enrichment files portable across machines (Journey 2)

### Implementation Considerations

**Tauri-Specific Architecture:**

**Backend (Rust):**
- **Core Engine**: Log parsing, indexing, filtering, search logic
- **API Client**: OPNsense API integration (reqwest crate with TLS)
- **File I/O**: Memory-mapped file access for large log files (memmap2 crate)
- **Concurrency**: Rayon crate for parallel indexing, Tokio for async API calls
- **Serialization**: bincode for index files, serde_json for API/IPC

**Frontend (Web Technologies):**
- **Framework Options**: React, Vue, Svelte (choice impacts bundle size)
  - React: ~130 KB (most ecosystem support)
  - Svelte: ~50 KB (smallest, fastest)
- **UI Components**: Headless UI (Radix, Headless UI) + Tailwind CSS
- **State Management**: Zustand (React), Pinia (Vue), or Svelte stores
- **Virtual Scrolling**:
  - React: `@tanstack/react-virtual` (headless, 10 KB)
  - Vue: `vue-virtual-scroller`
  - Svelte: `svelte-virtual-list`
- **Data Grid**: Handle 100K+ rows with virtual scrolling (only 50-100 DOM nodes rendered)

**IPC (Inter-Process Communication):**

**Tauri Command Pattern:**
```rust
#[tauri::command]
async fn index_log_file(
    path: String,
    progress_callback: tauri::Window,
) -> Result<IndexSummary, String> {
    // Indexing logic with progress updates
    progress_callback.emit("indexing-progress", progress_data)?;
}
```

**Streaming Large Results:**
- **Problem**: Returning 2M search results via single IPC call = memory explosion
- **Solution**: Tauri Event system for streaming
  ```rust
  #[tauri::command]
  fn search_logs(query: Query, window: tauri::Window) -> Result<(), String> {
      let results = execute_search(query);
      for chunk in results.chunks(1000) { // Chunk size: 1000 entries
          window.emit("search-results-chunk", chunk)?;
          // Frontend consumes chunk, backend waits for next request
      }
      window.emit("search-results-complete", ())?;
  }
  ```
- **Backpressure Handling**: Frontend sends "ready-for-next-chunk" event, backend pauses until ready
- **Cancellation**: Frontend sends "cancel-search" event, backend aborts mid-stream

**Chunk Size Optimization:**
- **Small chunks (100 rows)**: Low memory, high IPC overhead (many serialization roundtrips)
- **Large chunks (10,000 rows)**: High memory, low IPC overhead
- **Optimal**: 1,000 rows per chunk (balance between memory and IPC cost)
- **Dynamic Adjustment**: Increase chunk size if frontend keeps up, decrease if lagging

**Cross-Platform Compatibility:**

**Path Handling:**
```rust
use std::path::{Path, PathBuf};

// Cross-platform path construction
let config_dir = tauri::api::path::config_dir()
    .ok_or("Failed to get config directory")?
    .join("opnsense-log-viewer");

// Always use Path methods, never string concatenation
let index_path = config_dir.join("indexes").join("filter.idx");
```

**Line Ending Handling:**
- Windows: CRLF (`\r\n`)
- Linux/macOS: LF (`\n`)
- **Rust std handles automatically**: `BufRead::read_line()` strips both
- Edge case: Mixed line endings in same file (rare but possible) - handle gracefully

**File Locking (Cross-Platform):**
```rust
use fs2::FileExt;

let file = File::open(path)?;
file.lock_shared()?; // Read lock (multiple readers OK)
// OR
file.lock_exclusive()?; // Write lock (exclusive access)
```

**UI Testing (Cross-Platform):**
- **Visual Regression**: Percy, Chromatic for screenshot comparison
- **Platform-Specific Fonts**: Segoe UI (Windows), SF Pro (macOS), Liberation Sans (Linux)
- **Font Fallback**: CSS `font-family: system-ui, -apple-system, sans-serif`
- **Rendering Differences**: WebView2 vs WebKit may render shadows, borders slightly differently
- **Test on Real Hardware**: VM testing insufficient (GPU rendering, font hinting differ)

**Performance Optimization:**

**Native Compilation:**
- Rust compiles to native machine code (no JIT, no interpreter overhead)
- Platform-specific optimizations enabled via `rustc` flags
- Profile-Guided Optimization (PGO) for 10-20% indexing speedup (post-MVP)

**Memory Management:**
- **Rust Ownership**: Prevents memory leaks, double-frees, dangling pointers
- **No Garbage Collection**: Deterministic memory usage, no GC pauses
- **Memory Allocator**: System allocator (default) vs jemalloc (10-15% faster, post-MVP)

**Thread Pool Sizing:**
```rust
use rayon::prelude::*;

let num_threads = num_cpus::get().max(1);
rayon::ThreadPoolBuilder::new()
    .num_threads(num_threads)
    .build_global()?;
```
- Detect CPU cores at runtime (respects Docker/VM limits)
- Leave 1 core free for UI responsiveness (num_cpus - 1 for indexing)
- Hyperthreading: Use logical cores (num_cpus returns logical count)

**WebView Performance:**
- **Virtual Scrolling**: **Non-negotiable** for 100K+ results
  - Without: 100K `<tr>` elements = browser death (10+ seconds render, 500+ MB memory)
  - With: Only 50-100 visible `<tr>` elements in DOM, smooth 60 FPS scrolling
- **Debounced Filtering**: Filter input debounced 300ms to avoid excessive searches
- **Windowed Rendering**: Render results in pages (10K rows loaded, 1K visible)

**Security Considerations:**

**Tauri Sandboxing:**
- WebView runs in separate process from Rust backend
- IPC commands explicitly whitelisted in `tauri.conf.json`
- File system access restricted via Tauri API (no arbitrary file access from WebView)

**Content Security Policy (CSP):**
```html
<meta http-equiv="Content-Security-Policy"
      content="default-src 'self';
               script-src 'self';
               style-src 'self' 'unsafe-inline';
               img-src 'self' data:;
               connect-src 'self'">
```
- No external scripts, no CDN dependencies (all assets bundled)
- `'unsafe-inline'` for styles only (required by some CSS-in-JS libraries)
- No `eval()`, no inline event handlers

**Input Validation:**
```rust
#[tauri::command]
fn search_logs(query: String) -> Result<Vec<LogEntry>, String> {
    // Sanitize input before passing to search engine
    if query.len() > 1000 {
        return Err("Query too long".to_string());
    }
    // Regex validation if query contains regex pattern
    // SQL injection not applicable (no SQL database)
}
```

**File Access Security:**
- Tauri file picker API: User explicitly selects files (no arbitrary path access)
- Index storage: Application-owned directory (user cannot inject malicious paths)
- Export: User selects destination (sandboxed by OS file picker)

### Resource Management & Limits

**Memory Pressure Handling:**

**Backend Memory Monitoring:**
```rust
use sysinfo::{System, SystemExt};

let mut sys = System::new_all();
sys.refresh_memory();
let used_memory_mb = sys.used_memory() / 1024 / 1024;

if used_memory_mb > 500 {
    // Enter streaming-only mode (disable result caching)
    warn!("Memory pressure detected, switching to streaming mode");
}
```

**Graceful Degradation:**
- **Normal Mode (< 500 MB)**: Cache search results in memory for fast filtering
- **Streaming Mode (> 500 MB)**: Stream results directly to UI, no caching
- **Critical Mode (> 800 MB)**: Pause indexing, prompt user to close other applications

**OS Memory Pressure Signals:**
- **Windows**: Respond to `MEMORY_RESOURCE_NOTIFICATION` events
- **Linux**: Monitor `/proc/meminfo` for available memory
- **macOS**: Respond to `DISPATCH_SOURCE_TYPE_MEMORYPRESSURE` events

**Concurrent Operation Limits:**

**Single Index Operation:**
- **Rule**: Only one log file can be indexed at a time
- **Rationale**: Indexing 30 GB file consumes 400-500 MB RAM; concurrent indexing = OOM
- **UI Behavior**: Disable "Index File" button while indexing in progress
- **Queue**: If user selects multiple files, index sequentially with progress indicator

**Multiple Filter Operations:**
- **Rule**: Multiple simultaneous filter/search operations allowed
- **Rationale**: Filtering is lightweight (queries existing index, ~10-50 MB RAM per query)
- **Concurrency Limit**: Maximum 3 concurrent searches (prevent thread exhaustion)

**Index File Locking:**
- **Rule**: `.idx` file locked in shared mode (read-only) during use
- **Rationale**: Prevents external process from modifying index during active query
- **Detection**: If index modified externally (mtime changed), warn user to re-index

### Testing & Quality Assurance

**CI/CD Platform Matrix:**

**GitHub Actions Configuration:**
```yaml
strategy:
  matrix:
    os:
      - windows-latest       # Windows Server 2022 (x64)
      - ubuntu-20.04         # Ubuntu 20.04 LTS (x64)
      - macos-12             # macOS Monterey (Intel x86_64)
      - macos-14             # macOS Sonoma (Apple Silicon ARM64)
    include:
      - os: windows-latest
        artifact: windows-x64.exe
      - os: ubuntu-20.04
        artifact: linux-x64.tar.gz
      - os: macos-12
        artifact: macos-intel.dmg
      - os: macos-14
        artifact: macos-arm64.dmg
```

**Platform-Specific Tests:**

**WebView Rendering Tests:**
- Screenshot comparison across platforms (baseline images per OS)
- Font rendering differences (acceptable variance: ±2px)
- Color rendering (WebView2 vs WebKit color space handling)

**File Path Handling Tests:**
```rust
#[test]
fn test_paths_with_special_chars() {
    let paths = vec![
        "C:\\Users\\John Doe\\logs\\filter.log", // Windows with spaces
        "/home/user/logs/filter (old).log",      // Unix with spaces & parens
        "/home/user/日本語/filter.log",          // Unicode characters
        "\\\\network\\share\\filter.log",        // UNC path (Windows)
    ];
    for path in paths {
        assert!(parse_log_file(path).is_ok());
    }
}
```

**Keychain Integration Tests:**
- **Mock Tests**: Use test-only keychain (don't pollute user's real keychain)
- **Windows**: `CredWriteW`/`CredReadW` with test namespace
- **macOS**: Create temporary keychain for tests
- **Linux**: Mock Secret Service DBus interface

**Memory Limit Tests:**
- **Constraint**: Run tests in Docker container with 4 GB RAM limit
- **Validation**: Indexing 30 GB file stays under 500 MB peak memory
- **GitHub Actions**: Standard runners have 7 GB RAM (insufficient for minimum spec testing)
- **Self-Hosted Runners**: Required for realistic memory limit testing

**Index Compatibility Test Suite:**

**Forward Compatibility:**
```rust
#[test]
fn test_v1_1_reads_v1_0_index() {
    let v1_0_index = load_test_index("test_data/sample_v1.0.idx");
    let result = IndexReader::open(v1_0_index);
    assert!(result.is_ok(), "v1.1 should read v1.0 indexes");
}
```

**Backward Compatibility:**
```rust
#[test]
fn test_v1_0_cannot_read_v1_1_index() {
    let v1_1_index = load_test_index("test_data/sample_v1.1.idx");
    let result = OldIndexReader::open(v1_1_index);
    assert!(result.is_err(), "v1.0 should reject v1.1 indexes gracefully");
    assert_eq!(
        result.unwrap_err(),
        "Index created with newer version (v1.1), please upgrade application"
    );
}
```

**Migration Testing:**
```rust
#[test]
fn test_config_migration_v1_0_to_v1_1() {
    let old_config = load_test_config("test_data/config_v1.0.toml");
    let migrated = migrate_config(old_config, "1.0.0", "1.1.0")?;
    assert_eq!(migrated.app.version, "1.1.0");
    assert!(backup_exists("config.toml.v1.0.0.bak"));
}
```

**Corruption Detection:**
```rust
#[test]
fn test_corrupted_index_detected() {
    let mut index_data = load_test_index("test_data/sample.idx");
    // Flip random bits (simulate corruption)
    index_data[100] ^= 0xFF;
    let result = IndexReader::open_from_bytes(&index_data);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("checksum mismatch"));
}
```

**Stress Testing Scenarios:**

**1. Disk Full During Index:**
```rust
#[test]
fn test_disk_full_during_indexing() {
    // Setup: 95% disk usage, attempt to index 20 GB file
    // Expected: Detect disk space before indexing, show error
    // OR: Detect during indexing, clean up partial .idx.tmp, show error
}
```

**2. Interrupted Index (Process Kill):**
```rust
#[test]
fn test_sigterm_during_indexing() {
    // Simulate SIGTERM/SIGKILL mid-indexing
    // Expected: On next launch, detect orphaned .idx.tmp, clean up automatically
}
```

**3. Corrupted Log File:**
```rust
#[test]
fn test_invalid_utf8_in_log_file() {
    let log_data = b"Valid line 1\nValid line 2\n\xFF\xFE invalid UTF-8\nValid line 3\n";
    let result = parse_log_file(log_data);
    // Expected: Parse valid lines, skip/warn on invalid UTF-8, continue parsing
    assert_eq!(result.parsed_lines, 3);
    assert_eq!(result.skipped_lines, 1);
}
```

**4. Multiple Instances Same File:**
```rust
#[test]
fn test_concurrent_access_same_log_file() {
    // Instance 1: Opens filter.log for indexing
    // Instance 2: Attempts to open same filter.log for indexing
    // Expected: Instance 2 detects file lock, shows "File in use" error
}
```

**5. OS Hibernate/Resume:**
```rust
#[test]
fn test_resume_after_hibernate() {
    // Start indexing, simulate hibernate (pause all threads)
    // Resume after delay
    // Expected: Detect time jump, warn user "Indexing interrupted", offer restart
}
```

**Performance Regression Testing:**

**Reference Hardware Specification:**
- **Primary Benchmark**:
  - CPU: Intel i5-10400 (6-core, 2.9 GHz base) OR AMD Ryzen 5 3600 (6-core, 3.6 GHz base)
  - RAM: 8 GB DDR4
  - Disk: SATA SSD (500 MB/s read)
- **Minimum Spec**:
  - CPU: Intel i5-8250U (dual-core, 1.6 GHz base) OR AMD Ryzen 5 2500U
  - RAM: 4 GB DDR4
  - Disk: SATA SSD (500 MB/s read)

**Performance Gates (CI/CD):**
```yaml
# GitHub Actions self-hosted runner required
- name: Performance Test
  run: |
    cargo build --release
    ./target/release/opnsense-log-viewer benchmark \
      --test-file test_data/30gb_sample.log \
      --max-index-time-per-gb 7s \
      --max-search-latency 750ms \
      --max-memory-mb 600
  # Fails build if any gate exceeded
```

**Acceptable Variance:**
- Indexing: <7 seconds/GB (±15% tolerance = 8 seconds/GB max)
- Search: <750ms complex query (±50% tolerance = 1125ms max)
- Memory: <600 MB peak (±20% tolerance = 720 MB max)

**Nightly Stress Tests (Self-Hosted Runners):**
- 30 GB file indexing on minimum spec hardware (dual-core, 4 GB RAM)
- 100 consecutive filter operations (detect memory leaks)
- API timeout/retry under simulated network failure (500ms latency, 10% packet loss)

## Scope Definition & Prioritization

### MVP Scope (Version 1.0 - Months 1-4)

**Must-Have (Blockers - Cannot ship without):**

1. **High-Performance Log Indexation**
   - Index 20-30 GB files in 2-3 minutes
   - Hybrid index (inverted + bitmap)
   - Adaptive multi-threading
   - Persistent index storage with checksum validation
   - **Acceptance**: 30 GB file indexed in <3 minutes on reference hardware

2. **Advanced Filtering System**
   - Boolean logic: AND/OR/NOT
   - Operators: equals, contains, startswith, endswith, regex
   - Filter fields: IPs, ports, action, protocol, interface, timestamps
   - Sub-second search response
   - **Acceptance**: Complex 5-filter query returns in <500ms across 30 GB indexed file

3. **OPNsense API Enrichment**
   - Interface mapping (vtnet0 → LAN)
   - Firewall rule labels (hash → human-readable description)
   - Alias resolution (IP groups)
   - Graceful degradation when API unavailable
   - **Acceptance**: Successfully enriches 100% of log entries when API available

4. **Backup Enrichment System** (Journey 2 requirement)
   - JSON export/import of enrichment data
   - Offline investigation capability
   - Staleness indicators for old enrichment data
   - **Acceptance**: User can export enrichment, disconnect from API, and investigate offline with enriched data

5. **Reliable Core Operations**
   - Parse RFC3164, RFC5424, CSV filterlog formats
   - Memory-efficient (<500 MB for 30 GB files)
   - Export to JSON/CSV with metadata
   - Zero crashes on valid log files
   - **Acceptance**: Parse 1M log entries with 0% data loss, 0 crashes

6. **Cross-Platform Desktop Application**
   - Windows x64, Linux x64, macOS Universal
   - Portable executable (no installation)
   - Modern UI with virtual scrolling (100K+ results)
   - **Acceptance**: Runs on all 3 platforms with consistent UX

**Should-Have (Important but deferrable to 1.1):**

- **Saved Filter Library**: Save frequently-used filter combinations
- **Search History**: Recent searches accessible via dropdown
- **Multi-File Indexing**: Index multiple log files sequentially (currently: one at a time)
- **Index Management UI**: View/delete old indexes from within app
- **Keyboard Shortcuts**: Power-user navigation (Cmd/Ctrl+K for search, etc.)

**Could-Have (Nice-to-have, defer to 1.2+):**

- **Basic Timeline Visualization**: Simple bar chart showing traffic over time
- **Top Talkers Report**: Most frequent source/destination IPs
- **Export Templates**: Predefined export formats (compliance reports, security audits)
- **Theme Switcher UI**: Toggle between dark/light themes (currently defaults to dark mode)

**Won't-Have (Explicitly out of MVP scope):**

- ❌ Live log streaming (requires connection to OPNsense, complexity high)
- ❌ Anomaly detection (ML features, defer to Phase 2)
- ❌ Multi-firewall aggregation (defer to Phase 3)
- ❌ SIEM integration (defer to Phase 4)
- ❌ Cloud storage integration (conflicts with zero-infrastructure philosophy)
- ❌ Team collaboration features (defer post-MVP)
- ❌ Automated response actions (security risk, defer indefinitely)

### Post-MVP Roadmap

**Version 1.1 (Months 5-6): Usability Polish**
- Saved filter library
- Search history
- Keyboard shortcuts
- Index management UI
- Performance optimizations (Profile-Guided Optimization)

**Version 1.2 (Months 7-9): Visualization & Reporting**
- Basic timeline charts
- Top talkers report
- Export templates for compliance
- Enhanced UI themes (light mode support)

**Version 2.0 (Months 10-12): Multi-Firewall Support**
- Aggregate logs from multiple OPNsense instances
- Cross-firewall correlation
- Unified enrichment management
- Breaking change: Index format v2 for multi-source support

**Version 3.0 (Year 2): Enterprise Features**
- Team collaboration (shared queries, annotations)
- RBAC and audit logging
- Advanced anomaly detection (ML-based)
- SIEM export capabilities

### MVP Success Gates

**Technical Gates (All must pass):**
- ✅ Indexing performance: <7 seconds/GB on reference hardware
- ✅ Search performance: <750ms for complex queries
- ✅ Memory usage: <600 MB peak during indexing
- ✅ Crash rate: <0.1% across 1000 test sessions
- ✅ Parse accuracy: >99.9% of log entries parsed correctly

**User Validation Gates (At least 3 of 5 must pass):**
- ✅ First 10 users complete investigations faster than with Python tool
- ✅ Users successfully use API enrichment without configuration issues
- ✅ Zero critical bugs requiring workarounds in core filtering
- ✅ Users report tool as "reliable enough for production"
- ✅ 50+ downloads in first month indicating community interest

**Go/No-Go Decision Point:**
After 3-4 months of MVP development, evaluate against success gates. If technical gates fail, revisit architecture before adding features. If user validation fails, gather feedback and iterate on UX/features before proceeding to v1.1.

## Functional Requirements

### FR-001: Log File Management

**FR-001.1: File Selection**
- User can select single log file via native OS file picker dialog
- Supported formats: `.log`, `.txt`, `.csv` (filterlog format)
- File size limit: No hard limit (tested up to 30 GB)
- File location: Any user-accessible path (local, network share, USB drive)
- **Validation**: Display error if file >50 GB with warning "Large file may take extended time to index"

**FR-001.2: File Indexation**
- System creates persistent index (`.idx` file) in application data directory
- Index includes: inverted index (IPs, ports), bitmap index (action, protocol, interface), entry offset table
- Progress indicator shows: % completion, GB processed, estimated time remaining
- User can cancel indexation mid-process (cleanup orphaned `.idx.tmp` files)
- **Validation**: Index file size approximately 10% of source log file size

**FR-001.3: Index Persistence & Reuse**
- System stores index with SHA-256 hash of source file
- On re-opening same file, system checks hash and reuses existing index if match
- If source file modified (hash mismatch), system prompts: "Source file changed since last index. Re-index? (Yes/No)"
- User can view list of existing indexes with file names, creation dates, sizes
- User can delete old indexes to free disk space
- **Validation**: Re-opening indexed file loads in <2 seconds (no re-indexing)

### FR-002: Log Parsing & Display

**FR-002.1: Format Detection**
- System auto-detects log format from first 100 lines:
  - RFC3164: `<PRI>TIMESTAMP HOSTNAME PROCESS[PID]: MESSAGE`
  - RFC5424: `<PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID STRUCTURED-DATA MSG`
  - CSV filterlog: OPNsense-specific CSV format with comma-separated fields
- If format ambiguous or unrecognized, prompt user: "Select log format: [RFC3164] [RFC5424] [CSV filterlog]"
- **Validation**: Correctly detect format for 95% of real-world OPNsense logs

**FR-002.2: Log Entry Display**
- Display log entries in table format with columns: Timestamp, Interface, Source IP, Source Port, Destination IP, Destination Port, Protocol, Action, Rule Label
- Virtual scrolling: Render only visible rows (50-100 DOM elements) for performance
- Table sortable by: Timestamp (default descending), Source IP, Destination IP, Port
- Row selection: Single-click selects row, displays full entry details in bottom pane
- **Validation**: Display 100K entries with smooth 60 FPS scrolling

**FR-002.3: Entry Detail View**
- Bottom pane shows full raw log line (unparsed)
- Parsed fields displayed in key-value format
- Copy buttons for individual fields (copy IP, copy port, etc.)
- "Copy Full Entry" button copies entire raw log line to clipboard
- **Validation**: All parsed fields match raw log data (100% accuracy)

### FR-003: Filtering & Search

**FR-003.1: Filter Builder UI**
- Left sidebar collapsible filter panel
- Add filter button opens modal: "Select Field → Select Operator → Enter Value"
- Filter fields: Timestamp Range, Source IP, Destination IP, Source Port, Destination Port, Protocol, Action, Interface, Rule Label
- Filter operators:
  - Text fields: equals, contains, startswith, endswith, regex
  - Numeric fields: equals, greater than, less than, between
  - Select fields (Protocol, Action): equals, not equals
  - Timestamp: absolute range, relative (last 1h, 24h, 7d, 30d)
- Multiple filters combined with: AND (default), OR, NOT
- **Validation**: Apply 5 filters simultaneously with <500ms response time

**FR-003.2: Filter Execution**
- Filters execute against index (not raw log file) for performance
- Results update in real-time as filters added/removed
- Result count displayed: "Showing 1,247 of 2,450,000 entries"
- Empty result state: "No entries match current filters. Try adjusting your criteria."
- **Validation**: Complex 7-filter query returns in <1 second

**FR-003.3: Filter Management**
- "Clear All Filters" button resets to full dataset
- "Save Filter" button prompts for name, saves filter set to local storage
- "Load Filter" dropdown shows saved filters, applies on selection
- "Delete Filter" option in filter library
- Search history: Last 10 searches accessible via dropdown (persistence across sessions)
- **Validation**: Save 20 filters, load any filter in <100ms

### FR-004: OPNsense API Enrichment

**FR-004.1: API Connection Setup**
- Settings panel: "OPNsense API Configuration"
- Input fields: Endpoint URL (https://192.168.1.1), API Key, API Secret
- "Test Connection" button validates credentials and shows OPNsense version
- Credentials stored in OS keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- Connection status indicator: Green = Connected, Yellow = Degraded, Red = Disconnected
- **Validation**: Successfully connect to OPNsense 22.x, 23.x, 24.x versions

**FR-004.2: Interface Mapping**
- System calls `/api/diagnostics/interface/getInterfaceNames` on connection
- Maps physical interfaces (vtnet0, em0) to logical names (LAN, WAN, DMZ)
- Display logical names in Interface column and filter dropdown
- Hover tooltip shows physical interface name: "LAN (vtnet0)"
- **Validation**: Correctly map all interfaces for firewalls with 2-10 interfaces

**FR-004.3: Rule Label Enrichment**
- System calls `/api/firewall/filter/searchRule` for each unique rule hash
- Replace rule hashes with human-readable descriptions: "GeoIP Block - Non-EU", "Port Scan Detection"
- Cache rule labels in memory (avoid redundant API calls for same rule)
- If rule not found via API, display: "Rule [hash] (label unavailable)"
- **Validation**: Enrich 95%+ of rule references successfully

**FR-004.4: Alias Resolution**
- System calls `/api/firewall/alias/searchItem` for aliased IPs
- Display alias names alongside IPs: "192.168.1.100 (Servers_Group)"
- Hover tooltip shows alias members for group aliases
- **Validation**: Resolve aliases for 90%+ of aliased IPs

**FR-004.5: Graceful Degradation**
- If API unavailable (timeout, authentication failure), display banner: "API Offline - Showing raw data without enrichment. Load backup enrichment? [Load Backup]"
- User can continue investigation with raw interface names, rule hashes, IPs
- Backup enrichment option available (see FR-005)
- **Validation**: Application remains fully functional with API disconnected

### FR-005: Backup Enrichment System

**FR-005.1: Enrichment Export**
- "Export Enrichment Data" button in Settings panel
- System generates JSON file with:
  - Interface mappings: `{"vtnet0": "LAN", "vtnet1": "WAN"}`
  - Rule labels: `{"rule_hash_abc": "GeoIP Block - Non-EU"}`
  - Alias definitions: `{"Servers_Group": ["192.168.1.100", "192.168.1.101"]}`
  - Metadata: OPNsense version, export timestamp, configuration hash
- Default filename: `enrichment_<hostname>_<timestamp>.json`
- **Validation**: Export file size <5 MB for typical firewall configs

**FR-005.2: Enrichment Import**
- "Load Backup Enrichment" button opens file picker for JSON file
- System validates JSON schema before loading
- Display warning: "Enrichment data from 2026-01-10 (6 days old). Verify accuracy for critical investigations."
- Apply enrichment to current log view (replace raw data with enriched values)
- Enrichment persists until API reconnected or new backup loaded
- **Validation**: Successfully load enrichment files from 1-90 days old

**FR-005.3: Staleness Indicators**
- Visual indicator in top bar: "⚠️ Using backup enrichment (6 days old)" with yellow background
- Hover tooltip: "Interface mappings and rule labels may be outdated. Reconnect to API for current data."
- Option to ignore warning: "Don't show again for this session"
- **Validation**: Clear visual distinction between live API and backup enrichment

### FR-006: Export & Reporting

**FR-006.1: Filtered Result Export**
- "Export" button opens modal: "Select Format: [CSV] [JSON]"
- CSV format: Headers + comma-separated values, compatible with Excel/LibreOffice
- JSON format: Array of objects with all parsed fields
- Export includes metadata header:
  ```
  # Exported by: opnsense-log-viewer v1.0.0
  # Export Date: 2026-01-15T14:32:01Z
  # Source File: /path/to/filter.log (SHA-256: abc123...)
  # Filters Applied: action=block, interface=WAN, timestamp=2026-01-10 to 2026-01-15
  # Total Entries: 1,247 of 2,450,000
  ```
- Progress indicator for large exports (>10K rows)
- **Validation**: Export 100K entries to CSV in <10 seconds

**FR-006.2: Full Dataset Export**
- "Export All (Unfiltered)" option exports entire indexed dataset
- Warning for large exports: "Export 2.5M entries? This may take several minutes. Continue? [Yes] [Cancel]"
- Stream export to disk (don't load entire dataset into memory)
- **Validation**: Export 2M entries without exceeding 600 MB memory usage

**FR-006.3: Export Integrity**
- Include SHA-256 checksum of export file in metadata
- Row count verification: Display "Export complete: 1,247 entries written"
- Open export location: Button to open folder containing exported file
- **Validation**: 100% of exported rows match source data (no corruption)

## Non-Functional Requirements

### NFR-001: Performance Requirements

**NFR-001.1: Indexing Performance**
- **Requirement**: Index log files at ≥6 GB/min (≤10 seconds/GB) on reference hardware
- **Reference Hardware**: Intel i5-10400 or AMD Ryzen 5 3600, 8 GB RAM, SATA SSD
- **Measurement**: Average indexing speed across 10 test runs with 30 GB file
- **Acceptance**: Mean ≤7 seconds/GB, 95th percentile ≤8 seconds/GB (15% tolerance)
- **Degradation**: On minimum spec hardware (dual-core, 4 GB RAM), accept 10-15 seconds/GB

**NFR-001.2: Search & Filter Performance**
- **Requirement**: Return filtered results in <500ms for simple queries, <750ms for complex queries
- **Simple Query**: Single filter (e.g., action=block)
- **Complex Query**: 5+ filters with AND/OR/NOT logic, regex patterns, timestamp ranges
- **Measurement**: P95 latency across 1000 query executions
- **Acceptance**: P95 ≤ 750ms for complex queries
- **Scalability**: Performance maintained up to 30 GB indexed files (10M+ entries)

**NFR-001.3: UI Responsiveness**
- **Requirement**: Maintain 60 FPS (16ms frame time) during scrolling, filtering, UI interactions
- **Virtual Scrolling**: Render only 50-100 visible rows in DOM regardless of total result size
- **Input Debouncing**: Filter inputs debounced 300ms to avoid excessive re-renders
- **Measurement**: Chrome DevTools Performance profiling, target <16ms for UI operations
- **Acceptance**: Smooth scrolling through 100K+ results with no frame drops

**NFR-001.4: Memory Efficiency**
- **Requirement**: Peak memory usage ≤500 MB during indexing, ≤300 MB during search operations
- **Measurement**: OS memory monitoring (Task Manager, Activity Monitor, `top`)
- **Acceptance**: 95th percentile memory usage within limits
- **Graceful Degradation**: If memory exceeds 500 MB, enter streaming mode (no result caching)

**NFR-001.5: Startup Time**
- **Requirement**: Application cold start ≤3 seconds, hot start ≤1 second
- **Cold Start**: First launch after system boot (no caching)
- **Hot Start**: Subsequent launches (OS caches binary)
- **Measurement**: Time from process spawn to main window visible
- **Acceptance**: P95 cold start ≤4 seconds, hot start ≤1.5 seconds

**NFR-001.6: Index Load Time**
- **Requirement**: Load existing index in ≤2 seconds regardless of source file size
- **Measurement**: Time from "Open File" to results displayed
- **Acceptance**: 95% of index loads complete in ≤2 seconds for files up to 30 GB
- **Rationale**: Enables rapid re-investigation of previously indexed logs

### NFR-002: Reliability Requirements

**NFR-002.1: Crash Rate**
- **Requirement**: Application crash rate <0.1% per session
- **Session Definition**: From application launch to graceful exit
- **Measurement**: Crash telemetry (local logging, no cloud reporting)
- **Acceptance**: <1 crash per 1000 sessions across all platforms
- **Zero Crash Scenarios**: Valid log files, indexed files, configuration files must never cause crashes

**NFR-002.2: Data Integrity**
- **Requirement**: 100% parse accuracy for well-formed log entries, graceful handling of malformed entries
- **Well-Formed**: Log entries matching RFC3164, RFC5424, CSV filterlog specifications
- **Malformed Handling**: Log skipped line, continue parsing, report skipped count to user
- **Measurement**: Cross-validation against Python tool on golden log file test suite
- **Acceptance**: <0.1% discrepancy in parsed field values, 0% data loss for well-formed entries

**NFR-002.3: Index Corruption Recovery**
- **Requirement**: Detect corrupted indexes on load, prompt re-indexing, never crash on corruption
- **Corruption Detection**: Checksum validation in index header
- **User Action**: Display error "Index file corrupted. Re-index to continue? [Yes] [No]"
- **Acceptance**: 100% of corrupted indexes detected before use

**NFR-002.4: Idempotency**
- **Requirement**: Re-indexing same log file produces bit-identical index (excluding timestamps)
- **Rationale**: Enables index comparison, reproducibility for troubleshooting
- **Measurement**: SHA-256 hash of index file (excluding timestamp metadata section)
- **Acceptance**: 100% hash match across 10 re-index operations

**NFR-002.5: Graceful Degradation**
- **Requirement**: Application remains functional when API unavailable, enrichment missing, network disconnected
- **Degraded Mode Behaviors**:
  - API unavailable → Show raw data, offer backup enrichment option
  - Disk full → Prevent indexing, show error with space requirements
  - Low memory → Switch to streaming mode, disable result caching
- **Acceptance**: Zero crashes in degraded modes, all core functionality available

### NFR-003: Security Requirements

**NFR-003.1: Credential Protection**
- **Requirement**: API credentials encrypted at rest, never logged in plaintext
- **Primary Storage**: OS keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- **Fallback Storage**: AES-256-GCM encrypted file with Argon2id key derivation
- **Acceptance**: Manual inspection of log files, config files shows no plaintext credentials

**NFR-003.2: Local Data Processing**
- **Requirement**: 100% of log data processing occurs locally, zero data transmission to external servers
- **Exceptions**: OPNsense API calls (user's firewall, not cloud), optional version check (GitHub API)
- **Measurement**: Network traffic monitoring during indexing and search operations
- **Acceptance**: Zero HTTP requests to non-user-specified endpoints during log processing

**NFR-003.3: Input Validation**
- **Requirement**: All user inputs sanitized before processing (file paths, filter values, API endpoints)
- **Protections**: Path traversal prevention, injection attack prevention (XSS, command injection)
- **Tauri Sandboxing**: WebView isolated from Rust backend, file system access restricted
- **Acceptance**: Penetration testing reveals no input validation vulnerabilities

**NFR-003.4: Secure Defaults**
- **Requirement**: Application ships with secure configuration out-of-box
- **Secure Defaults**:
  - HTTPS enforced for API connections (reject HTTP)
  - TLS certificate validation enabled (reject invalid certs)
  - Content Security Policy (CSP) restricts inline scripts
  - No debug logging in production builds
- **Acceptance**: Security audit finds no insecure default settings

### NFR-004: Usability Requirements

**NFR-004.1: Learnability**
- **Requirement**: First-time users complete basic investigation (open file, apply filter, export) within 10 minutes without documentation
- **Measurement**: User testing with 5 network admins unfamiliar with tool
- **Acceptance**: 4 of 5 users complete task successfully within time limit
- **Support**: Embedded tooltips, clear UI labels, helpful error messages

**NFR-004.2: Efficiency**
- **Requirement**: Expert users complete routine investigations 3x faster than with Python tool
- **Routine Investigation**: Open 15 GB file, apply 3 filters, export 500 results
- **Measurement**: Time from application launch to export completion
- **Baseline**: Python tool average 8-12 minutes (with crashes/retries)
- **Target**: Tauri tool average 3-4 minutes
- **Acceptance**: Mean time <5 minutes across 10 test investigations

**NFR-004.3: Error Messages**
- **Requirement**: Error messages provide actionable guidance, specific failure reasons, recovery steps
- **Anti-Pattern**: "Failed to load file" (vague, no action)
- **Good Pattern**: "Failed to load filter.log: File contains invalid UTF-8 at byte offset 45,231. Lines with invalid characters will be skipped. Continue? [Yes] [No]"
- **Acceptance**: User testing shows 80%+ of users recover from errors without external help

**NFR-004.4: Accessibility**
- **Requirement**: Keyboard navigation for all core features, screen reader compatibility
- **Keyboard Shortcuts**: Tab navigation, Enter to select, Esc to cancel, Cmd/Ctrl+F for filter
- **Screen Reader**: Semantic HTML, ARIA labels, alt text for icons
- **Measurement**: Manual testing with NVDA (Windows), VoiceOver (macOS)
- **Acceptance**: All core workflows completable via keyboard + screen reader

**NFR-004.5: Responsive Design**
- **Requirement**: UI adapts to screen sizes from 1280x720 (13" laptop) to 2560x1440 (27" desktop)
- **Responsive Behaviors**:
  - Collapsible sidebar on small screens
  - Table columns prioritized (hide less important columns on narrow screens)
  - Font sizes scaled appropriately
- **Acceptance**: Manual testing on 3 screen sizes shows all features accessible

### NFR-005: Maintainability Requirements

**NFR-005.1: Code Quality**
- **Requirement**: Rust code passes `clippy` linter with zero warnings, 80%+ test coverage
- **Linting**: Run `cargo clippy -- -D warnings` in CI/CD
- **Testing**: Unit tests (Rust), integration tests (end-to-end scenarios)
- **Coverage Measurement**: `cargo tarpaulin` or `grcov`
- **Acceptance**: CI/CD fails on clippy warnings or coverage drop below 75%

**NFR-005.2: Documentation**
- **Requirement**: All public APIs documented with rustdoc, complex algorithms explained with comments
- **Documentation Standards**: Rustdoc for public functions, inline comments for non-obvious logic
- **User Documentation**: Embedded user guide (HTML), README with quick start
- **Acceptance**: `cargo doc` generates complete API documentation without warnings

**NFR-005.3: Logging & Diagnostics**
- **Requirement**: Application logs errors, warnings, performance metrics to local file
- **Log Location**:
  - Windows: `%APPDATA%\opnsense-log-viewer\logs\app.log`
  - macOS: `~/Library/Application Support/opnsense-log-viewer/logs/app.log`
  - Linux: `~/.local/share/opnsense-log-viewer/logs/app.log`
- **Log Rotation**: Max 10 MB per log file, keep last 5 files
- **Privacy**: Never log API credentials, user's log data, or PII
- **Acceptance**: Error scenarios produce actionable log entries for debugging

### NFR-006: Portability Requirements

**NFR-006.1: Cross-Platform Consistency**
- **Requirement**: Identical feature set and UX across Windows, Linux, macOS
- **No Platform-Specific Features**: All functionality available on all platforms
- **Visual Consistency**: UI looks native on each platform (system fonts, window chrome)
- **Acceptance**: Side-by-side comparison shows no functional differences

**NFR-006.2: Platform Compatibility**
- **Requirement**: Support OS versions from last 5 years
- **Windows**: Windows 10 version 1809 (October 2018) and later
- **Linux**: Ubuntu 20.04 (April 2020) and later, equivalent RHEL/Arch releases
- **macOS**: macOS 10.15 Catalina (October 2019) and later
- **Acceptance**: Application runs successfully on oldest supported OS version

**NFR-006.3: Portable Deployment**
- **Requirement**: Single executable runs without installation, dependencies, or admin rights
- **Exceptions**: OS-provided dependencies (WebView2 on Windows, WebKitGTK on Linux)
- **USB Drive Deployment**: Copy executable to USB, run on any compatible machine
- **Acceptance**: Application runs from USB drive without prior installation

### NFR-007: Scalability Requirements

**NFR-007.1: File Size Limits**
- **Requirement**: Support log files up to 30 GB without degradation
- **Tested Sizes**: 100 MB, 1 GB, 10 GB, 20 GB, 30 GB
- **Degradation Threshold**: Performance targets hold up to 30 GB; 30-50 GB acceptable with reduced performance
- **Hard Limit**: Reject files >50 GB with error: "File too large. Split into smaller files or contact support for enterprise version."
- **Acceptance**: All performance targets met up to 30 GB files

**NFR-007.2: Result Set Size**
- **Requirement**: Display up to 10M filtered results without UI freezing
- **Virtual Scrolling**: Render only visible rows (50-100 in DOM)
- **Chunked Loading**: Load results in 10K chunks, paginate if needed
- **Acceptance**: Smooth scrolling through 1M+ results with <16ms frame time

**NFR-007.3: Concurrent Sessions**
- **Requirement**: Support 1 active indexing operation + unlimited search operations
- **Rationale**: Indexing is memory-intensive (500 MB), searches are lightweight (50 MB each)
- **Limit**: Disable "Index" button while indexing in progress
- **Acceptance**: 3 concurrent search operations execute without memory overflow

## Implementation Notes & Technical Constraints

### Technical Stack Summary

**Backend (Rust):**
- **Parser**: Custom parsers for RFC3164, RFC5424, CSV using `nom` or regex
- **Indexing**: Custom inverted index (HashMap<IP, Vec<EntryID>>), `roaring` crate for bitmaps
- **Concurrency**: `rayon` for parallel indexing, `tokio` for async API calls
- **Serialization**: `bincode` for index files, `serde_json` for API/enrichment
- **File I/O**: `memmap2` for memory-mapped file reading (large files)
- **HTTP Client**: `reqwest` with TLS for OPNsense API calls
- **Cryptography**: `ring` or `rust-crypto` for AES-256-GCM, `argon2` for key derivation

**Frontend (Web Technologies):**
- **Framework**: React (recommended for ecosystem) or Svelte (smallest bundle)
- **UI Library**: Tailwind CSS + Headless UI (Radix UI for React, Bits UI for Svelte)
- **Virtual Scrolling**: `@tanstack/react-virtual` (React) or `svelte-virtual-list` (Svelte)
- **State Management**: Zustand (React) or Svelte stores
- **Build Tool**: Vite (fast builds, HMR)

**Desktop Framework:**
- **Tauri**: v2.x (latest stable)
- **System WebView**: WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux)

### Edge Cases & Error Handling

**Edge Case 1: Empty Log File**
- **Scenario**: User opens 0-byte log file or file with only whitespace
- **Behavior**: Display message "Log file is empty. No entries to index."
- **No Error**: Don't treat as error, gracefully handle

**Edge Case 2: Mixed Log Formats**
- **Scenario**: Log file contains lines in different formats (RFC3164 + CSV)
- **Behavior**: Parse each line independently, skip unrecognized formats, report "Skipped 142 lines (unrecognized format)"
- **Auto-Detection**: Use majority format for auto-detection (e.g., if 95% RFC3164, treat as RFC3164)

**Edge Case 3: Invalid UTF-8**
- **Scenario**: Log file contains non-UTF-8 bytes (binary data, corrupted file)
- **Behavior**: Skip invalid lines, use lossy UTF-8 conversion, log warning
- **User Notification**: "File contains invalid UTF-8. 37 lines skipped. Continue? [Yes] [No]"

**Edge Case 4: Very Long Lines**
- **Scenario**: Single log line exceeds 10 KB (e.g., base64-encoded data in message field)
- **Behavior**: Truncate line to 10 KB for display, keep full line in index for search
- **Performance**: Prevent DOM slowdown from rendering multi-KB strings

**Edge Case 5: Clock Skew / Invalid Timestamps**
- **Scenario**: Log contains timestamps in future (2030-01-01) or distant past (1970-01-01)
- **Behavior**: Accept timestamps as-is, allow filtering but warn if >90% of entries have suspicious timestamps
- **Warning**: "Detected unusual timestamps. Check firewall clock synchronization."

**Edge Case 6: OPNsense API Version Mismatch**
- **Scenario**: Tool developed against OPNsense 24.x, user runs 21.x (older API)
- **Behavior**: Detect version via `/api/core/firmware/status`, adapt API calls or disable unsupported features
- **Graceful Fallback**: If API endpoint not found (404), log error and continue without that enrichment

**Edge Case 7: Network Share Disconnect**
- **Scenario**: User opens log file from network share (SMB/NFS), network disconnects mid-read
- **Behavior**: Detect I/O error, display "Network connection lost. Retry? [Retry] [Cancel]"
- **Partial Index**: If index partially built, offer to save partial index or discard

**Edge Case 8: Disk Full During Index**
- **Scenario**: Indexing 25 GB file, disk fills up at 80% completion
- **Behavior**: Detect disk full (I/O error), abort indexing, clean up `.idx.tmp`, show error "Insufficient disk space. 7.5 GB required for index."
- **Pre-Check**: Before indexing, check available disk space and warn if insufficient

### Development Guidelines

**Code Style:**
- Rust: Follow Rust style guide, use `rustfmt` for formatting
- Frontend: Follow Airbnb style guide (React) or Svelte official guide
- Naming: `snake_case` for Rust, `camelCase` for JavaScript/TypeScript

**Error Handling Pattern:**
- Rust: Use `Result<T, E>` everywhere, never `unwrap()` in production code
- Display errors to user with context: `map_err(|e| format!("Failed to open file: {}", e))`
- Log errors to application log file for debugging

**Testing Strategy:**
- Unit tests: Test individual functions (parsers, filters, index operations)
- Integration tests: Test end-to-end workflows (index file → search → export)
- Platform tests: Run full test suite on Windows, Linux, macOS in CI/CD

**Performance Optimization:**
- Profile before optimizing: Use `cargo flamegraph` to identify bottlenecks
- Optimize hot paths: Indexing loop, search query execution
- Avoid premature optimization: Readable code > micro-optimizations

### Known Limitations

**Limitation 1: Single-Threaded Frontend**
- **Issue**: JavaScript in WebView runs on single thread (no Web Workers in Tauri yet)
- **Impact**: Heavy JSON parsing (large API responses) can block UI thread
- **Mitigation**: Stream data in small chunks, process in Rust backend when possible

**Limitation 2: No Real-Time Log Streaming**
- **Reason**: Requires persistent connection to OPNsense (SSH or API polling), adds complexity
- **Workaround**: User can re-index file periodically to see new entries
- **Future**: Consider in v2.0 as "Live Mode" feature

**Limitation 3: Limited Regex Engine**
- **Issue**: Rust `regex` crate doesn't support lookbehind/lookahead
- **Impact**: Some advanced regex queries may not work
- **Mitigation**: Document regex limitations, provide examples of supported patterns

**Limitation 4: macOS Gatekeeper Friction**
- **Issue**: Unsigned app requires user to bypass Gatekeeper warning
- **Impact**: 20-30% user drop-off during first launch
- **Mitigation**: Provide clear instructions, consider code signing in v1.1

## Acceptance Criteria & Validation

### Feature: Log File Indexation

**AC-INDEX-001: File Selection**
- [ ] User can open file picker via "Open File" button in toolbar
- [ ] File picker filters to `.log`, `.txt`, `.csv` extensions by default
- [ ] File picker shows "All Files (*.*)" option for non-standard extensions
- [ ] Selected file path displays in window title bar
- [ ] If file >30 GB, warning message displayed before indexing begins

**AC-INDEX-002: Indexing Progress**
- [ ] Progress bar shows % completion (0-100%)
- [ ] Progress bar shows GB processed / Total GB
- [ ] Estimated time remaining displayed and updates every second
- [ ] "Cancel" button available during indexing
- [ ] If cancelled, cleanup `.idx.tmp` files automatically

**AC-INDEX-003: Indexing Performance**
- [ ] 30 GB file indexes in <3.5 minutes on reference hardware (Intel i5-10400, 8 GB RAM, SSD)
- [ ] Memory usage stays below 500 MB during indexing
- [ ] CPU usage scales with available cores (8-core machine uses 80%+ CPU)
- [ ] Progress updates smoothly (no UI freezing)

**AC-INDEX-004: Index Persistence**
- [ ] Index file created in `<AppData>/indexes/` directory
- [ ] Index filename format: `<source_filename>_<hash>.idx`
- [ ] Re-opening same file detects existing index and skips indexing
- [ ] Load time for existing index <2 seconds for 30 GB source file

### Feature: Filtering & Search

**AC-FILTER-001: Filter UI**
- [ ] "Add Filter" button opens filter modal
- [ ] Filter modal shows: Field dropdown, Operator dropdown, Value input
- [ ] Field dropdown includes all parseable fields (IP, Port, Protocol, Action, Interface, Timestamp)
- [ ] Operator dropdown shows relevant operators per field type (text: equals/contains/regex, numeric: equals/gt/lt)
- [ ] "Add" button adds filter to active filter list
- [ ] "Cancel" button closes modal without adding filter

**AC-FILTER-002: Filter Execution**
- [ ] Single filter applies in <500ms
- [ ] 5 filters (AND logic) apply in <750ms
- [ ] Result count updates: "Showing 1,247 of 2,450,000 entries"
- [ ] Empty result state shows helpful message: "No matches found. Try adjusting filters."
- [ ] Filters persist when switching between result view and detail view

**AC-FILTER-003: Filter Management**
- [ ] "Clear All" button removes all filters and shows full dataset
- [ ] Each filter chip shows "X" button to remove individual filter
- [ ] "Save Filter" prompts for name, saves to local storage
- [ ] "Load Filter" dropdown shows all saved filters
- [ ] Selecting saved filter applies it immediately

### Feature: OPNsense API Enrichment

**AC-API-001: Connection Setup**
- [ ] Settings panel accessible via "Settings" button in toolbar
- [ ] API section shows: Endpoint URL, API Key, API Secret input fields
- [ ] "Test Connection" button validates credentials
- [ ] Success: Green checkmark + "Connected to OPNsense 24.1.1"
- [ ] Failure: Red X + error message "Authentication failed. Check API credentials."

**AC-API-002: Enrichment Application**
- [ ] Interface column shows logical names (LAN, WAN) instead of physical (vtnet0, vtnet1)
- [ ] Hover tooltip on interface shows physical name
- [ ] Rule Label column shows human-readable descriptions instead of hashes
- [ ] If rule label unavailable, shows "Rule [hash] (label unavailable)"
- [ ] Enrichment applies to all log entries within 5 seconds of API connection

**AC-API-003: Offline Fallback**
- [ ] If API unavailable, banner displays: "API Offline - Load backup enrichment? [Load Backup]"
- [ ] Clicking "Load Backup" opens file picker for JSON enrichment file
- [ ] Loading backup enrichment applies to current view
- [ ] Staleness warning shows if enrichment >7 days old
- [ ] Raw data displayed if no backup enrichment available

### Feature: Export

**AC-EXPORT-001: CSV Export**
- [ ] "Export" button opens format selection modal
- [ ] Selecting "CSV" prompts for save location
- [ ] CSV file includes header row with column names
- [ ] CSV file includes metadata comment block (tool version, export date, filters applied)
- [ ] CSV file contains all filtered results (not full dataset if filters active)
- [ ] Export completes with success message: "Exported 1,247 entries to C:\Users\...\export.csv"

**AC-EXPORT-002: JSON Export**
- [ ] Selecting "JSON" prompts for save location
- [ ] JSON structure: `{"metadata": {...}, "entries": [...]}`
- [ ] Each entry includes all parsed fields as key-value pairs
- [ ] Export completes with success message showing row count and file path

**AC-EXPORT-003: Large Export Performance**
- [ ] Exporting 100K entries completes in <10 seconds
- [ ] Progress indicator shows during export
- [ ] Memory usage stays below 600 MB during export
- [ ] User can cancel export mid-process

### Feature: UI/UX

**AC-UX-001: First-Time Experience**
- [ ] On first launch, welcome screen explains basic workflow: Open → Index → Filter → Export
- [ ] "Don't show again" checkbox available
- [ ] Example log file link provided for testing (downloads 10 MB sample)

**AC-UX-002: Virtual Scrolling**
- [ ] Table renders smoothly with 100K+ results
- [ ] Scrolling maintains 60 FPS (no frame drops)
- [ ] Only 50-100 rows rendered in DOM at any time
- [ ] Scroll position persists when applying filters

**AC-UX-003: Responsive Design**
- [ ] Sidebar collapsible on screens <1440px wide
- [ ] Table columns prioritized: Timestamp, Source IP, Dest IP, Action (always visible)
- [ ] Less important columns (Port, Protocol) hidden on narrow screens
- [ ] Horizontal scroll available if needed

**AC-UX-004: Error Messages**
- [ ] All error messages include specific failure reason
- [ ] All error messages suggest recovery action
- [ ] Example: "Failed to parse line 45,231: Invalid UTF-8. Skip malformed lines? [Skip] [Abort]"
- [ ] No generic errors like "Operation failed" without context

### Feature: Cross-Platform Compatibility

**AC-PLATFORM-001: Windows**
- [ ] Application runs on Windows 10 version 1809+
- [ ] WebView2 runtime detection on launch, error message if missing
- [ ] Credential storage via Windows Credential Manager functional
- [ ] File paths with spaces, backslashes handled correctly

**AC-PLATFORM-002: Linux**
- [ ] Application runs on Ubuntu 20.04, Debian 11, Fedora 34+
- [ ] WebKitGTK dependency check on launch, error message if missing
- [ ] Credential storage via libsecret functional (fallback to encrypted file if unavailable)
- [ ] File paths with spaces, forward slashes handled correctly

**AC-PLATFORM-003: macOS**
- [ ] Universal binary runs on Intel (x86_64) and Apple Silicon (ARM64)
- [ ] Application runs on macOS 10.15 Catalina+
- [ ] Credential storage via Keychain Services functional
- [ ] Gatekeeper bypass instructions in README

---

**END OF PRODUCT REQUIREMENTS DOCUMENT**

**Document Status**: Complete - All 11 steps finalized
**Ready for**: Architecture Design Phase
**Next Step**: Create architecture document using completed PRD, Product Brief, and UX Design Specification as inputs

