---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments:
  - "_bmad-output/planning-artifacts/product-brief-opnsense-log-viewer-2026-01-14.md"
  - "_bmad-output/planning-artifacts/prd.md"
  - "_bmad-output/planning-artifacts/ux-design-specification.md"
  - "docs/Opnsense_doc/architecture.rst"
  - "docs/Opnsense_doc/api.rst"
  - "docs/Rust_doc/"
  - "docs/Tauriv2_doc/"
workflowType: 'architecture'
project_name: 'opnsense-log-viewer'
user_name: 'Shay'
date: '2026-01-16'
lastStep: 8
status: 'complete'
completedAt: '2026-01-16'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**

Le projet opnsense-log-viewer doit fournir une application desktop capable de:

1. **Indexation haute performance** - Traiter des fichiers de logs de 20-30 GB en 2-3 minutes avec une architecture d'indexation hybride combinant inverted multi-column index (pour IP/ports) et bitmap index (pour action/protocol/interface)

2. **Système de filtrage avancé** - Logique booléenne complète (AND/OR/NOT), opérateurs variés (equals, contains, startswith, endswith, regex), filtrage sur tous les attributs de logs avec temps de réponse <1 seconde

3. **Enrichissement via API OPNsense** - Mapping d'interfaces (vtnet0 → LAN), labels de règles firewall, résolution d'alias, avec dégradation gracieuse et support offline via backup JSON

4. **Opérations core fiables** - Parsing multi-format (RFC3164, RFC5424, CSV filterlog), opération memory-efficient sans chargement complet en RAM, export JSON/CSV avec métadonnées de compliance, persistence d'index pour réouverture rapide

5. **Application multiplateforme** - Architecture Tauri + Rust pour Windows/Linux/macOS, exécutable portable, interface moderne intuitive, zéro installation/dépendances/infrastructure serveur

**Non-Functional Requirements:**

**Performance (critiques pour le succès):**
- Indexation: <6 secondes par GB (gates CI/CD: <7 sec/GB avec ±15% tolérance)
- Recherche: <500ms pour requêtes complexes (gate: <750ms avec ±50% tolérance)
- Utilisation RAM: <500 MB pendant opérations (gate: <600 MB avec ±20% tolérance)
- Zéro crashes sur fichiers jusqu'à 30 GB (vs 100% d'échec Python au-delà de 2-3 GB)

**Sécurité:**
- Credentials API chiffrés au repos via OS keychain natif (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- Traitement 100% local, aucune donnée envoyée vers services externes
- Parsing sandboxé avec privilèges minimaux, pas d'accès réseau, pas d'écriture filesystem hors cache désigné
- Validation backup enrichment contre injection JSON malveillante
- Memory-safe parsing (avantage Rust - pas de buffer overflows)

**Conformité & Auditabilité:**
- Export avec métadonnées complètes: timestamp ISO 8601, version outil, hash SHA-256 du fichier source, critères de filtrage appliqués, opérateur identification
- Intégrité chaîne de preuve pour investigations légales/compliance
- Option anonymisation IP pour reporting GDPR
- Indexes avec checksums de détection de corruption
- Rétention configurable avec auto-expiration des indexes

**Fiabilité:**
- Opérations idempotentes (ré-indexation produit résultats identiques)
- Récupération automatique corruption d'index (cleanup indexes partiels)
- Gestion d'erreurs explicite (anti-pattern: optimistic error handling)
- Graceful degradation: API indisponible → backup enrichment → raw data

**Scale & Complexity:**

- **Domaine primaire**: Desktop Application - Log Analysis - Network Security Operations
- **Niveau de complexité**: Medium-High
  - Performance extrême requise (30 GB, <3 min indexation)
  - Integration complexe (OPNsense API multi-endpoint)
  - UX riche avec données denses mais intuitive
  - Compliance/security multi-couches

- **Composants architecturaux estimés**: 10-12 composants majeurs
  - Indexation Engine (Rust - inverted + bitmap indexes)
  - Multi-Format Parser (RFC3164/RFC5424/CSV)
  - Query Engine avec filtrage booléen complexe
  - OPNsense API Client avec retry/fallback logic
  - Enrichment Manager (API enrichment + backup JSON)
  - Frontend UI (React ou Vue + Tailwind CSS)
  - IPC Layer (Tauri commands - frontend ↔ backend)
  - Index Persistence Layer avec checksums
  - Export Engine (JSON/CSV avec métadonnées compliance)
  - Credential Manager (OS keychain integration)
  - Configuration Manager (user preferences, connection history)
  - Search History Manager

### Technical Constraints & Dependencies

**Contraintes Technologiques:**

1. **Stack imposé**: Tauri v2 + Rust backend
   - Justification: Performance critique, memory safety, portable executables
   - Documentation référence: docs/Rust_doc/, docs/Tauriv2_doc/

2. **Contrainte mémoire stricte**: <500 MB RAM
   - Implique: Architecture streaming obligatoire, pas de readlines() complet
   - Chunking intelligent pour grandes données en RAM limitée

3. **Formats de logs OPNsense**: RFC3164, RFC5424, CSV filterlog
   - Référence: docs/Opnsense_doc/ pour spécifications exactes
   - Parsing robuste avec gestion d'entrées malformées (pas de crash)

4. **OPNsense API Integration**:
   - Endpoints: `/api/diagnostics/interface/getInterfaceNames`, `/api/firewall/filter/searchRule`, `/api/firewall/alias/searchItem`
   - Authentication: API key + secret
   - Référence: docs/Opnsense_doc/api.rst
   - Contrainte: Compatibilité multi-versions OPNsense (détection via `/api/core/firmware/status`)

5. **Cross-Platform File Handling**:
   - Windows (backslashes) vs Linux/macOS (forward slashes)
   - Atomic index creation: `.idx.tmp` → `.idx` rename pattern
   - File locking pour concurrent access

**Dépendances Externes:**

- **Tauri v2**: IPC, windowing, OS integrations
- **Rust async runtime**: Tokio ou async-std pour multi-threading adaptatif
- **Serde**: Serialization JSON/CSV pour exports
- **OS Keychain libraries**: Windows (Credential Manager), macOS (Keychain), Linux (Secret Service)
- **Compression libraries**: Potentiellement pour index storage optimization
- **HTTP client**: Pour OPNsense API calls avec timeout/retry logic

### Cross-Cutting Concerns Identified

**1. Performance End-to-End**
- Impacte: Indexation engine, query engine, UI responsiveness, IPC latency
- Stratégie: CI/CD performance gates obligatoires, benchmarking continu, profiling intégré
- Risque: Régression performance = build failure

**2. Gestion Mémoire Efficace**
- Impacte: Tous les composants manipulant données volumineuses
- Stratégie: Streaming architecture, chunking intelligent, LRU caching avec limites strictes
- Anti-pattern critique: `f.readlines()` qui charge fichier complet (erreur Python app)

**3. Sécurité Multi-Couches**
- Impacte: Credential storage, API communication, parsing, file operations
- Stratégie: Defense-in-depth - OS keychain, sandboxed parsing, validation inputs, memory safety via Rust
- Compliance: Logs jamais envoyés vers externes, 100% local processing

**4. Offline-First avec Connectivité Optionnelle**
- Impacte: Architecture enrichment, UX design (graceful degradation), export capabilities
- Stratégie: Backup enrichment JSON system, visual indicators de staleness, workflow jamais bloqué par API unavailable

**5. Evidence Integrity pour Compliance**
- Impacte: Export engine, index persistence, metadata tracking
- Stratégie: Checksums, métadonnées complètes (timestamp, hash, filter criteria), exports tamper-evident
- Use case: Audits légaux/compliance nécessitent chaîne de preuve intacte

**6. Observabilité & Debugging**
- Impacte: Tous composants - troubleshooting issues de performance ou correctness
- Stratégie: Structured logging, performance metrics, validation cross-tool (vs Python app), test suite comprehensive

**7. UX Dense mais Intuitive**
- Impacte: Frontend design, IPC payload optimization, state management
- Stratégie: Virtual scrolling (render only visible), collapsible UI, progressive disclosure, theme switching (dark/light)
- Principe design: "Results First" - maximiser espace pour affichage logs critiques

## Starter Template Evaluation

### Primary Technology Domain

**Desktop Application** (Tauri v2 + Rust + React) basé sur les exigences de performance critique et portabilité multiplateforme (Windows/Linux/macOS).

### Architectural Context: Greenfield vs Brownfield

**Decision:** Projet traité comme **greenfield** bien qu'un codebase Python existe.
- Codebase Python existant (`/src/opnsense_log_viewer/`) servira de **référence** pour parsing logs et patterns
- Architecture complètement nouvelle avec Tauri + Rust (pas de migration/port du Python)
- Rationale: Exigences performance (30 GB en 2-3 min, <500ms queries) nécessitent architecture from-scratch optimisée

### Starter Options Considered

**create-tauri-app v4.6.0** (outil officiel - template minimal)
- Template: `react-ts` pour React + TypeScript
- Maintenu par Tauri core team
- Minimal boilerplate, maximum flexibilité
- Documentation complète et communauté active

**Alternative évaluée: dannysmith/tauri-template** (production-ready)
- Inclut: Vitest setup, comprehensive testing, additional tooling
- Rejeté car: Optimisé pour CRUD apps, pas pour high-performance log indexers
- Trade-off: "Unlearning overhead" de patterns over-engineered > bénéfice test setup

### Selected Starter: create-tauri-app (react-ts template)

**Rationale for Selection:**

1. **Simplicité appropriée pour custom architecture**: Projet nécessite 95% de code custom (indexation hybride inverted+bitmap, parsing multi-format RFC3164/5424/CSV, streaming 30GB). Template minimal évite dependencies inutiles et patterns à contourner.

2. **Separation of Concerns claire**: Structure `src-tauri/` (Backend Rust - Stories 1.x-5.x) vs `src/` (Frontend React - Stories 6.x+) établit frontières architecturales nettes.

3. **Flexibilité maximale**: Pas de dependencies à uninstall, pas de conventions over-engineered à bypasser. Setup exactement ce qu'on veut pour nos besoins spécifiques.

4. **Time-to-value optimal**: 2h scaffolding + 4h post-configuration (tests + Tailwind) = 6h total avant Story 1.0. Acceptable vs complexité projet (plusieurs semaines implémentation custom).

5. **Test Architecture sur-mesure**: Story 0.2 établit exactement l'infrastructure requise (cargo bench pour performance gates, property-based testing, 30GB fixtures) plutôt que template generic.

**Initialization Command:**

```bash
npm create tauri-app@latest opnsense-log-viewer -- --template react-ts
cd opnsense-log-viewer
npm install
npm run tauri dev
```

### Architectural Decisions Provided by Starter

**Language & Runtime:**
- Frontend: TypeScript (strict mode) avec React 18+
- Backend: Rust 1.70+ avec Tokio async runtime (défaut Tauri)
- Build: Vite 5+ pour frontend, Cargo pour Rust
- IPC: Tauri commands avec serde serialization (type-safe)

**Styling Solution:**
- Base: CSS modules via Vite
- PostCSS integration native
- **Post-Starter (Story 0.3):** Ajout Tailwind CSS (requis par UX Design Spec)

**Build Tooling:**
- Vite: Development server HMR, production bundling optimisé
- Cargo: Rust compilation avec profils debug/release
- Tauri CLI: Cross-platform packaging (Windows MSI, macOS DMG, Linux AppImage/deb)
- Tree-shaking automatique pour bundle size optimal

**Testing Framework:**

**Base (Starter provides):**
- Backend: `cargo test` avec Rust testing framework natif
- Aucune configuration test frontend (setup manuel requis)

**Post-Starter Configuration (Story 0.2 - MANDATORY avant Story 1.0):**

```yaml
Story 0.2: Test Infrastructure Setup
Priority: CRITICAL (bloque Stories 1.x+)
Tasks:
  - Setup Vitest + React Testing Library pour frontend
  - Setup cargo bench framework pour performance regression tests
  - Create 30GB synthetic log fixture generator (RFC3164/5424/CSV formats)
  - Configure CI/CD performance gates automatisés:
    * Indexation: <7 sec/GB (±15% tolerance) = build FAIL si dépassé
    * Query: <750ms complex queries (±50% tolerance) = build FAIL si dépassé
    * Memory: <600 MB peak (±20% tolerance) = build FAIL si dépassé
  - Setup property-based testing (proptest crate) pour parser robustness
  - Create integration test suite pour Tauri IPC commands
```

**Testing Architecture:**
- **Frontend:** Vitest (API Jest-compatible) + React Testing Library
- **Backend Unit:** `cargo test` avec assertions standards
- **Backend Performance:** `cargo bench` avec criterion.rs pour regression detection
- **Parser Robustness:** Property-based testing via `proptest` (fuzzing malformed logs)
- **E2E:** Tauri WebDriver pour validation cross-platform
- **CI/CD Enforcement:** Performance gates automatiques - régression = build failure

**Code Organization:**

```
opnsense-log-viewer/
├── src/                          # Frontend React + TypeScript
│   ├── components/               # React components
│   │   ├── FilterBuilder/        # Visual filter UI (Field+Operator+Value)
│   │   ├── LogResultsTable/      # Data-dense results display (virtual scrolling)
│   │   └── SearchHistory/        # Recent searches panel
│   ├── hooks/                    # Custom React hooks
│   ├── types/                    # TypeScript type definitions
│   ├── App.tsx                   # Root component
│   └── main.tsx                  # Entry point
├── src-tauri/                    # Backend Rust
│   ├── src/
│   │   ├── main.rs               # Tauri entry point
│   │   ├── indexer/              # Indexation engine (à créer)
│   │   │   ├── inverted.rs       # Inverted multi-column index (IPs, ports)
│   │   │   ├── bitmap.rs         # Bitmap index (action, protocol, interface)
│   │   │   └── hybrid.rs         # Hybrid orchestration
│   │   ├── parser/               # Multi-format parser (à créer)
│   │   │   ├── rfc3164.rs        # Legacy Syslog parser
│   │   │   ├── rfc5424.rs        # Modern Syslog parser
│   │   │   └── csv_filterlog.rs  # OPNsense CSV format
│   │   ├── query/                # Query engine (à créer)
│   │   │   ├── executor.rs       # Boolean query execution (AND/OR/NOT)
│   │   │   └── optimizer.rs      # Query optimization
│   │   ├── api_client/           # OPNsense API client (à créer)
│   │   │   ├── client.rs         # HTTP client avec retry logic
│   │   │   └── enrichment.rs     # Interface/rule/alias enrichment
│   │   ├── storage/              # Index persistence (à créer)
│   │   └── export/               # Export engine JSON/CSV (à créer)
│   ├── Cargo.toml                # Rust dependencies
│   ├── tauri.conf.json           # Tauri configuration
│   └── capabilities/             # Permissions & capabilities
├── tests/                        # Test suites (à créer - Story 0.2)
│   ├── unit/                     # Unit tests
│   ├── integration/              # Integration tests
│   ├── performance/              # Performance benchmarks
│   └── fixtures/                 # 30GB test data generator
├── package.json
├── vite.config.ts
└── tailwind.config.js            # À créer - Story 0.3
```

**Development Experience:**
- Hot Module Replacement via Vite (frontend changes instant)
- Rust recompilation automatique pour Tauri commands
- TypeScript IntelliSense complet avec types Tauri
- Debugging: Chrome DevTools (frontend), rust-analyzer + LLDB (backend)
- Linting: ESLint + Prettier (frontend), clippy + rustfmt (backend)

**Post-Starter Configuration Required:**

**Story 0.3: Tailwind CSS Integration** (UX Design Spec requirement)
```bash
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p
```
- Configuration Tailwind avec design tokens (colors, spacing, typography)
- Dark/light theme setup via Tailwind `dark:` variant
- Integration avec Vite build pipeline

**Initial Implementation Sequence:**

```
Story 0.1: Project Scaffolding (create-tauri-app)
Story 0.2: Test Infrastructure Setup (MANDATORY - bloque Stories 1.x+)
Story 0.3: Tailwind CSS Integration (UX requirement)
Story 1.0: Core Indexation Engine - Inverted Index
Story 1.1: Core Indexation Engine - Bitmap Index
Story 1.2: Hybrid Index Orchestration
Story 2.0: Multi-Format Parser (RFC3164/5424/CSV)
...
```

**Note:** Stories 0.2-0.3 sont des **prerequisites critiques** établissant fondations testing et styling avant implémentation features core.

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**

1. **Index Persistence Format**: bincode 2.0.1 - Bloque module storage, requis pour Stories 1.x
2. **Compression Strategy**: Zstd 0.13.3 (level 1 initial, tunable) - Impacte performance indexation
3. **HTTP Client**: reqwest 0.13.1 - Bloque OPNsense API integration (Story 3.x)
4. **OS Keychain**: keyring 3.6.3 - Bloque credential management sécurisé
5. **State Management**: Zustand 5.0.10 - Bloque frontend architecture
6. **Virtual Scrolling**: TanStack Virtual 3.13.18 - Bloque results display (Story 6.x)
7. **Error Handling UX**: react-hot-toast - Bloque error display patterns (toutes Stories)

**Important Decisions (Shape Architecture):**

1. **Async Runtime**: Tokio 1.x avec features optimisées
2. **Logging**: tracing 0.1.x + tracing-subscriber
3. **Error Types**: thiserror 2.x + anyhow 1.x
4. **Form Handling**: React Hook Form 7.x
5. **Date/Time**: date-fns 3.x
6. **Icons**: lucide-react

**Deferred Decisions (Post-MVP):**

1. **CLI Arguments**: clap v4 si demandé par users
2. **Multi-Instance Credentials**: Support multiple OPNsense firewalls
3. **Advanced Visualizations**: Charts, graphs, timelines
4. **Plugin Architecture**: Extensibility pour custom enrichment sources

### Data Architecture

**Decision 1: Index Persistence Format**

**Choice**: **bincode 2.0.1**

**Rationale**:
- Performance native Rust, serialization/deserialization optimales
- Integration seamless avec serde (déjà utilisé pour Tauri IPC)
- Format compact et rapide pour indexes 30GB+
- Mature et bien maintenu (387M+ downloads crates.io)

**Configuration**:
```rust
use bincode::{serialize, deserialize};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct PersistedIndex {
    version: u32,
    checksum: [u8; 32],
    inverted_index: InvertedIndex,
    bitmap_index: BitmapIndex,
    metadata: IndexMetadata,
}
```

**Affects**: Module `src-tauri/src/storage/`

---

**Decision 2: Index Compression**

**Choice**: **Zstd 0.13.3** (compression level 1, tunable via profiling)

**Rationale**:
- Excellent ratio compression/vitesse pour production use
- Level 1: Optimisation vitesse (vs level 3 original), ratio ~2x acceptable
- Décompression rapide (<50ms pour 1GB sur hardware moderne)
- Ajustement post-profiling avec 30GB real data si needed
- Production-proven: Facebook, PostgreSQL, kernel Linux

**Configuration**:
```rust
use zstd::stream::{Encoder, Decoder};

// Compression level 1 (fast), tunable
let mut encoder = Encoder::new(output_file, 1)?;
encoder.include_checksum(true)?;
bincode::serialize_into(&mut encoder, &index)?;
encoder.finish()?;
```

**Trade-off Analysis**:
- Performance: +~200ms compression overhead acceptable vs NFR <7s/GB
- Disk space: 30GB logs → ~5-10GB index → ~2-5GB compressed (net savings)
- Rationale level 1 vs 3: Diminishing returns sur desktop hardware (Party Mode review - Amelia)

**Affects**: Module `src-tauri/src/storage/persistence.rs`

---

**Decision 3: Checksum & Integrity**

**Choice**: **SHA-256** via `sha2` crate

**Rationale**:
- Detect index corruption (partial writes, disk errors)
- Detect source file modification post-indexation
- Compliance requirement (evidence integrity)

**Implementation**:
```rust
use sha2::{Sha256, Digest};

struct IndexMetadata {
    source_file_hash: [u8; 32],  // SHA-256 of original log file
    index_hash: [u8; 32],         // SHA-256 of serialized index
    created_at: DateTime<Utc>,
    source_file_path: PathBuf,
}
```

**Affects**: Modules `storage/`, `export/`

### Backend Architecture & Dependencies

**Decision 4: HTTP Client & Retry Logic**

**Choice**: **reqwest 0.13.1** + **reqwest-middleware 0.3** + **reqwest-retry 0.6**

**Configuration**:
```toml
[dependencies]
reqwest = { version = "0.13", features = ["json", "rustls-tls"] }
reqwest-middleware = "0.3"
reqwest-retry = "0.6"
```

**Retry Policy** (ajouté suite Party Mode - Amelia):
- **Max retries**: 3
- **Backoff strategy**: Exponential (100ms, 400ms, 1.6s)
- **Timeout per request**: 10s
- **Retry conditions**: Network errors, 5xx status codes, timeouts
- **No retry**: 4xx client errors (immediate fail)

**Rationale**:
- API ergonomique avec async/await (compatible Tokio)
- Retry logic robuste pour OPNsense API sous load
- TLS via rustls (pas OpenSSL dependency), cross-platform simple
- JSON deserialization intégrée via serde
- Production-ready: utilisé par AWS SDK, Kubernetes clients

**Implementation Example**:
```rust
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

fn build_api_client() -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(Duration::from_millis(100), Duration::from_millis(1600))
        .build_with_max_retries(3);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}
```

**Affects**: Module `src-tauri/src/api_client/`

---

**Decision 5: Credential Storage**

**Choice**: **keyring 3.6.3**

**Configuration**:
```toml
[dependencies]
keyring = { version = "3.6", features = ["apple-native", "windows-native", "sync-secret-service"] }
```

**Rationale**:
- Cross-platform: Windows Credential Manager, macOS Keychain, Linux Secret Service
- API uniforme, single crate pour tous OS
- Satisfait NFR sécurité (OS keychain natif, pas plaintext)

**MVP Scope** (ajouté suite Party Mode - Amelia):
- **Single OPNsense instance** credentials seulement
- Multi-instance support = post-MVP (évite Windows 1KB credential limit issues)

**Implementation**:
```rust
use keyring::Entry;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct ApiCredentials {
    hostname: String,
    api_key: String,
    api_secret: String,
}

fn save_credentials(creds: &ApiCredentials) -> Result<()> {
    let entry = Entry::new("opnsense-log-viewer", "api-credentials")?;
    let json = serde_json::to_string(creds)?;
    entry.set_password(&json)?;
    Ok(())
}
```

**Affects**: Module `src-tauri/src/credentials/`

---

**Decision 6: Async Runtime**

**Choice**: **Tokio 1.x** (features optimisées - ajusté suite Party Mode - Barry)

**Configuration**:
```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "fs", "io-util", "time"] }
```

**Rationale Optimisation**:
- ❌ **Pas** `features = ["full"]` (100+ features, slow compile times)
- ✅ **Seulement** features nécessaires:
  - `rt-multi-thread`: Multi-threaded runtime (adaptive threading)
  - `fs`: Async file I/O (log reading)
  - `io-util`: Streaming utilities (chunked reads)
  - `time`: Timers, delays (retry backoff)

**Impact**: Compile time optimization (~30% faster builds), developer experience

**Affects**: Tous modules async (indexer, parser, api_client)

---

**Decision 7: Logging & Observability**

**Choice**: **tracing 0.1.x** + **tracing-subscriber 0.3.x**

**Configuration**:
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
```

**Log Levels**:
- **ERROR**: Crashes, data corruption, critical failures
- **WARN**: Recoverable errors, performance degradation, API failures
- **INFO**: Indexation progress, search queries, API calls
- **DEBUG**: Detailed execution flow, performance metrics
- **TRACE**: Ultra-verbose (development only)

**Format**: JSON structured logs pour parsing automatique

**Implementation**:
```rust
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(true)
        .init();
}
```

**Affects**: Tous modules

---

**Decision 8: Error Handling**

**Choice**: **thiserror 2.x** + **anyhow 1.x**

**Pattern**:
- **thiserror**: Define custom error types (library code, modules exposés)
- **anyhow**: Error propagation (application code, internal logic)

**Implementation**:
```rust
// Library code (parser module)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid RFC3164 format: {0}")]
    InvalidRfc3164(String),

    #[error("Malformed timestamp: {0}")]
    InvalidTimestamp(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Application code (main.rs, commands)
use anyhow::{Context, Result};

fn index_file(path: PathBuf) -> Result<Index> {
    let logs = parse_logs(&path)
        .context("Failed to parse log file")?;

    build_index(logs)
        .context("Failed to build index")
}
```

**Affects**: Tous modules

### Frontend Architecture

**Decision 9: State Management**

**Choice**: **Zustand 5.0.10**

**Rationale**:
- Minimal boilerplate (vs Redux verbosité)
- Performance excellente (selective re-renders)
- TypeScript support natif
- DevTools via browser extension

**Store Architecture**:

```typescript
// FilterStore
interface FilterStore {
  filters: Filter[];
  addFilter: (filter: Filter) => void;
  removeFilter: (id: string) => void;
  clearFilters: () => void;
  executeSearch: () => void;
}

// SearchHistoryStore
interface SearchHistoryStore {
  history: SearchQuery[];
  addToHistory: (query: SearchQuery) => void;
  rerunQuery: (id: string) => void;
}

// UIPreferencesStore
interface UIPreferencesStore {
  theme: 'light' | 'dark';
  sidebarCollapsed: boolean;
  toggleTheme: () => void;
  toggleSidebar: () => void;
}
```

**Affects**: Tous composants React frontend

---

**Decision 10: Virtual Scrolling**

**Choice**: **TanStack Virtual 3.13.18**

**Rationale**:
- Performance optimale pour milliers de lignes logs
- Variable row heights support (logs multi-lignes)
- TypeScript first-class, integration Tailwind facile
- Moderne (2024+), bien maintenu

**Implementation**:
```typescript
import { useVirtualizer } from '@tanstack/react-virtual';

function LogResultsTable({ results }: { results: LogEntry[] }) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: results.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 35, // Estimated row height
    overscan: 10,
  });

  return (
    <div ref={parentRef} className="h-screen overflow-auto">
      <div style={{ height: `${virtualizer.getTotalSize()}px` }}>
        {virtualizer.getVirtualItems().map((virtualRow) => (
          <LogRow key={virtualRow.key} entry={results[virtualRow.index]} />
        ))}
      </div>
    </div>
  );
}
```

**Affects**: Component `LogResultsTable`

---

**Decision 11: Error Display Pattern**

**Choice**: **react-hot-toast** (ajouté suite Party Mode - Barry)

**Rationale**:
- CRITIQUE: Pattern manquant identifié en Party Mode
- Lightweight, non-blocking notifications
- TypeScript support, Tailwind-compatible
- Customizable (success, error, warning, loading states)

**Pattern**:
```typescript
import toast from 'react-hot-toast';

// Backend error via Tauri IPC
try {
  await invoke('index_file', { path });
} catch (error) {
  toast.error(`Indexation failed: ${error}`);
}

// Success feedback
toast.success('Index created successfully');

// Loading state
const toastId = toast.loading('Indexing file...');
// ... async operation
toast.success('Done!', { id: toastId });
```

**Error Categories**:
- **Critical**: Modal dialog (blocker errors)
- **Important**: Toast error (recoverable, user action needed)
- **Info**: Toast info (FYI, no action needed)

**Affects**: Tous composants React, module error handling

---

**Decision 12: Form Handling**

**Choice**: **React Hook Form 7.x**

**Rationale**:
- Minimal re-renders (performance)
- TypeScript support excellent
- Validation intégrée
- Perfect for filter builder (Field + Operator + Value inputs)

**Affects**: Component `FilterBuilder`

---

**Decision 13: Date/Time Library**

**Choice**: **date-fns 3.x**

**Rationale**:
- Lightweight (vs moment.js), tree-shakeable
- Immutable (functional style)
- Time range filtering, timestamp formatting
- TypeScript support natif

**Affects**: Components time range filters, timestamp display

---

**Decision 14: Icon Library**

**Choice**: **lucide-react**

**Rationale**:
- Modern, consistent design
- Lightweight, tree-shakeable
- TypeScript support
- Icons needed: Filter, Search, Download, Settings, X, ChevronDown, etc.

**Affects**: Tous composants UI

### Testing Strategy

**Decision 15: Property-Based Testing**

**Choice**: **proptest 1.x**

**Rationale**:
- Fuzzing parser avec logs malformés (RFC3164/5424/CSV)
- Generate random IPs, timestamps, protocols, actions
- Catch edge cases impossibles à anticiper manuellement
- Stratégie anti-crash critique (NFR: 0% crash rate)

**Learning Curve Note** (Party Mode - Barry):
- ⚠️ Team training: 2 jours apprentissage property-based testing patterns
- Factor dans Story 0.2 timeline (12h réaliste vs 4h initial)

**Implementation Example**:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_random_rfc3164_never_panics(
        timestamp in any::<u64>(),
        hostname in "[a-z]{3,10}",
        message in ".*"
    ) {
        let log_line = format!("<134>{} {} {}", timestamp, hostname, message);
        let result = parse_rfc3164(&log_line);
        // Should return Ok or Err, never panic
        assert!(result.is_ok() || result.is_err());
    }
}
```

**Affects**: Module `parser/` tests

---

**Decision 16: Performance Benchmarking**

**Choice**: **criterion.rs 0.5.x**

**Rationale**:
- Statistical benchmarking (mean, median, std dev)
- Detect performance regressions automatiquement
- CI/CD integration pour enforce performance gates
- HTML reports avec graphs

**Learning Curve Note** (Party Mode - Barry):
- ⚠️ Similar à proptest: 1-2 jours apprentissage benchmarking patterns
- Factor dans Story 0.2 timeline

**Implementation**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_indexation(c: &mut Criterion) {
    let test_data = generate_1gb_logs();

    c.bench_function("index 1GB logs", |b| {
        b.iter(|| {
            let index = build_index(black_box(&test_data));
            black_box(index);
        });
    });
}

criterion_group!(benches, benchmark_indexation);
criterion_main!(benches);
```

**Performance Gates** (CI/CD):
- Indexation: <7 sec/GB (±15%)
- Query: <750ms complex queries (±50%)
- Memory: <600 MB peak (±20%)

**Affects**: All performance-critical modules

---

**Decision 17: Test Fixtures Strategy**

**Choice**: **Streaming Generator** (Option C - ajouté suite Party Mode - Murat)

**Rationale**:
- ❌ Pas de 30GB file pré-généré (LFS storage, CI download overhead)
- ❌ Pas de génération on-demand disque (I/O bottleneck, 2-3 min CI delay)
- ✅ **Streaming generator**: Génère logs en mémoire, feed directement à indexer
- Aligns avec NFR memory constraints (<500 MB)

**Implementation**:
```rust
struct StreamingLogGenerator {
    format: LogFormat,
    lines_generated: u64,
    target_size_gb: u64,
}

impl Iterator for StreamingLogGenerator {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        if self.size_reached() {
            return None;
        }
        Some(self.generate_random_log_line())
    }
}

// Usage in tests
let generator = StreamingLogGenerator::new(LogFormat::RFC3164, 30);
let index = build_index_streaming(generator)?;
```

**Affects**: Module `tests/fixtures/`

---

**Decision 18: Integration Test Strategy**

**Choice**: **Tauri mock builder** (ajouté suite Party Mode - Murat)

**Implementation**:
```rust
#[cfg(test)]
mod tests {
    use tauri::test::{mock_builder, mock_context};

    #[test]
    fn test_index_file_command() {
        let app = mock_builder()
            .invoke_handler(tauri::generate_handler![index_file])
            .build(mock_context())
            .unwrap();

        // Test IPC command without full Tauri runtime
        let result = app.invoke("index_file", json!({ "path": "/test/logs" }));
        assert!(result.is_ok());
    }
}
```

**Affects**: Tous Tauri commands tests

---

**Decision 19: E2E Test Scope**

**Choice**: **Limited Critical Paths** (ajouté suite Party Mode - Murat)

**Rationale**:
- ❌ Pas exhaustive E2E (slow, flaky, expensive CI)
- ✅ 2 critical paths seulement:
  1. **Happy Path**: Load 1GB file → index → filter → export
  2. **Error Path**: Load corrupt file → graceful error display

**Skip**:
- Exhaustive filter combinations (covered by unit tests)
- UI styling variations (visual regression out of MVP scope)

**Implementation**: Tauri WebDriver

**Affects**: Story E2E tests (post-core implementation)

---

**Decision 20: Code Coverage Targets**

**Choice**: **cargo-llvm-cov** avec targets précis (ajouté suite Party Mode - Murat)

**Coverage Targets**:
- **Critical modules** (indexer/, parser/, query/): **90%**
- **Important modules** (api_client/, export/): **75%**
- **UI components**: **60%**

**Rationale**:
- Précision vs vague ">80% modules critiques"
- Différenciation selon criticité business
- Enforcement via CI/CD

**Configuration**:
```toml
# .cargo/config.toml
[target.'cfg(coverage)']
rustflags = ["-C", "instrument-coverage"]
```

**Affects**: CI/CD pipeline, Story 0.2 setup

### Infrastructure & Deployment

**Decision 21: CI/CD Platform**

**Choice**: **GitHub Actions**

**Rationale**:
- Native integration GitHub repo
- Matrix builds (Windows/Linux/macOS) simples
- Performance gates enforcement via workflow
- Free pour open source

**Workflows**:
1. **test.yml**: Unit + integration tests sur chaque PR
2. **bench.yml**: Performance benchmarks, enforce gates
3. **build.yml**: Cross-platform builds
4. **release.yml**: Packaging + publish releases

**Performance Gates Workflow**:
```yaml
- name: Run benchmarks
  run: cargo bench --all-features

- name: Check performance gates
  run: |
    # Parse criterion output
    # Fail if >7s/GB indexation or >750ms query
```

**Affects**: Repository CI/CD

---

**Decision 22: Release Packaging**

**Choice**: **Tauri bundler natif**

**Outputs**:
- **Windows**: MSI installer (code signing post-MVP)
- **macOS**: DMG + universal binary (Apple Silicon + Intel)
- **Linux**: AppImage, .deb, .rpm

**Configuration**: `tauri.conf.json`

```json
{
  "bundle": {
    "active": true,
    "targets": ["msi", "dmg", "appimage", "deb"],
    "identifier": "com.opnsense-log-viewer",
    "icon": ["icons/icon.png"]
  }
}
```

**Affects**: Release process

---

**Decision 23: Version Management**

**Choice**: **Semantic Versioning** + **cargo-release**

**Pattern**: MAJOR.MINOR.PATCH
- MAJOR: Breaking changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes

**Automation**: cargo-release pour bump versions, tags, changelog

**Affects**: Release process

### Complete Dependencies Summary

**Cargo.toml (Backend Rust)**:

```toml
[package]
name = "opnsense-log-viewer"
version = "0.1.0"
edition = "2021"

[dependencies]
# Tauri Core
tauri = { version = "2", features = ["protocol-asset", "dialog-open"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Async Runtime (optimized features)
tokio = { version = "1", features = ["rt-multi-thread", "fs", "io-util", "time"] }

# HTTP & API
reqwest = { version = "0.13", features = ["json", "rustls-tls"] }
reqwest-middleware = "0.3"
reqwest-retry = "0.6"

# Security
keyring = { version = "3.6", features = ["apple-native", "windows-native", "sync-secret-service"] }

# Storage & Serialization
bincode = "2"
zstd = "0.13"
sha2 = "0.10"

# Logging & Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }

# Error Handling
thiserror = "2"
anyhow = "1"

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
# Testing
proptest = "1"
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "indexation_benchmark"
harness = false
```

**package.json (Frontend React)**:

```json
{
  "name": "opnsense-log-viewer",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build",
    "test": "vitest"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "@tanstack/react-virtual": "^3.13.18",
    "zustand": "^5.0.10",
    "react-hook-form": "^7.53.2",
    "date-fns": "^3.6.0",
    "lucide-react": "^0.468.0",
    "react-hot-toast": "^2.4.1"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.3.4",
    "typescript": "^5.7.2",
    "vite": "^6.0.5",
    "@tauri-apps/cli": "^2.1.0",
    "tailwindcss": "^3.4.17",
    "postcss": "^8.4.49",
    "autoprefixer": "^10.4.20",
    "vitest": "^2.1.8",
    "@testing-library/react": "^16.1.0",
    "@testing-library/jest-dom": "^6.6.3",
    "eslint": "^9.18.0",
    "prettier": "^3.4.2"
  }
}
```

### Decision Impact Analysis

**Implementation Sequence (Ordered by Dependencies)**:

**Phase 0: Foundation (Stories 0.1-0.3)**
1. ✅ Project scaffolding (create-tauri-app)
2. ✅ Test infrastructure (proptest, criterion, fixtures) - **12h realistic**
3. ✅ Tailwind CSS + design tokens - **6h realistic**
4. ✅ Error handling pattern (react-hot-toast integration)

**Phase 1: Backend Core (Stories 1.x-3.x)**
5. Storage layer (bincode + zstd) - Depends on: #2 (tests)
6. Parser (RFC3164/5424/CSV) - Depends on: #5, #2 (proptest)
7. Indexer (inverted + bitmap) - Depends on: #6, #5
8. Query engine - Depends on: #7
9. API client (reqwest + retry) - Depends on: #2
10. Credential manager (keyring) - Depends on: #9

**Phase 2: Frontend Core (Stories 6.x+)**
11. State management (Zustand stores) - Depends on: #4
12. Filter builder UI - Depends on: #11, #3 (Tailwind)
13. Results table (TanStack Virtual) - Depends on: #11
14. Search history - Depends on: #11
15. Export functionality - Depends on: #8, #11

**Phase 3: Integration (Stories 8.x+)**
16. IPC layer (Tauri commands) - Depends on: #8, #11
17. E2E tests (critical paths) - Depends on: ALL above
18. CI/CD (GitHub Actions + gates) - Depends on: #2 (benchmarks)

**Cross-Component Dependencies**:

**bincode → storage → indexer → query → IPC → frontend**
- Storage format impacte tous modules downstream
- Change = cascade rebuild indexes

**reqwest → api_client → enrichment → frontend display**
- Retry policy impacte UX (loading states durée)
- Error handling pattern flows to toast notifications

**Zustand → tous components React**
- State architecture impacte component design
- Store changes = component refactoring

**tracing → tous modules**
- Log format (JSON) impacte debugging workflow
- Filter levels = performance impact production

**Tailwind → tous components**
- Design tokens = consistent theming
- Dark mode = all components must support

### Architecture Review Summary (Party Mode Insights)

**Critical Gaps Resolved**:
1. ✅ Retry policy specified (3 retries, exponential backoff, 10s timeout)
2. ✅ Multi-instance credentials deferred to post-MVP
3. ✅ Error handling UX pattern defined (react-hot-toast)
4. ✅ Integration test strategy (Tauri mock builder)
5. ✅ 30GB fixture strategy (streaming generator)
6. ✅ E2E scope limited (2 critical paths)

**Optimizations Applied**:
1. ✅ Zstd level 1 (vs 3) - profile-driven tuning
2. ✅ Tokio features optimized (compile time improvement)
3. ✅ Coverage targets précisés (90%/75%/60% par module type)
4. ✅ Story time estimates ajustés (learning curves factored)

**Risk Mitigations**:
1. ✅ Windows credential 1KB limit acknowledged (single instance MVP)
2. ✅ proptest/criterion learning curve factored (Story 0.2: 12h)
3. ✅ Tailwind dark mode complexity recognized (Story 0.3: 6h)
4. ✅ Performance benchmarking strategy seeded RNG (deterministic)

**Architecture Confidence Level**: **HIGH**
- All critical decisions made and validated
- Implementation blockers removed
- Dependencies clearly mapped
- Timeline realistic avec learning curves

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:** 15 zones où différents agents AI pourraient faire des choix incompatibles

**Purpose:** Ces patterns assurent que tous les agents AI (dev, quick-flow, etc.) écrivent du code cohérent et compatible. Chaque pattern répond à la question: "Comment éviter que 2 agents prennent des décisions différentes pour la même situation?"

---

### Naming Patterns

#### Backend Rust Naming Conventions

**Module Naming:**
- ✅ **snake_case** (Rust standard)
- Files: `indexer.rs`, `api_client.rs`, `parser.rs`, `storage.rs`
- Module declarations: `mod indexer;`, `mod api_client;`
- ❌ **NOT**: `indexation.rs`, `apiClient.rs`, `Indexer.rs`

**Function Naming:**
- ✅ **snake_case verbs**
- `build_index()`, `parse_log()`, `save_credentials()`, `execute_query()`
- **Semantic prefixes**:
  - `create_`: Allocate new instance (e.g., `create_index()`)
  - `build_`: Construct from components (e.g., `build_index()`)
  - `get_`: Retrieve existing data (e.g., `get_credentials()`)
  - `save_`: Persist to storage (e.g., `save_index()`)
  - `load_`: Read from storage (e.g., `load_index()`)
- ❌ **NOT**: `buildIndex()`, `BuildIndex()`, `make_index()`

**Struct/Enum/Type Naming:**
- ✅ **PascalCase**
- Structs: `LogEntry`, `IndexMetadata`, `ApiClient`, `FilterQuery`
- Enums: `LogFormat`, `IndexType`, `QueryOperator`
- Error types: `ParseError`, `IndexError`, `ApiError` (Error suffix)
- ❌ **NOT**: `log_entry`, `API_Client`, `Indexerror`

**Constant Naming:**
- ✅ **SCREAMING_SNAKE_CASE**
- `MAX_RETRIES`, `DEFAULT_COMPRESSION_LEVEL`, `API_TIMEOUT_SECS`
- `INDEX_FILE_EXTENSION`, `LOG_BUFFER_SIZE`
- ❌ **NOT**: `maxRetries`, `Max_Retries`, `max_retries`

**Example (Rust module):**
```rust
// ✅ GOOD
mod indexer;

use crate::parser::LogEntry;

const MAX_INDEX_SIZE: usize = 1_000_000;

pub struct IndexMetadata {
    source_file_hash: [u8; 32],
    created_at: DateTime<Utc>,
}

pub enum IndexError {
    Io(std::io::Error),
    InvalidFormat(String),
}

pub fn build_index(entries: Vec<LogEntry>) -> Result<Index, IndexError> {
    // ...
}
```

---

#### Frontend React/TypeScript Naming Conventions

**Component Naming:**
- ✅ **PascalCase** component name, **kebab-case** file name
- Component: `FilterBuilder`, `LogResultsTable`, `SearchHistory`
- File: `filter-builder.tsx`, `log-results-table.tsx`, `search-history.tsx`
- Rationale: React convention + URL-friendly filenames
- ❌ **NOT**: `filterBuilder.tsx`, `FilterBuilder.tsx`, `filter_builder.tsx`

**Hook Naming:**
- ✅ **use + PascalCase descriptive name**
- `useFilters()`, `useSearchHistory()`, `useVirtualScrolling()`
- Custom hooks ALWAYS start with `use` (React rule)
- File: `use-filters.ts`, `use-search-history.ts`
- ❌ **NOT**: `filters()`, `getFilters()`, `filterHook()`

**Type/Interface Naming:**
- ✅ **PascalCase, NO I prefix**
- `LogEntry`, `FilterState`, `SearchQuery`, `ApiResponse`
- ❌ **NOT**: `ILogEntry`, `TFilterState`, `log_entry`
- Rationale: Modern TypeScript convention, I-prefix is outdated Hungarian notation

**Props Naming:**
- ✅ **camelCase**
- `onFilterChange`, `isLoading`, `results`, `className`
- Boolean props: `isLoading`, `hasError`, `canSubmit` (is/has/can prefix)
- ❌ **NOT**: `OnFilterChange`, `is_loading`, `Results`

**Event Handler Naming:**
- ✅ **handle + PascalCase event**
- `handleFilterAdd`, `handleSearchExecute`, `handleExport`, `handleDelete`
- Pattern: `handle` + noun + verb
- ❌ **NOT**: `onFilterAdd` (that's the prop name), `filterAddHandler`

**Store Naming:**
- ✅ **use + Name + Store**
- `useFilterStore`, `useSearchHistoryStore`, `useUIPreferencesStore`
- File: `filter-store.ts`, `search-history-store.ts`
- ❌ **NOT**: `filterStore`, `FilterStore`, `useFilters`

**Example (React component):**
```typescript
// ✅ GOOD
// File: filter-builder.tsx

import { useState } from 'react';
import { useFilterStore } from '@/stores/filter-store';
import type { Filter, FilterOperator } from '@/types';

interface FilterBuilderProps {
  onFilterChange: (filters: Filter[]) => void;
  isDisabled?: boolean;
}

export function FilterBuilder({ onFilterChange, isDisabled }: FilterBuilderProps) {
  const { filters, addFilter } = useFilterStore();
  const [selectedOperator, setSelectedOperator] = useState<FilterOperator>('equals');

  const handleFilterAdd = () => {
    addFilter({ operator: selectedOperator, value: '' });
    onFilterChange(filters);
  };

  return (
    <div className="filter-builder">
      {/* ... */}
    </div>
  );
}
```

---

#### IPC Tauri Command Naming Conventions

**Command Names:**
- ✅ **snake_case** (Rust backend defines)
- `index_file`, `execute_query`, `save_credentials`, `get_interfaces`
- Pattern: verb + noun
- ❌ **NOT**: `indexFile`, `ExecuteQuery`, `index-file`

**Command Parameters:**
- ✅ **snake_case** (Rust convention)
- `file_path`, `query_params`, `credential_data`
- ❌ **NOT**: `filePath`, `queryParams`

**Event Names:**
- ✅ **kebab-case** (Web convention)
- `indexation-progress`, `query-complete`, `api-error`, `index-created`
- Pattern: noun + verb (past tense for completion events)
- ❌ **NOT**: `indexation_progress`, `IndexationProgress`, `indexationProgress`

**Example (Tauri IPC):**
```rust
// ✅ GOOD Backend (Rust)
#[tauri::command]
fn index_file(file_path: String, compression_level: u8) -> Result<IndexMetadata, String> {
    // ...
    app.emit("indexation-progress", IndexProgress { percent: 50 });
    // ...
}
```

```typescript
// ✅ GOOD Frontend (TypeScript)
const result = await invoke<IndexMetadata>('index_file', {
  filePath: selectedFile, // camelCase for TypeScript
  compressionLevel: 1,
});

const unlisten = await listen<IndexProgress>('indexation-progress', (event) => {
  setProgress(event.payload.percent);
});
```

---

### Structure Patterns

#### Backend Rust Project Structure

**Mandatory Structure:**

```
src-tauri/src/
├── main.rs                    # Tauri entry point, app builder
├── commands/                  # Tauri command handlers (IPC)
│   ├── mod.rs
│   ├── indexation.rs          # index_file, load_index commands
│   ├── query.rs               # execute_query, get_results commands
│   └── credentials.rs         # save_credentials, get_credentials commands
├── indexer/                   # Indexation engine
│   ├── mod.rs                 # Public API exports
│   ├── inverted.rs            # Inverted index implementation
│   ├── bitmap.rs              # Bitmap index implementation
│   └── hybrid.rs              # Hybrid orchestration
├── parser/                    # Log parsers
│   ├── mod.rs
│   ├── rfc3164.rs             # RFC3164 parser
│   ├── rfc5424.rs             # RFC5424 parser
│   └── csv_filterlog.rs       # OPNsense CSV parser
├── query/                     # Query execution
│   ├── mod.rs
│   ├── executor.rs            # Query execution logic
│   └── optimizer.rs           # Query optimization
├── api_client/                # OPNsense API client
│   ├── mod.rs
│   ├── client.rs              # HTTP client with retry
│   └── enrichment.rs          # Data enrichment logic
├── storage/                   # Index persistence
│   ├── mod.rs
│   └── persistence.rs         # Bincode + Zstd persistence
├── export/                    # Export engine
│   ├── mod.rs
│   └── metadata.rs            # Export metadata generation
├── credentials/               # Credential management
│   ├── mod.rs
│   └── manager.rs             # Keyring integration
└── errors.rs                  # Custom error types (ParseError, IndexError, etc.)

tests/                         # Integration tests
├── indexation_tests.rs
├── query_tests.rs
└── fixtures/
    └── log_generator.rs

benches/                       # Criterion benchmarks
└── indexation_benchmark.rs
```

**Module Organization Rules:**
- ✅ One module = one directory with `mod.rs`
- ✅ `mod.rs` exports public API, re-exports submodules
- ✅ Submodules are implementation details (can be private)
- ❌ **NOT**: Flat structure with all files in `src/`

**Test Location Rules:**
- ✅ **Unit tests**: Inline `#[cfg(test)] mod tests` in same file
- ✅ **Integration tests**: `tests/` folder (test external API)
- ✅ **Benchmarks**: `benches/` folder with `harness = false`

**Example module structure:**
```rust
// ✅ GOOD: src-tauri/src/indexer/mod.rs
pub mod inverted;
pub mod bitmap;
pub mod hybrid;

// Re-export public types
pub use inverted::InvertedIndex;
pub use bitmap::BitmapIndex;
pub use hybrid::HybridIndex;

// Module-level types
pub struct IndexMetadata {
    // ...
}
```

---

#### Frontend React Project Structure

**Mandatory Structure:**

```
src/
├── main.tsx                   # Entry point, React render
├── App.tsx                    # Root component, routing
├── components/                # React components (feature-based)
│   ├── filter-builder/
│   │   ├── filter-builder.tsx      # Main component
│   │   ├── filter-row.tsx          # Subcomponent
│   │   ├── filter-builder.test.tsx # Tests
│   │   └── index.ts                # Export barrel
│   ├── log-results-table/
│   │   ├── log-results-table.tsx
│   │   ├── log-row.tsx
│   │   ├── log-row.test.tsx
│   │   └── index.ts
│   └── search-history/
│       ├── search-history.tsx
│       ├── history-item.tsx
│       └── index.ts
├── hooks/                     # Custom React hooks
│   ├── use-filters.ts
│   ├── use-search-history.ts
│   ├── use-virtual-scrolling.ts
│   └── use-tauri-event.ts
├── stores/                    # Zustand stores
│   ├── filter-store.ts
│   ├── search-history-store.ts
│   └── ui-preferences-store.ts
├── types/                     # TypeScript type definitions
│   ├── log-entry.ts
│   ├── filter.ts
│   ├── query.ts
│   ├── api.ts
│   └── index.ts               # Export barrel
├── utils/                     # Utility functions
│   ├── date-utils.ts
│   ├── format-utils.ts
│   └── validation-utils.ts
└── styles/                    # Global styles
    └── globals.css
```

**Component Organization Rules:**
- ✅ **Feature-based folders** (filter-builder/, log-results-table/)
- ✅ **index.ts barrel exports** for clean imports
- ✅ **Tests co-located** with components (*.test.tsx)
- ❌ **NOT**: Type-based folders (buttons/, inputs/, forms/)

**Import Order Rules:**
- ✅ **Grouped and ordered**:
  1. External libraries (React, third-party)
  2. Internal absolute imports (@/)
  3. Relative imports (./)
  4. Type imports (import type)
  5. CSS imports

**Example:**
```typescript
// ✅ GOOD
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useVirtualizer } from '@tanstack/react-virtual';

import { useFilterStore } from '@/stores/filter-store';
import { formatDate } from '@/utils/date-utils';

import { LogRow } from './log-row';
import { Pagination } from './pagination';

import type { LogEntry, Filter } from '@/types';

import './log-results-table.css';
```

**Barrel Export Pattern:**
```typescript
// ✅ GOOD: components/filter-builder/index.ts
export { FilterBuilder } from './filter-builder';
export { FilterRow } from './filter-row';
export type { FilterBuilderProps } from './filter-builder';

// Usage elsewhere:
import { FilterBuilder } from '@/components/filter-builder';
```

---

### Format Patterns

#### Tauri IPC Response Format

**Standard Pattern:**
- ✅ **Use Rust Result<T, E>** directly (Tauri serializes to JSON)
- ✅ Frontend handles via try/catch

**Backend (Rust):**
```rust
// ✅ GOOD
#[tauri::command]
fn index_file(file_path: String) -> Result<IndexMetadata, String> {
    let index = build_index(&file_path)
        .map_err(|e| e.to_string())?;

    Ok(IndexMetadata {
        source_file_hash: calculate_hash(&file_path),
        created_at: Utc::now(),
        entry_count: index.len(),
    })
}

// Tauri serializes to:
// Success: {"Ok": {"sourceFileHash": "...", "createdAt": "...", "entryCount": 1000}}
// Error: {"Err": "Failed to parse log file: Invalid format"}
```

**Frontend (TypeScript):**
```typescript
// ✅ GOOD
try {
  const metadata = await invoke<IndexMetadata>('index_file', {
    filePath: selectedFile,
  });

  toast.success(`Indexed ${metadata.entryCount} entries`);
  setMetadata(metadata);
} catch (error) {
  // Error is the string from Rust Err variant
  toast.error(`Indexation failed: ${error}`);
  console.error('Indexation error:', error);
}
```

---

#### Date/Time Format Standards

**Serialization Format:**
- ✅ **ISO 8601** for JSON/IPC (e.g., `2026-01-16T14:30:00Z`)
- ✅ **Unix timestamp** (u64) for internal performance comparisons
- ✅ **chrono::DateTime<Utc>** in Rust, **Date** in TypeScript

**Rust:**
```rust
// ✅ GOOD
use chrono::{DateTime, Utc};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexMetadata {
    created_at: DateTime<Utc>,  // Serializes to ISO 8601
    file_modified_at: DateTime<Utc>,
}
```

**TypeScript:**
```typescript
// ✅ GOOD
import { parseISO, format } from 'date-fns';

interface IndexMetadata {
  createdAt: string; // ISO 8601 from backend
  fileModifiedAt: string;
}

// Display formatting
const displayDate = format(parseISO(metadata.createdAt), 'PPpp');
// → "Jan 16, 2026, 2:30:00 PM"
```

---

#### JSON Field Naming Convention

**Standard: camelCase in JSON, snake_case in Rust**

**Rust side:**
```rust
// ✅ GOOD
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]  // ← CRITICAL: Always use this
struct LogEntry {
    source_ip: String,        // → sourceIp in JSON
    destination_port: u16,    // → destinationPort in JSON
    log_timestamp: DateTime<Utc>, // → logTimestamp in JSON
}
```

**TypeScript side:**
```typescript
// ✅ GOOD
interface LogEntry {
  sourceIp: string;
  destinationPort: number;
  logTimestamp: string;
}
```

**Rationale:**
- TypeScript convention: camelCase
- Rust convention: snake_case
- serde handles conversion automatically with `rename_all = "camelCase"`

---

### Communication Patterns

#### Zustand Store Pattern

**Standard Structure:**

```typescript
// ✅ GOOD: stores/filter-store.ts
import { create } from 'zustand';
import type { Filter, FilterOperator } from '@/types';

interface FilterStore {
  // State
  filters: Filter[];
  activeFilterId: string | null;

  // Actions (verb-based naming)
  addFilter: (filter: Filter) => void;
  removeFilter: (id: string) => void;
  updateFilter: (id: string, updates: Partial<Filter>) => void;
  clearFilters: () => void;
  setActiveFilter: (id: string | null) => void;
}

export const useFilterStore = create<FilterStore>((set) => ({
  // Initial state
  filters: [],
  activeFilterId: null,

  // Actions (immutable updates)
  addFilter: (filter) => set((state) => ({
    filters: [...state.filters, filter],
  })),

  removeFilter: (id) => set((state) => ({
    filters: state.filters.filter((f) => f.id !== id),
  })),

  updateFilter: (id, updates) => set((state) => ({
    filters: state.filters.map((f) =>
      f.id === id ? { ...f, ...updates } : f
    ),
  })),

  clearFilters: () => set({ filters: [], activeFilterId: null }),

  setActiveFilter: (id) => set({ activeFilterId: id }),
}));
```

**Usage Pattern:**
```typescript
// ✅ GOOD: Selective subscription (prevents unnecessary re-renders)
function FilterBuilder() {
  const addFilter = useFilterStore((state) => state.addFilter);
  const filters = useFilterStore((state) => state.filters);

  // Component logic...
}
```

---

#### Tauri Event Pattern

**Backend Emits:**
```rust
// ✅ GOOD
use tauri::Manager;

pub fn index_file_with_progress(
    app: tauri::AppHandle,
    file_path: String,
) -> Result<IndexMetadata, String> {
    let total_lines = count_lines(&file_path)?;

    for (idx, line) in read_lines(&file_path)?.enumerate() {
        // Process line...

        // Emit progress event every 1000 lines
        if idx % 1000 == 0 {
            let percent = (idx as f64 / total_lines as f64 * 100.0) as u8;
            app.emit("indexation-progress", IndexProgress {
                percent,
                lines_processed: idx,
                current_file: file_path.clone(),
            })?;
        }
    }

    app.emit("indexation-complete", IndexMetadata { /* ... */ })?;
    Ok(metadata)
}
```

**Frontend Listens:**
```typescript
// ✅ GOOD
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

function IndexationProgress() {
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;

    // Setup listeners
    listen<IndexProgress>('indexation-progress', (event) => {
      setProgress(event.payload.percent);
    }).then((fn) => { unlistenProgress = fn; });

    listen<IndexMetadata>('indexation-complete', (event) => {
      toast.success('Indexation complete!');
      setProgress(100);
    }).then((fn) => { unlistenComplete = fn; });

    // Cleanup on unmount
    return () => {
      unlistenProgress?.();
      unlistenComplete?.();
    };
  }, []);

  return <ProgressBar value={progress} />;
}
```

---

### Process Patterns

#### Error Handling Pattern

**Backend Rust:**

```rust
// ✅ GOOD: thiserror for library errors
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid RFC3164 format: {0}")]
    InvalidRfc3164(String),

    #[error("Malformed timestamp: {0}")]
    InvalidTimestamp(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ✅ GOOD: anyhow for application code
use anyhow::{Context, Result};

fn process_log_file(path: &Path) -> Result<Index> {
    let logs = parse_logs(path)
        .context("Failed to parse log file")?;

    let index = build_index(logs)
        .context("Failed to build index")?;

    save_index(&index)
        .context("Failed to save index to disk")?;

    Ok(index)
}

// ✅ GOOD: Tauri command error handling
#[tauri::command]
fn index_file(path: String) -> Result<IndexMetadata, String> {
    process_log_file(Path::new(&path))
        .map_err(|e| format!("{:#}", e)) // Pretty-print error chain
}
```

**Frontend TypeScript:**

```typescript
// ✅ GOOD: Toast for recoverable errors
async function handleIndexFile() {
  setIsLoading(true);

  try {
    const metadata = await invoke<IndexMetadata>('index_file', {
      filePath: selectedFile,
    });

    toast.success('Index created successfully');
    setMetadata(metadata);
  } catch (error) {
    toast.error(`Indexation failed: ${error}`);
    console.error('Indexation error:', error);
  } finally {
    setIsLoading(false);
  }
}

// ✅ GOOD: Modal for critical errors (blocker)
async function handleCriticalOperation() {
  try {
    await invoke('critical_operation');
  } catch (error) {
    setModalError({
      title: 'Critical Error',
      message: `Operation failed: ${error}`,
      details: 'This error prevents the application from continuing.',
      canRetry: false,
    });
  }
}
```

---

#### Loading State Pattern

**Simple Boolean (Single Operation):**
```typescript
// ✅ GOOD: Simple loading flag
const [isLoading, setIsLoading] = useState(false);

async function handleAction() {
  setIsLoading(true);
  try {
    await invoke('some_command');
  } finally {
    setIsLoading(false);
  }
}
```

**Status Enum (Complex States):**
```typescript
// ✅ GOOD: Multiple states
type LoadingStatus = 'idle' | 'loading' | 'success' | 'error';

const [status, setStatus] = useState<LoadingStatus>('idle');
const [error, setError] = useState<string | null>(null);

async function handleAction() {
  setStatus('loading');
  setError(null);

  try {
    await invoke('some_command');
    setStatus('success');
  } catch (err) {
    setStatus('error');
    setError(String(err));
  }
}

// UI rendering based on status
if (status === 'loading') return <Spinner />;
if (status === 'error') return <ErrorMessage message={error} />;
if (status === 'success') return <SuccessView />;
```

---

### Enforcement Guidelines

**All AI Agents MUST:**

1. **Follow naming conventions exactly** - No variations or personal preferences
2. **Use specified project structure** - No reorganization without updating this document
3. **Maintain format consistency** - JSON field naming, date formats, error structures
4. **Apply patterns uniformly** - Same pattern for same situation across codebase
5. **Reference this document** - When uncertain, check patterns before implementing

**Pattern Verification:**

✅ **Automated enforcement** (where possible):
- Rust: `clippy` linting rules
- TypeScript: ESLint rules for naming, imports
- Formatting: `rustfmt` (Rust), `prettier` (TypeScript)

✅ **Code review checklist**:
- [ ] Naming follows conventions (snake_case Rust, camelCase TS)
- [ ] Files placed in correct directories
- [ ] Tests co-located properly
- [ ] Import order follows grouping rules
- [ ] Error handling uses standard patterns

**Pattern Updates:**

- ⚠️ Patterns can be updated if team consensus + architectural review
- ✅ Update process: Document change → Review → Update architecture.md → Communicate to all agents
- ❌ Individual agents CANNOT deviate from patterns without approval

---

### Pattern Examples

#### Good Examples (Follow These)

**Example 1: Adding New Backend Module**

```rust
// ✅ GOOD: src-tauri/src/enrichment/mod.rs

pub mod interface_mapper;
pub mod rule_resolver;
pub mod alias_expander;

use crate::api_client::ApiClient;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EnrichmentError {
    #[error("API client error: {0}")]
    ApiClient(String),

    #[error("Mapping not found for interface: {0}")]
    InterfaceNotFound(String),
}

pub struct EnrichmentManager {
    api_client: ApiClient,
    cache: HashMap<String, String>,
}

impl EnrichmentManager {
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client,
            cache: HashMap::new(),
        }
    }

    pub async fn enrich_log_entry(&mut self, entry: &mut LogEntry) -> Result<(), EnrichmentError> {
        // Implementation...
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enrich_log_entry() {
        // Test implementation...
    }
}
```

**Example 2: Adding New React Component**

```typescript
// ✅ GOOD: components/export-dialog/export-dialog.tsx

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import toast from 'react-hot-toast';

import { useFilterStore } from '@/stores/filter-store';
import { formatDate } from '@/utils/date-utils';

import type { ExportFormat, ExportOptions } from '@/types';

interface ExportDialogProps {
  isOpen: boolean;
  onClose: () => void;
  resultCount: number;
}

export function ExportDialog({ isOpen, onClose, resultCount }: ExportDialogProps) {
  const filters = useFilterStore((state) => state.filters);
  const [format, setFormat] = useState<ExportFormat>('json');
  const [isExporting, setIsExporting] = useState(false);

  const handleExport = async () => {
    setIsExporting(true);

    try {
      const options: ExportOptions = {
        format,
        filters,
        includeMetadata: true,
      };

      await invoke('export_results', { options });
      toast.success(`Exported ${resultCount} entries as ${format.toUpperCase()}`);
      onClose();
    } catch (error) {
      toast.error(`Export failed: ${error}`);
    } finally {
      setIsExporting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="export-dialog">
      {/* Dialog content */}
    </div>
  );
}
```

```typescript
// ✅ GOOD: components/export-dialog/index.ts
export { ExportDialog } from './export-dialog';
export type { ExportDialogProps } from './export-dialog';
```

**Example 3: Adding Zustand Store**

```typescript
// ✅ GOOD: stores/export-store.ts

import { create } from 'zustand';
import type { ExportFormat, ExportHistory } from '@/types';

interface ExportStore {
  format: ExportFormat;
  history: ExportHistory[];
  isExporting: boolean;

  setFormat: (format: ExportFormat) => void;
  addToHistory: (entry: ExportHistory) => void;
  clearHistory: () => void;
  setExporting: (isExporting: boolean) => void;
}

export const useExportStore = create<ExportStore>((set) => ({
  format: 'json',
  history: [],
  isExporting: false,

  setFormat: (format) => set({ format }),

  addToHistory: (entry) => set((state) => ({
    history: [entry, ...state.history].slice(0, 10), // Keep last 10
  })),

  clearHistory: () => set({ history: [] }),

  setExporting: (isExporting) => set({ isExporting }),
}));
```

---

#### Anti-Patterns (Avoid These)

**Anti-Pattern 1: Inconsistent Naming**

```rust
// ❌ BAD: Mixing conventions
pub fn buildIndex(entries: Vec<LogEntry>) -> Result<Index, IndexError> {
    // Should be: build_index
}

pub struct apiClient {
    // Should be: ApiClient
}

const maxRetries: u32 = 3;
// Should be: MAX_RETRIES
```

**Anti-Pattern 2: Wrong File Structure**

```
// ❌ BAD: Flat structure
src-tauri/src/
├── main.rs
├── indexer.rs
├── inverted_index.rs
├── bitmap_index.rs
├── parser_rfc3164.rs
├── parser_rfc5424.rs
└── ...all files flat

// ✅ GOOD: Hierarchical with mod.rs
src-tauri/src/
├── main.rs
├── indexer/
│   ├── mod.rs
│   ├── inverted.rs
│   └── bitmap.rs
└── parser/
    ├── mod.rs
    ├── rfc3164.rs
    └── rfc5424.rs
```

**Anti-Pattern 3: Inconsistent Error Handling**

```typescript
// ❌ BAD: Mixing error display methods
try {
  await invoke('index_file');
} catch (error) {
  alert(error); // Don't use alert!
}

try {
  await invoke('query');
} catch (error) {
  console.log(error); // Silent failure, user sees nothing!
}

try {
  await invoke('export');
} catch (error) {
  setError(error); // Inconsistent with toast pattern
}

// ✅ GOOD: Consistent toast pattern
try {
  await invoke('any_command');
} catch (error) {
  toast.error(`Operation failed: ${error}`);
}
```

**Anti-Pattern 4: Wrong Import Order**

```typescript
// ❌ BAD: Random order
import './styles.css';
import type { LogEntry } from '@/types';
import { FilterRow } from './filter-row';
import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { useFilterStore } from '@/stores/filter-store';

// ✅ GOOD: Grouped order
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

import { useFilterStore } from '@/stores/filter-store';

import { FilterRow } from './filter-row';

import type { LogEntry } from '@/types';

import './styles.css';
```

---

### Pattern Compliance Checklist

**Before committing code, verify:**

- [ ] **Naming**: All names follow conventions (snake_case Rust, camelCase TS, PascalCase types)
- [ ] **Structure**: Files in correct directories, proper hierarchy
- [ ] **Imports**: Grouped and ordered correctly
- [ ] **Error Handling**: Uses standard patterns (thiserror/anyhow backend, toast frontend)
- [ ] **Loading States**: Boolean or status enum as appropriate
- [ ] **Tests**: Co-located with code (inline Rust, *.test.tsx React)
- [ ] **Types**: Interfaces defined in types/ folder, exported via barrel
- [ ] **Events**: kebab-case names, consistent payload structures
- [ ] **IPC**: snake_case commands, camelCase JSON fields with serde rename_all

**Pattern Confidence Level**: **HIGH**
- 15 conflict points identified and resolved
- Concrete examples for all patterns
- Anti-patterns documented for clarity
- Enforcement via linting + code review

## Project Structure & Boundaries

### Complete Project Directory Structure

```
opnsense-log-viewer/
├── README.md
├── LICENSE
├── .gitignore
├── package.json                    # Frontend dependencies (React, TypeScript, Vite)
├── tsconfig.json                   # TypeScript configuration (strict mode)
├── vite.config.ts                  # Vite build configuration + Tauri plugin
├── tailwind.config.js              # Tailwind CSS + dark mode config
├── postcss.config.js               # PostCSS with autoprefixer
├── .eslintrc.json                  # ESLint rules (TypeScript, React)
├── .prettierrc                     # Prettier formatting rules
├── .github/
│   └── workflows/
│       ├── test.yml                # CI: Unit + integration tests
│       ├── bench.yml               # CI: Performance benchmarks + gates
│       ├── build.yml               # CI: Multi-platform builds
│       └── release.yml             # CD: Packaging + release automation
├── src/                              # Frontend React + TypeScript
│   ├── main.tsx                    # React entry point
│   ├── App.tsx                     # Root component, layout
│   ├── globals.css                 # Global styles + Tailwind imports
│   ├── components/                 # React components (feature-based)
│   │   ├── filter-builder/
│   │   │   ├── filter-builder.tsx  # Main filter builder component
│   │   │   ├── filter-row.tsx      # Single filter row subcomponent
│   │   │   ├── operator-select.tsx # Operator dropdown
│   │   │   ├── value-input.tsx     # Value input field
│   │   │   ├── filter-builder.test.tsx
│   │   │   └── index.ts            # Barrel export
│   │   ├── log-results-table/
│   │   │   ├── log-results-table.tsx   # Virtual scrolling table
│   │   │   ├── log-row.tsx         # Single log entry row
│   │   │   ├── column-header.tsx   # Sortable column header
│   │   │   ├── pagination.tsx      # Pagination controls
│   │   │   ├── log-results-table.test.tsx
│   │   │   └── index.ts
│   │   ├── search-history/
│   │   │   ├── search-history.tsx  # Recent searches panel
│   │   │   ├── history-item.tsx    # Single history entry
│   │   │   ├── search-history.test.tsx
│   │   │   └── index.ts
│   │   ├── file-selector/
│   │   │   ├── file-selector.tsx   # File picker dialog
│   │   │   ├── file-info.tsx       # File metadata display
│   │   │   ├── file-selector.test.tsx
│   │   │   └── index.ts
│   │   ├── export-dialog/
│   │   │   ├── export-dialog.tsx   # Export configuration modal
│   │   │   ├── format-selector.tsx # JSON/CSV format selector
│   │   │   ├── export-options.tsx  # Metadata options
│   │   │   ├── export-dialog.test.tsx
│   │   │   └── index.ts
│   │   ├── credentials-form/
│   │   │   ├── credentials-form.tsx    # OPNsense API credentials
│   │   │   ├── connection-test.tsx # Test connection button
│   │   │   ├── credentials-form.test.tsx
│   │   │   └── index.ts
│   │   ├── settings-panel/
│   │   │   ├── settings-panel.tsx  # App settings
│   │   │   ├── theme-toggle.tsx    # Dark/light theme switch
│   │   │   ├── settings-panel.test.tsx
│   │   │   └── index.ts
│   │   └── ui/                     # Reusable UI primitives
│   │       ├── button.tsx
│   │       ├── input.tsx
│   │       ├── select.tsx
│   │       ├── modal.tsx
│   │       ├── spinner.tsx
│   │       ├── progress-bar.tsx
│   │       └── index.ts
│   ├── hooks/                      # Custom React hooks
│   │   ├── use-filters.ts          # Filter state management
│   │   ├── use-search-history.ts   # Search history management
│   │   ├── use-virtual-scrolling.ts    # Virtual scrolling logic
│   │   ├── use-tauri-event.ts      # Tauri event listener hook
│   │   ├── use-tauri-command.ts    # Tauri command wrapper hook
│   │   ├── use-theme.ts            # Dark/light theme management
│   │   └── use-file-dialog.ts      # File selection dialog
│   ├── stores/                     # Zustand stores
│   │   ├── filter-store.ts         # Filter state + actions
│   │   ├── search-history-store.ts # Search history state
│   │   ├── ui-preferences-store.ts # Theme, sidebar, preferences
│   │   ├── file-store.ts           # Current file, index metadata
│   │   └── export-store.ts         # Export configuration state
│   ├── types/                      # TypeScript type definitions
│   │   ├── log-entry.ts            # LogEntry interface
│   │   ├── filter.ts               # Filter, FilterOperator types
│   │   ├── query.ts                # QueryParams, QueryResult types
│   │   ├── api.ts                  # API response types
│   │   ├── export.ts               # Export format, options types
│   │   ├── credentials.ts          # API credentials types
│   │   └── index.ts                # Barrel export
│   └── utils/                      # Utility functions
│       ├── date-utils.ts           # Date formatting, parsing
│       ├── format-utils.ts         # Data formatting (IPs, ports)
│       ├── validation-utils.ts     # Input validation
│       └── export-utils.ts         # Export helpers
├── src-tauri/                        # Backend Rust
│   ├── Cargo.toml                  # Rust dependencies
│   ├── tauri.conf.json             # Tauri app configuration
│   ├── build.rs                    # Build script
│   ├── icons/                      # App icons (PNG, ICO)
│   │   ├── icon.png
│   │   ├── icon.ico
│   │   └── icon.icns
│   ├── capabilities/               # Tauri permissions & capabilities
│   │   └── default.json
│   └── src/
│       ├── main.rs                 # Tauri entry point, app builder
│       ├── lib.rs                  # Library exports (optional)
│       ├── errors.rs               # Custom error types (ParseError, IndexError, etc.)
│       ├── commands/               # Tauri command handlers (IPC layer)
│       │   ├── mod.rs
│       │   ├── indexation.rs       # index_file, load_index, list_indexes
│       │   ├── query.rs            # execute_query, get_results, count_results
│       │   ├── credentials.rs      # save_credentials, get_credentials, test_connection
│       │   ├── export.rs           # export_results (JSON/CSV)
│       │   └── enrichment.rs       # fetch_enrichment_data
│       ├── indexer/                # Indexation engine
│       │   ├── mod.rs              # Public API exports
│       │   ├── inverted.rs         # Inverted multi-column index (IPs, ports)
│       │   ├── bitmap.rs           # Bitmap index (action, protocol, interface)
│       │   ├── hybrid.rs           # Hybrid orchestration
│       │   └── metadata.rs         # Index metadata structures
│       ├── parser/                 # Log parsers
│       │   ├── mod.rs
│       │   ├── rfc3164.rs          # Legacy Syslog (RFC3164) parser
│       │   ├── rfc5424.rs          # Modern Syslog (RFC5424) parser
│       │   ├── csv_filterlog.rs    # OPNsense CSV filterlog parser
│       │   ├── detector.rs         # Auto-detect log format
│       │   └── types.rs            # Common types (LogEntry, LogFormat)
│       ├── query/                  # Query execution engine
│       │   ├── mod.rs
│       │   ├── executor.rs         # Boolean query execution (AND/OR/NOT)
│       │   ├── optimizer.rs        # Query optimization strategies
│       │   ├── operators.rs        # Operator implementations (equals, contains, regex)
│       │   └── types.rs            # Query types (Filter, QueryParams)
│       ├── api_client/             # OPNsense API client
│       │   ├── mod.rs
│       │   ├── client.rs           # HTTP client with retry logic
│       │   ├── enrichment.rs       # Interface/rule/alias enrichment
│       │   ├── endpoints.rs        # API endpoint definitions
│       │   └── types.rs            # API response types
│       ├── storage/                # Index persistence
│       │   ├── mod.rs
│       │   ├── persistence.rs      # Bincode + Zstd serialization
│       │   ├── checksum.rs         # SHA-256 checksum generation
│       │   └── types.rs            # PersistedIndex structure
│       ├── export/                 # Export engine
│       │   ├── mod.rs
│       │   ├── json.rs             # JSON export implementation
│       │   ├── csv.rs              # CSV export implementation
│       │   └── metadata.rs         # Export metadata generation
│       └── credentials/            # Credential management
│           ├── mod.rs
│           └── manager.rs          # OS keychain integration (keyring crate)
├── tests/                            # Integration tests
│   ├── indexation_tests.rs         # Full indexation flow tests
│   ├── query_tests.rs              # Query execution tests
│   ├── api_client_tests.rs         # OPNsense API integration tests
│   ├── persistence_tests.rs        # Index save/load tests
│   ├── export_tests.rs             # Export JSON/CSV tests
│   └── fixtures/                   # Test fixtures
│       ├── log_generator.rs        # Streaming log generator (30GB capability)
│       ├── sample_logs/            # Small sample log files
│       │   ├── rfc3164.log
│       │   ├── rfc5424.log
│       │   └── filterlog.csv
│       └── mock_api/               # Mock OPNsense API responses
│           ├── interfaces.json
│           ├── rules.json
│           └── aliases.json
└── benches/                          # Criterion benchmarks
    ├── indexation_benchmark.rs     # Indexation performance (1GB, 10GB, 30GB)
    ├── query_benchmark.rs          # Query execution performance
    └── parser_benchmark.rs         # Parser performance by format
```

### Architectural Boundaries

#### API Boundaries

**Tauri IPC Commands (Frontend ↔ Backend):**

```rust
// Indexation Commands
index_file(file_path: String, compression_level: u8) -> Result<IndexMetadata, String>
load_index(index_path: String) -> Result<IndexMetadata, String>
list_indexes() -> Result<Vec<IndexInfo>, String>
delete_index(index_id: String) -> Result<(), String>

// Query Commands
execute_query(query_params: QueryParams) -> Result<QueryResult, String>
count_results(query_params: QueryParams) -> Result<usize, String>
get_paginated_results(query_params: QueryParams, page: usize, page_size: usize) -> Result<PaginatedResult, String>

// Credentials Commands
save_credentials(creds: ApiCredentials) -> Result<(), String>
get_credentials() -> Result<Option<ApiCredentials>, String>
test_connection(creds: ApiCredentials) -> Result<ConnectionStatus, String>
delete_credentials() -> Result<(), String>

// Export Commands
export_results(format: ExportFormat, filters: Vec<Filter>, output_path: String) -> Result<ExportMetadata, String>

// Enrichment Commands
fetch_enrichment_data(credential_id: String) -> Result<EnrichmentData, String>
refresh_enrichment_cache() -> Result<(), String>
```

**Tauri Events (Backend → Frontend):**

```typescript
// Progress Events
'indexation-progress' → IndexProgress { percent: u8, lines_processed: u64, current_file: String }
'indexation-complete' → IndexMetadata
'indexation-error' → String

'query-progress' → QueryProgress { percent: u8 }
'query-complete' → QueryResult

'export-progress' → ExportProgress { percent: u8 }
'export-complete' → ExportMetadata
```

**OPNsense External API Endpoints:**

```
GET  /api/diagnostics/interface/getInterfaceNames
     → { rows: [{ value: "vtnet0", selected: 0, description: "LAN" }] }

POST /api/firewall/filter/searchRule
     → { rows: [{ uuid: "...", description: "Allow HTTP", ... }] }

POST /api/firewall/alias/searchItem
     → { rows: [{ uuid: "...", name: "RFC1918", content: "..." }] }

GET  /api/core/firmware/status
     → { product_version: "24.1.0", ... }
```

**Authentication:**
- Basic Auth (API key + secret)
- Stored in OS keychain via `keyring` crate
- Retrieved per-request

#### Component Boundaries

**Frontend Component Communication:**

```
FilterBuilder → FilterStore (Zustand)
    ↓
    ├── Adds/removes/updates filters in store
    └── Triggers search execution

LogResultsTable → FileStore (query results)
    ↓
    ├── Subscribes to results changes
    └── Renders via TanStack Virtual (virtualized)

SearchHistory → SearchHistoryStore
    ↓
    ├── Persists recent searches
    └── Re-runs previous queries

CredentialsForm → invoke('save_credentials')
    ↓
    └── Backend saves to OS keychain

ExportDialog → invoke('export_results')
    ↓
    └── Backend writes JSON/CSV to disk
```

**State Management Boundaries:**

```
Zustand Stores (Frontend only):
├── filter-store.ts          # Filter state, never persisted
├── search-history-store.ts  # Recent searches, localStorage
├── ui-preferences-store.ts  # Theme, sidebar, localStorage
├── file-store.ts            # Current file, index metadata (ephemeral)
└── export-store.ts          # Export config (ephemeral)

Backend State (Rust):
├── Index in-memory cache (LRU, 500 MB max)
├── Enrichment data cache (JSON backup + memory)
└── Credentials in OS keychain (persistent, encrypted)
```

#### Service Boundaries

**Backend Module Boundaries:**

```
commands/ (IPC layer)
    ↓
    ├── Validates inputs
    ├── Calls domain services (indexer, parser, query)
    └── Returns Result<T, String> to frontend

indexer/ (Domain service)
    ↓
    ├── Pure business logic, no IPC knowledge
    ├── Depends on: parser, storage
    └── Returns Result<Index, IndexError>

parser/ (Domain service)
    ↓
    ├── Stateless parsing logic
    ├── No dependencies on other services
    └── Returns Result<Vec<LogEntry>, ParseError>

query/ (Domain service)
    ↓
    ├── Depends on: indexer (reads Index)
    ├── Pure query execution logic
    └── Returns Result<QueryResult, QueryError>

api_client/ (External integration)
    ↓
    ├── Isolated HTTP client
    ├── Depends on: credentials
    └── Returns Result<EnrichmentData, ApiError>

storage/ (Infrastructure)
    ↓
    ├── Persistence abstraction
    ├── Used by: indexer
    └── Returns Result<(), StorageError>

credentials/ (Infrastructure)
    ↓
    ├── OS keychain abstraction
    ├── Used by: commands/credentials.rs
    └── Returns Result<ApiCredentials, CredentialsError>
```

**Service Dependency Graph:**

```
commands/
├── indexer → parser → storage
├── query → indexer
├── enrichment → api_client → credentials
└── export → query

No circular dependencies allowed
```

#### Data Boundaries

**Data Flow:**

```
1. Indexation Flow:
   Log File (disk)
   → parser/ (streaming reads)
   → indexer/ (builds Index)
   → storage/ (persists Index with Zstd compression)
   → Index File (.idx.zst on disk)

2. Query Flow:
   User filters (frontend)
   → invoke('execute_query')
   → query/ (executes against Index)
   → QueryResult
   → Frontend (virtual scrolling display)

3. Enrichment Flow:
   invoke('fetch_enrichment_data')
   → api_client/ (HTTP GET to OPNsense)
   → EnrichmentData
   → Cached in-memory + JSON backup (disk)
   → Used during query execution

4. Export Flow:
   QueryResult (in-memory)
   → export/ (formats as JSON/CSV)
   → File (disk)
```

**Data Access Patterns:**

```
Frontend:
- Read-only access to backend data via IPC
- Manages UI state locally (Zustand stores)
- NO direct file system access
- NO direct API calls (proxied via backend)

Backend:
- Exclusive file system access (logs, indexes, exports)
- Exclusive API client access (OPNsense)
- Exclusive credential storage access (OS keychain)
- Streams large data (no full-file reads)
```

**Caching Strategy:**

```
Backend In-Memory Cache (LRU):
- Loaded Index: Up to 1 active index (500 MB max)
- Enrichment Data: Interface mappings, rules, aliases (10 MB max)
- Eviction: Least Recently Used when memory limit reached

Disk Cache:
- Index Files (.idx.zst): Persistent, checksummed
- Enrichment Backup (enrichment.json): Persistent, used when API unavailable

Frontend Cache:
- Search History: localStorage (last 50 searches)
- UI Preferences: localStorage (theme, sidebar state)
- NO result data caching (backend manages this)
```

### Requirements to Structure Mapping

#### Functional Requirements → Implementation

**FR1: Indexation haute performance**
- **Backend Modules:**
  - `src-tauri/src/indexer/` → Hybrid inverted + bitmap index
  - `src-tauri/src/parser/` → Multi-format parsers (RFC3164/5424/CSV)
  - `src-tauri/src/storage/` → Bincode + Zstd persistence
- **IPC Commands:**
  - `index_file()` → Entry point
  - `indexation-progress` event → Real-time feedback
- **Tests:**
  - `tests/indexation_tests.rs` → Integration tests
  - `benches/indexation_benchmark.rs` → Performance gates (<7s/GB)
- **Files Affected:** ~15 files

**FR2: Système de filtrage avancé**
- **Backend Modules:**
  - `src-tauri/src/query/` → Boolean query executor (AND/OR/NOT)
  - `src-tauri/src/query/operators.rs` → Operator implementations
- **Frontend Components:**
  - `src/components/filter-builder/` → Visual filter UI
  - `src/stores/filter-store.ts` → Filter state management
- **IPC Commands:**
  - `execute_query()` → Query execution
  - `count_results()` → Result counting
- **Tests:**
  - `tests/query_tests.rs` → Query correctness
  - `benches/query_benchmark.rs` → Performance gates (<750ms)
- **Files Affected:** ~12 files

**FR3: Enrichissement via API OPNsense**
- **Backend Modules:**
  - `src-tauri/src/api_client/` → HTTP client + retry logic
  - `src-tauri/src/api_client/enrichment.rs` → Interface/rule/alias enrichment
  - `src-tauri/src/credentials/` → OS keychain integration
- **Frontend Components:**
  - `src/components/credentials-form/` → Credential input UI
- **IPC Commands:**
  - `fetch_enrichment_data()` → API call orchestration
  - `save_credentials()`, `get_credentials()` → Credential management
- **Backup:**
  - `enrichment.json` (disk) → Offline fallback
- **Tests:**
  - `tests/api_client_tests.rs` → API integration tests
  - `tests/fixtures/mock_api/` → Mock responses
- **Files Affected:** ~10 files

**FR4: Opérations core fiables**
- **Backend Modules:**
  - `src-tauri/src/parser/` → Robust parsing with error handling
  - `src-tauri/src/storage/persistence.rs` → Checksummed persistence
  - `src-tauri/src/export/` → JSON/CSV export with metadata
- **Frontend Components:**
  - `src/components/export-dialog/` → Export configuration UI
- **IPC Commands:**
  - `export_results()` → Export orchestration
- **Tests:**
  - `tests/persistence_tests.rs` → Save/load integrity
  - `tests/export_tests.rs` → Export correctness
- **Files Affected:** ~8 files

**FR5: Application multiplateforme**
- **Configuration:**
  - `tauri.conf.json` → Platform-specific bundling
  - `.github/workflows/build.yml` → CI matrix builds (Windows/Linux/macOS)
- **Build Outputs:**
  - Windows: MSI installer
  - macOS: DMG + universal binary
  - Linux: AppImage, .deb, .rpm
- **Files Affected:** ~5 configuration files

#### Non-Functional Requirements → Implementation

**Performance Gates:**
- **Implementation:**
  - `benches/indexation_benchmark.rs` → Criterion benchmarks
  - `benches/query_benchmark.rs` → Query performance
  - `.github/workflows/bench.yml` → CI enforcement
- **Gates:**
  - Indexation: <7 sec/GB (±15%)
  - Query: <750ms (±50%)
  - Memory: <600 MB (±20%)
- **Failure:** Build fails if gates exceeded

**Security:**
- **Implementation:**
  - `src-tauri/src/credentials/manager.rs` → OS keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service)
  - `src-tauri/src/parser/` → Memory-safe Rust parsing
  - `src-tauri/src/api_client/client.rs` → TLS via rustls
- **Validation:**
  - No plaintext credentials in config files
  - Sandboxed parsing (no filesystem write outside cache)

**Conformité & Auditabilité:**
- **Implementation:**
  - `src-tauri/src/export/metadata.rs` → Export metadata generation (timestamp, hash, filter criteria)
  - `src-tauri/src/storage/checksum.rs` → SHA-256 checksums
- **Metadata Fields:**
  - Timestamp (ISO 8601)
  - Source file hash (SHA-256)
  - Filter criteria applied
  - Tool version
  - Operator identification

#### Cross-Cutting Concerns → Implementation

**Logging & Observability:**
- **Implementation:**
  - `tracing` + `tracing-subscriber` (all Rust modules)
  - JSON structured logs
  - Log levels: ERROR, WARN, INFO, DEBUG, TRACE
- **Files Affected:** All Rust modules (via `tracing::instrument`)

**Error Handling:**
- **Implementation:**
  - `src-tauri/src/errors.rs` → Custom error types (thiserror)
  - `anyhow` for application code
  - `react-hot-toast` for frontend error display
- **Pattern:**
  - Backend: Result<T, CustomError> → IPC → Result<T, String>
  - Frontend: try/catch → toast.error()

**Testing Strategy:**
- **Unit Tests:**
  - Inline `#[cfg(test)] mod tests` in all Rust modules
  - Vitest for React components (`*.test.tsx`)
- **Integration Tests:**
  - `tests/` directory (Rust)
  - E2E: Tauri WebDriver (2 critical paths)
- **Property-Based Testing:**
  - `proptest` for parser fuzzing
- **Performance Testing:**
  - `criterion` benchmarks with CI gates

### Integration Points

#### Internal Communication

**Frontend ↔ Backend (Tauri IPC):**

```typescript
// Frontend invokes backend commands
const result = await invoke<IndexMetadata>('index_file', {
  filePath: selectedFile,
  compressionLevel: 1,
});

// Backend emits events to frontend
app.emit("indexation-progress", IndexProgress { percent: 50 });

// Frontend listens to events
const unlisten = await listen<IndexProgress>('indexation-progress', (event) => {
  setProgress(event.payload.percent);
});
```

**Pattern:**
- **Commands:** Frontend initiates, backend responds (request/response)
- **Events:** Backend pushes updates, frontend subscribes (pub/sub)
- **Serialization:** serde (Rust) ↔ JSON ↔ TypeScript types

**Backend Module Communication:**

```rust
// Commands call domain services
#[tauri::command]
fn index_file(file_path: String) -> Result<IndexMetadata, String> {
    let logs = parser::parse_file(&file_path)?;  // Parser service
    let index = indexer::build_index(logs)?;      // Indexer service
    storage::save_index(&index)?;                 // Storage service
    Ok(index.metadata())
}

// Services return domain errors (thiserror)
pub fn parse_file(path: &Path) -> Result<Vec<LogEntry>, ParseError> {
    // Implementation...
}

// Commands convert to String errors for IPC
.map_err(|e| e.to_string())
```

**Pattern:**
- Commands orchestrate services (thin layer)
- Services contain business logic (thick layer)
- No circular dependencies

#### External Integrations

**OPNsense API Integration:**

```rust
// api_client/client.rs
pub struct ApiClient {
    http_client: ClientWithMiddleware,  // reqwest + retry middleware
    base_url: String,
    credentials: ApiCredentials,
}

impl ApiClient {
    pub async fn fetch_interfaces(&self) -> Result<Vec<Interface>, ApiError> {
        let response = self.http_client
            .get(&format!("{}/api/diagnostics/interface/getInterfaceNames", self.base_url))
            .basic_auth(&self.credentials.api_key, Some(&self.credentials.api_secret))
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        // Retry policy: 3 retries, exponential backoff (100ms, 400ms, 1.6s)
        // Handled by reqwest-retry middleware
    }
}
```

**Integration Characteristics:**
- **Protocol:** HTTPS (TLS via rustls)
- **Auth:** Basic Auth (API key + secret)
- **Retry:** 3 retries, exponential backoff
- **Timeout:** 10s per request
- **Fallback:** JSON backup file (offline mode)

**Graceful Degradation:**
1. Try API call
2. If fails → Load from backup JSON
3. If backup missing → Display raw data (no enrichment)
4. UI always shows staleness indicators

**OS Keychain Integration:**

```rust
// credentials/manager.rs
use keyring::Entry;

pub fn save_credentials(creds: &ApiCredentials) -> Result<(), CredentialsError> {
    let entry = Entry::new("opnsense-log-viewer", "api-credentials")?;
    let json = serde_json::to_string(creds)?;
    entry.set_password(&json)?;
    Ok(())
}

pub fn get_credentials() -> Result<Option<ApiCredentials>, CredentialsError> {
    let entry = Entry::new("opnsense-log-viewer", "api-credentials")?;
    match entry.get_password() {
        Ok(json) => Ok(Some(serde_json::from_str(&json)?)),
        Err(_) => Ok(None),  // No credentials saved yet
    }
}
```

**Platform-Specific Storage:**
- **Windows:** Credential Manager (`CredWrite`, `CredRead`)
- **macOS:** Keychain Services (`SecItemAdd`, `SecItemCopyMatching`)
- **Linux:** Secret Service API (GNOME Keyring, KWallet)
- **Abstraction:** `keyring` crate handles platform differences

#### Data Flow

**Complete Data Flow Diagram:**

```
┌────────────────────────────────────────────────────────────────────┐
│                         USER INTERACTIONS                           │
└────────────────────────────────────────────────────────────────────┘
                                   │
                ┌──────────────────┼──────────────────┐
                │                  │                  │
        [Select File]      [Build Filters]     [Configure Export]
                │                  │                  │
                ▼                  ▼                  ▼
    ┌───────────────────┐  ┌────────────────┐  ┌─────────────────┐
    │ File Selector     │  │ Filter Builder │  │ Export Dialog   │
    │ Component         │  │ Component      │  │ Component       │
    └───────┬───────────┘  └────────┬───────┘  └────────┬────────┘
            │                       │                   │
            │                       ▼                   │
            │            ┌──────────────────┐           │
            │            │ Filter Store     │           │
            │            │ (Zustand)        │           │
            │            └──────────────────┘           │
            │                                            │
            ▼                       ▼                   ▼
    ┌─────────────────────────────────────────────────────────┐
    │            Tauri IPC Layer (Commands)                    │
    ├──────────────────┬──────────────────┬───────────────────┤
    │ index_file()     │ execute_query()  │ export_results()  │
    └────────┬─────────┴────────┬─────────┴──────────┬────────┘
             │                  │                     │
             ▼                  ▼                     ▼
    ┌────────────────┐  ┌────────────────┐  ┌────────────────┐
    │ INDEXER        │  │ QUERY          │  │ EXPORT         │
    │ SERVICE        │  │ SERVICE        │  │ SERVICE        │
    └────┬───────────┘  └────┬───────────┘  └────┬───────────┘
         │                   │                    │
         │ ┌─────────────────┴─────────┐          │
         │ │                           │          │
         ▼ ▼                           ▼          ▼
    ┌────────────┐            ┌─────────────┐  ┌──────────┐
    │ PARSER     │            │ ENRICHMENT  │  │ METADATA │
    │ SERVICE    │            │ (API Client)│  │ GENERATOR│
    └────┬───────┘            └─────┬───────┘  └────┬─────┘
         │                          │               │
         │                          │               │
    ┌────▼──────────┐      ┌────────▼────────┐     │
    │ Log File      │      │ OPNsense API    │     │
    │ (Disk)        │      │ (External)      │     │
    └───────────────┘      └─────────────────┘     │
                                   │                │
                           ┌───────▼────────┐       │
                           │ Enrichment     │       │
                           │ Cache (JSON)   │       │
                           └────────────────┘       │
         │                                           │
         ▼                                           │
    ┌────────────────┐                              │
    │ STORAGE        │                              │
    │ SERVICE        │                              │
    └────┬───────────┘                              │
         │                                           │
         ▼                                           ▼
    ┌────────────────┐                       ┌─────────────┐
    │ Index File     │                       │ Export File │
    │ (.idx.zst)     │                       │ (.json/csv) │
    └────────────────┘                       └─────────────┘
         │
         │ [Load Index]
         │
         ▼
    ┌────────────────┐
    │ In-Memory      │
    │ Index Cache    │
    │ (LRU, 500MB)   │
    └────────────────┘
         │
         └──────────► [Query Execution]
                           │
                           ▼
                    ┌──────────────────┐
                    │ Query Results    │
                    │ (JSON via IPC)   │
                    └─────────┬────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │ Log Results      │
                    │ Table Component  │
                    │ (Virtual Scroll) │
                    └──────────────────┘
```

**Key Data Transformations:**

```
1. Log File (Text)
   → Parser (RFC3164/5424/CSV detection)
   → Vec<LogEntry> (Rust structs)

2. Vec<LogEntry>
   → Indexer (Hybrid inverted + bitmap)
   → Index (in-memory)
   → Storage (Bincode serialization)
   → Compressed (Zstd level 1)
   → Index File (.idx.zst on disk)

3. User Filters (Frontend)
   → JSON (IPC)
   → QueryParams (Rust struct)
   → Query Executor (Boolean logic)
   → QueryResult (Rust struct)
   → JSON (IPC)
   → LogEntry[] (TypeScript)
   → Virtual Scrolling (TanStack Virtual)

4. API Credentials (Frontend input)
   → JSON (IPC)
   → ApiCredentials (Rust struct)
   → JSON (serde serialization)
   → OS Keychain (platform-specific encrypted storage)
```

### File Organization Patterns

#### Configuration Files

**Root Configuration:**
- `package.json` → Frontend dependencies, scripts
- `tsconfig.json` → TypeScript compiler options (strict mode, paths)
- `vite.config.ts` → Vite build config + Tauri plugin
- `tailwind.config.js` → Tailwind CSS + dark mode setup
- `.eslintrc.json` → ESLint rules (TypeScript, React)
- `.prettierrc` → Code formatting rules

**Tauri Configuration:**
- `src-tauri/Cargo.toml` → Rust dependencies, features
- `src-tauri/tauri.conf.json` → App metadata, permissions, bundling
- `src-tauri/capabilities/default.json` → Tauri security capabilities

**CI/CD Configuration:**
- `.github/workflows/test.yml` → Unit + integration tests
- `.github/workflows/bench.yml` → Performance benchmarks + gates
- `.github/workflows/build.yml` → Multi-platform builds
- `.github/workflows/release.yml` → Release automation

**Pattern:** Configuration files at project root, nested configs in respective subdirectories

#### Source Organization

**Frontend (src/):**
- **Feature-based components** → `components/{feature-name}/`
- **Co-located tests** → `{component-name}.test.tsx`
- **Barrel exports** → `index.ts` in each folder
- **Absolute imports** → `@/components/...`, `@/stores/...`

**Backend (src-tauri/src/):**
- **Module-based organization** → `{module-name}/mod.rs` + submodules
- **Public API exports** → `mod.rs` re-exports
- **Implementation details** → Private submodules
- **Error types** → Centralized in `errors.rs`

**Shared Patterns:**
- **Types separate from logic** → `types.ts` or `types.rs` files
- **Tests co-located** → Inline `#[cfg(test)]` (Rust) or `.test.tsx` (React)
- **Utils separate** → `utils/` folder for pure functions

#### Test Organization

**Frontend Tests:**
- **Unit tests:** `components/{feature}/{component}.test.tsx`
- **Hook tests:** `hooks/{hook-name}.test.ts`
- **Test utilities:** `tests/utils/` (setup, mocks)

**Backend Tests:**
- **Unit tests:** Inline `#[cfg(test)] mod tests` in each module
- **Integration tests:** `tests/{feature}_tests.rs`
- **Fixtures:** `tests/fixtures/` (log generators, mock data)
- **Benchmarks:** `benches/{feature}_benchmark.rs`

**E2E Tests:**
- Tauri WebDriver-based tests
- Limited to 2 critical paths (happy + error)
- Located in `tests/e2e/` (post-MVP)

#### Asset Organization

**Static Assets:**
- **Icons:** `src-tauri/icons/` (PNG, ICO, ICNS)
- **Frontend assets:** `src/assets/` (images, fonts)
- **Build outputs:** `target/` (Rust), `dist/` (Vite)

**Generated Assets:**
- **Index files:** User-selected directory (e.g., `~/.opnsense-log-viewer/indexes/`)
- **Enrichment cache:** `~/.opnsense-log-viewer/enrichment.json`
- **Logs:** `~/.opnsense-log-viewer/logs/` (app logs)

### Development Workflow Integration

#### Development Server Structure

**Development Mode:**

```bash
npm run tauri:dev
```

**Process:**
1. Vite dev server starts (`http://localhost:1420`)
2. Tauri builds Rust backend (debug mode)
3. Tauri window opens, loads Vite dev server
4. HMR enabled (frontend changes instant)
5. Rust recompilation on save

**File Watchers:**
- Vite watches `src/**/*` (frontend)
- Cargo watches `src-tauri/src/**/*` (backend)
- Tauri CLI orchestrates both

#### Build Process Structure

**Production Build:**

```bash
npm run tauri:build
```

**Steps:**
1. TypeScript compilation (`tsc`)
2. Vite production build (`vite build` → `dist/`)
3. Cargo release build (`cargo build --release` → `target/release/`)
4. Tauri bundler creates platform-specific packages

**Build Artifacts:**
- **Windows:** `target/release/bundle/msi/opnsense-log-viewer_0.1.0_x64_en-US.msi`
- **macOS:** `target/release/bundle/dmg/opnsense-log-viewer_0.1.0_universal.dmg`
- **Linux:**
  - `target/release/bundle/appimage/opnsense-log-viewer_0.1.0_amd64.AppImage`
  - `target/release/bundle/deb/opnsense-log-viewer_0.1.0_amd64.deb`

#### Deployment Structure

**Release Process:**

```
1. Version bump (cargo-release)
   ├── Update Cargo.toml version
   ├── Update package.json version
   ├── Update tauri.conf.json version
   └── Git tag (vX.Y.Z)

2. CI/CD pipeline (.github/workflows/release.yml)
   ├── Build matrix (Windows, macOS, Linux)
   ├── Run tests + benchmarks
   ├── Create bundles
   └── Upload to GitHub Releases

3. GitHub Release
   ├── Changelog (auto-generated from commits)
   ├── Installers (MSI, DMG, AppImage, deb)
   └── Checksums (SHA-256)
```

**Distribution:**
- GitHub Releases (primary)
- No auto-update in MVP (manual download)
- Code signing post-MVP (Windows/macOS)

---

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:**

Toutes les décisions architecturales fonctionnent ensemble sans conflits:
- ✅ **Stack cohérente:** Tauri v2 + Rust (Tokio) + React 18+ + TypeScript (strict mode)
- ✅ **Persistence compatible:** bincode 2.0.1 + Zstd 0.13.3 (serialization + compression, pas de conflits de version)
- ✅ **HTTP stack intégré:** reqwest 0.13.1 + reqwest-middleware + reqwest-retry (retry logic native)
- ✅ **Cross-platform unifié:** keyring 3.6.3 (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- ✅ **Frontend moderne:** Zustand 5.0.10 + TanStack Virtual 3.13.18 + react-hot-toast (React 18+ compatible)
- ✅ **Async runtime optimal:** Tokio 1.x avec features optimisées (rt-multi-thread, fs, io-util, time)
- ✅ **Observabilité cohérente:** tracing + tracing-subscriber + thiserror + anyhow (stack d'erreurs intégrée)

**Toutes les versions sont précisément spécifiées (pas de wildcard), garantissant la reproductibilité.**

**Pattern Consistency:**

Les patterns d'implémentation supportent parfaitement les décisions technologiques:
- ✅ **Naming conventions alignées:** snake_case (Rust) + camelCase (TypeScript) + serde rename_all (conversion automatique JSON)
- ✅ **Structure patterns cohérents:** Module-based (Rust avec mod.rs) + Feature-based (React avec barrel exports)
- ✅ **Tauri IPC patterns définis:** Commands (snake_case) + Events (kebab-case) + Result<T, String> pour errors
- ✅ **Error handling unifié:** thiserror (library errors) + anyhow (application code) + toast (frontend display)
- ✅ **Communication patterns clairs:** Zustand stores (selective subscriptions) + Tauri events (pub/sub) + HTTP retry (exponential backoff)

**Aucune contradiction détectée entre patterns et décisions technologiques.**

**Structure Alignment:**

La structure du projet supporte toutes les décisions architecturales:
- ✅ **Backend structure:** `src-tauri/src/` avec modules séparés (indexer/, parser/, query/, api_client/, storage/, export/, credentials/)
- ✅ **Frontend structure:** `src/` avec components feature-based (filter-builder/, log-results-table/, search-history/)
- ✅ **Test structure:** `tests/` (integration), `benches/` (performance), inline unit tests (`#[cfg(test)]`)
- ✅ **Boundaries respectées:** IPC layer (commands/), Domain services (indexer/, parser/, query/), Infrastructure (storage/, credentials/)
- ✅ **Integration points mappés:** Frontend↔Backend (Tauri IPC), Backend↔OPNsense API (HTTP), Backend↔OS keychain (keyring)

**La structure permet une séparation claire des préoccupations et évite les dépendances circulaires.**

### Requirements Coverage Validation ✅

**Epic/Feature Coverage:**

Tous les Functional Requirements (FR1-FR5) sont architecturalement supportés:

**FR1: Indexation haute performance (30GB en 2-3 min)**
- ✅ **Module `indexer/`** → Hybrid inverted multi-column index (IPs, ports) + bitmap index (action, protocol, interface)
- ✅ **Module `parser/`** → Streaming parsing (RFC3164, RFC5424, CSV filterlog) sans full-file read en RAM
- ✅ **Module `storage/`** → bincode serialization + Zstd compression (level 1 optimisé vitesse)
- ✅ **Performance gates CI/CD** → <7 sec/GB (±15% tolérance), build FAIL si dépassé
- ✅ **Benchmarks criterion** → Regression detection automatique
- ✅ **Files:** ~15 fichiers (indexer/inverted.rs, indexer/bitmap.rs, parser/rfc3164.rs, storage/persistence.rs, benches/indexation_benchmark.rs)

**FR2: Système de filtrage avancé (AND/OR/NOT, opérateurs)**
- ✅ **Module `query/executor.rs`** → Boolean query execution (AND/OR/NOT logic)
- ✅ **Module `query/operators.rs`** → Operator implementations (equals, contains, startswith, endswith, regex)
- ✅ **Frontend `filter-builder/`** → Visual filter UI (Field + Operator + Value)
- ✅ **Zustand `filter-store.ts`** → Filter state management avec actions immutables
- ✅ **Performance gates** → <750ms complex queries (±50% tolérance)
- ✅ **Files:** ~12 fichiers (query/executor.rs, filter-builder.tsx, filter-store.ts, benches/query_benchmark.rs)

**FR3: Enrichissement via API OPNsense**
- ✅ **Module `api_client/`** → HTTP client avec retry logic (3 retries, exponential backoff 100ms→400ms→1.6s)
- ✅ **Module `api_client/enrichment.rs`** → Interface mapping (vtnet0→LAN), rule labels, alias resolution
- ✅ **Module `credentials/`** → OS keychain integration (Windows/macOS/Linux native)
- ✅ **Backup JSON** → Graceful degradation (API unavailable → backup enrichment.json → raw data)
- ✅ **Frontend `credentials-form/`** → Credential input UI avec test connection button
- ✅ **Files:** ~10 fichiers (api_client/client.rs, enrichment.rs, credentials/manager.rs, credentials-form.tsx)

**FR4: Opérations core fiables**
- ✅ **Module `parser/`** → Multi-format parsing (RFC3164/5424/CSV) avec error handling robuste (proptest fuzzing)
- ✅ **Module `storage/persistence.rs`** → Checksums SHA-256, atomic writes (.idx.tmp → .idx rename)
- ✅ **Module `export/`** → JSON/CSV export avec métadonnées compliance (timestamp, hash, filter criteria)
- ✅ **Memory-safe Rust** → Pas de buffer overflows, validation inputs
- ✅ **Idempotence** → Ré-indexation produit résultats identiques (checksums vérifiables)
- ✅ **Files:** ~8 fichiers (parser/rfc3164.rs, storage/persistence.rs, export/metadata.rs)

**FR5: Application multiplateforme (Windows/Linux/macOS)**
- ✅ **Tauri v2** → Cross-platform par design, native windowing
- ✅ **Configuration `tauri.conf.json`** → Bundling MSI (Windows), DMG (macOS), AppImage/deb (Linux)
- ✅ **CI/CD matrix builds** → `.github/workflows/build.yml` (Windows, macOS, Linux parallèles)
- ✅ **OS keychain abstraction** → `keyring` crate unifie Windows Credential Manager, macOS Keychain, Linux Secret Service
- ✅ **Files:** ~5 configuration files (tauri.conf.json, build.yml, Cargo.toml target configs)

**Functional Requirements Coverage:** **100%** - Tous les FR mappés à des modules et fichiers spécifiques.

**Non-Functional Requirements Coverage:**

**Performance (critiques pour succès):**
- ✅ **Indexation <6 sec/GB** → Gates CI/CD enforced (<7s/GB avec ±15% tolérance) via criterion benchmarks
- ✅ **Recherche <500ms** → Gates CI/CD enforced (<750ms avec ±50% tolérance)
- ✅ **Utilisation RAM <500 MB** → Streaming architecture, chunking intelligent, LRU cache avec limites strictes
- ✅ **Zero crashes sur 30GB** → Memory-safe Rust, proptest fuzzing, error handling explicite (thiserror + anyhow)
- ✅ **Enforcement:** Build failures automatiques si gates dépassés

**Sécurité (defense-in-depth):**
- ✅ **Credentials chiffrés** → OS keychain natif (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- ✅ **Traitement 100% local** → Aucune donnée envoyée vers services externes
- ✅ **Parsing sandboxé** → Privilèges minimaux, pas d'accès réseau, pas d'écriture filesystem hors cache désigné
- ✅ **Validation inputs** → Backup enrichment JSON validé contre injection malveillante
- ✅ **Memory-safe parsing** → Rust garantit pas de buffer overflows
- ✅ **TLS via rustls** → Cross-platform, pas de dépendance OpenSSL

**Conformité & Auditabilité:**
- ✅ **Export metadata complet** → Timestamp ISO 8601, SHA-256 hash fichier source, critères filtrage appliqués, version outil, identification opérateur
- ✅ **Chaîne de preuve intacte** → Checksums indexes, métadonnées tamper-evident
- ✅ **Option anonymisation IP** → GDPR-ready pour reporting
- ✅ **Rétention configurable** → Auto-expiration indexes avec checksums détection corruption

**Fiabilité:**
- ✅ **Opérations idempotentes** → Ré-indexation produit résultats identiques (checksums vérifiables)
- ✅ **Récupération automatique** → Cleanup indexes partiels, graceful degradation API unavailable
- ✅ **Error handling explicite** → Pattern thiserror + anyhow + toast (pas d'optimistic error handling)
- ✅ **Graceful degradation cascade** → API indisponible → backup enrichment JSON → raw data (UI jamais bloquée)

**Non-Functional Requirements Coverage:** **100%** - Tous les NFRs architecturalement supportés avec enforcement automatisé.

### Implementation Readiness Validation ✅

**Decision Completeness:**

- ✅ **23 décisions architecturales documentées** avec versions précises (bincode 2.0.1, Zstd 0.13.3, reqwest 0.13.1, keyring 3.6.3, etc.)
- ✅ **Rationales explicites** pour chaque décision (performance, cross-platform, security, developer experience)
- ✅ **Trade-offs analysés** et documentés (ex: Zstd level 1 vs 3 → vitesse vs compression, diminishing returns identifiés)
- ✅ **Party Mode review intégrée** (2 sessions):
  - Session 1: Retry policy, multi-instance credentials, error UX pattern, integration test strategy, fixture strategy, E2E scope
  - Session 2: Zstd level optimization, Tokio features, coverage targets, timeline ajustements learning curves
- ✅ **Configuration examples fournis** pour chaque décision majeure (code snippets Rust + TypeScript)

**Aucune décision critique manquante - AI agents peuvent implémenter immédiatement.**

**Structure Completeness:**

- ✅ **100+ fichiers spécifiés** dans le project tree complet
- ✅ **Tous les répertoires définis** avec leur rôle (src/, src-tauri/src/, tests/, benches/, .github/workflows/)
- ✅ **Tous les fichiers de configuration listés** (package.json, Cargo.toml, tsconfig.json, tailwind.config.js, tauri.conf.json, etc.)
- ✅ **Mapping requirements → files** explicite (FR1 → indexer/inverted.rs + bitmap.rs, FR2 → query/executor.rs, etc.)
- ✅ **Integration points spécifiés** avec exemples de code (Tauri IPC commands, Zustand stores, HTTP client, OS keychain)
- ✅ **Boundaries architecturales claires** (IPC, Component, Service, Data boundaries documentées avec diagrammes)

**Structure prête pour scaffolding immédiat via `npm create tauri-app@latest`.**

**Pattern Completeness:**

- ✅ **15 conflict points identifiés et résolus** avec patterns exhaustifs:
  - Naming (Backend Rust, Frontend React/TypeScript, IPC Tauri, Events)
  - Structure (Backend modules, Frontend components, Tests)
  - Format (IPC responses, Date/Time, JSON fields)
  - Communication (Zustand stores, Tauri events, HTTP retry)
  - Process (Error handling, Loading states)
- ✅ **Exemples concrets ✅ DO / ❌ DON'T** pour chaque pattern (code snippets complets)
- ✅ **Anti-patterns documentés** avec corrections (inconsistent naming, wrong file structure, wrong import order)
- ✅ **Enforcement guidelines** (automated linting via clippy/ESLint, code review checklist)
- ✅ **Pattern Compliance Checklist** pour vérification avant commits

**AI agents peuvent implémenter sans ambiguïté - tous les conflict points sont résolus.**

### Gap Analysis Results

**Critical Gaps:** ✅ **AUCUN GAP CRITIQUE IDENTIFIÉ**

Tous les éléments bloquants pour l'implémentation sont documentés:
- ✅ Toutes les décisions critiques prises (persistence format, compression, HTTP client, credentials, state management, virtual scrolling, error display)
- ✅ Tous les patterns de conflit résolus (15 conflict points avec exemples DO/DON'T)
- ✅ Structure complète et spécifique (100+ fichiers définis)
- ✅ Integration points tous mappés (IPC, API, OS keychain)

**Important Gaps:** ✅ **AUCUN GAP IMPORTANT IDENTIFIÉ**

Tous les éléments importants pour une implémentation fluide sont couverts:
- ✅ Requirements 100% mappés à la structure (FR1-FR5 → modules spécifiques)
- ✅ Boundaries clairement définies (API, Component, Service, Data)
- ✅ Data flow complet documenté (diagrammes + transformations)
- ✅ Test strategy comprehensive (Unit, Integration, Property-based, E2E, Benchmarks)
- ✅ CI/CD enforcement strategy (performance gates, coverage targets)

**Nice-to-Have Gaps (Déférés Post-MVP - Acceptés):**

⚠️ **CLI Arguments Support (clap v4):**
- **Status:** Déféré post-MVP
- **Rationale:** Pas requis pour GUI desktop app MVP, peut être ajouté si users demandent automation
- **Impact:** Aucun sur MVP scope

⚠️ **Multi-Instance Credentials:**
- **Status:** Déféré post-MVP
- **Rationale:** Single OPNsense instance suffit pour MVP, évite Windows 1KB credential limit complexity
- **Impact:** Feature add-on future, architecture supporte déjà (keyring permet multiple entries)

⚠️ **Advanced Visualizations (Charts, Graphs, Timelines):**
- **Status:** Déféré post-MVP
- **Rationale:** Focus MVP sur filtrage et export, visualizations = value-add post-launch
- **Impact:** UI peut intégrer libraries (Chart.js, Recharts) sans refactoring architectural

⚠️ **Plugin Architecture (Extensibility):**
- **Status:** Déféré post-MVP
- **Rationale:** Pas de requirement externe plugins, architecture modulaire permet ajout facile future
- **Impact:** Peut exposer Rust modules via dynamic loading post-MVP si demandé

**Documentation Supplémentaire Possible (Optionnelle):**
💡 Guide de contribution pour développeurs externes (CONTRIBUTING.md)
💡 Architecture Decision Records (ADR) formalisés (docs/adr/)
💡 Diagrammes UML/C4 pour visualisation (docs/diagrams/)

**Tooling Recommandations (Optionnelles):**
💡 Pre-commit hooks (rustfmt, clippy, prettier via husky)
💡 Conventional commits enforcement (commitlint)
💡 Automated dependency updates (Dependabot configuration)

**Ces gaps nice-to-have et suggestions optionnelles ne bloquent PAS l'implémentation et peuvent être ajoutés progressivement selon besoins utilisateurs.**

### Validation Issues Addressed

**Critical Issues:** ✅ **AUCUN ISSUE CRITIQUE DÉTECTÉ**

**Important Issues:** ✅ **AUCUN ISSUE IMPORTANT DÉTECTÉ**

**Minor Observations Addressed:**

✅ **Party Mode Session 1 - Gaps Comblés:**
- ✅ Retry policy spécifié (3 retries, exponential backoff 100ms→400ms→1.6s, timeout 10s)
- ✅ Multi-instance credentials decision (déféré post-MVP, single instance MVP)
- ✅ Error UX pattern défini (react-hot-toast pour recoverable, modal pour critical)
- ✅ Integration test strategy (Tauri mock builder pour IPC commands)
- ✅ 30GB fixture strategy (streaming generator, pas de LFS storage overhead)
- ✅ E2E scope limité (2 critical paths: happy path + error path, pas exhaustive)

✅ **Party Mode Session 2 - Optimisations Appliquées:**
- ✅ Zstd compression level optimisé (level 1 vs 3, profile-driven tuning rationale)
- ✅ Tokio features optimisés (rt-multi-thread, fs, io-util, time - pas "full" features)
- ✅ Coverage targets précisés (90% critical, 75% important, 60% UI - pas vague ">80%")
- ✅ Timeline estimates ajustés (proptest learning 12h vs 4h initial, Tailwind 6h vs 2h)

✅ **Technical Preferences Clarification:**
- User confirmed: React (choix 1), TypeScript (suggestion acceptée), Tokio (suggestion acceptée)
- User délégué décisions techniques: "tu es mieux placé pour moi pour savoir quels choix faire"

**Toutes les préoccupations identifiées pendant le processus ont été adressées et résolues.**

### Architecture Completeness Checklist

#### ✅ Requirements Analysis

- [x] **Project context thoroughly analyzed** (Product brief + PRD + UX design spec explorés)
- [x] **Scale and complexity assessed** (Medium-High complexity, 10-12 composants majeurs, 30GB logs en 2-3 min)
- [x] **Technical constraints identified** (Tauri v2 + Rust imposé, <500 MB RAM strict, multi-format parsing, cross-platform)
- [x] **Cross-cutting concerns mapped** (Performance end-to-end, gestion mémoire efficace, sécurité multi-couches, offline-first, evidence integrity, observabilité, UX dense mais intuitive)

#### ✅ Architectural Decisions

- [x] **Critical decisions documented with versions** (23 décisions avec versions précises: bincode 2.0.1, Zstd 0.13.3, reqwest 0.13.1, keyring 3.6.3, Zustand 5.0.10, TanStack Virtual 3.13.18, etc.)
- [x] **Technology stack fully specified** (Tauri v2 + Rust (Tokio 1.x) + React 18+ + TypeScript strict mode + Vite 6+ + Tailwind CSS 3.4+)
- [x] **Integration patterns defined** (Tauri IPC commands/events, OPNsense API HTTP retry, OS keychain abstraction, Zustand stores selective subscriptions)
- [x] **Performance considerations addressed** (Gates CI/CD <7s/GB <750ms <600MB enforced, streaming architecture, chunking intelligent, LRU caching, compression Zstd level 1)

#### ✅ Implementation Patterns

- [x] **Naming conventions established** (snake_case Rust, camelCase TypeScript, PascalCase types, SCREAMING_SNAKE_CASE constants, kebab-case files/events)
- [x] **Structure patterns defined** (Module-based Rust avec mod.rs, Feature-based React avec barrel exports, tests co-located)
- [x] **Communication patterns specified** (Tauri IPC Result<T,String>, Zustand immutable updates, HTTP exponential backoff, Event pub/sub)
- [x] **Process patterns documented** (Error handling thiserror+anyhow+toast, Loading states boolean/status enum, Graceful degradation cascade)

#### ✅ Project Structure

- [x] **Complete directory structure defined** (100+ fichiers spécifiés: src/, src-tauri/src/, tests/, benches/, .github/workflows/)
- [x] **Component boundaries established** (IPC layer commands/, Domain services indexer/parser/query/, Infrastructure storage/credentials/, Frontend components/)
- [x] **Integration points mapped** (Frontend↔Backend Tauri IPC, Backend↔OPNsense API HTTP, Backend↔OS keychain keyring, Backend modules dependency graph)
- [x] **Requirements to structure mapping complete** (FR1→indexer/parser/storage ~15 files, FR2→query/filter-builder ~12 files, FR3→api_client/credentials ~10 files, FR4→parser/export ~8 files, FR5→tauri.conf.json/CI ~5 files)

### Architecture Readiness Assessment

**Overall Status:** ✅ **READY FOR IMPLEMENTATION**

**Confidence Level:** **HIGH**

**Rationale:**

1. ✅ **Validation complète passée** sans issues critiques ou importantes
2. ✅ **Tous les requirements architecturalement supportés** (FR1-FR5 100%, NFRs 100%)
3. ✅ **Patterns exhaustifs avec exemples concrets** (15 conflict points résolus avec DO/DON'T examples)
4. ✅ **Structure prête pour scaffolding** (100+ fichiers définis, commande init fournie)
5. ✅ **Party Mode review intégrée** (2 sessions complètes, gaps identifiés et résolus)
6. ✅ **Timeline réaliste avec learning curves factored** (proptest 12h vs 4h, Tailwind 6h vs 2h)
7. ✅ **Performance gates enforced** (Build failures automatiques si régression)
8. ✅ **Security-first design** (OS keychain, memory-safe, 100% local processing)

**Key Strengths:**

1. **Hybrid Indexation Strategy** → Performance optimale (inverted multi-column index pour IPs/ports + bitmap index pour action/protocol/interface) permet requêtes <500ms sur 30GB
2. **Comprehensive Pattern Documentation** → 15 conflict points résolus avec exemples ✅ DO / ❌ DON'T prévient divergence AI agents
3. **Graceful Degradation Design** → Cascade API unavailable → backup enrichment JSON → raw data garantit UI jamais bloquée
4. **Performance Gates Enforcement** → CI/CD build failures automatiques si <7s/GB ou <750ms ou <600MB dépassés, régression impossible
5. **Security-First Approach** → OS keychain natif (Windows Credential Manager, macOS Keychain, Linux Secret Service) + memory-safe Rust + TLS rustls + 100% local processing + sandboxed parsing
6. **Complete Test Strategy** → Unit tests (inline + Vitest), Integration tests (tests/), Property-based testing (proptest fuzzing), E2E tests (Tauri WebDriver 2 critical paths), Performance benchmarks (criterion avec gates)

**Areas for Future Enhancement (Post-MVP):**

- CLI arguments support via clap v4 (si users demandent automation/scripting)
- Multi-instance OPNsense credentials (évite Windows 1KB credential limit pour MVP)
- Advanced visualizations (charts, graphs, timelines - value-add post-launch)
- Plugin architecture pour extensibilité (custom enrichment sources, export formats)
- Code signing pour Windows/macOS distributions (production-ready releases)
- Automated dependency updates (Dependabot, renovate)
- Pre-commit hooks (rustfmt, clippy, prettier enforcement)
- Architecture Decision Records formalisés (ADR documentation)

### Implementation Handoff

**AI Agent Guidelines:**

1. ✅ **Follow all architectural decisions exactly** as documented:
   - Use specified versions précises (bincode 2.0.1, Zstd 0.13.3, reqwest 0.13.1, keyring 3.6.3, Zustand 5.0.10, TanStack Virtual 3.13.18)
   - Apply configurations exactes (Tokio features, Zstd level 1, retry policy 3 retries exponential backoff, HTTP timeout 10s)
   - Respect trade-offs analysés (pas de modifications sans review architecturale)

2. ✅ **Use implementation patterns consistently** across all components:
   - Naming: snake_case (Rust functions), PascalCase (types), camelCase (TypeScript), kebab-case (files/events), SCREAMING_SNAKE_CASE (constants)
   - Structure: Module-based (Rust avec mod.rs + submodules), Feature-based (React avec barrel exports index.ts)
   - Communication: Tauri IPC Result<T, String>, Zustand immutable updates, HTTP exponential backoff, Event pub/sub
   - Error handling: thiserror (library) + anyhow (application) + toast (frontend display)

3. ✅ **Respect project structure and boundaries** (no reorganization without updating this document):
   - Backend: src-tauri/src/ avec modules séparés (indexer/, parser/, query/, api_client/, storage/, export/, credentials/, commands/)
   - Frontend: src/ avec components feature-based (filter-builder/, log-results-table/, search-history/, export-dialog/, credentials-form/)
   - Tests: tests/ (integration), benches/ (performance), inline unit tests (#[cfg(test)] Rust, *.test.tsx React)
   - No circular dependencies allowed (dependency graph: commands/ → domain services → infrastructure)

4. ✅ **Refer to this document for all architectural questions**:
   - Patterns: Section "Implementation Patterns & Consistency Rules" avec 15 conflict points résolus
   - Decisions: Section "Core Architectural Decisions" avec 23 décisions documentées
   - Structure: Section "Project Structure & Boundaries" avec 100+ fichiers définis
   - Validation: Cette section "Architecture Validation Results" pour coherence checks

**First Implementation Step:**

```bash
# Story 0.1: Project Scaffolding avec create-tauri-app (2h)
npm create tauri-app@latest opnsense-log-viewer -- --template react-ts
cd opnsense-log-viewer
npm install

# Verify scaffolding
npm run tauri dev  # Should open empty Tauri window with React
```

**Implementation Sequence (Phase 0 - Foundation):**

```yaml
Story 0.1: Project Scaffolding
  Duration: 2h
  Command: npm create tauri-app@latest opnsense-log-viewer -- --template react-ts
  Deliverable: Tauri + React + TypeScript base project

Story 0.2: Test Infrastructure Setup (CRITICAL - bloque Stories 1.x+)
  Duration: 12h (proptest + criterion learning curve factored)
  Tasks:
    - Setup Vitest + React Testing Library (frontend unit tests)
    - Setup cargo bench framework (criterion.rs for performance regression tests)
    - Create 30GB synthetic log fixture generator (streaming, RFC3164/5424/CSV formats)
    - Configure CI/CD performance gates (.github/workflows/bench.yml):
      * Indexation: <7 sec/GB (±15% tolerance) = build FAIL si dépassé
      * Query: <750ms complex queries (±50% tolerance) = build FAIL si dépassé
      * Memory: <600 MB peak (±20% tolerance) = build FAIL si dépassé
    - Setup property-based testing (proptest crate for parser robustness fuzzing)
    - Create integration test suite (Tauri IPC commands via mock builder)
  Deliverable: Complete test infrastructure avec performance gates enforced

Story 0.3: Tailwind CSS Integration (UX Design Spec requirement)
  Duration: 6h (dark mode complexity factored)
  Tasks:
    - Install Tailwind CSS + PostCSS + Autoprefixer
    - Configure tailwind.config.js (design tokens: colors, spacing, typography)
    - Setup dark/light theme via Tailwind dark: variant
    - Integrate avec Vite build pipeline
    - Create base UI components (Button, Input, Select, Modal, Spinner, ProgressBar)
  Deliverable: Tailwind CSS configured avec dark mode + base UI primitives

Story 0.4: Error Handling Pattern Setup
  Duration: 2h
  Tasks:
    - Install react-hot-toast
    - Create error handling utilities (toast wrappers)
    - Define error display patterns (recoverable toast, critical modal)
    - Setup error boundaries (React)
  Deliverable: Error handling patterns ready for all components
```

**Ensuite:** **Phase 1 - Backend Core** (Stories 1.x-3.x)
- Story 1.0: Core Indexation Engine - Inverted Index
- Story 1.1: Core Indexation Engine - Bitmap Index
- Story 1.2: Hybrid Index Orchestration
- Story 2.0: Multi-Format Parser (RFC3164/5424/CSV)
- Story 3.0: OPNsense API Client avec retry logic
- etc.

**Reference Document:**
`_bmad-output/planning-artifacts/architecture.md` (ce document complet)

**Performance Gates Reminder:**
- ❌ Build FAILS si indexation >7s/GB (±15%)
- ❌ Build FAILS si query >750ms (±50%)
- ❌ Build FAILS si memory >600MB (±20%)

**Security Requirements Reminder:**
- ✅ Credentials MUST use OS keychain (keyring crate)
- ✅ Parsing MUST be sandboxed (pas d'écriture filesystem hors cache)
- ✅ API calls MUST use TLS (rustls)
- ✅ NO external data transmission (100% local processing)

**Pattern Compliance Reminder:**
- ✅ Check Pattern Compliance Checklist avant chaque commit
- ✅ Run automated linting (clippy, ESLint, rustfmt, prettier)
- ✅ Refer to DO/DON'T examples si incertain

---

**Architecture Document Complete - Ready for Implementation** ✅

## Architecture Completion Summary

### Workflow Completion

**Architecture Decision Workflow:** COMPLETED ✅
**Total Steps Completed:** 8
**Date Completed:** 2026-01-16
**Document Location:** `_bmad-output/planning-artifacts/architecture.md`

### Final Architecture Deliverables

**📋 Complete Architecture Document**

- All architectural decisions documented with specific versions (23 décisions avec bincode 2.0.1, Zstd 0.13.3, reqwest 0.13.1, keyring 3.6.3, Zustand 5.0.10, TanStack Virtual 3.13.18, etc.)
- Implementation patterns ensuring AI agent consistency (15 conflict points résolus avec exemples ✅ DO / ❌ DON'T)
- Complete project structure with all files and directories (100+ fichiers spécifiés)
- Requirements to architecture mapping (FR1-FR5 → modules spécifiques)
- Validation confirming coherence and completeness (Coherence ✅, Coverage ✅, Readiness ✅)

**🏗️ Implementation Ready Foundation**

- **23 architectural decisions made** (Data architecture, Backend architecture, Frontend architecture, Testing strategy, Infrastructure & deployment)
- **15 implementation patterns defined** (Naming, Structure, Format, Communication, Process patterns)
- **10-12 architectural components specified** (Indexation Engine, Multi-Format Parser, Query Engine, OPNsense API Client, Enrichment Manager, Frontend UI, IPC Layer, Index Persistence, Export Engine, Credential Manager, Configuration Manager, Search History Manager)
- **5 functional requirements fully supported** (FR1: Indexation haute performance, FR2: Système de filtrage avancé, FR3: Enrichissement API OPNsense, FR4: Opérations core fiables, FR5: Application multiplateforme)

**📚 AI Agent Implementation Guide**

- **Technology stack with verified versions:**
  - Backend: Tauri v2, Rust 1.70+, Tokio 1.x (features optimisées)
  - Frontend: React 18+, TypeScript (strict mode), Vite 6+, Tailwind CSS 3.4+
  - Persistence: bincode 2.0.1, Zstd 0.13.3 (level 1)
  - HTTP: reqwest 0.13.1 + reqwest-middleware + reqwest-retry
  - Security: keyring 3.6.3 (Windows/macOS/Linux)
  - State: Zustand 5.0.10
  - Virtual scrolling: TanStack Virtual 3.13.18
  - Testing: proptest 1.x, criterion 0.5.x, Vitest 2.1+

- **Consistency rules that prevent implementation conflicts:**
  - Naming: snake_case (Rust), camelCase (TypeScript), PascalCase (types), kebab-case (files/events)
  - Structure: Module-based (Rust avec mod.rs), Feature-based (React avec barrel exports)
  - Error handling: thiserror (library) + anyhow (application) + react-hot-toast (frontend)
  - IPC: Commands (snake_case), Events (kebab-case), Result<T, String> errors

- **Project structure with clear boundaries:**
  - API Boundaries: Tauri IPC (Commands + Events), OPNsense API (HTTP endpoints), OS keychain (credentials)
  - Component Boundaries: Frontend components → Zustand stores → Tauri IPC → Backend services
  - Service Boundaries: Commands → Domain services (indexer, parser, query) → Infrastructure (storage, credentials)
  - Data Boundaries: Log files → Parser → Indexer → Storage (compressed indexes) → Query → Export

- **Integration patterns and communication standards:**
  - Frontend ↔ Backend: Tauri IPC commands (request/response) + Events (pub/sub)
  - Backend ↔ OPNsense API: HTTP client avec retry logic (3 retries, exponential backoff)
  - Backend ↔ OS keychain: keyring abstraction (Windows Credential Manager, macOS Keychain, Linux Secret Service)
  - Frontend state: Zustand stores avec selective subscriptions (immutable updates)

### Implementation Handoff

**For AI Agents:**

This architecture document is your complete guide for implementing **opnsense-log-viewer**. Follow all decisions, patterns, and structures exactly as documented.

**First Implementation Priority:**

```bash
# Story 0.1: Project Scaffolding (2h)
npm create tauri-app@latest opnsense-log-viewer -- --template react-ts
cd opnsense-log-viewer
npm install

# Verify scaffolding
npm run tauri dev  # Should open empty Tauri window with React
```

**Development Sequence:**

1. **Initialize project using documented starter template**
   - Execute: `npm create tauri-app@latest opnsense-log-viewer -- --template react-ts`
   - Verify: `npm run tauri dev` opens Tauri window

2. **Set up development environment per architecture (Story 0.2-0.4)**
   - Story 0.2: Test Infrastructure Setup (12h - proptest/criterion learning factored)
     - Vitest + React Testing Library (frontend unit tests)
     - cargo bench framework (criterion.rs for performance gates)
     - 30GB synthetic log fixture generator (streaming, RFC3164/5424/CSV)
     - CI/CD performance gates (.github/workflows/bench.yml: <7s/GB, <750ms query, <600MB RAM)
     - Property-based testing (proptest for parser fuzzing)
     - Integration test suite (Tauri IPC commands via mock builder)
   - Story 0.3: Tailwind CSS Integration (6h - dark mode complexity factored)
     - Install Tailwind CSS + PostCSS + Autoprefixer
     - Configure tailwind.config.js (design tokens, dark mode)
     - Create base UI components (Button, Input, Select, Modal, Spinner, ProgressBar)
   - Story 0.4: Error Handling Pattern Setup (2h)
     - Install react-hot-toast
     - Create error handling utilities (toast wrappers)
     - Define error display patterns (recoverable toast, critical modal)

3. **Implement core architectural foundations (Phase 1 - Backend Core)**
   - Story 1.0: Core Indexation Engine - Inverted Index
   - Story 1.1: Core Indexation Engine - Bitmap Index
   - Story 1.2: Hybrid Index Orchestration
   - Story 2.0: Multi-Format Parser (RFC3164/5424/CSV)
   - Story 3.0: OPNsense API Client avec retry logic
   - Story 4.0: Credential Manager (OS keychain integration)

4. **Build features following established patterns (Phase 2 - Frontend Core)**
   - Story 6.0: Filter Builder UI (React Hook Form + Zustand)
   - Story 6.1: Log Results Table (TanStack Virtual scrolling)
   - Story 6.2: Search History Panel (localStorage persistence)
   - Story 7.0: Export Dialog (JSON/CSV with metadata)

5. **Maintain consistency with documented rules**
   - Follow Pattern Compliance Checklist before each commit
   - Run automated linting (clippy, ESLint, rustfmt, prettier)
   - Refer to DO/DON'T examples si incertain
   - Respect performance gates (build failures si régression)

### Quality Assurance Checklist

#### ✅ Architecture Coherence

- [x] All decisions work together without conflicts (Stack cohérente Tauri+Rust+React+TypeScript, versions compatibles)
- [x] Technology choices are compatible (bincode+Zstd, reqwest+retry, Tokio+Tauri, Zustand+React 18+)
- [x] Patterns support the architectural decisions (Naming conventions alignées, Structure cohérente, IPC patterns définis)
- [x] Structure aligns with all choices (Backend modules séparés, Frontend feature-based, Boundaries respectées)

#### ✅ Requirements Coverage

- [x] All functional requirements are supported (FR1→indexer/parser/storage, FR2→query/filter-builder, FR3→api_client/credentials, FR4→parser/export, FR5→tauri.conf.json/CI)
- [x] All non-functional requirements are addressed (Performance gates CI/CD, Security OS keychain+memory-safe, Conformité export metadata+checksums, Fiabilité idempotence+graceful degradation)
- [x] Cross-cutting concerns are handled (Logging tracing+JSON, Error handling thiserror+anyhow+toast, Testing Unit+Integration+Property-based+E2E+Benchmarks)
- [x] Integration points are defined (Frontend↔Backend Tauri IPC, Backend↔OPNsense API HTTP, Backend↔OS keychain keyring, Backend modules dependency graph)

#### ✅ Implementation Readiness

- [x] Decisions are specific and actionable (23 décisions avec versions précises, configurations exactes, rationales explicites)
- [x] Patterns prevent agent conflicts (15 conflict points résolus avec exemples ✅ DO / ❌ DON'T, enforcement guidelines)
- [x] Structure is complete and unambiguous (100+ fichiers spécifiés, directory tree complet, mapping requirements→files)
- [x] Examples are provided for clarity (Code snippets Rust+TypeScript pour chaque pattern, anti-patterns documentés)

### Project Success Factors

**🎯 Clear Decision Framework**

Every technology choice was made collaboratively with clear rationale (performance optimale, cross-platform support, developer experience, security-first approach), ensuring all stakeholders understand the architectural direction. Party Mode review sessions (2 sessions complètes) ont identifié et résolu gaps critiques (retry policy, multi-instance credentials, error UX pattern, test strategy, fixture strategy, E2E scope).

**🔧 Consistency Guarantee**

Implementation patterns and rules ensure that multiple AI agents will produce compatible, consistent code that works together seamlessly. 15 conflict points identifiés et résolus avec exemples concrets ✅ DO / ❌ DON'T préviennent divergence implementation. Automated enforcement via linting (clippy, ESLint, rustfmt, prettier) garantit compliance.

**📋 Complete Coverage**

All project requirements are architecturally supported, with clear mapping from business needs to technical implementation:
- FR1 (Indexation 30GB en 2-3 min) → indexer/inverted.rs + bitmap.rs + storage/persistence.rs (~15 files)
- FR2 (Filtrage avancé AND/OR/NOT) → query/executor.rs + filter-builder/ + filter-store.ts (~12 files)
- FR3 (Enrichissement API OPNsense) → api_client/ + credentials/ + credentials-form/ (~10 files)
- FR4 (Opérations core fiables) → parser/ + export/ + storage/checksums (~8 files)
- FR5 (Multiplateforme Windows/Linux/macOS) → tauri.conf.json + CI matrix builds (~5 files)

**🏗️ Solid Foundation**

The chosen starter template (create-tauri-app react-ts) and architectural patterns provide a production-ready foundation following current best practices:
- Tauri v2: Cross-platform desktop app framework avec native performance
- Rust + Tokio: Memory-safe, high-performance backend avec async/await
- React 18+ + TypeScript: Modern frontend avec strict type safety
- Tailwind CSS: Utility-first styling avec dark mode support
- Comprehensive testing: Unit (inline + Vitest), Integration (tests/), Property-based (proptest), E2E (Tauri WebDriver), Performance (criterion avec gates CI/CD)

---

**Architecture Status:** ✅ **READY FOR IMPLEMENTATION**

**Next Phase:** Begin implementation using the architectural decisions and patterns documented herein.

**Document Maintenance:** Update this architecture when major technical decisions are made during implementation. Minor adjustments (dependency version bumps, pattern refinements) can be made in-line, but major changes (technology stack changes, architectural pattern shifts) should be documented via architecture updates.

**Reference Guide:** This document serves as the single source of truth for all technical decisions. AI agents must read relevant sections before implementing any code to ensure consistency and prevent conflicts.

---

_Architecture Decision Document - Version 1.0 - Ready for Implementation_
