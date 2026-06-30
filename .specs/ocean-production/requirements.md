# Requirements Document: Ocean Production Readiness

## Introduction

Ocean currently runs as a single-mode CLI tool with minimal observability and no deployment configuration. Section 10 (Performance & Scaling) and Section 11 (Final System Integration) of the project plan define production-grade capabilities: runtime modes for different deployment targets, an observability stack with structured events and metrics, a deployment model for standalone vs. server operation, and security hardening. This phase implements these remaining items to make Ocean production-ready.

The scope covers: runtime mode selection, a lightweight metrics/events system, deployment configuration, and security hardening (sandboxing, read-only mode, query isolation).

---

## Glossary

- **Runtime Mode**: A system-wide operational mode that tunes thread counts, cache sizes, and pipeline behavior for a specific deployment target (Desktop, Server, Embedded).
- **Observability**: The system's ability to emit structured events, counters, and timing data for monitoring and debugging. Not full APM — just structured console output and optional JSON event log.
- **Metadata Event**: A structured data point emitted at key system boundaries (index start/complete, query executed, cache hit/miss) with a timestamp, duration, and typed payload.
- **Sandboxing**: Restricting filesystem access to a designated workspace directory, preventing the indexer from reading files outside the configured path.
- **Read-Only Mode**: A query-only mode where indexing and graph-building operations are disabled. The system serves cached/previous index data only.
- **Tenant Isolation**: (Future) Separation of query results so that one user cannot see another user's indexed documents.

---

## Requirements

### R1: Runtime Mode Selection

**User Story:** As a system operator, I want to choose a runtime mode (Desktop, Server, Embedded) so that Ocean automatically tunes its resource usage for the deployment target.

#### Acceptance Criteria

1. A `RuntimeMode` enum SHALL exist with variants: `Desktop`, `Server`, `Embedded`.
2. THE mode SHALL be configurable via CLI flag `--mode desktop|server|embedded` on the `ocean index` and `ocean query` commands.
3. THE mode SHALL be configurable via the config file: `runtime.mode`.
4. Resolution order: CLI flag > config file > auto-detect > default `Desktop`.
5. EACH mode SHALL set the following defaults:

   | Setting | Desktop | Server | Embedded |
   |---------|---------|--------|----------|
   | `io_threads` | `num_cpus * 2` | `num_cpus * 4` | `2` |
   | `cpu_threads` | `num_cpus` | `num_cpus * 2` | `1` |
   | `max_ai_concurrent` | `2` | `4` | `1` |
   | `embedding_cache_size` | `1000` | `5000` | `100` |
   | `query_cache_size` | `100` | `500` | `20` |
   | `max_in_flight` | `10` | `50` | `3` |
   | `max_queue_size` | `10_000` | `100_000` | `1_000` |
   | Embedding batch size | `10` | `32` | `4` |

6. USERS MAY override individual settings via CLI flags even when a mode is set (CLI flag > mode > config > default).

---

### R2: Observability — Structured Event Logging

**User Story:** As a system operator, I want key system events to be emitted in a structured format (JSON) so that I can pipe them to log aggregators or analysis tools.

#### Acceptance Criteria

1. A `SystemEvent` enum SHALL exist with variants covering: `IndexStarted`, `IndexComplete`, `FileProcessed`, `QueryExecuted`, `CacheHit`, `CacheMiss`, `BackpressureEvent`, `ErrorEvent`.
2. EACH variant SHALL carry a timestamp (`Instant`), duration (`Duration`), and a typed payload.
3. A `EventEmitter` trait SHALL exist with methods: `emit(event: SystemEvent)`, `set_output(OutputTarget)`.
4. TWO implementations SHALL exist: `ConsoleEmitter` (human-readable, same as current `ConsoleReporter`), `JsonEmitter` (JSON lines, one event per line).
5. THE output target SHALL be configurable: `--log-format console|json` CLI flag.
6. THE JSON emitter SHALL write to stderr (so stdout remains clean for programmatic use).
7. WHEN `--log-file <path>` is specified, events SHALL also be written to the given file.

---

### R3: Observability — Metrics Counters

**User Story:** As a system operator, I want to query runtime counters (queries served, files indexed, cache hit rate, embedding API calls) so that I can monitor system health.

#### Acceptance Criteria

1. A `MetricsCollector` struct SHALL exist with atomic counters for: `queries_total`, `queries_cached`, `files_indexed`, `files_failed`, `embedding_calls`, `embedding_cached`, `graph_expansions`, `cache_hits`, `cache_misses`.
2. EACH counter SHALL be `AtomicU64` for lock-free concurrent access.
3. `MetricsCollector::snapshot() -> MetricsSnapshot` SHALL return a point-in-time copy of all counters.
4. THE metrics SHALL be accessible via `ocean info --metrics` CLI command.
5. A `MetricsDisplay` struct SHALL format the snapshot for human-readable output.

---

### R4: Filesystem Sandboxing

**User Story:** As a security-conscious operator, I want the indexing pipeline restricted to the specified workspace directory so that it cannot accidentally read sensitive files outside the workspace.

#### Acceptance Criteria

1. A `Sandbox` struct SHALL exist with a `workspace_root: PathBuf` and `allowed_extensions: Vec<String>`.
2. `Sandbox::validate(path: &Path) -> Result<(), SecurityError>` SHALL return an error if the path is outside `workspace_root` (canonicalized comparison).
3. THE `IndexOrchestrator` SHALL use `Sandbox::validate()` before processing each file.
4. THE `FileWatcher` SHALL use `Sandbox::validate()` before dispatching watch events.
5. A `SecurityError` enum SHALL exist with variants: `PathOutsideWorkspace`, `UnsupportedExtension`, `SymlinkDenied`.
6. Sandboxing SHALL be enabled by default and SHALL be disableable via `--no-sandbox` CLI flag.

---

### R5: Read-Only Mode

**User Story:** As a system operator, I want to start Ocean in read-only mode so that query operations work without any risk of modifying the index.

#### Acceptance Criteria

1. A `--read-only` CLI flag SHALL exist on the `ocean query` command.
2. IN read-only mode, ALL storage write operations SHALL return an error: `StorageError::ReadOnlyMode`.
3. THE `IndexOrchestrator`, `IndexPipeline`, and `GraphBuilder` SHALL NOT be constructable or callable in read-only mode.
4. THE `QueryEngine` SHALL work identically in read-only mode (reads are unaffected).
5. THE `ocean index`, `ocean scan`, `ocean watch` commands SHALL refuse to run in read-only mode with a clear error message.

---

### R6: Query Isolation (Optional — Future)

**User Story:** As a multi-tenant operator, I want query results to be scoped to the requesting tenant's files so that one tenant cannot see another's documents.

#### Acceptance Criteria

1. (FUTURE — not implemented in this phase) A `tenant_id` field SHALL be added to `SearchFilter`.
2. (FUTURE) The `VectorStore` and `GraphStore` SHALL support filtering by `tenant_id`.
3. THIS phase SHALL only add the `SearchFilter::tenant_id` field as a no-op placeholder for future implementation.

---

### R7: Deployment Configuration

**User Story:** As a system operator, I want a single configuration file that captures all runtime, cache, storage, and security settings for repeatable deployments.

#### Acceptance Criteria

1. THE `OceanConfig` struct SHALL be extended with a `runtime` section (mode, thread counts), `cache` section (sizes, TTL, path), `security` section (sandbox, read-only), `observability` section (log format, log file).
2. A `--config <path>` CLI flag SHALL allow specifying a custom config file path (default: `./.ocean/config.json` then `~/.ocean/config.json`).
3. A `ocean config show` command SHALL display the effective merged configuration (CLI > config > env > defaults).
4. A `ocean config validate` command SHALL check the config file for errors (unknown keys, invalid values, missing paths).

---

### R8: Error Handling

**User Story:** As a system operator, I want clear typed errors for all production-level operations.

#### Acceptance Criteria

1. A `SecurityError` enum SHALL exist with `Display` + `Error` impls.
2. A `ConfigError` enum SHALL exist for config validation failures.
3. ALL new error types SHALL be convertible to `ApiError` via `From` impls.
