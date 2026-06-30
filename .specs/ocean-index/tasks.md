# Implementation Plan: Ocean Index Orchestrator

## Overview

Extract indexing orchestration logic into a dedicated `ocean_index` module with `IndexOrchestrator`, per-file `FileProcessor`, `ProgressReporter` trait, state-driven incremental indexing, and formal `IndexReport` output. Work is structured as 10 tasks, each 1–4 hours, with testing embedded in each task.

## Pre-requisites

- All Phase 1–7 modules exist and tests pass (`cargo test` — 203+ tests green).
- `ocean_storage` module exists with all five sub-store traits and `SurrealStorage`.
- `ocean_vector::pipeline::IndexPipeline` exists and is stable.
- `ocean_graph::GraphBuilder` exists and is stable.
- CLI `cmd_index` exists in `ocean_cli::run.rs`.
- `AGENTS.md` exists with module patterns.

## Tasks

- [ ] 1. **Create `ocean_index` module skeleton**
  - Create `src/ocean_index/` directory with `mod.rs`, `error.rs`, `config.rs`, `progress.rs`, `report.rs`, `orchestrator.rs`, `processor.rs`.
  - Register `pub mod ocean_index;` in `src/lib.rs`.
  - Register test module in `src/tests.rs`.
  - Define `IndexError` enum with all variants from design.
  - Implement `Display` + `Error` for `IndexError`.
  - Define `IndexMode` enum (Full, Incremental, Watch).
  - Define `IndexConfig` struct with all fields from design.
  - Define `ProgressEvent` enum with all variants from design.
  - Define `ProgressReporter` trait with `report()` method.
  - Implement `ConsoleReporter` that prints formatted output.
  - Implement `SilentReporter` that discards all events.
  - Define `IndexReport` struct, `FileResult`, `FileIndexStatus`.
  - Re-export all public types from `mod.rs`.
  - Verify `cargo build` succeeds — no logic yet.
  - _Requirements: R1, R5, R6_

  - [ ] 1.1 Create directory and files
  - [ ] 1.2 Register module in `lib.rs` and `tests.rs`
  - [ ] 1.3 `IndexError` with `Display` + `Error`
  - [ ] 1.4 `IndexMode`, `IndexConfig`
  - [ ] 1.5 `ProgressEvent`, `ProgressReporter` trait
  - [ ] 1.6 `ConsoleReporter`, `SilentReporter`
  - [ ] 1.7 `IndexReport`, `FileResult`, `FileIndexStatus`
  - [ ] 1.8 Re-exports and `cargo build`

- [ ] 2. **Implement `FileProcessor` (per-file pipeline)**
  - Create `src/ocean_index/processor.rs`.
  - `FileProcessor` struct holds: `embedder`, `storage` (or individual store `Arc` refs).
  - Implement `process(file, config) -> Result<FileResult, IndexError>`.
  - Pipeline stages (delegate to existing modules):
    - **Parse**: call `ocean_parser::read_all_blocks(file.path)` → `Vec<ReadResult>`.
    - **Chunk**: call `ocean_chunk::chunker::chunk(blocks, file.id, config.chunk_config)` → `Vec<Chunk>`.
    - **Embed**: call `IndexPipeline::index_chunks(chunks, embedder, config)` → `Vec<ChunkRecord>`.
    - **Graph**: call `GraphBuilder::from_chunks(chunks, file.id, config.graph_config)` → `(Vec<Node>, Vec<Edge>)`.
    - **Store**: write chunks to `ChunkStore`, chunk records to `VectorStore`, nodes+edges to `GraphStore`.
    - **State**: call `StateStore::update_state(file.id, file.hash, Indexed)`.
  - Each stage catches errors and returns `IndexError::FileProcessError { stage }`.
  - Wrap the entire function in a timing measurement.
  - Write unit tests: test each stage in isolation with mock stores.
  - Register test file: `processor_test.rs`.
  - Verify `cargo test --lib processor` passes.
  - _Requirements: R4_

  - [ ] 2.1 `FileProcessor` struct and constructor
  - [ ] 2.2 Parse + Chunk stages
  - [ ] 2.3 Embed stage (delegate to IndexPipeline)
  - [ ] 2.4 Graph stage (delegate to GraphBuilder)
  - [ ] 2.5 Store + State stages
  - [ ] 2.6 Error handling and timing
  - [ ] 2.7 Unit tests

- [ ] 3. **Implement `IndexOrchestrator::run()` — Full mode**
  - Create `src/ocean_index/orchestrator.rs`.
  - `IndexOrchestrator` struct holds: `storage` (Arc), `embedder` (Arc), `reporter` (Box).
  - `run(config)` method:
    1. Emit `ScanStarted`.
    2. Scan directory: call `ocean_cli::walk::walk_supported_files()` or directly use `ocean_fs::scanner`.
    3. Filter by extension + mode:
       - Full: process all files.
       - Incremental: query StateStore for each, skip if hash matches.
    4. For each file with progress reporting:
       - Emit `FileProcessing`.
       - Call `FileProcessor::process()`.
       - On success: emit `FileComplete`.
       - On skip: emit `FileSkipped`.
       - On error: emit `FileFailed`; update StateStore with `Failed`.
    5. Emit `GraphProgress` with aggregate counts.
    6. Emit `IndexComplete(report)`.
    7. Return `IndexReport`.
  - Write unit tests with in-memory storage and mock embedder.
  - Register test file: `orchestrator_test.rs`.
  - Verify `cargo test --lib orchestrator` passes.
  - _Requirements: R1, R2, R3, R4, R5, R6_

  - [ ] 3.1 `IndexOrchestrator` struct and constructor
  - [ ] 3.2 Scan + filter by mode
  - [ ] 3.3 File processing loop with progress events
  - [ ] 3.4 StateStore integration for Incremental mode
  - [ ] 3.5 IndexReport aggregation
  - [ ] 3.6 Unit tests (Full mode, Incremental mode)

- [ ] 4. **Implement Incremental mode with StateStore query**
  - In `orchestrator.rs`, add incremental filtering logic:
    1. Before processing each file, query `StateStore::get_state(file_id)`.
    2. If state exists and `state.hash == file.hash`, skip (emit `FileSkipped`).
    3. If no state or hash differs, process.
    4. After successful processing, call `state_store::update_state(file_id, hash, Indexed)`.
    5. On failure, call `state_store::update_state(file_id, hash, Failed)`.
  - Test: index 3 files, run again in Incremental mode — verify 0 processed.
  - Test: index 3 files, modify one file's hash, run Incremental — verify only the modified file is processed.
  - _Requirements: R2, R3_

- [ ] 5. **Implement Watch mode**
  - In `orchestrator.rs`, add Watch mode:
    1. Run a full or incremental pass first.
    2. Call `ocean_fs::watcher::watch()` on the directory.
    3. For each `FileEvent`, re-index the affected file via `FileProcessor`.
    4. Run until a cancellation signal (Ctrl+C or channel close).
  - Use `crossbeam_channel` for event delivery (same as existing watcher).
  - Test: run with watch, create a file, verify it gets indexed.
  - _Requirements: R2_

  - [ ] 5.1 Watch loop after initial pass
  - [ ] 5.2 File event handling (Created/Modified)
  - [ ] 5.3 Graceful shutdown on Ctrl+C
  - [ ] 5.4 Unit test with simulated events

- [ ] 6. **Add retry logic for transient errors**
  - In `FileProcessor::process()`:
    - Wrap the embed stage in a retry loop.
    - `max_retries = 3` (configurable in `IndexConfig`).
    - Backoff: 100ms, 500ms, 2s.
    - Only retry on errors that look transient (timeout, connection refused).
    - Non-transient errors (invalid file, parse error) fail immediately.
  - Test: inject a transient error that succeeds on retry 3, verify success.
  - _Requirements: R7_

- [ ] 7. **Update CLI `cmd_index` to delegate to `IndexOrchestrator`**
  - In `ocean_cli::run.rs`, rewrite `cmd_index`:
    1. Resolve embedder from CLI flags / config (existing logic).
    2. Resolve storage from `--db-path` or config (existing logic).
    3. Build `IndexConfig` from CLI flags.
    4. Create `IndexOrchestrator` with storage, embedder, and `ConsoleReporter`.
    5. Call `orchestrator.run(config)`.
    6. Print errors if any.
    7. Exit with appropriate code (0 = all indexed, 1 = some failed).
  - Add `--watch` flag to `IndexArgs` in `ocean_cli::args.rs`.
  - Remove the inline indexing loop from `cmd_index`.
  - Verify CLI output is identical to current output format.
  - _Requirements: R8_

  - [ ] 7.1 Add `--watch` flag to `IndexArgs`
  - [ ] 7.2 Rewrite `cmd_index` to use `IndexOrchestrator`
  - [ ] 7.3 Verify CLI output format matches
  - [ ] 7.4 Integration test: `cargo run -- index test-cwd --db-path test.db`

- [ ] 8. **Move `IndexPipeline` under `ocean_index` (deprecation path)**
  - Add a re-export in `ocean_index` that points to `ocean_vector::pipeline::IndexPipeline`.
  - Update `ocean_vector::pipeline` doc to indicate "prefer ocean_index::IndexOrchestrator".
  - Do NOT delete `ocean_vector::pipeline` yet (backwards compat).
  - _Requirements: R9_

- [ ] 9. **Integration tests for `ocean_index`**
  - Create `tests/index_integration.rs` (or add to existing integration test file).
  - Test scenarios:
    1. Full index of test-cwd (5+ files), verify all chunks/nodes/edges stored.
    2. Incremental re-index: verify 0 files processed.
    3. Incremental with modified file: change hash, verify 1 file processed.
    4. Error recovery: corrupt file, verify it's marked Failed, other files processed.
    5. Watch mode: start watch, create temp file, verify it gets indexed.
  - Use in-memory storage for speed.
  - _Requirements: R1–R9_

  - [ ] 9.1 Full index integration test
  - [ ] 9.2 Incremental index integration test
  - [ ] 9.3 Error recovery integration test
  - [ ] 9.4 Watch mode integration test (with timeout)

- [ ] **Validation & Cleanup**
  - Run full test suite: `cargo test` — all tests must pass.
  - Verify `cargo build --release` succeeds.
  - Update `AGENTS.md` with `ocean_index` module conventions.
  - Remove any dead code from `ocean_cli::run` related to the old inline indexing loop.
  - Print summary of files changed.
  - _Requirements: R8, R9_

## Notes

- **Task order**: 1→2→3→4→5→6→7→8→9. Tasks 2-5 are sequential; 6 can be done in parallel with 5.
- **Dependencies**: Task 2 depends on ocean_parser, ocean_chunk, ocean_vector::pipeline, ocean_graph::GraphBuilder, ocean_storage. Task 3 depends on task 2. Task 7 depends on tasks 1-6.
- **Testing approach**: Unit tests in `processor_test.rs` and `orchestrator_test.rs`. Integration tests in `tests/index_integration.rs`. Use in-memory storage and `MockEmbedder` for all tests.
- **Backwards compatibility**: `IndexPipeline` in `ocean_vector::pipeline` is NOT deleted. Existing consumers continue to work. Only CLI `cmd_index` is rewritten.
- **Performance considerations**: The orchestrator processes files sequentially by default. Parallel file processing can be added in a future phase via `rayon::par_iter()` on the file list.
- **CLI output preservation**: The existing `ocean index` output format must be preserved exactly. Compare output character-by-character in tests.
