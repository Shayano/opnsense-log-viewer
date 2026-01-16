# OPNsense Log Viewer

High-performance desktop application for investigating OPNsense firewall logs (30GB+) with instant search and API enrichment.

## Features

- **High Performance**: Index and search 30GB+ log files in 2-3 minutes
- **Instant Search**: Query millions of log entries in <500ms
- **Smart Indexing**: Hybrid inverted + bitmap index with persistent caching
- **API Enrichment**: Transform cryptic data (vtnet0, rule hashes) into readable context (LAN, "Block RFC1918")
- **Offline Mode**: Export enrichment data for investigations without API access
- **Modern UI**: VS Code-inspired layout with dark/light themes
- **Cross-Platform**: Windows, macOS, and Linux support

## Tech Stack

- **Backend**: Tauri v2 + Rust 1.70+
- **Frontend**: React 18 + TypeScript 5.7 + Vite 6
- **UI Framework**: Tailwind CSS 3.4+ (coming in Story 0.3)
- **State Management**: Zustand 5.0
- **Virtual Scrolling**: TanStack Virtual 3.13

## Quick Start

### Prerequisites

- **Node.js 18+** - [Download](https://nodejs.org/)
- **Rust 1.70+** - [Install via rustup](https://rustup.rs/)
- **Platform-specific dependencies**:
  - **Windows**: WebView2 (usually pre-installed on Windows 10+)
  - **macOS**: Xcode Command Line Tools
  - **Linux**: WebKitGTK, libssl-dev

### Install Dependencies

```bash
npm install
```

### Development Mode

```bash
npm run tauri dev
```

### Build for Production

```bash
npm run tauri build
```

The build will create platform-specific installers in `src-tauri/target/release/bundle/`.

## Development Commands

### Frontend (TypeScript + React)

```bash
npm run dev          # Vite dev server
npm run build        # Production build
npm run lint         # ESLint
npm run lint:fix     # Auto-fix linting issues
npm run format       # Format with Prettier
npm run format:check # Check formatting
```

### Backend (Rust + Tauri)

```bash
cd src-tauri
cargo fmt            # Format Rust code
cargo clippy         # Linting
cargo test           # Run tests
cargo build --release # Release build
```

## Project Structure

```
opnsense-log-viewer/
├── src/                     # React frontend
│   ├── components/          # React components
│   ├── hooks/               # Custom React hooks
│   ├── stores/              # Zustand state management
│   ├── types/               # TypeScript types
│   └── App.tsx              # Main app component
├── src-tauri/               # Rust backend
│   ├── src/
│   │   ├── commands/        # Tauri IPC handlers (coming in later stories)
│   │   ├── indexer/         # Log indexing logic (Story 1.3+)
│   │   ├── parser/          # Log parsers (Story 1.2)
│   │   ├── query/           # Query execution (Story 2.2)
│   │   ├── api_client/      # OPNsense API (Epic 3)
│   │   ├── storage/         # Index persistence (Story 1.4)
│   │   └── export/          # Export functionality (Epic 5)
│   ├── Cargo.toml           # Rust dependencies
│   └── tauri.conf.json      # Tauri configuration
├── _bmad-output/            # Planning artifacts
│   ├── planning-artifacts/  # PRD, Architecture, UX Design, Epics
│   └── implementation-artifacts/  # User stories
└── python-legacy/           # Original Python implementation (reference)
```

## Planning & Architecture

This project follows the BMad Method for structured software development:

- **PRD**: `_bmad-output/planning-artifacts/prd.md`
- **Architecture**: `_bmad-output/planning-artifacts/architecture.md`
- **UX Design**: `_bmad-output/planning-artifacts/ux-design-specification.md`
- **Epics & Stories**: `_bmad-output/planning-artifacts/epics.md`
- **Project Context**: `_bmad-output/project-context.md` (AI agent coding rules)

## Development Status

**Current Story**: Epic 0, Story 0.1 - Project Scaffolding ✅ Complete

**Next Steps**:
- Story 0.2: Comprehensive Test Infrastructure (MANDATORY before Epic 1)
- Story 0.3: Tailwind CSS & Design System Foundation
- Epic 1: Core Log Investigation Capability

See `_bmad-output/implementation-artifacts/sprint-status.yaml` for full project status.

## Performance Targets

- **Indexing**: <7 sec/GB (2-3 minutes for 20-30GB logs)
- **Query Execution**: <500ms simple queries, <750ms complex queries
- **Memory Usage**: <600 MB peak during indexing
- **UI Responsiveness**: 60 FPS scrolling with 100K+ entries

## License

MIT License - See [LICENSE](LICENSE) for details.

## Previous Version

The original Python + tkinter implementation is archived in `python-legacy/` for reference.
