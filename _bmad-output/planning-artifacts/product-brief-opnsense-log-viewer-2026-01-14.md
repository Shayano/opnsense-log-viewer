---
stepsCompleted: [1, 2, 3, 4, 5]
inputDocuments:
  - "_bmad-output/analysis/brainstorming-session-2026-01-14.md"
  - "docs/DEVELOPER_GUIDE.md"
  - "README.md"
  - "docs/Introduction_API.html"
  - "docs/API_Reference.html"
  - "docs/Firewall_API.html"
  - "src/opnsense_log_viewer/services/virtual_log_manager.py"
  - "src/opnsense_log_viewer/services/log_parser.py"
  - "src/opnsense_log_viewer/services/log_filter.py"
  - "src/opnsense_log_viewer/services/ssh_client.py"
  - "src/opnsense_log_viewer/utils/file_utils.py"
date: 2026-01-14
author: Shay
---

# Product Brief: opnsense-log-viewer

## Executive Summary

OPNsense Log Viewer is a high-performance desktop application designed to eliminate the frustration network administrators face when analyzing large firewall log files. The current Python-based solution crashes when filtering files larger than 2-3 GB, forcing administrators to restart investigations from scratch and wasting critical time during problem diagnosis. This redesigned solution, built on Tauri + Rust architecture, targets support for 20-30 GB log files with instant search capabilities through intelligent indexing strategies, transforming what is currently a 30+ minute crash-prone process into a 2-3 minute indexation followed by sub-second search responses.

---

## Core Vision

### Problem Statement

Network administrators need to investigate communication issues and security incidents by analyzing historical OPNsense firewall logs. Current solutions fail catastrophically when dealing with production-scale log files (>2-3 GB), causing application crashes that force administrators to restart their analysis from zero. The root cause is architectural: the Python application loads entire files into memory during filtering operations (`f.readlines()` bottleneck), making it impossible to handle the 20-30 GB files commonly encountered in production environments.

### Problem Impact

**Time Loss:** Investigations that should take minutes stretch into hours due to repeated crashes and restarts. Administrators lose all filtering progress when the application fails.

**Delayed Resolution:** When troubleshooting requires analyzing logs from days or weeks prior (the typical use case), administrators cannot access the historical data needed for root cause analysis.

**Workaround Burden:** Administrators are forced to use primitive workarounds like SSH access to OPNsense, manual file splitting, or relying on incomplete data from smaller time windows.

### Why Existing Solutions Fall Short

**Current Python Application:** Fundamentally limited by memory constraints. The VirtualLogManager's chunking system (1000 entries/chunk, LRU cache with 50 chunks) helps with viewing but fails during filtering operations which load the entire file into RAM.

**Generic Log Analyzers:** Tools like Elasticsearch, Splunk, or Loki are over-engineered for this specific use case, requiring complex server infrastructure, ongoing maintenance, and expertise beyond typical network admin skill sets.

**SSH + Command Line:** Direct server access works but lacks the user-friendly filtering, visualization, and interface mapping features administrators need for efficient investigation workflows.

### Proposed Solution

A redesigned multiplatform desktop application (Windows/Linux/macOS) built on Tauri + Rust that eliminates memory constraints through intelligent indexing strategies:

**Inverted Multi-Column Index:** Elasticsearch-inspired indexing for high-cardinality fields (IP addresses, ports) enabling instant filtering on any field combination.

**Bitmap Index:** ClickHouse-inspired compression for low-cardinality fields (action, protocol, interface) dramatically reducing index size while maintaining query performance.

**Adaptive Multi-Threading:** PostgreSQL-inspired parallel query execution that automatically scales based on available system resources.

**Trade-off Philosophy:** Accept 2-3 minutes of upfront indexation time to deliver sub-second search responses across 20-30 GB files - a massive improvement over current 30+ minute processes that ultimately crash.

**API-First Integration:** Replace fragile SSH-based rule label extraction with direct OPNsense API integration for reliable metadata enrichment.

### Key Differentiators

1. **Specialized for OPNsense:** Purpose-built log format parsing (RFC3164, RFC5424, CSV filterlog) with native interface mapping (vtnet0 → LAN) and rule label integration.

2. **Portable Desktop Application:** Single ~15 MB executable requiring no server infrastructure, no installation, no maintenance overhead - just download and run.

3. **Simplicity Over Feature Creep:** Focused on core investigation workflows rather than attempting to be a full observability platform. Does one thing exceptionally well.

4. **Hybrid Index Architecture:** Combines multiple database indexing strategies (inverted index + bitmap index) optimized specifically for firewall log characteristics.

5. **Graceful Performance Scaling:** Adaptive threading and intelligent caching ensure the application performs well on both modest laptops and powerful workstations.

6. **Offline-First Operation:** Works primarily with local log files, with optional API connectivity when needed - no dependency on network availability during critical investigations.

---

## Target Users

### Primary Users

**Network Administrator - SMB Solo Operator**

Marc is a 35-year-old network administrator managing the complete IT infrastructure for a 50-employee manufacturing company. He handles everything from user support to firewall configuration, working largely independently with OPNsense as his primary security gateway. Marc has intermediate technical skills - mostly self-taught with some vendor certifications - and values tools that "just work" without requiring deep expertise or ongoing maintenance.

**Current Pain:** When communication issues arise (VPN connectivity problems, blocked application traffic, suspected security events), Marc needs to dig through days or weeks of firewall logs to understand what happened. The Python application crashes repeatedly when filtering large files, forcing him to restart investigations multiple times and waste hours on what should be a 15-minute task. He often resorts to SSH and grep commands, which work but lack the filtering sophistication he needs for complex investigations.

**Success Vision:** Marc can open a 20 GB log file, index it once in 2-3 minutes while getting coffee, then perform instant searches across any combination of IPs, ports, interfaces, or actions. When a department manager asks "why can't I access this server?", Marc finds the answer in under 5 minutes instead of abandoning the investigation due to crashes. The tool is simple enough that he doesn't need to read documentation - it just makes sense.

### Secondary Users

**Junior Network Administrator**

Sarah, a recent graduate in her first network admin role, uses the tool to learn firewall behavior patterns and investigate issues under senior guidance. She needs intuitive interface mapping (vtnet0 → LAN) and clear rule labels so she can understand what's happening without deep OPNsense expertise.

**Enterprise Network Team Member**

In larger organizations, specialized team members (NOC analysts, SecOps engineers) use the tool for their specific domains - security incident response, compliance auditing, or performance troubleshooting. They need advanced filtering capabilities (regex, complex AND/OR/NOT logic) and the ability to handle very large files (20-30 GB) from high-traffic environments.

**IT Manager / Auditor**

Indirect beneficiaries who need occasional access to investigate specific incidents or produce compliance reports. They value the export capabilities (JSON/CSV) and the ability to quickly answer questions without becoming tool experts.

### User Journey

**Discovery:** Network administrators discover the tool through OPNsense community forums, GitHub searches for log analysis tools, or recommendations from peers facing similar firewall log investigation challenges.

**First Experience:** Download a single portable executable (~15 MB), open a log file, watch the indexation progress (2-3 minutes), then immediately perform searches that return results in milliseconds. The "aha!" moment comes when they realize they can filter a 20 GB file without crashes - something impossible with previous tools.

**Core Usage Pattern:**
1. Periodic file indexation (weekly/monthly when investigating historical issues)
2. Rapid search iterations with multiple filter combinations
3. Interface mapping and rule label enrichment via API for context
4. Export filtered results for documentation or reporting

**Long-term Value:** The tool becomes the go-to solution for any firewall-related investigation. Administrators stop using workarounds (SSH grep, file splitting, incomplete analysis) because this tool reliably handles their largest files. Time saved on investigations is reinvested in proactive network improvements rather than firefighting.

---

## Success Metrics

### User Success Indicators

**Performance Achievement:**
- **Indexation Performance:** Successfully index 20-30 GB log files in 2-3 minutes on standard hardware (compared to 30+ minutes with current solution)
- **Search Performance:** Return filtered results in <1 second for any query combination across indexed files
- **Zero Crashes:** 0% crash rate when handling files up to 30 GB (compared to 100% failure rate with current Python application beyond 2-3 GB)

**Productivity Impact:**
- **Investigation Time Reduction:** Users complete typical log investigations in <10 minutes (compared to hours with current workarounds)
- **First-Time Success:** Users successfully complete their first investigation without consulting documentation or support
- **Workflow Integration:** Users abandon SSH/grep workarounds within first week of adoption

**User Satisfaction:**
- **Ease of Use:** Users rate the tool as "intuitive" without requiring training
- **Reliability Confidence:** Users trust the tool for critical incident investigations
- **Feature Adoption:** 80%+ of users utilize core features (indexing, filtering, interface mapping, API integration)

### Business Objectives

**Open Source Community Growth:**
- **3-Month Goals:**
  - 100+ GitHub stars indicating community interest
  - 500+ downloads from network admin community
  - 5+ community contributions (bug reports, feature requests, PRs)

- **12-Month Goals:**
  - 1000+ active users (measured by unique download IPs)
  - Recognition in OPNsense community forums as recommended tool
  - 20+ community contributions demonstrating active engagement

**Technical Excellence:**
- **Code Quality:** Maintain clean, well-documented codebase enabling community contributions
- **Cross-Platform Support:** Verified functionality on Windows/Linux/macOS with consistent UX
- **Performance Benchmarks:** Documented performance characteristics across hardware profiles

**Market Position:**
- **Primary Solution:** Become the go-to desktop application for OPNsense log analysis
- **Competitive Advantage:** Maintain unique positioning vs generic log analyzers (Elasticsearch/Splunk) and CLI tools

### Key Performance Indicators

**Adoption Metrics:**
- Downloads per month (target: 50+ by month 3, 200+ by month 12)
- Active users (unique tool launches tracked via optional telemetry)
- GitHub watch/star/fork activity indicating community interest

**Performance Metrics:**
- Maximum file size successfully processed (target: 30 GB without degradation)
- Average indexation time per GB (target: <6 seconds per GB)
- Average search response time (target: <500ms for complex queries)

**Quality Metrics:**
- Crash rate per session (target: <0.1%)
- User-reported bugs per release (target: <3 critical bugs)
- Time to fix critical issues (target: <48 hours)

**Engagement Metrics:**
- Feature utilization rate (% of users using advanced filtering, API integration)
- Return usage frequency (users returning weekly/monthly for investigations)
- Export feature usage (indicating users create reports/documentation)

**Community Health:**
- Response time to community issues (target: <24 hours for initial response)
- Community contribution acceptance rate
- Active community discussion threads

---

## MVP Scope

### Core Features

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
- Single portable executable (~15 MB)
- Modern GUI with intuitive interface
- No installation, no dependencies, no server infrastructure required

### Out of Scope for MVP

**Deferred to Post-MVP Releases:**

From the 54 ideas in the brainstorming session, the following are explicitly out of scope for MVP:

**Advanced Analytics (Post-MVP):**
- Automatic anomaly detection and pattern recognition
- Machine learning for threat identification
- Predictive analytics and trend forecasting
- Behavioral analysis and profiling

**Enhanced Visualization (Post-MVP):**
- Interactive dashboards and charts
- Geographic IP mapping
- Traffic flow diagrams
- Real-time monitoring displays

**Enterprise Features (Post-MVP):**
- Multi-firewall aggregation and correlation
- Centralized management for multiple OPNsense instances
- Team collaboration features (shared queries, annotations)
- RBAC and audit logging

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

**Rationale:** The MVP focuses exclusively on solving the core problem - reliable, fast analysis of large log files with proper context enrichment. Advanced features risk delaying delivery and introducing complexity that conflicts with the "Simplicity > Feature Creep" principle.

### MVP Success Criteria

**Technical Validation:**
- Successfully index and filter 30 GB log files without crashes
- Achieve <3 minute indexation time and <1 second search response
- Maintain <500 MB RAM usage during operations
- Zero data corruption or parsing errors on production log files

**User Validation:**
- First 10 users complete investigations faster than with current Python tool
- Users successfully enrich logs via API without configuration difficulties
- No critical bugs requiring workarounds in core filtering functionality
- Users report the tool as "reliable enough for production investigations"

**Adoption Signal:**
- 50+ downloads in first month indicating community interest
- 5+ positive testimonials from network administrators
- GitHub issues demonstrate real-world usage patterns
- Community requests align with deferred feature roadmap

**Go/No-Go Decision Point:**
If MVP meets these criteria after 3-4 months of development, proceed with post-MVP enhancements. If technical performance targets aren't met, revisit architecture before adding features.

### Future Vision

**Phase 2: Intelligence Layer (Months 5-8)**
- Automatic anomaly detection highlighting unusual traffic patterns
- Saved query library for common investigation scenarios
- Basic visualization: timeline charts, top talkers, protocol distribution
- Query result caching for repeated investigations

**Phase 3: Multi-Instance Support (Months 9-12)**
- Aggregate logs from multiple OPNsense firewalls
- Cross-firewall correlation for complex network environments
- Centralized query execution across distributed deployments
- Unified interface mapping and rule label management

**Phase 4: Ecosystem Integration (Year 2+)**
- SIEM export capabilities for enterprise workflows
- Plugin architecture for custom enrichment sources
- Webhook alerts for automated monitoring
- Advanced ML-based threat detection

**Long-Term Vision:**
Become the definitive desktop application for OPNsense log analysis, balancing power-user capabilities with simplicity. Expand cautiously into multi-firewall scenarios while maintaining the portable, zero-infrastructure philosophy that differentiates it from enterprise log management platforms.
