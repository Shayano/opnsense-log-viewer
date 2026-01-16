---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - "_bmad-output/planning-artifacts/product-brief-opnsense-log-viewer-2026-01-14.md"
  - "_bmad-output/analysis/brainstorming-session-2026-01-14.md"
  - "docs/DEVELOPER_GUIDE.md"
  - "README.md"
---

# UX Design Specification opnsense-log-viewer

**Author:** Shay
**Date:** 2026-01-14

---

## Executive Summary

### Project Vision

OPNsense Log Viewer is a high-performance desktop application that transforms firewall log investigation from a frustrating, crash-prone experience into a fast, reliable workflow. Built on Tauri + Rust architecture, it enables network administrators to analyze 20-30 GB log files through intelligent indexing (2-3 minute indexation) followed by instant search responses (<1 second). The application prioritizes "Simplicity > Feature Creep" - delivering powerful filtering capabilities through an intuitive, modern interface that requires no expertise to use effectively. Cross-platform support (Windows/Linux/macOS) with a portable ~15 MB executable ensures administrators can investigate issues anywhere, anytime, without infrastructure dependencies.

### Target Users

**Primary User: Marc - SMB Solo Network Administrator**
- Manages complete IT infrastructure independently for 50-employee company
- Intermediate technical skills (self-taught + vendor certifications)
- Values tools that "just work" without deep expertise requirements
- Works on both desktop (large screens) and laptop (smaller screens)
- Performs retrospective investigations to understand communication issues
- Typical workflow: Start with time range, add action filter, refine with IP/port filters
- Often under pressure to resolve issues quickly

**Secondary Users:**
- **Junior Administrators:** Learning firewall behavior, need intuitive interfaces with clear labeling
- **Enterprise Teams (SecOps/NOC):** Advanced filtering needs (regex, complex boolean logic), handle very large files
- **IT Managers/Auditors:** Occasional use for specific investigations or compliance reports, value export capabilities

**User Context:**
- Usage scenarios range from routine investigations at the office to urgent troubleshooting at 2am from home
- Investigations are highly varied: time-based analysis, IP-specific communication tracking, blocked traffic overview
- Users typically combine 3+ filters (time range + action + IP/port combinations), often iterating to refine results

### Key Design Challenges

**1. Multi-Filter Management Without Friction**
- Users apply 3+ combined filters in typical investigations (time + action + IP/port)
- Critical issue from Python app: removing/modifying filters triggered immediate re-processing (5+ minute wait)
- **Challenge:** Enable filter construction in "draft mode" with explicit search execution, preventing accidental expensive operations
- **Challenge:** Support quick filter iteration without sacrificing advanced capabilities (AND/OR/NOT, regex)

**2. Responsive Multi-Screen Adaptability**
- Must work equally well on desktop workstations (large screens) and laptops (constrained space)
- Log data is inherently information-dense (timestamps, IPs, ports, actions, protocols, interfaces)
- **Challenge:** Maintain information density while adapting gracefully to different screen sizes
- **Challenge:** Balance horizontal space usage for wide log entries vs vertical space for result lists

**3. Performance Perception and Feedback**
- Indexation takes 2-3 minutes (unavoidable trade-off for instant searches)
- Users accustomed to Python app's poor performance need confidence this is different
- **Challenge:** Communicate indexation progress clearly and make waiting acceptable
- **Challenge:** Instant search feedback (<1s) must feel noticeably different from old experience
- **Challenge:** Show users when indexed data is ready for fast queries vs when re-indexing is needed

**4. Progressive Complexity**
- Power users need advanced features (regex, complex boolean logic, API enrichment)
- Marc needs "just works" simplicity without reading documentation
- **Challenge:** Default to simple, intuitive interactions while making advanced features discoverable
- **Challenge:** Avoid overwhelming beginners while not hiding power-user capabilities

### Design Opportunities

**1. Modern, Professional Visual Design**
- Python app was "austere but functional" - significant opportunity for visual upgrade
- **Opportunity:** Modern, clean aesthetic that makes the tool feel professional and valuable
- **Opportunity:** Theme switcher (dark ↔ light modes) allowing users to choose based on environment and preference
- **Opportunity:** Thoughtful typography and spacing that reduces cognitive load during complex investigations

**2. Intelligent Filter Builder with Explicit Execution**
- Replace auto-trigger filtering with deliberate "Search" action
- **Opportunity:** Visual filter builder showing active filters with clear add/edit/remove controls
- **Opportunity:** Filter templates for common scenarios: "Show Blocked Traffic", "Communication with IP", "Time Range Analysis"
- **Opportunity:** Filter preview showing potential result count before executing expensive queries
- **Opportunity:** Recent searches history for quick re-execution

**3. Rich Contextual Feedback**
- API enrichment provides valuable context (interface names, rule labels, aliases)
- **Opportunity:** Inline display of enriched data with clear indication when API unavailable
- **Opportunity:** Progressive enhancement - show raw data immediately, enrich asynchronously
- **Opportunity:** Help text and tooltips that educate users about filter syntax and capabilities
- **Opportunity:** Result summaries showing distribution of actions, top IPs, protocol breakdown

**4. Competitive UX Differentiation**
- Generic log analyzers (Splunk/ELK) are complex and overwhelming for this use case
- CLI tools (SSH + grep) lack discoverability and require memorized commands
- **Opportunity:** Intuitive interface that becomes the killer feature vs competitors
- **Opportunity:** Zero-configuration experience - download, open file, start investigating
- **Opportunity:** Export capabilities integrated seamlessly into investigation workflow
- **Opportunity:** Visual design that communicates reliability and professional quality

---

## Core User Experience

### Defining Experience

The core experience of OPNsense Log Viewer centers on **building filters and analyzing results**, with the critical interaction being **results display and visualization**. Users follow a consistent workflow: (1) Load log file triggering indexation (2-3 minutes), (2) Connect to OPNsense via SSH/API for data enrichment, (3) Build filter queries using visual controls, (4) Execute search and analyze displayed results.

The make-or-break interaction is **results display** - users need to scan through potentially thousands of log entries to understand communication patterns, identify blocked traffic, and diagnose issues. The interface must excel at presenting information-dense log data with clarity, making patterns visible and specific entries easy to locate. Filtering is frequent and iterative, with users typically combining 3+ filters (time range + action + IP/port) and refining based on initial results.

A unique requirement: users must be able to filter on **enriched data** (interface names like "LAN" instead of "vtnet0", rule descriptions instead of hashes, alias names instead of IP groups) as naturally as filtering on raw log fields. This enrichment transforms cryptic technical data into human-readable context essential for efficient investigation.

### Platform Strategy

**Desktop Application - Multiplatform Focus**
- Tauri + Rust architecture targeting Windows, Linux, and macOS
- Portable executable (~15 MB) requiring no installation or dependencies
- Mouse and keyboard interaction model (no touch optimization needed)
- Optimized for both desktop workstations (large screens) and laptops (constrained space)

**Offline-First with Optional Connectivity**
- Primary use case: Local log file analysis without network dependency
- Optional API connectivity for data enrichment when available
- Graceful degradation: Show raw data when enrichment unavailable
- Users work with downloaded/exported log files rather than live streaming

**No Server Infrastructure**
- Zero configuration required to start using the tool
- All processing happens locally on user's machine
- No account creation, authentication, or cloud dependencies
- Aligns with "Simplicity > Feature Creep" principle

### Effortless Interactions

**1. File Loading**
- Standard File > Open menu for log file selection
- Clear feedback during indexation process (2-3 minutes)
- Progress indication showing indexing status and estimated completion
- Once indexed, instant query responses (<1 second)

**2. API Enrichment Configuration**
- Connection details requested at session start (hostname, credentials, API keys)
- Credentials **saved locally** for quick reuse with same device
- Easy switching between different OPNsense devices without re-typing common values
- Clear visual indication of enrichment status (connected/disconnected/enriching)

**3. Visual Filter Builder**
- Dropdown-based filter construction: Field + Operator + Value
- No syntax memorization required - all options visually presented
- Filters include **both raw fields and enriched data** (interface names, rule labels, aliases)
- "Add Filter" control for building complex queries incrementally
- Filters accumulated in "draft mode" without triggering expensive operations
- Explicit "Search" button to execute query when filter construction complete

**4. Search History**
- Automatic history of recent filter combinations
- Quick re-execution of previous complex queries
- Eliminates need to reconstruct multi-filter queries manually
- History persists across sessions for frequently-used investigations

**5. Results Navigation**
- Scannable results with clear visual hierarchy
- Enriched data displayed inline (LAN instead of vtnet0, rule names instead of hashes)
- Export selected results to JSON/CSV integrated into workflow
- Keyboard shortcuts for power users (navigation, filtering, export)

### Critical Success Moments

**First-Time "Aha!" Moment**
When Marc loads a 20 GB log file, watches the 2-3 minute indexation complete successfully (instead of crashing after 30 minutes), then executes his first filter and sees instant results (<1 second). This moment proves "this tool is fundamentally different."

**Enrichment Clarity**
When users see "LAN → WAN: BLOCK" with the rule description "Block RFC1918 Networks" instead of cryptic "vtnet0 → vtnet1: pass, rule hash 031d9d1e". The enriched context transforms raw technical data into immediately understandable information.

**Iterative Investigation Success**
When Marc builds a complex query (time range + blocked traffic + specific source IP), sees 2,431 results instantly, realizes he needs to refine by destination port, adds another filter, and gets refined results in <1 second. No waiting, no crashes, no lost progress.

**Recurring Productivity Gain**
When Marc opens the tool for the third time and realizes: (1) His common searches are in history, (2) API connection remembered his credentials, (3) He finds answers in <5 minutes instead of giving up after crashes. The tool becomes trusted and valued.

### Experience Principles

**1. "Results First" - Display is King**
The interface must excel at visualizing large quantities of log data with clarity and optimal information density. Results must be scannable quickly, with visual hierarchy making patterns visible and specific entries easy to locate. Everything else supports this critical interaction.

**2. "No Accidental Triggers" - Explicit Control**
Filters build in "draft mode" without triggering expensive operations. Users control when to execute via explicit "Search" button. Removing or modifying filters never causes unexpected re-processing. Users stay in control.

**3. "Enrich by Default" - Automatic Context**
API enrichment (interface names, rule labels, aliases) integrates naturally into the experience. Connections are remembered to avoid re-entry while allowing device switching. Enriched data is as filterable as raw data.

**4. "Learn from History" - Capitalize on Experience**
Search history allows reusing complex filter combinations without manual reconstruction. Common investigation patterns become faster over time. The tool learns from user behavior without explicit configuration.

**5. "Visual Over Command" - Discoverability Before Memorization**
Dropdowns and visual builders replace syntax memorization. All filtering capabilities are discoverable through the interface. Filters on enriched data (interface names, rule descriptions) are as natural as filters on raw fields (IPs, ports).

---

## Desired Emotional Response

### Primary Emotional Goals

**Confidence and Control**
The primary emotional goal is for users to feel **confident** and **in control** when investigating firewall logs. After experiencing crashes and unpredictable behavior with the Python application, users need to trust that this tool will reliably complete their investigations. They should feel empowered to build complex filter queries without fear of triggering expensive operations or losing their work. Control manifests through explicit execution ("Search" button), draft mode editing, and predictable performance.

**Relief and Efficiency**
Users should experience **relief** - the burden of wrestling with unreliable tools is lifted. Investigations that previously took hours (or were abandoned entirely) now complete in minutes. This relief transforms into sustained **efficiency** as users realize they can trust the tool for critical work. The emotional shift from "will this work?" to "I can solve this quickly" is fundamental to product success.

**Professional Accomplishment**
When users successfully diagnose network issues using enriched, clear data, they should feel **professionally accomplished**. The tool makes them look competent and efficient. Finding "LAN → WAN: BLOCK - RFC1918 Networks" instead of cryptic hashes makes users feel like experts, even when they're learning.

### Emotional Journey Mapping

**Discovery (First Launch):**
- **Curiosity + Cautious Optimism:** "Let's see if this really works better than the Python tool"
- Design implication: Clean, professional interface that immediately signals quality and reliability

**Initial File Load (Indexation 2-3 min):**
- **Anticipation + Trust Building:** Clear progress indication makes waiting feel acceptable
- Design implication: Transparent progress feedback, estimated completion, visual confirmation that work is happening

**First Successful Search (<1 sec results):**
- **Surprise → Relief → Excitement:** "It actually works! And it's instant!"
- Design implication: Instant feedback, clear result counts, immediate value demonstration

**Building Complex Filters:**
- **Focused Control:** Users feel methodical, deliberate, in command of the investigation
- Design implication: Visual filter builder, draft mode, no accidental triggers, clear current state

**Viewing Enriched Results:**
- **Clarity + Professional Confidence:** Technical data becomes immediately understandable
- Design implication: Inline enrichment, clear labeling (LAN vs vtnet0), visual hierarchy

**Returning for Third+ Use:**
- **Trusted Routine:** The tool becomes a valued part of their workflow
- Design implication: Remembered credentials, search history, consistent performance

**When Issues Occur:**
- **Supported, Not Abandoned:** Clear error messages, graceful degradation (show raw data if API fails)
- Design implication: Helpful error states, never blocking workflow entirely

### Micro-Emotions

**Confidence vs. Confusion**
- **Target:** High confidence through visual discoverability
- Users should never feel lost or confused about available actions
- All filtering capabilities presented visually (dropdowns), not hidden behind syntax

**Trust vs. Skepticism**
- **Target:** Build trust immediately through reliability
- First successful 20+ GB file indexation without crashes = trust established
- Consistent <1 second search responses reinforce trust over time

**Control vs. Helplessness**
- **Target:** Complete user control over operations
- Draft mode prevents accidental expensive operations
- Explicit "Search" button - users decide when to execute
- Contrast with Python app's auto-trigger frustration

**Accomplishment vs. Frustration**
- **Target:** Frequent micro-accomplishments throughout investigation
- Each successful filter refinement = small win
- Finding the answer in <5 minutes = major accomplishment
- Export results = tangible evidence of work completed

**Clarity vs. Overwhelm**
- **Target:** Information density without cognitive overload
- Enriched data (interface names, rule labels) reduces mental translation
- Visual hierarchy makes scanning results effortless
- Progressive disclosure: simple by default, advanced when needed

### Design Implications

**To Create Confidence:**
- Professional visual design that signals quality and reliability
- Consistent, predictable behavior across all interactions
- Clear feedback for all operations (indexing, searching, enriching)
- Zero crashes or data loss under any circumstances

**To Enable Control:**
- Draft mode for filter construction with explicit execution
- "Search" button that clearly indicates when query will run
- Ability to modify/remove filters without triggering re-processing
- Saved API credentials user can edit/change anytime

**To Deliver Efficiency:**
- <1 second search responses that feel instant
- Search history for re-executing common queries
- Keyboard shortcuts for power users
- Integrated export workflow (no context switching)

**To Build Trust:**
- Transparent progress during 2-3 minute indexation
- Clear indication of enrichment status (connected/enriching/offline)
- Graceful degradation when API unavailable (show raw data)
- Honest error messages that explain what happened and suggest next steps

**To Provide Clarity:**
- Enriched data displayed inline (LAN not vtnet0)
- Visual hierarchy in results (timestamps, actions, IPs clearly separated)
- Tooltips and help text for advanced features
- Theme switcher (dark ↔ light) for user comfort

### Emotional Design Principles

**1. "Reliability Creates Confidence"**
Every interaction must work exactly as expected, every time. The Python app's unreliability created anxiety; this tool's consistency builds confidence. Zero tolerance for crashes, data loss, or unpredictable behavior.

**2. "Transparency Builds Trust"**
Users should always understand what's happening: indexation progress, enrichment status, search execution, error states. Hidden processes create anxiety; transparent processes build trust.

**3. "Control Eliminates Frustration"**
Give users explicit control over expensive operations. Draft mode, explicit "Search" button, and predictable behavior transform frustration into satisfaction. Users should never feel the tool is working against them.

**4. "Clarity Enables Accomplishment"**
Enriched data and clear visual presentation reduce cognitive load. When users quickly understand what they're seeing, they feel accomplished and professional. Technical complexity should be handled by the tool, not the user.

**5. "Consistency Enables Flow"**
Once users learn the filter builder, search history, and result navigation, these patterns should work identically across all sessions. Consistency allows users to enter a productive flow state where the tool disappears and investigation becomes natural.

---

## UX Pattern Analysis & Inspiration

### Inspiring Products Analysis

**VS Code - Developer Tool Excellence**
- **What they do well:** Information-dense interface that never feels overwhelming through excellent visual hierarchy
- **Key patterns:** Command palette for advanced users, sidebar collapsible for space management, instant search with clear result counts, theme switcher (dark ↔ light) integrated seamlessly
- **Why it works:** Progressive disclosure - simple for beginners, powerful for experts. Keyboard shortcuts discoverable but not required. Performance is priority #1.
- **Compelling experience:** Users trust it to handle massive files without crashes. Search is instant. State is always clear.

**Postman - Technical Tool with Modern UX**
- **What they do well:** Complex technical operations made approachable through visual builders instead of syntax
- **Key patterns:** Collections/history for reusing complex requests, explicit "Send" button (no auto-trigger), saved configurations (environments), clear request/response separation
- **Why it works:** Building complex API requests feels controlled and methodical. History makes repeated work effortless. Visual feedback for all operations.
- **Compelling experience:** Professional aesthetic signals quality. Users feel competent using a sophisticated tool.

**TablePlus/DataGrip - Database Query Tools**
- **What they do well:** Filter builders that construct SQL visually, results grid optimized for scanning dense tabular data, saved queries/favorites
- **Key patterns:** Field + Operator + Value dropdowns, query history, connection credentials saved securely, export integrated into workflow
- **Why it works:** Technical users can build complex filters without memorizing syntax. Results display prioritizes scannability. Credentials remembered but editable.
- **Compelling experience:** Instant query execution, clear result counts, professional data grid presentation

**Spotify Desktop - Modern Desktop Performance**
- **What they do well:** Instant search responses even with massive libraries, clean modern aesthetic, theme switching, offline-first functionality
- **Key patterns:** Search as you type with instant results, clear current state (what's playing, what's queued), keyboard navigation throughout
- **Why it works:** Performance creates trust. Modern visual design makes using the app pleasant. Consistent behavior across sessions.
- **Compelling experience:** Tool feels responsive and reliable. Never crashes. State always understandable.

### Transferable UX Patterns

**Navigation & Layout Patterns:**
- **Sidebar + Main Content:** VS Code pattern - collapsible sidebar for filters/history, main area for results. Adapts to screen size while maintaining density.
- **Command/Search Palette:** Quick access to all features via keyboard (Ctrl+P, Ctrl+Shift+F pattern). Power users love shortcuts, beginners ignore them.
- **Collapsible Sections:** Allow users to maximize space for what matters most (results) while keeping controls accessible.

**Interaction Patterns:**
- **Visual Query Builder:** TablePlus pattern - Field + Operator + Value dropdowns eliminate syntax memorization. Perfect for our filter construction.
- **Explicit Execution Button:** Postman pattern - "Send" / "Search" button gives users control over expensive operations. Draft mode before execution.
- **History/Collections:** Postman pattern - Recent searches automatically saved, favorites/collections for common queries. Reduces repeated work.
- **Progressive Search:** Spotify pattern - Start typing, see suggestions, refine quickly. Could work for IP/interface name quick filters.

**Visual & Feedback Patterns:**
- **Theme Switcher:** VS Code / Spotify pattern - Toggle dark ↔ light mode easily. Modern standard, user preference.
- **Progress Indication:** Clear progress bars with estimated completion for long operations (indexation). Builds trust during unavoidable waits.
- **Result Counts:** "2,431 results in 0.8s" - immediate feedback on search effectiveness. Helps users know if they need to refine.
- **Inline Status Indicators:** Clear visual indication of enrichment status, connection state, current filters active.

**Data Display Patterns:**
- **Grid/Table Optimization:** TablePlus pattern - Monospace fonts for technical data, alternating row colors, column resizing, horizontal scrolling for wide data.
- **Visual Hierarchy:** VS Code pattern - Use color, weight, spacing to create scannable information density without overwhelming.
- **Contextual Actions:** Right-click menus, inline buttons for export/copy/filter-by-this-value on results.

### Anti-Patterns to Avoid

**1. Auto-Execution on Filter Change**
- **Why avoid:** Python app's critical mistake - removing a filter triggers expensive re-processing
- **Lesson:** Always require explicit execution for costly operations. Draft mode is essential.

**2. Hidden Advanced Features**
- **Why avoid:** Regex/advanced filtering buried in menus creates "power user vs. beginner" divide
- **Lesson:** Make advanced features discoverable but optional. Toggle "Advanced" mode or expand inline.

**3. Cryptic Technical Data Without Context**
- **Why avoid:** Showing "vtnet0" to non-experts creates cognitive load and confusion
- **Lesson:** Enrich by default. Show human-readable context (LAN, rule names) first, raw data on hover if needed.

**4. Blocking Workflows on External Dependencies**
- **Why avoid:** If API fails, tool becomes useless. Users can't work.
- **Lesson:** Graceful degradation. Show raw data if enrichment unavailable. Never block core functionality.

**5. Inconsistent State Across Sessions**
- **Why avoid:** Re-entering credentials, rebuilding common filters = frustration and wasted time
- **Lesson:** Remember user state (credentials, history, preferences) across sessions. Make changing state easy when needed.

**6. Over-Engineering for Edge Cases**
- **Why avoid:** Feature creep destroys simplicity. Splunk/ELK complexity overwhelms for this use case.
- **Lesson:** "Simplicity > Feature Creep" - focus on core investigation workflow, defer non-essential features.

**7. Poor Performance Feedback**
- **Why avoid:** Users don't know if tool is working or frozen. Creates anxiety and distrust.
- **Lesson:** Always show what's happening: progress bars, spinners, status messages. Transparency builds trust.

### Design Inspiration Strategy

**What to Adopt:**

**VS Code-Style Information Density**
- Adopt: Collapsible sidebars, clear visual hierarchy, theme switcher, keyboard shortcuts as enhancement not requirement
- Because: Supports "Results First" principle - maximize space for critical results display while keeping controls accessible

**Postman-Style Explicit Control**
- Adopt: Visual builder for filters (dropdowns), explicit "Search" button, collections/history for reusability, saved configurations
- Because: Aligns with "No Accidental Triggers" principle - users control expensive operations explicitly

**TablePlus-Style Filter Construction**
- Adopt: Field + Operator + Value pattern, filter preview, query history, export integration
- Because: Supports "Visual Over Command" principle - no syntax memorization, all capabilities discoverable

**Spotify-Style Performance & Polish**
- Adopt: Instant feedback (<1s searches), modern clean aesthetic, offline-first approach, consistent behavior
- Because: Creates "Confidence and Control" emotional response - users trust reliable, fast, polished tools

**What to Adapt:**

**Command Palette → Quick Filter**
- Adapt VS Code's command palette concept into a quick filter search bar for common scenarios
- Simplify for intermediate skill level - suggest common patterns ("Blocked traffic last hour")

**Collections → Search Templates**
- Adapt Postman's collections into pre-built filter templates for common investigations
- Tailor specifically to firewall log scenarios ("Show Blocked Traffic", "Communication with IP", "Port Scan Detection")

**Grid Display → Log-Optimized Table**
- Adapt database grid patterns for log-specific needs
- Prioritize timestamp, action, source/dest IPs/ports, interface names - firewall-specific column ordering

**What to Avoid:**

**Command-Line Interfaces**
- Avoid CLI-style syntax requirements (grep regex patterns, complex query languages)
- Conflicts with "Visual Over Command" principle and intermediate user skill level

**Real-Time Streaming Complexity**
- Avoid live log tailing, hot reloading, real-time dashboards
- Users work with historical data (retrospective investigations), not live monitoring

**Enterprise Over-Engineering**
- Avoid RBAC, multi-user collaboration, centralized management in MVP
- Conflicts with "Simplicity > Feature Creep" - defer to post-MVP phases

**Generic Log Analysis Features**
- Avoid trying to be Splunk/ELK for all log types
- Stay focused on OPNsense firewall logs specifically - specialized tool wins over generalized complexity

---

## Design System Foundation

### Design System Choice

**Selected: Custom Lightweight System with Tailwind CSS**

For OPNsense Log Viewer, we're adopting a **custom lightweight design system built on Tailwind CSS** with purpose-built components. This is not a pre-packaged system like Material UI or Ant Design, but rather a tailored approach that prioritizes performance and flexibility for desktop applications.

**Components:**
- **Tailwind CSS** for utility-first styling and theme management
- **Custom React/Vue components** (depending on Tauri frontend choice) for specialized data-dense displays
- **Headless UI** or **Radix UI** for accessible, unstyled primitives (dropdowns, dialogs, etc.)
- **Monaco Editor** or similar for potential advanced filter query input (if regex/advanced mode needed)

### Rationale for Selection

**Performance is Critical:**
- Tauri desktop apps benefit from lightweight CSS-first approaches over heavy JavaScript component libraries
- No runtime overhead from large UI frameworks - critical for handling 20-30 GB file indexation UI responsiveness
- Tailwind's purging removes unused CSS, keeping bundle size minimal (~15 MB target)

**Information Density Requirements:**
- Pre-built UI libraries (Material, Ant) are optimized for consumer apps with generous spacing
- Our log viewer needs data-dense tables, compact filters, and efficient space usage
- Custom components give full control over information hierarchy and density

**Technical Stack Alignment:**
- Tauri + Rust backend pairs well with lean frontend approaches
- Theme switching (dark ↔ light) is straightforward with Tailwind's built-in dark mode
- No conflicts with Rust/Tauri's performance-first philosophy

**Flexibility for Unique Requirements:**
- Log result display needs custom table/grid implementation optimized for scanning
- Filter builder requires specialized dropdown/input combinations not standard in UI kits
- Enrichment status indicators, progress bars, and connection state visuals are domain-specific

**Speed of Development:**
- Tailwind's utility classes enable rapid prototyping and iteration
- Headless UI/Radix provide accessible foundations without visual opinions
- Can start building immediately without learning a complex component library API

### Implementation Approach

**Phase 1: Foundation Setup**
1. **Install Tailwind CSS** with dark mode configuration
2. **Define Design Tokens:**
   - Color palette (primary, success, warning, error, neutral scale)
   - Typography scale (optimized for technical data - monospace for IPs/logs, sans-serif for UI)
   - Spacing system (tight spacing for data density)
   - Border radius, shadows (subtle - professional not playful)

**Phase 2: Core Component Library**
3. **Build Foundation Components:**
   - Button (primary, secondary, ghost variations)
   - Input (text, number, select/dropdown)
   - Checkbox, Radio, Toggle
   - Modal/Dialog
   - Progress Bar (for indexation feedback)
   - Toast/Notification (for errors, success messages)

4. **Build Specialized Components:**
   - **FilterBuilder** - Field + Operator + Value dropdowns with add/remove controls
   - **LogResultsTable** - Data-dense grid with virtual scrolling, column resizing, enrichment highlighting
   - **ConnectionStatusIndicator** - Visual API/SSH connection state
   - **SearchHistoryPanel** - Collapsible sidebar with recent searches
   - **ExportDialog** - JSON/CSV export with options

**Phase 3: Layout & Patterns**
5. **Define Layout System:**
   - Sidebar + Main Content pattern (VS Code-inspired)
   - Collapsible sections for space optimization
   - Responsive breakpoints (desktop-first, laptop-optimized)

6. **Establish Interaction Patterns:**
   - Hover states for all interactive elements
   - Focus management for keyboard navigation
   - Loading states for async operations (indexation, search, API calls)
   - Empty states with helpful messaging

### Customization Strategy

**Theme Switching (Dark ↔ Light):**
- Tailwind's `dark:` variant for all components
- User preference stored locally, persists across sessions
- Toggle in top-right corner (standard modern app pattern)
- Default to system preference on first launch

**Data-Dense Styling:**
- Tighter line-height and spacing than typical consumer apps
- Monospace fonts for technical data (timestamps, IPs, ports)
- Alternating row colors in results table for scannability
- Subtle hover highlights without disrupting visual scanning
- Color-coding for actions (block=red, pass=green, reject=orange)

**Professional Aesthetic:**
- Restrained color palette - not playful, not austere (balance)
- Consistent 4px or 8px border radius (modern but not rounded buttons)
- Subtle shadows for depth (elevation hierarchy)
- Clear visual hierarchy through size, weight, and color contrast

**Performance Optimization:**
- Virtual scrolling for log results (render only visible rows)
- Lazy loading for heavy components (export dialog, advanced filters)
- Debounced inputs where appropriate (search-as-you-type scenarios)
- Optimistic UI updates (show filter immediately, execute search on button click)

**Accessibility Baseline:**
- ARIA labels on all interactive components
- Keyboard navigation throughout (Tab, Enter, Esc patterns)
- Focus indicators clearly visible in both themes
- Color is never the only indicator (use icons + text for status)
