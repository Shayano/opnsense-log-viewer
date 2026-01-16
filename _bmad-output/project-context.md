---
project_name: 'opnsense-log-viewer'
user_name: 'Shay'
date: '2026-01-16'
sections_completed: ['technology_stack', 'language_rules', 'framework_rules', 'testing_rules', 'code_quality', 'workflow_rules', 'critical_rules']
existing_patterns_found: 15
status: 'complete'
completedAt: '2026-01-16'
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

---

## Technology Stack & Versions

### Core Technologies

**Backend (Rust + Tauri):**
- **Tauri v2** - Desktop framework (NOT v1, breaking changes)
- **Rust 1.70+** with **Tokio 1.x** async runtime
  - ⚠️ Tokio features: `["rt-multi-thread", "fs", "io-util", "time"]` ONLY
  - ❌ DO NOT use `features = ["full"]` (bloats compile time)
- **bincode 2.0.1** - Binary serialization
- **Zstd 0.13.3** - Compression (level 1, optimized for speed)
- **reqwest 0.13.1** - HTTP client with TLS via **rustls** (NOT OpenSSL)
  - **reqwest-middleware 0.3** + **reqwest-retry 0.6** for retry logic
- **keyring 3.6.3** - OS keychain integration
  - Features: `["apple-native", "windows-native", "sync-secret-service"]`
- **sha2 0.10** - SHA-256 checksums
- **tracing 0.1** + **tracing-subscriber 0.3** - Structured logging (JSON format)
- **thiserror 2.x** + **anyhow 1.x** - Error handling (library vs application)
- **chrono 0.4** - Date/time with serde support
- **serde 1.x** + **serde_json 1.x** - Serialization

**Frontend (React + TypeScript):**
- **React 18.3+** - Modern React with concurrent features
- **TypeScript 5.7** - Strict mode REQUIRED
- **Vite 6.0+** - Dev server + build tool
- **Tailwind CSS 3.4+** - Utility-first CSS with dark mode
- **Zustand 5.0.10** - State management (lightweight)
- **TanStack Virtual 3.13.18** - Virtual scrolling for large datasets
- **React Hook Form 7.x** - Form handling
- **date-fns 3.x** - Date utilities
- **lucide-react** - Icon library
- **react-hot-toast 2.4+** - Toast notifications

**Testing & Quality:**
- **proptest 1.x** - Property-based testing (parser fuzzing)
- **criterion 0.5.x** - Performance benchmarking with HTML reports
- **Vitest 2.1+** - Frontend unit tests
- **@testing-library/react 16.1+** + **@testing-library/jest-dom 6.6+**
- **cargo-llvm-cov** - Code coverage tool

**Build & Development:**
- **Cargo** - Rust package manager
- **npm/pnpm** - Frontend package manager
- **ESLint 9.18+** + **Prettier 3.4+** - Linting and formatting
- **rustfmt** + **clippy** - Rust code quality tools

### Critical Version Constraints

⚠️ **Version Rules:**
- ❌ **NO wildcard versions** - All dependencies MUST specify exact versions (e.g., `"1.0.5"` NOT `"^1.0.0"`)
- ❌ **NO Tokio "full" features** - Use only: `["rt-multi-thread", "fs", "io-util", "time"]`
- ✅ **Zstd compression level 1** - Optimized for speed (architectural decision, NOT level 3)
- ✅ **Tauri v2 ONLY** - Not compatible with v1 API patterns
- ✅ **React 18+ required** - Uses new concurrent features
- ✅ **TypeScript strict mode** - Non-negotiable for type safety

**Compatibility Notes:**
- reqwest uses **rustls** (NOT OpenSSL) for cross-platform TLS
- keyring 3.6 requires platform-specific features enabled
- serde MUST use `#[serde(rename_all = "camelCase")]` for Rust ↔ TypeScript JSON interop

---

## Critical Implementation Rules

_To be documented in subsequent categories_

### Language-Specific Rules

#### Rust Backend Rules

**Naming Conventions (MANDATORY):**
- ✅ Functions: `snake_case` (e.g., `build_index()`, `parse_log()`, `save_credentials()`)
- ✅ Types (Struct/Enum): `PascalCase` (e.g., `LogEntry`, `IndexError`, `ApiClient`)
- ✅ Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_RETRIES`, `DEFAULT_COMPRESSION_LEVEL`)
- ✅ Modules: `snake_case` files (e.g., `indexer.rs`, `api_client.rs`)
- ❌ DO NOT use camelCase for Rust functions (`buildIndex()` is WRONG)

**Module Organization (MANDATORY):**
- ✅ Use `mod.rs` pattern for modules with submodules
- ✅ Re-export public API in `mod.rs` (e.g., `pub use inverted::InvertedIndex;`)
- ✅ Keep implementation details private (submodules can be private)
- ❌ DO NOT create flat file structure in `src/` - use hierarchical modules

**Error Handling Pattern (MANDATORY):**
- ✅ Library code: Use `thiserror` for custom error types
- ✅ Application code: Use `anyhow` for error propagation
- ✅ Tauri commands: Convert to `Result<T, String>` for IPC
- ❌ DO NOT use `panic!()` or `unwrap()` in production code

**Serde JSON Interop (CRITICAL):**
- ✅ ALWAYS use `#[serde(rename_all = "camelCase")]` for structs crossing IPC boundary
- ❌ DO NOT forget rename_all - TypeScript expects camelCase

**Async/Await Patterns:**
- ✅ Use Tokio for async operations (file I/O, HTTP requests)
- ✅ Use `.await?` for error propagation in async code
- ❌ DO NOT block async runtime with synchronous operations

**Memory Safety (CRITICAL):**
- ✅ Use streaming reads for large files (NO `readlines()` loading full file)
- ✅ Implement chunking for 30GB+ log files with `BufReader`
- ❌ DO NOT load entire files into memory (causes OOM on large logs)

#### TypeScript Frontend Rules

**Naming Conventions (MANDATORY):**
- ✅ Functions/Variables: `camelCase` (e.g., `handleFilterAdd`, `isLoading`)
- ✅ Components: `PascalCase` (e.g., `FilterBuilder`, `LogResultsTable`)
- ✅ Types/Interfaces: `PascalCase` WITHOUT "I" prefix (e.g., `LogEntry` NOT `ILogEntry`)
- ✅ Files: `kebab-case` (e.g., `filter-builder.tsx`, `use-filters.ts`)
- ✅ Boolean props: `is/has/can` prefix (e.g., `isLoading`, `hasError`)
- ✅ Event handlers: `handle` + event (e.g., `handleFilterAdd`)

**Import Order (MANDATORY):**
1. External libraries (React, third-party)
2. Internal absolute imports (`@/stores`, `@/utils`)
3. Relative imports (`./log-row`)
4. Type imports (`import type`)
5. CSS imports

**TypeScript Strict Mode (MANDATORY):**
- ✅ Use `strict: true` in tsconfig.json
- ✅ Explicit return types for functions
- ✅ No implicit `any` types
- ❌ DO NOT use `any` unless absolutely necessary (use `unknown` instead)

**Tauri IPC Invocation (CRITICAL):**
```typescript
// ✅ CORRECT - Type-safe invocation
const result = await invoke<IndexMetadata>('index_file', {
  filePath: selectedFile,  // camelCase for TypeScript
});

// ✅ Error handling pattern
try {
  const result = await invoke<T>('command_name', params);
  toast.success('Success message');
} catch (error) {
  toast.error(`Failed: ${error}`);
}
```

**React Hooks Usage:**
- ✅ Custom hooks MUST start with `use` (e.g., `useFilters()`)
- ✅ File naming: `use-filters.ts` (kebab-case)
- ❌ DO NOT call hooks conditionally or in loops

**Async/Await Patterns:**
- ✅ Use `async/await` for Tauri IPC calls
- ✅ Handle errors with try/catch + toast notifications
- ❌ DO NOT use `.then()/.catch()` chains (prefer async/await)



### Framework-Specific Rules

#### React Patterns (MANDATORY)

**Component Structure:**
- ✅ Feature-based folders: `filter-builder/`, `log-results-table/`
- ✅ Barrel exports via `index.ts` in each component folder
- ✅ Co-locate tests: `filter-builder.test.tsx` next to `filter-builder.tsx`
- ❌ DO NOT use type-based folders (`buttons/`, `inputs/`)

**Zustand State Management (CRITICAL):**
```typescript
// ✅ CORRECT - Selective subscriptions
const addFilter = useFilterStore((state) => state.addFilter);
const filters = useFilterStore((state) => state.filters);

// ✅ Store pattern - Immutable updates
addFilter: (filter) => set((state) => ({
  filters: [...state.filters, filter],
})),

// ❌ WRONG - Full store subscription (causes unnecessary re-renders)
const store = useFilterStore();
```

**TanStack Virtual for Large Datasets:**
- ✅ Use for log results table (thousands of rows)
- ✅ Variable row heights supported
- ✅ Configure overscan for smooth scrolling

**React Hook Form:**
- ✅ Use for FilterBuilder (Field + Operator + Value inputs)
- ✅ TypeScript integration with form types
- ✅ Validation with Zod or yup

#### Tauri-Specific Patterns (CRITICAL)

**IPC Commands (Backend → Frontend):**
```rust
// ✅ CORRECT - snake_case command names
#[tauri::command]
fn index_file(file_path: String, compression_level: u8) -> Result<IndexMetadata, String>

#[tauri::command]
fn execute_query(query_params: QueryParams) -> Result<QueryResult, String>
```

**IPC Events (Backend → Frontend Pub/Sub):**
```rust
// ✅ CORRECT - kebab-case event names
app.emit("indexation-progress", IndexProgress { percent: 50 });
app.emit("indexation-complete", metadata);
app.emit("indexation-error", error_message);
```

```typescript
// ✅ Frontend listens
const unlisten = await listen<IndexProgress>('indexation-progress', (event) => {
  setProgress(event.payload.percent);
});

// ✅ Cleanup on unmount
useEffect(() => {
  return () => { unlisten?.(); };
}, []);
```

**Error Display Pattern (MANDATORY):**
- ✅ Recoverable errors: `toast.error()` (non-blocking)
- ✅ Critical errors: Modal dialog (blocks user)
- ✅ Info messages: `toast.success()` or `toast.info()`
- ❌ DO NOT use `alert()` (bad UX)

### Testing Rules

**Unit Tests (MANDATORY):**
- ✅ Rust: Inline `#[cfg(test)] mod tests` in each module
- ✅ React: `*.test.tsx` co-located with components using Vitest
- ✅ Test file naming: Same as component + `.test.tsx` suffix

**Property-Based Testing (CRITICAL for Parser):**
```rust
// ✅ CORRECT - Use proptest for parser fuzzing
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_random_rfc3164_never_panics(
        timestamp in any::<u64>(),
        hostname in "[a-z]{3,10}",
        message in ".*"
    ) {
        let result = parse_rfc3164(&log_line);
        assert!(result.is_ok() || result.is_err());  // Never panic
    }
}
```

**Performance Benchmarking (CI/CD ENFORCED):**
```rust
// ✅ CORRECT - criterion benchmarks with gates
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_indexation(c: &mut Criterion) {
    let test_data = generate_1gb_logs();
    c.bench_function("index 1GB logs", |b| {
        b.iter(|| build_index(black_box(&test_data)));
    });
}
```

**Performance Gates (BUILD FAILS if exceeded):**
- ❌ Indexation >7 sec/GB (±15%)
- ❌ Query execution >750ms (±50%)
- ❌ Memory usage >600 MB (±20%)

**Integration Tests:**
- ✅ Use Tauri mock builder for IPC command tests
- ✅ Place in `tests/` directory
- ✅ Test real data flows, not implementation details

**E2E Tests (LIMITED SCOPE):**
- ✅ Only 2 critical paths: Happy path + Error path
- ❌ DO NOT write exhaustive E2E (slow, flaky)

**Code Coverage Targets:**
- ✅ Critical modules (indexer/, parser/, query/): **90%**
- ✅ Important modules (api_client/, export/): **75%**
- ✅ UI components: **60%**

### Code Quality & Style Rules

**Linting & Formatting (MANDATORY):**
- ✅ Run `cargo fmt` before every commit (Rust)
- ✅ Run `cargo clippy` and fix warnings (Rust)
- ✅ Run ESLint + Prettier before commit (TypeScript)
- ✅ Use `.editorconfig` for consistent indentation
- ❌ DO NOT commit code with linting errors

**File & Folder Structure (MANDATORY):**
```
src-tauri/src/
├── commands/          # Tauri IPC handlers
│   ├── mod.rs
│   ├── indexation.rs  # index_file, load_index
│   ├── query.rs       # execute_query
│   └── credentials.rs # save_credentials
├── indexer/           # Domain service - NO IPC knowledge
│   ├── mod.rs
│   ├── inverted.rs
│   └── bitmap.rs
```

**NO Circular Dependencies:**
- ✅ Dependency graph: `commands/` → domain services → infrastructure
- ❌ DO NOT create circular imports

**Documentation Requirements:**
- ✅ Public API functions: Doc comments with examples
- ✅ Complex algorithms: Inline comments explaining WHY
- ✅ Tauri commands: Document parameters and return types
- ❌ DO NOT over-comment obvious code

### Development Workflow Rules

**Git Workflow:**
- ✅ Branch naming: `feature/`, `fix/`, `refactor/` prefix
- ✅ Commit messages: Descriptive present tense (e.g., "Add hybrid index orchestration")
- ✅ Include co-author in commits: `Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>`

**CI/CD Pipeline (.github/workflows/):**
- ✅ `test.yml`: Unit + integration tests on every PR
- ✅ `bench.yml`: Performance benchmarks with gates enforcement
- ✅ `build.yml`: Multi-platform builds (Windows, macOS, Linux)
- ✅ `release.yml`: Automated releases with changelog

**Pre-Commit Checks:**
- ✅ Run tests locally before pushing
- ✅ Run linters (clippy, ESLint)
- ✅ Verify performance benchmarks pass gates

**PR Requirements:**
- ✅ All tests pass
- ✅ Performance gates pass
- ✅ Code coverage meets targets
- ✅ No linting errors

### Critical Don't-Miss Rules

#### Anti-Patterns to AVOID (CRITICAL)

**❌ DO NOT use full-file reads:**
```rust
// ❌ WRONG - Loads entire 30GB file into memory
let contents = fs::read_to_string(&path)?;

// ✅ CORRECT - Streaming with BufReader
let file = File::open(&path)?;
let reader = BufReader::new(file);
for line in reader.lines() {
    // Process line by line
}
```

**❌ DO NOT use Hungarian notation:**
```typescript
// ❌ WRONG
interface ILogEntry { }
type TFilter = { };

// ✅ CORRECT
interface LogEntry { }
type Filter = { };
```

**❌ DO NOT use wildcard dependencies:**
```toml
# ❌ WRONG
reqwest = "*"
serde = "^1.0"

# ✅ CORRECT
reqwest = "0.13.1"
serde = "1.0.215"
```

**❌ DO NOT use Tokio "full" features:**
```toml
# ❌ WRONG - Bloats compile time
tokio = { version = "1", features = ["full"] }

# ✅ CORRECT - Specific features only
tokio = { version = "1", features = ["rt-multi-thread", "fs", "io-util", "time"] }
```

**❌ DO NOT forget serde rename_all:**
```rust
// ❌ WRONG - JSON will have snake_case keys
#[derive(Serialize)]
struct LogEntry {
    source_ip: String,
}

// ✅ CORRECT - JSON will have camelCase keys
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LogEntry {
    source_ip: String,  // → sourceIp in JSON
}
```

#### Security Rules (MANDATORY)

**Credentials Storage:**
- ✅ MUST use OS keychain via `keyring` crate
- ❌ NEVER store credentials in plaintext files
- ❌ NEVER log credentials (not even in debug mode)

**Data Processing:**
- ✅ All processing MUST be 100% local
- ❌ NEVER send log data to external services
- ✅ Validate all inputs from external sources (API, backup JSON)

**Parsing Safety:**
- ✅ Use memory-safe Rust (no buffer overflows)
- ✅ Sandbox parser (no filesystem write outside cache)
- ✅ Property-based testing to catch edge cases

#### Performance Gotchas (CRITICAL)

**Streaming Architecture:**
- ✅ MUST use streaming for files >1GB
- ✅ Use chunked reads with `BufReader`
- ✅ LRU cache with strict memory limits (500 MB max)
- ❌ NEVER load full dataset into memory

**Zstd Compression:**
- ✅ MUST use level 1 (optimized for speed)
- ❌ DO NOT use level 3+ (diminishing returns, architectural decision)

**Query Optimization:**
- ✅ Use hybrid index (inverted + bitmap) for optimal query performance
- ✅ Execute queries against in-memory index (not raw log files)
- ✅ Implement query optimizer for complex boolean logic

**Virtual Scrolling:**
- ✅ MUST use TanStack Virtual for >1000 rows
- ✅ Render only visible rows (not entire dataset)
- ❌ DO NOT render thousands of DOM elements

#### Edge Cases to Handle

**File Handling:**
- ✅ Handle files up to 30GB without OOM
- ✅ Handle malformed log entries gracefully (no crash)
- ✅ Support multiple log formats (RFC3164, RFC5424, CSV)
- ✅ Atomic index writes (`.idx.tmp` → `.idx` rename)

**API Integration:**
- ✅ Handle API unavailable → fallback to backup JSON → raw data
- ✅ Retry logic: 3 retries, exponential backoff (100ms, 400ms, 1.6s)
- ✅ Timeout: 10s per request
- ✅ Graceful degradation (UI never blocked)

**Cross-Platform:**
- ✅ Handle path separators (Windows backslash vs Unix forward slash)
- ✅ OS-specific keychain APIs (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- ✅ Test on all platforms (Windows, macOS, Linux)

**Idempotence:**
- ✅ Re-indexing same file produces identical results (checksums verify)
- ✅ Operations can be safely retried without side effects

---

## Quick Reference

**Start Development:**
```bash
# Initialize project
npm create tauri-app@latest opnsense-log-viewer -- --template react-ts
cd opnsense-log-viewer
npm install

# Verify setup
npm run tauri dev
```

**Key Commands:**
```bash
# Backend (Rust)
cargo fmt                    # Format code
cargo clippy                 # Linting
cargo test                   # Unit tests
cargo bench                  # Performance benchmarks
cargo build --release        # Release build

# Frontend (TypeScript)
npm run lint                 # ESLint
npm run format               # Prettier
npm test                     # Vitest unit tests
npm run build                # Production build

# Full project
npm run tauri dev            # Development mode
npm run tauri build          # Production build (all platforms)
```

**Performance Gates (CI/CD):**
- Indexation: <7 sec/GB (±15%)
- Query: <750ms (±50%)
- Memory: <600 MB (±20%)

**Coverage Targets:**
- Critical modules: 90%
- Important modules: 75%
- UI components: 60%

---

**Reference Documents:**
- Architecture: `_bmad-output/planning-artifacts/architecture.md`
- PRD: `_bmad-output/planning-artifacts/prd.md`
- UX Design: `_bmad-output/planning-artifacts/ux-design-specification.md`

---

## Usage Guidelines

**For AI Agents:**

- ✅ Read this file BEFORE implementing any code
- ✅ Follow ALL rules exactly as documented
- ✅ When in doubt, prefer the more restrictive option
- ✅ Consult architecture.md for deeper context
- ✅ Update this file if new critical patterns emerge

**For Humans:**

- ✅ Keep this file lean and focused on agent needs
- ✅ Update when technology stack changes
- ✅ Review quarterly for outdated rules
- ✅ Remove rules that become obvious over time
- ✅ Add new anti-patterns as they are discovered

**Maintenance:**
- Last Updated: 2026-01-16
- Review Frequency: Quarterly or after major stack changes
- File Location: `_bmad-output/project-context.md`

