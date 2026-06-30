# Design Document: Ocean Index Orchestrator

## Overview

ocean-index extracts the indexing orchestration logic currently embedded in `ocean_cli::run::cmd_index` and `ocean_vector::pipeline::IndexPipeline` into a single, coherent module with a formal `IndexOrchestrator` struct, state-driven incremental indexing, progress reporting, and atomic file processing.

The current codebase has indexing logic split across two locations:
1. `ocean_cli::run::cmd_index` — scans directory, iterates files, calls pipeline, prints progress inline.
2. `ocean_vector::pipeline::IndexPipeline::index_chunks` — batches chunks, embeds, stores.

There is no state tracking, no retry logic, no progress abstraction, and no clean separation between orchestration and execution. The orchestrator runs synchronously in a single loop.

Phase 8 creates `ocean_index` as the single entry point for all indexing operations, integrating `ocean_storage::StateStore` for change detection, adding a `ProgressReporter` trait for decoupled output, and formalising the three pipeline modes (Full, Incremental, Watch).

### Key Design Decisions

1. **Orchestrator owns the loop, not the pipeline** — `IndexOrchestrator` decides which files to process and in what order. It delegates per-file processing to a `FileProcessor` that runs the parse→chunk→embed→graph→store chain. This keeps the orchestrator focused on coordination, not content processing.
2. **StateStore is the source of truth for incremental decisions** — The orchestrator queries StateStore before each file; it does not maintain its own in-memory change set. This makes the system resilient to restarts and consistent with the storage layer.
3. **ProgressReporter trait for decoupled output** — CLI output, logging, and test observers all use the same trait. The default `ConsoleReporter` matches the current `ocean index` output format exactly.
4. **Atomic file processing via per-file commit** — Instead of a single massive transaction, each file is processed and committed atomically. This avoids holding large staging buffers and allows partial progress to be saved.
5. **IndexPipeline remains as the embed+store engine** — `IndexPipeline` is too coupled to the vector store to refactor in this phase. The orchestrator calls it as a delegate. A future phase may absorb it.

---

## Architecture

### High-level module structure

```text
src/ocean_index/
├── mod.rs              — IndexOrchestrator, IndexMode, public re-exports
├── orchestrator.rs     — IndexOrchestrator struct, run() lifecycle
├── processor.rs        — FileProcessor, per-file pipeline stages
├── progress.rs         — ProgressEvent, ProgressReporter trait, ConsoleReporter
├── report.rs           — IndexReport, FileResult
├── error.rs            — IndexError enum
└── config.rs           — IndexConfig struct
```

### Data flow

```text
CLI: ocean index ./docs [flags]
         │
         ▼
   IndexOrchestrator::run(config)
         │
         ├── detect mode (Full / Incremental / Watch)
         │
         ├── scan directory ──► Vec<FileMeta>
         │
         ├── filter by StateStore (Incremental mode)
         │      │
         │      ├── hash matches  ──► skip (FileSkipped event)
         │      └── hash differs  ──► process
         │
         ├── for each changed file:
         │      │
         │      └── FileProcessor::process(file)
         │             │
         │             ├── parse ──► Vec<ReadResult>
         │             ├── chunk ──► Vec<Chunk>
         │             ├── embed ──► Vec<ChunkRecord>  (via IndexPipeline)
         │             ├── graph  ──► Vec<(Node, Edge)>  (via GraphBuilder)
         │             ├── store ──► Storage writes
         │             └── state ──► StateStore::update_state(Indexed)
         │
         └── emit IndexReport
```

### Directory layout (no new files on disk — uses ocean_storage paths)

No additional storage directories. All data persists via `ocean_storage` sub-stores using their existing paths.

---

## Components and Interfaces

### 1. IndexMode Enum

```rust
pub enum IndexMode {
    Full,         // Re-index every file, ignore StateStore
    Incremental,  // Skip files whose hash matches StateStore
    Watch,        // Run once, then watch for changes
}
```

### 2. IndexConfig Struct

```rust
pub struct IndexConfig {
    pub mode: IndexMode,
    pub dir: String,
    pub chunk_config: ocean_chunk::config::ChunkConfig,
    pub graph_config: ocean_graph::GraphConfig,
    pub batch_size: usize,
    pub max_retries: u32,
    pub no_graph: bool,
    pub no_references: bool,
    pub no_entities: bool,
}
```

### 3. IndexOrchestrator (Core Entry Point)

```rust
pub struct IndexOrchestrator {
    storage: Arc<dyn ocean_storage::Storage>,
    file_store: Arc<dyn ocean_storage::FileStore>,
    chunk_store: Arc<dyn ocean_storage::ChunkStore>,
    vector_store: Arc<dyn ocean_storage::VectorStore>,
    graph_store: Arc<dyn ocean_storage::GraphStore>,
    state_store: Arc<dyn ocean_storage::StateStore>,
    embedder: Arc<dyn ocean_vector::embedder::Embedder>,
    reporter: Box<dyn ProgressReporter>,
    pipeline: ocean_vector::pipeline::IndexPipeline,
}

impl IndexOrchestrator {
    pub fn new(
        storage: Arc<dyn ocean_storage::Storage>,
        embedder: Arc<dyn ocean_vector::embedder::Embedder>,
        reporter: Box<dyn ProgressReporter>,
    ) -> Self;

    pub fn run(&self, config: IndexConfig) -> Result<IndexReport, IndexError>;
}
```

### 4. FileProcessor (Per-File Pipeline)

```rust
struct FileProcessor {
    embedder: Arc<dyn ocean_vector::embedder::Embedder>,
    storage: Arc<dyn ocean_storage::Storage>,
    chunk_store: Arc<dyn ocean_storage::ChunkStore>,
    vector_store: Arc<dyn ocean_storage::VectorStore>,
    graph_store: Arc<dyn ocean_storage::GraphStore>,
    state_store: Arc<dyn ocean_storage::StateStore>,
}

impl FileProcessor {
    pub fn process(
        &self,
        file: &ocean_storage::FileMeta,
        config: &IndexConfig,
    ) -> Result<FileResult, IndexError>;
}
```

Internal stages:

| Stage | Input | Output | Delegate |
|-------|-------|--------|----------|
| Parse | `FileMeta` | `Vec<ReadResult>` | `ocean_parser::read_all_blocks()` |
| Chunk | `Vec<ReadResult>` + `file_id` | `Vec<Chunk>` | `ocean_chunk::chunker::chunk()` |
| Embed | `Vec<Chunk>` | `Vec<ChunkRecord>` | `IndexPipeline::index_chunks()` |
| Graph | `Vec<Chunk>` + `file_id` | `(Vec<Node>, Vec<Edge>)` | `GraphBuilder::from_chunks()` |
| Store | chunks + nodes + edges | — | `chunk_store`, `vector_store`, `graph_store` |
| State | `file_id` + `hash` | — | `state_store::update_state()` |

### 5. Progress Reporter

```rust
pub enum ProgressEvent {
    ScanStarted { total: u64 },
    FileProcessing { path: String, current: u64, total: u64 },
    FileComplete { path: String, chunks: u64, edges: u64, duration_ms: u64 },
    FileSkipped { path: String },
    FileFailed { path: String, error: String },
    GraphProgress { total_nodes: u64, total_edges: u64 },
    IndexComplete(IndexReport),
}

pub trait ProgressReporter: Send {
    fn report(&self, event: ProgressEvent);
}

pub struct ConsoleReporter;   // prints formatted output to stdout
pub struct SilentReporter;    // discards all events
```

### 6. IndexReport

```rust
pub struct IndexReport {
    pub total_files: u64,
    pub indexed: u64,
    pub skipped: u64,
    pub failed: u64,
    pub total_chunks: u64,
    pub total_edges: u64,
    pub total_nodes: u64,
    pub duration_ms: u64,
    pub per_file: Vec<FileResult>,
}

pub struct FileResult {
    pub path: String,
    pub status: FileIndexStatus,  // Indexed | Skipped | Failed
    pub chunks: u64,
    pub edges: u64,
    pub nodes: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
}

pub enum FileIndexStatus {
    Indexed,
    Skipped,
    Failed,
}
```

### 7. IndexError

```rust
pub enum IndexError {
    FileProcessError {
        file_id: String,
        stage: String,       // "parse" | "chunk" | "embed" | "graph" | "store" | "state"
        error: String,
    },
    StorageError(ocean_storage::StorageError),
    ScanError(String),
    Aborted,
}
```

---

## Correctness Properties

### Property 1: Deterministic Incremental Filtering

*For any* set of files with known StateStore contents, `run(Incremental)` SHALL process exactly those files whose hash differs from the stored hash, and SHALL skip all files whose hash matches.

**Validates:** R2, R3

### Property 2: Atomic Per-File Processing

*For any* file that fails during processing, the storage layer SHALL NOT contain any partial data for that file (chunks without embeddings, embeddings without graph nodes, etc.).

**Validates:** R4

### Property 3: Progress Event Ordering

*For any* successful indexing run, the sequence of `ProgressEvent` values emitted SHALL follow the order: `ScanStarted` → zero or more `FileProcessing`/`FileComplete`/`FileSkipped`/`FileFailed` → `GraphProgress` (optional) → `IndexComplete`.

**Validates:** R5

### Property 4: Report Accuracy

*For any* indexing run, the `IndexReport` counts SHALL satisfy: `total_files == indexed + skipped + failed`.

**Validates:** R6

### Property 5: Crash Recovery

*For any* file whose StateStore status is `Failed`, a subsequent `run(Incremental)` SHALL attempt to re-index that file (because its hash will not match the stored Pending/Failed state's hash).

**Validates:** R3, R7

### Property 6: Backwards Compatibility

*For any* existing test that exercises `ocean index` via the CLI or `IndexPipeline` directly, the same test SHALL pass after `cmd_index` delegates to `IndexOrchestrator::run()`.

**Validates:** R8, R9

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| File parse fails (corrupt file) | Catch error, mark as `Failed` in StateStore, emit `FileFailed`, continue to next file |
| Embedding API timeout | Retry up to `max_retries` with exponential backoff (100ms, 500ms, 2s); if all fail, mark as `Failed` |
| Storage write fails mid-commit | Mark file as `Failed` in StateStore; operator can re-run in Incremental mode to retry |
| Directory does not exist | `IndexError::ScanError("directory not found")` |
| Embedder dimension mismatch | `IndexError::FileProcessError { stage: "embed" }` with descriptive message |
| Ctrl+C during indexing | The orchestrator SHALL finish the current file, then return `IndexReport` with partial results |

---

## Testing Strategy

### Unit Tests

- `IndexMode` serialization/deserialization
- `IndexReport` aggregation math (`total_files == indexed + skipped + failed`)
- `ProgressEvent` display formatting matches current CLI output
- `FileProcessor` stage error propagation (simulate parse failure, verify correct error variant)

### Integration Tests (in-memory storage)

- Full mode: index 3 files, verify all stored in chunk/vector/graph stores
- Incremental mode: index once, run again — verify 0 files processed (all skipped)
- Incremental mode with changed file: modify file hash, re-run — verify only changed file is re-indexed
- Watch mode: run with watch, create a file, verify it gets indexed (with timeout)
- Failure recovery: inject parse error, verify file is `Failed` in StateStore
- CLI integration: run `ocean index test-cwd --db-path :memory:`, verify output matches expected format

### Property-Based Tests

- Property 1 (deterministic filtering): generate random `Vec<FileMeta>`, set StateStore for a subset, run Incremental, verify only subset without stored state are processed.
- Property 4 (report accuracy): generate N files, randomly mark some as indexed/skipped/failed, verify report counts.
