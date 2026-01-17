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
- **UI Framework**: Tailwind CSS 3.4+ with custom design tokens
- **State Management**: Zustand 5.0
- **Virtual Scrolling**: TanStack Virtual 3.13
- **Icons**: lucide-react
- **Notifications**: react-hot-toast 2.4

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
cargo test           # Run tests (Story 0.2+)
cargo bench          # Performance benchmarks (Story 0.2+)
cargo build --release # Release build
```

## Testing

This project has comprehensive test infrastructure established in **Story 0.2**:

### Frontend Testing (Vitest + React Testing Library)

```bash
npm test                 # Run unit tests with Vitest
npm run test:ui          # Open Vitest UI for interactive testing
npm run test:coverage    # Generate code coverage report
```

**Coverage Target**: 60% for UI components

### Backend Unit Testing (Rust + cargo test)

```bash
cd src-tauri
cargo test               # Run all unit tests
cargo test --release     # Run tests in release mode
cargo test parser        # Run tests for specific module
```

**Coverage Targets**:
- Critical modules (indexer/, parser/, query/): **90%**
- Important modules (api_client/, export/): **75%**
- UI components: **60%**

### Performance Benchmarking (criterion.rs)

```bash
cd src-tauri
cargo bench              # Run all benchmarks with criterion
cargo bench indexation   # Run specific benchmark suite
```

Benchmark results are generated as HTML reports in `src-tauri/target/criterion/`.

**Performance Gates (CI/CD Enforced)**:
- ❌ **BUILD FAILS** if indexation >8.05 sec/GB (7 sec/GB ±15%)
- ❌ **BUILD FAILS** if query execution >1125ms (750ms ±50%)
- ❌ **BUILD FAILS** if memory usage >720 MB (600 MB ±20%)

### Property-Based Testing (proptest)

Parser robustness is validated with property-based fuzzing:

```bash
cd src-tauri
cargo test --test '*'    # Run all tests including proptest
```

Proptest automatically generates 1000+ random test cases to verify parser never panics.

### Integration Testing (Tauri IPC)

Test Rust ↔ TypeScript communication across the IPC boundary:

```bash
npm run test:integration # Run Tauri IPC integration tests
```

Integration tests validate:
- JSON serialization/deserialization with `#[serde(rename_all = "camelCase")]`
- Error handling across IPC boundary (`Result<T, String>`)
- Type safety enforcement

### Test Fixture Generation

Generate synthetic OPNsense firewall logs (30GB) for testing:

**Windows (PowerShell)**:
```powershell
.\scripts\generate-fixtures.ps1          # Generate 3x10GB = 30GB
.\scripts\generate-fixtures.ps1 -SizeMB 1024  # Generate 3x1GB = 3GB (for CI)
```

**Linux/macOS (Bash)**:
```bash
./scripts/generate-fixtures.sh          # Generate 3x10GB = 30GB
./scripts/generate-fixtures.sh 1024     # Generate 3x1GB = 3GB (for CI)
```

Fixtures are generated in `tests/fixtures/` with realistic OPNsense data patterns:
- RFC3164 format (legacy syslog)
- RFC5424 format (modern syslog)
- CSV filterlog format (OPNsense-specific)

⚠️ **Note**: Test fixtures are gitignored (30GB files should not be committed).

### CI/CD Pipelines

Automated testing runs on every push and pull request:

- **`.github/workflows/test.yml`**: Frontend + Backend unit tests
- **`.github/workflows/bench.yml`**: Performance benchmarks with gates
- **`.github/workflows/build.yml`**: Multi-platform builds (Windows, macOS, Linux)

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

## Design System

### Tailwind CSS Configuration

The project uses a custom Tailwind CSS design system optimized for data-dense interfaces:

**Color Palette**:
- **Primary**: Professional blue scale for main actions and emphasis
- **Success (Green)**: For "pass" firewall actions and positive states
- **Warning (Orange)**: For "reject" firewall actions and warnings
- **Error (Red)**: For "block" firewall actions and error states
- **Neutral**: Gray scale (50-950) for backgrounds, borders, and text

**Typography**:
- **Monospace** (`JetBrains Mono`, `Consolas`): For technical data (IPs, timestamps, log entries)
- **Sans-serif** (`Inter`, system fonts): For UI labels and controls
- **Tight line-height** (1.3-1.5): For data-dense display

**Spacing**:
- Custom tight spacing values (2px, 4px, 8px) for data density
- Standard Tailwind spacing available for general layout

**Theme Support**:
- **Dark/Light modes** using class strategy (`dark:` variant)
- **System preference detection** on first launch
- **localStorage persistence** across sessions
- **Theme toggle** in app header

### Base Components

All base components support dark/light themes and include full accessibility (ARIA labels, keyboard navigation):

- **Button**: Primary, secondary, ghost variants with sm/md/lg sizes
- **Input**: Text and number inputs with error states and labels
- **Select**: Native dropdown with custom styling
- **Checkbox**: Custom checkbox with checkmark icon
- **Toggle**: Switch component with smooth animation
- **Modal**: Dialog with backdrop, focus trap, and ESC/click-outside close
- **ProgressBar**: Progress indicator for indexation feedback
- **Toast**: Success/error/info notifications (react-hot-toast)

**Usage Example**:
```typescript
import { Button, Input, toast } from './components/base';

// Use components with theme support
<Button variant="primary" onClick={() => toast.success('Done!')}>
  Submit
</Button>

<Input
  label="Log file path"
  placeholder="C:\logs\firewall.log"
  error={errorMsg}
/>
```

### Component Showcase

Run the development server to view the component showcase:

```bash
npm run tauri dev
```

The showcase demonstrates all base components in both light and dark themes with interactive examples.

## Development Status

**Current Story**: Epic 0, Story 0.3 - Tailwind CSS & Design System Foundation ✅ Complete

**Completed**:
- ✅ Story 0.1: Project Scaffolding with Tauri + React
- ✅ Story 0.2: Comprehensive Test Infrastructure
- ✅ Story 0.3: Tailwind CSS & Design System Foundation

**Next Steps**:
- Epic 1: Core Log Investigation Capability
- Story 1.1: File Selection with Native OS Picker
- Story 1.2: Multi-Format Log Parser

See `_bmad-output/implementation-artifacts/sprint-status.yaml` for full project status.

## Performance Targets

- **Indexing**: <7 sec/GB (±15%) - 2-3 minutes for 20-30GB logs
- **Query Execution**: <750ms (±50%) - Complex boolean queries
- **Memory Usage**: <600 MB (±20%) - Peak during indexing
- **UI Responsiveness**: 60 FPS scrolling with 100K+ entries

## License

MIT License - See [LICENSE](LICENSE) for details.

## Previous Version

The original Python + tkinter implementation is archived in `python-legacy/` for reference.
