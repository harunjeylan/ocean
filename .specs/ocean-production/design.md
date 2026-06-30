# Design Document: Ocean Production Readiness

## Overview

This phase adds production-grade capabilities: runtime modes with pre-tuned profiles, structured observability (JSON event log + metrics counters), filesystem sandboxing, read-only mode, and deployment configuration validation. These are the final remaining items from Sections 10–11 of the project plan.

### Key Design Decisions

1. **Runtime mode as a profile selector** — Rather than deeply changing behavior, each mode is a set of pre-tuned defaults for thread counts, cache sizes, and batch sizes. The same code paths run regardless of mode. This keeps the system simple while allowing optimization per deployment.
2. **JSON events over structured logging library** — A lightweight `serde_json`-based event emitter avoids adding a logging framework dependency. Events are written to stderr to not interfere with stdout-based CLI output.
3. **Atomic counters for metrics** — `AtomicU64` per counter is lock-free, zero-cost for the happy path, and sufficient for the level of observability needed.
4. **Canonicalized path sandboxing** — Using `std::fs::canonicalize()` on both the workspace root and the target path, then checking the target starts with the root, prevents symlink traversal attacks.

---

## Architecture

### Component diagram

```text
CLI / API
    │
    ├── RuntimeMode (Desktop | Server | Embedded)
    │     └── provides tuned defaults for config
    │
    ├── Sandbox
    │     └── validate(path) → Ok/Err
    │
    ├── EventEmitter
    │     ├── ConsoleEmitter (human-readable)
    │     └── JsonEmitter (JSON lines → stderr)
    │
    ├── MetricsCollector
    │     └── atomic counters + snapshot()
    │
    └── ReadOnlyGuard
          └── blocks write operations
```

### Event flow

```text
IndexOrchestrator::run()
    ├── emit(IndexStarted { dir, total_files })
    ├── for each file:
    │     ├── Sandbox::validate(path)
    │     ├── emit(FileProcessed { path, status, duration_ms })
    │     └── MetricsCollector::increment_files_indexed()
    └── emit(IndexComplete { report })

QueryEngine::execute()
    ├── MetricsCollector::increment_queries_total()
    ├── check query cache
    │     ├── hit → MetricsCollector::increment_queries_cached()
    │     └── miss → MetricsCollector::increment_embedding_calls()
    ├── emit(QueryExecuted { query, results, duration_ms })
    └── MetricsCollector::snapshot() on request
```

---

## Components and Interfaces

### 1. RuntimeMode

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeMode {
    Desktop,
    Server,
    Embedded,
}

impl RuntimeMode {
    /// Returns the default configuration overrides for this mode.
    pub fn defaults(&self) -> ModeDefaults;

    /// Auto-detect from available resources.
    pub fn auto_detect() -> Self;
}

#[derive(Debug, Clone)]
pub struct ModeDefaults {
    pub io_threads: Option<usize>,       // None = keep existing default
    pub cpu_threads: Option<usize>,
    pub max_ai_concurrent: Option<usize>,
    pub embedding_cache_size: Option<usize>,
    pub query_cache_size: Option<usize>,
    pub max_in_flight: Option<usize>,
    pub max_queue_size: Option<usize>,
    pub embedding_batch_size: Option<usize>,
}

impl Default for ModeDefaults {
    fn default() -> Self {
        RuntimeMode::Desktop.defaults()
    }
}
```

### 2. SystemEvent

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum SystemEvent {
    IndexStarted {
        timestamp: u64,  // unix millis
        dir: String,
        total_files: u64,
    },
    IndexComplete {
        timestamp: u64,
        duration_ms: u64,
        indexed: u64,
        skipped: u64,
        failed: u64,
    },
    FileProcessed {
        timestamp: u64,
        path: String,
        status: FileIndexStatus,  // Indexed | Skipped | Failed
        duration_ms: u64,
    },
    QueryExecuted {
        timestamp: u64,
        query: String,
        mode: String,
        num_results: usize,
        duration_ms: u64,
        cached: bool,
    },
    BackpressureEvent {
        timestamp: u64,
        action: String,  // "paused" | "resumed"
        queue_len: usize,
        in_flight: u32,
    },
    ErrorEvent {
        timestamp: u64,
        severity: String, // "warn" | "error" | "fatal"
        module: String,
        message: String,
    },
}
```

### 3. EventEmitter

```rust
pub trait EventEmitter: Send + Sync {
    fn emit(&self, event: SystemEvent);
    fn set_output(&mut self, target: OutputTarget);
}

pub enum OutputTarget {
    Stderr,
    File(PathBuf),
    Both(PathBuf),
}

pub struct ConsoleEmitter;  // human-readable to stderr
pub struct JsonEmitter {
    output: Mutex<OutputTarget>,
}

impl EventEmitter for ConsoleEmitter {
    fn emit(&self, event: SystemEvent) {
        // eprintln!("[{timestamp}] IndexStarted: {dir} ({total_files} files)")
    }
}

impl EventEmitter for JsonEmitter {
    fn emit(&self, event: SystemEvent) {
        // serde_json::to_writer( &mut *output.lock(), &event )
    }
}
```

### 4. MetricsCollector

```rust
pub struct MetricsCollector {
    pub queries_total: AtomicU64,
    pub queries_cached: AtomicU64,
    pub files_indexed: AtomicU64,
    pub files_skipped: AtomicU64,
    pub files_failed: AtomicU64,
    pub embedding_calls: AtomicU64,
    pub embedding_cached: AtomicU64,
    pub graph_expansions: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub queries_total: u64,
    pub queries_cached: u64,
    pub files_indexed: u64,
    pub files_skipped: u64,
    pub files_failed: u64,
    pub embedding_calls: u64,
    pub embedding_cached: u64,
    pub graph_expansions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,  // cache_hits / (cache_hits + cache_misses)
    pub uptime_seconds: u64,
}

impl MetricsCollector {
    pub fn new() -> Self;
    pub fn snapshot(&self) -> MetricsSnapshot;
    pub fn increment(&self, counter: &AtomicU64);
    pub fn reset(&self);
}

// Global singleton for easy access
pub fn global_metrics() -> &'static MetricsCollector;
```

### 5. Sandbox

```rust
pub struct Sandbox {
    workspace_root: PathBuf,
    allowed_extensions: Vec<String>,
}

#[derive(Debug)]
pub enum SecurityError {
    PathOutsideWorkspace { path: String, workspace: String },
    UnsupportedExtension { path: String, extension: String },
    SymlinkDenied { path: String },
    CanonicalizationFailed { path: String, error: String },
}

impl Sandbox {
    pub fn new(workspace_root: &Path) -> Result<Self, SecurityError>;

    /// Returns Ok(()) if the path is within workspace_root and has an allowed extension.
    pub fn validate(&self, path: &Path) -> Result<(), SecurityError>;

    /// Add an allowed extension.
    pub fn allow_extension(&mut self, ext: &str);
}

impl Display for SecurityError { ... }
impl std::error::Error for SecurityError { ... }
```

### 6. ReadOnlyGuard

```rust
pub struct ReadOnlyGuard {
    enabled: AtomicBool,
}

impl ReadOnlyGuard {
    pub fn new(enabled: bool) -> Self;

    /// Returns an error if read-only mode is active.
    pub fn check_write_allowed(&self) -> Result<(), &'static str>;

    /// Enable or disable read-only mode.
    pub fn set_enabled(&self, enabled: bool);

    pub fn is_read_only(&self) -> bool;
}
```

### 7. ConfigShow / ConfigValidate

```rust
// CLI commands:
//   ocean config show    → prints effective merged config as JSON
//   ocean config validate → validates config file, prints errors or "config OK"

pub fn cmd_config_show() -> Result<(), ConfigError> {
    let config = OceanConfig::load()?;
    println!("{}", serde_json::to_string_pretty(&config)?);
    Ok(())
}

pub fn cmd_config_validate() -> Result<(), ConfigError> {
    let path = resolve_config_path()?;
    let content = std::fs::read_to_string(&path)?;
    let config: OceanConfig = serde_json::from_str(&content)
        .map_err(|e| ConfigError::ParseError { path, detail: e.to_string() })?;
    config.validate()?;
    println!("config OK (path: {})", path.display());
    Ok(())
}

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    ParseError { path: PathBuf, detail: String },
    ValidationError { fields: Vec<String> },
}
```

---

## Data Models

### Extended OceanConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OceanConfig {
    pub runtime: Option<RuntimeConfig>,
    pub cache: Option<CacheConfigSection>,
    pub security: Option<SecurityConfig>,
    pub observability: Option<ObservabilityConfig>,
    pub embedding: Option<EmbeddingConfigSection>,
    pub index: Option<IndexConfigSection>,
    pub query: Option<QueryConfigSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub mode: Option<String>,  // "desktop" | "server" | "embedded"
    pub io_threads: Option<usize>,
    pub cpu_threads: Option<usize>,
    pub max_ai_concurrent: Option<usize>,
    pub max_in_flight: Option<usize>,
    pub max_queue_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub sandbox: Option<bool>,     // default true
    pub read_only: Option<bool>,   // default false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub log_format: Option<String>, // "console" | "json"
    pub log_file: Option<String>,
    pub metrics_interval_secs: Option<u64>, // future: periodic snapshot
}
```

---

## Integration Points

| Module | Change |
|--------|--------|
| `ocean_cli::args` | Add `Commands::Config(ConfigArgs)` with subcommands `Show`, `Validate` |
| `ocean_cli::run` | Add `cmd_config_show()` and `cmd_config_validate()` handlers |
| `ocean_cli::config` | Extend `OceanConfig` with runtime/cache/security/observability sections, add `validate()` method |
| `ocean_index::orchestrator` | Add `Sandbox` validation before each file, emit `SystemEvent` on start/complete/file |
| `ocean_index::progress` | Add mode-aware defaults in `ConsoleReporter` |
| `ocean_query::engine` | Add `MetricsCollector` integration, emit `QueryExecuted` event |
| `ocean_vector::pipeline` | Add `MetricsCollector` counter for embedding calls |
| `ocean_cache` | Add `MetricsCollector` counter for cache hits/misses |
| `ocean_storage` | Add `ReadOnlyGuard` check in write methods |
| `ocean_api` | Add `--read-only`, `--log-format`, `--log-file` parameters |

---

## Correctness Properties

### Property 1: Sandbox Invariant

*For any* path processed by the index orchestrator, the path SHALL be a descendant of the workspace root (after canonicalization), OR the operation SHALL fail with `SecurityError::PathOutsideWorkspace`.

**Validates:** R4

### Property 2: Read-Only Isolation

*For any* storage write operation attempted while read-only mode is active, the operation SHALL fail with an error and SHALL NOT modify any data.

**Validates:** R5

### Property 3: Mode Determinism

*For any* given `RuntimeMode` and hardware, the resulting configuration defaults SHALL be deterministic (same hardware, same mode → same defaults).

**Validates:** R1

### Property 4: Non-Blocking Metrics

*For any* concurrent access to metrics counters, all increment and snapshot operations SHALL be lock-free (using atomic operations) and SHALL NOT block the calling thread.

**Validates:** R3

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| Path outside workspace | `SecurityError::PathOutsideWorkspace` — file skipped, error logged |
| Symlink target outside workspace | `SecurityError::SymlinkDenied` — file skipped |
| Write in read-only mode | `ApiError::ReadOnlyMode("indexing is disabled in read-only mode")` |
| Config file parse error | `ConfigError::ParseError` with file path + serde error detail |
| Config validation failure | `ConfigError::ValidationError` listing invalid fields |
| Log file cannot be opened | Fall back to stderr only, log warning |

---

## Testing Strategy

### Unit Tests

- `RuntimeMode::defaults()` returns correct values for each mode.
- `Sandbox::validate()` accepts paths inside workspace, rejects paths outside.
- `Sandbox::validate()` rejects symlinks pointing outside workspace.
- `ReadOnlyGuard::check_write_allowed()` returns error when enabled.
- `MetricsCollector` counters increment correctly and snapshot returns consistent values.
- `JsonEmitter::emit()` produces valid JSON lines on stderr.

### Integration Tests

- `ocean config validate` with valid/invalid config files.
- `ocean index --no-sandbox` processes files without sandboxing.
- `ocean query --read-only` succeeds; `ocean index --read-only` fails with clear error.
- `--log-format json` produces JSON lines on stderr.

### Property-Based Tests

- Property 1 (sandbox): Generate random paths inside/outside workspace, verify validation is correct.
- Property 3 (mode defaults): Verify all `RuntimeMode` profiles produce valid, non-zero defaults.
