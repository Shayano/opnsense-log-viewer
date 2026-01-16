# Story 0.1: Project Scaffolding with Tauri + React

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want the project initialized with Tauri + React using the official create-tauri-app template,
So that I have a clean, production-ready foundation to build the log viewer application.

## Acceptance Criteria

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

## Tasks / Subtasks

- [ ] Initialize Tauri project with create-tauri-app v4.6.0 using react-ts template (AC: All criteria)
  - [ ] Run `npm create tauri-app@latest opnsense-log-viewer -- --template react-ts`
  - [ ] Verify directory structure: `src-tauri/` (Rust backend) and `src/` (React frontend)
  - [ ] Verify Cargo.toml exists with Tauri v2 dependencies
  - [ ] Verify package.json exists with React 18+ and TypeScript dependencies
  - [ ] Verify Vite config exists (vite.config.ts)
  - [ ] Verify tauri.conf.json exists with basic window configuration

- [ ] Install dependencies and verify dev build (AC: Application launches)
  - [ ] Run `npm install` to install frontend dependencies
  - [ ] Run `npm run tauri dev` to verify build and launch
  - [ ] Confirm application window opens with default Tauri + React template content
  - [ ] Verify no build errors in console
  - [ ] Verify hot-reload works (modify App.tsx and see changes)

- [ ] Configure TypeScript strict mode (AC: TypeScript strict mode enabled)
  - [ ] Open tsconfig.json
  - [ ] Ensure `"strict": true` is set
  - [ ] Ensure `"noImplicitAny": true`
  - [ ] Ensure `"strictNullChecks": true`
  - [ ] Verify TypeScript compiles without errors

- [ ] Setup linting and formatting (AC: ESLint, Prettier, clippy, rustfmt)
  - [ ] Verify ESLint config exists (.eslintrc.cjs or eslint.config.js)
  - [ ] Install and configure Prettier if not present
  - [ ] Create .prettierrc with project style (2-space indent, single quotes)
  - [ ] Verify Rust rustfmt.toml exists or use default
  - [ ] Verify clippy is available (comes with Rust toolchain)
  - [ ] Run `npm run lint` to verify ESLint works
  - [ ] Run `cargo fmt --check` to verify rustfmt works
  - [ ] Run `cargo clippy` to verify no warnings in default template

- [ ] Initialize Git repository with .gitignore (AC: Git repository initialized)
  - [ ] Verify `.git/` directory exists (create-tauri-app should initialize it)
  - [ ] Verify .gitignore includes:
    - `/target/` (Rust build artifacts)
    - `/node_modules/` (npm dependencies)
    - `/dist/` (Vite build output)
    - `/src-tauri/target/` (Tauri build artifacts)
  - [ ] Make initial commit with message "Initialize Tauri + React project with create-tauri-app v4.6.0"

- [ ] Update README.md (AC: README contains project details)
  - [ ] Update project name to "opnsense-log-viewer" if not already set
  - [ ] Add description: "High-performance desktop application for investigating OPNsense firewall logs (30GB+) with instant search and API enrichment"
  - [ ] Add "Quick Start" section with:
    - Prerequisites: Node.js 18+, Rust 1.70+, platform-specific dependencies
    - Install: `npm install`
    - Dev mode: `npm run tauri dev`
    - Build: `npm run tauri build`
  - [ ] Add "Tech Stack" section listing Tauri v2, React 18, TypeScript 5.7, Vite 6
  - [ ] Add reference to planning artifacts: `_bmad-output/planning-artifacts/`

## Dev Notes

### Architecture Context

**Project Initialization Requirements (from Architecture):**
- Use `create-tauri-app v4.6.0` specifically with `react-ts` template
- Project structure MUST follow: `src-tauri/` (Rust backend) vs `src/` (React frontend)
- This is Epic 0, Story 0.1 - Foundation for all subsequent stories
- Story 0.2 (Test Infrastructure) and Story 0.3 (Tailwind CSS) are MANDATORY before Epic 1 implementation

**Technology Stack Decisions (from Architecture & Project Context):**

Backend (Rust + Tauri):
- Tauri v2 (NOT v1 - breaking changes exist)
- Rust 1.70+ with Tokio 1.x async runtime
- Exact dependency versions REQUIRED (no wildcards like `"*"` or `"^1.0"`)
- Tokio features: ONLY `["rt-multi-thread", "fs", "io-util", "time"]` - NOT `"full"` (bloats compile time)

Frontend (React + TypeScript):
- React 18.3+ (requires concurrent features)
- TypeScript 5.7 with strict mode MANDATORY
- Vite 6.0+ for dev server and build
- NO Hungarian notation (ILogEntry ❌, LogEntry ✅)

**Critical Implementation Rules:**

Naming Conventions:
- Rust: `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE_CASE` constants
- TypeScript: `camelCase` functions/variables, `PascalCase` components/types, `kebab-case` files
- Tauri IPC commands: `snake_case` (e.g., `index_file`, `execute_query`)
- Tauri events: `kebab-case` (e.g., `indexation-progress`)

Error Handling:
- Library code: Use `thiserror` for custom error types
- Application code: Use `anyhow` for error propagation
- Tauri commands: Convert to `Result<T, String>` for IPC boundary

Serde JSON Interop (CRITICAL):
- ALWAYS use `#[serde(rename_all = "camelCase")]` for structs crossing Rust ↔ TypeScript IPC
- TypeScript expects camelCase, Rust uses snake_case internally

**Anti-Patterns to AVOID:**
- ❌ DO NOT use wildcard dependencies (`"*"` or `"^1.0"`)
- ❌ DO NOT use Tokio `features = ["full"]` (bloats compile time)
- ❌ DO NOT use Hungarian notation in TypeScript (`ILogEntry`, `TFilter`)
- ❌ DO NOT forget `#[serde(rename_all = "camelCase")]` on IPC structs
- ❌ DO NOT use `panic!()` or `unwrap()` in production Rust code

**File & Folder Structure (from Project Context):**
```
src-tauri/src/
├── commands/          # Tauri IPC handlers (will be added in later stories)
│   ├── mod.rs
│   ├── indexation.rs  # index_file, load_index commands
│   ├── query.rs       # execute_query command
│   └── credentials.rs # save_credentials command
├── indexer/           # Domain service - indexing logic (Story 1.3+)
│   ├── mod.rs
│   ├── inverted.rs    # Inverted index for IPs/ports
│   └── bitmap.rs      # Bitmap index for actions/protocols
├── parser/            # Log parsing (Story 1.2)
│   ├── mod.rs
│   ├── rfc3164.rs
│   ├── rfc5424.rs
│   └── csv_filterlog.rs
├── query/             # Query execution (Story 2.2)
│   ├── mod.rs
│   ├── executor.rs
│   └── optimizer.rs
├── api_client/        # OPNsense API (Epic 3)
│   ├── mod.rs
│   ├── client.rs
│   └── enrichment.rs
├── storage/           # Index persistence (Story 1.4)
├── export/            # Export functionality (Epic 5)
└── main.rs            # Tauri app entry point

src/
├── components/        # React components
│   ├── base/          # Base design system (Story 0.3)
│   ├── filter-builder/ # Filter builder (Story 2.1)
│   └── log-results-table/ # Log results table (Story 1.5)
├── hooks/             # Custom React hooks
├── stores/            # Zustand state management
├── types/             # TypeScript types
└── App.tsx            # Main application component
```

**UX Requirements for Story 0.1:**
- Dark/light theme support foundation (Story 0.3 will implement)
- VS Code-inspired layout foundation (collapsible sidebars, main content area)
- Professional modern aesthetic (NOT playful, NOT austere)
- Accessibility baseline: keyboard navigation, ARIA labels (will be enhanced in Story 0.3)

### Project Structure Notes

**Alignment with Project Context:**
- This story creates the foundational structure that ALL subsequent stories depend on
- NO domain logic implementation yet - pure scaffolding
- Test infrastructure (Story 0.2) and Tailwind CSS (Story 0.3) MUST be completed before Epic 1

**Detected Dependencies:**
- Story 0.2 (Test Infrastructure) depends on this story completing successfully
- Story 0.3 (Tailwind CSS) depends on this story completing successfully
- Epic 1+ (All feature implementation) depends on Stories 0.1, 0.2, 0.3 completing

**Version Control:**
- Git repository MUST be initialized
- Initial commit should include template scaffolding with NO modifications
- .gitignore MUST exclude build artifacts (`/target/`, `/node_modules/`, `/dist/`)

### References

**Source Documents:**
- [Source: _bmad-output/planning-artifacts/epics.md#Story 0.1]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Setup]
- [Source: _bmad-output/project-context.md#Technology Stack & Versions]
- [Source: _bmad-output/project-context.md#Critical Implementation Rules]

**External Documentation:**
- Tauri v2 Documentation: https://v2.tauri.app/
- create-tauri-app: https://github.com/tauri-apps/create-tauri-app
- React 18 Documentation: https://react.dev/
- Vite Documentation: https://vitejs.dev/

**Performance Gates (CI/CD - Story 0.2):**
- Indexation: <7 sec/GB (±15%) - NOT applicable to Story 0.1
- Query: <750ms (±50%) - NOT applicable to Story 0.1
- Memory: <600 MB (±20%) - NOT applicable to Story 0.1

**Code Coverage Targets (Story 0.2):**
- Critical modules: 90% - NOT applicable to Story 0.1 (no domain logic yet)
- Important modules: 75% - NOT applicable to Story 0.1
- UI components: 60% - NOT applicable to Story 0.1

## Dev Agent Record

### Agent Model Used

_To be filled by dev agent_

### Debug Log References

_To be filled by dev agent during implementation_

### Completion Notes List

_To be filled by dev agent upon completion:_
- Template version used
- Any deviations from standard template
- Verification of all acceptance criteria
- Build success confirmation
- Next steps (Story 0.2 - Test Infrastructure)

### File List

_To be filled by dev agent with all files created:_
- Directory structure
- Configuration files (Cargo.toml, package.json, tsconfig.json, etc.)
- Source files (main.rs, App.tsx, etc.)
- Build outputs verified
