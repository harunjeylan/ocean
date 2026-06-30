# Requirements Document: Ocean Index Orchestrator

## Introduction

ocean-index is the **control center of the entire Ocean system**. It orchestrates the full indexing pipeline: scanning the filesystem, parsing documents, chunking content, computing embeddings, building the knowledge graph, and persisting everything through the storage layer — safely, incrementally, and deterministically.

Currently, indexing logic is split between `ocean_cli::run::cmd_index` (CLI orchestration) and `ocean_vector::pipeline::IndexPipeline` (batch embed+store). There is no single `ocean_index` module, no state-driven incremental indexing, no transaction-backed file processing, no progress reporting abstraction, and no formal separation between the orchestrator and the individual pipeline stages. This phase extracts a dedicated `ocean_index` module that owns the full indexing lifecycle: change detection → file processing pipeline → storage commit → state tracking.

The scope is MVP: extract a dedicated orchestrator that owns the full lifecycle, integrate StateStore for incremental indexing, add progress reporting, and move orchestration logic out of CLI handlers.

---

## Glossary

- **Index Orchestrator**: The top-level controller that decides which files to process, in what order, and with what configuration.
- **File Processing Unit**: A self-contained pipeline stage that takes a single `FileMeta` through all indexing steps (parse → chunk → embed → graph → store).
- **Change Detection**: The mechanism by which the orchestrator determines whether a file needs re-indexing, based on comparing current file hash + modified time against stored state.
- **Indexing State**: Per-file record in StateStore indicating the last known hash, last indexed timestamp, and status (Pending/Indexed/Failed).
- **Pipeline Mode**: The operational mode of the index run — FULL (reindex all files), INCREMENTAL (only changed files), or WATCH (continuous monitoring).
- **IndexReport**: Aggregate result of an indexing run, containing counts of indexed/skipped/failed files and per-file timing.
- **Progress Reporter**: Abstraction for emitting indexing progress events (file started, file completed, error) to the CLI or other consumers.

---

## Requirements

### R1: Dedicated Index Orchestrator Module

**User Story:** As a system architect, I want a single `ocean_index` module responsible for orchestrating the entire indexing pipeline, so that orchestration logic is not scattered across CLI handlers and pipelines.

#### Acceptance Criteria

1. A `src/ocean_index/` module SHALL exist with `mod.rs` re-exporting public items.
2. The module SHALL be registered in `src/lib.rs` as `pub mod ocean_index;`.
3. A top-level `IndexOrchestrator` struct SHALL own the indexing lifecycle.
4. THE `IndexOrchestrator` SHALL accept all external dependencies via its constructor (embedder, storage, parser registry, chunk config, graph config) — never construct them internally.
5. THE `ocean index` CLI command SHALL delegate to `IndexOrchestrator::run()` instead of implementing the loop inline.
6. THE `IndexPipeline` in `ocean_vector` SHALL be absorbed or called by the orchestrator; no orchestration logic SHALL remain in `ocean_vector::pipeline`.

---

### R2: Three Pipeline Modes

**User Story:** As a CLI user, I want to choose between full reindex, incremental (changed files only), and continuous watch mode, so that I can balance thoroughness vs. speed.

#### Acceptance Criteria

1. THE orchestrator SHALL support three modes: `IndexMode::Full`, `IndexMode::Incremental`, `IndexMode::Watch`.
2. `IndexMode::Full` SHALL process every supported file in the scanned directory, regardless of stored state.
3. `IndexMode::Incremental` SHALL skip files whose `hash` in `StateStore` matches the current file hash.
4. `IndexMode::Watch` SHALL run a full or incremental pass, then call `ocean_fs::watch()` and re-index files as they change.
5. THE default mode SHALL be `Incremental`.
6. THE `--reindex` CLI flag SHALL map to `IndexMode::Full`.

---

### R3: Change Detection via StateStore

**User Story:** As the index orchestrator, I want to use `StateStore` to detect which files have changed, so that I only re-index files whose content or metadata differs.

#### Acceptance Criteria

1. BEFORE processing a file, the orchestrator SHALL query `StateStore::get_state(file_id)`.
2. IF `state` exists AND `state.hash == current_file.hash`, THEN the file SHALL be skipped (counted as "skipped" in report).
3. IF `state` does not exist OR `state.hash != current_file.hash`, THEN the file SHALL be processed.
4. AFTER a file is successfully indexed, the orchestrator SHALL call `StateStore::update_state(file_id, hash, Indexed)`.
5. IF indexing fails, the orchestrator SHALL call `StateStore::update_state(file_id, hash, Failed)`.

---

### R4: Atomic File Processing (Parse → Chunk → Embed → Graph → Store)

**User Story:** As the index orchestrator, I want each file to be processed atomically — all stages succeed or none are persisted — so that partial indexing never leaves storage in an inconsistent state.

#### Acceptance Criteria

1. THE file processing unit SHALL execute all stages for one file: parse, chunk, embed, graph build, storage write.
2. WHEN all stages succeed, the results SHALL be committed to storage (via `Storage::commit()` if in a transaction, or via individual writes with error handling).
3. WHEN any stage fails, the file SHALL be marked as `Failed` in StateStore and any partial writes SHALL be rolled back.
4. THE file processing unit SHALL be self-contained and callable in parallel (the orchestrator manages concurrency).
5. EACH stage SHALL produce a typed intermediate result: `FileMeta → Vec<ReadResult> → Vec<Chunk> → Vec<ChunkRecord> → (nodes, edges)`.

---

### R5: Progress Reporting

**User Story:** As a CLI user, I want to see real-time progress during indexing — which file is being processed, how many chunks were created, how many vectors were indexed, and any errors — so that I know the system is working.

#### Acceptance Criteria

1. A `ProgressEvent` enum SHALL exist with variants: `ScanStarted(u64)`, `FileProcessing { path, current, total }`, `FileComplete { path, chunks, duration_ms }`, `FileSkipped { path }`, `FileFailed { path, error }`, `IndexComplete(IndexReport)`.
2. A `ProgressReporter` trait SHALL define a method `report(event: ProgressEvent)`.
3. A `ConsoleReporter` implementation SHALL print formatted progress to stdout (same format as current `ocean index` output).
4. A `SilentReporter` implementation SHALL discard all events (useful for tests and programmatic use).
5. THE orchestrator SHALL accept any `Box<dyn ProgressReporter>` and emit events at each stage.

---

### R6: IndexReport Output

**User Story:** As a CLI user, I want a summary report at the end of indexing showing how many files were indexed, skipped, or failed, and how long it took, so that I can verify the operation.

#### Acceptance Criteria

1. `IndexReport` SHALL contain: `total_files`, `indexed`, `skipped`, `failed`, `total_chunks`, `total_edges`, `total_nodes`, `duration_ms`, `per_file: Vec<FileResult>`.
2. `FileResult` SHALL contain: `path`, `status` (Indexed/Skipped/Failed), `chunks: u64`, `edges: u64`, `duration_ms: u64`, `error: Option<String>`.
3. THE orchestrator SHALL return `IndexReport` from `run()`.
4. THE `ConsoleReporter` SHALL print the report summary on `IndexComplete`.
5. THE report SHALL be deterministic (identical input → identical report fields except timing).

---

### R7: Error Handling and Recovery

**User Story:** As a system operator, I want the indexing pipeline to handle errors gracefully — skip individual files without aborting the entire run — so that a single corrupt file does not block indexing of the rest.

#### Acceptance Criteria

1. IF a file fails during any stage, the orchestrator SHALL catch the error, mark the file as `Failed` in StateStore, emit a `FileFailed` event, and continue to the next file.
2. IF a transient error occurs (e.g. embedding API timeout), the orchestrator MAY retry up to `max_retries` (default 3) with exponential backoff before marking as Failed.
3. IF the storage connection fails entirely, the orchestrator SHALL abort the run and return an error.
4. THE `IndexError` enum SHALL have variants: `FileProcessError { file_id, stage, error }`, `StorageError(ocean_storage::StorageError)`, `ScanError(String)`, `Aborted`.
5. THE orchestrator SHALL NOT panic on any file-level error.

---

### R8: CLI Integration

**User Story:** As a CLI user, I want the existing `ocean index` command to work identically to before, with the same flags and behavior, but powered by the new orchestrator internally.

#### Acceptance Criteria

1. THE `ocean index` command SHALL accept all existing flags: `--model`, `--provider`, `--ollama-url`, `--api-key`, `--dimension`, `--db-path`, `--batch-size`, `--reindex`, `--no-graph`, `--no-references`, `--no-entities`.
2. THE `--reindex` flag SHALL map to `IndexMode::Full`.
3. THE output format SHALL be identical to the current `ocean index` output.
4. THE `--watch` flag SHALL map to `IndexMode::Watch`.
5. ALL existing integration tests for `ocean index` SHALL continue to pass.

---

### R9: Backwards Compatibility

**User Story:** As an existing consumer of `IndexPipeline`, I want the pipeline to continue working without modification after the ocean-index module is introduced.

#### Acceptance Criteria

1. THE existing `IndexPipeline` in `ocean_vector::pipeline` SHALL continue to exist and work identically (for consumers who use it directly).
2. THE new `IndexOrchestrator` SHALL use `IndexPipeline` internally for the embed+store stage.
3. ALL existing tests SHALL continue to pass without modification.
