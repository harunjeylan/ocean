# Implementation Plan: Ocean Storage Layer

## Overview

Extract a unified `ocean_storage` module from the existing `ocean_vector::store` and `ocean_graph::store`, adding `FileStore`, `StateStore`, a master `Storage` trait, transaction scaffolding, and path resolution. Work is structured as 12 tasks, each 1–4 hours, with testing embedded in each task.

## Pre-requisites

- All Phase 1–6 modules exist and tests pass (`cargo test` — 223+ tests green).
- `ocean_vector::store::VectorStore` and `ocean_graph::store::GraphStore` are stable.
- The `~/.ocean/store/` convention is established (Phase 7 uses `~/.ocean/store/{cwd}/`).
- `AGENTS.md` exists with module patterns.

## Tasks

- [ ] 1. **Create `ocean_storage` module skeleton**
  - Create `src/ocean_storage/` directory with `mod.rs`, `error.rs`, `config.rs`.
  - Register `pub mod ocean_storage;` in `src/lib.rs`.
  - Register test module in `src/tests.rs`.
  - Define `StorageError` enum with all variants from design.
  - Implement `Display` + `Error` for `StorageError`.
  - Verify `cargo build` succeeds — no logic yet.
  - _Requirements: R8_

  - [ ] 1.1 Create directory and files
  - [ ] 1.2 Register module in `lib.rs` and `tests.rs`
  - [ ] 1.3 Implement `StorageError` enum with `Display` + `Error`
  - [ ] 1.4 Verify `cargo build --lib`

- [ ] 2. **Define sub-store traits**
  - Define `FileStore` trait (6 methods) in `src/ocean_storage/file_store.rs`.
  - Define `ChunkStore` trait (7 methods) in `src/ocean_storage/chunk_store.rs`.
  - Define `VectorStore` trait (6 methods) in `src/ocean_storage/vector_store.rs`.
  - Define `GraphStore` trait (10 methods) in `src/ocean_storage/graph_store.rs`.
  - Define `StateStore` trait (5 methods + `IndexStatus` enum + `StateRecord` struct) in `src/ocean_storage/state_store.rs`.
  - Define `FileMeta` struct in `src/ocean_storage/file_store.rs`.
  - Re-export all traits and types from `mod.rs`.
  - Verify `cargo build` succeeds.
  - _Requirements: R1, R4, R5_

  - [ ] 2.1 `FileStore` trait + `FileMeta`
  - [ ] 2.2 `ChunkStore` trait
  - [ ] 2.3 `VectorStore` trait
  - [ ] 2.4 `GraphStore` trait
  - [ ] 2.5 `StateStore` trait + `IndexStatus` + `StateRecord`
  - [ ] 2.6 Re-exports and `cargo build`

- [ ] 3. **Define `Storage` master trait + `StorageStats`**
  - Define `Storage` trait in `src/ocean_storage/mod.rs` with accessor methods, transaction methods, `storage_path()`, `count_all()`.
  - Implement `StorageStats` struct.
  - Verify `cargo build` succeeds.
  - _Requirements: R1, R3_

- [ ] 4. **Implement `SurrealFileStore`**
  - Create `src/ocean_storage/file_store_impl.rs` (or inline in `file_store.rs`).
  - Implement `SurrealFileStore` struct with `new_persistent(path)`, `new_memory()`.
  - SurrealDB schema: `file` table with `file_id`, `path`, `hash`, `size`, `modified`, `extension`, `last_indexed`.
  - Indexes: UNIQUE on `file_id`, UNIQUE on `path`.
  - Implement all 6 `FileStore` trait methods.
  - Write unit tests: CRUD, path lookup, `needs_update` with same/different hash.
  - Register test file: `file_store_test.rs`.
  - Verify `cargo test --lib file_store` passes.
  - _Requirements: R4_

  - [ ] 4.1 Schema definition and connection management
  - [ ] 4.2 `upsert_file`, `get_file`, `get_file_by_path`
  - [ ] 4.3 `delete_file`, `list_files`, `needs_update`
  - [ ] 4.4 Unit tests

- [ ] 5. **Implement `SurrealChunkStore`**
  - Create `src/ocean_storage/chunk_store_impl.rs`.
  - Implement `SurrealChunkStore` struct.
  - Schema matches existing `ChunkRecord` (`chunk` table).
  - Implement all 7 `ChunkStore` trait methods (delegates to SurrealDB queries).
  - Write unit tests: insert, upsert, get, delete by file, count, exists, get by file+heading.
  - Register test file: `chunk_store_test.rs`.
  - Verify `cargo test --lib chunk_store` passes.
  - _Requirements: R1_

  - [ ] 5.1 Schema definition and connection
  - [ ] 5.2 CRUD methods
  - [ ] 5.3 Query methods (by file, by heading, exists)
  - [ ] 5.4 Unit tests

- [ ] 6. **Implement `SurrealVectorStore` wrapping `ocean_vector::store::VectorStore`**
  - Create `src/ocean_storage/vector_store_impl.rs`.
  - `SurrealVectorStore` wraps `ocean_vector::store::VectorStore` internally.
  - Implement all 6 `VectorStore` trait methods by delegating to the wrapped store.
  - Add `new_persistent(path)` that creates the wrapped store.
  - Write smoke tests confirming delegation works.
  - _Requirements: R1, R6_

- [ ] 7. **Implement `SurrealGraphStore` wrapping `ocean_graph::store::GraphStore`**
  - Create `src/ocean_storage/graph_store_impl.rs`.
  - `SurrealGraphStore` wraps `ocean_graph::store::GraphStore` internally.
  - Implement all 10 `GraphStore` trait methods by delegating.
  - Add `new_persistent(path)` that creates the wrapped store.
  - Write smoke tests confirming delegation works.
  - _Requirements: R1, R6_

- [ ] 8. **Implement `SurrealStateStore`**
  - Create `src/ocean_storage/state_store_impl.rs`.
  - SurrealDB schema: `index_state` table with `file_id`, `hash`, `last_indexed`, `status`.
  - Implement all 5 `StateStore` trait methods.
  - `list_pending()` returns records where `status != "Indexed"` or hash differs from current file hash (requires cross-referencing with FileStore).
  - Write unit tests: update, get, delete, list_pending, list_all.
  - Register test file: `state_store_test.rs`.
  - Verify `cargo test --lib state_store` passes.
  - _Requirements: R5_

- [ ] 9. **Implement `SurrealStorage` struct**
  - Create all five sub-store connections in `new(base_path)` and `new_memory()`.
  - Shared single `tokio::runtime::Runtime`.
  - Path resolution: `{base_path}/files.db`, `{base_path}/chunks.db`, etc.
  - Implement all `Storage` trait methods.
  - Implement `count_all()` that queries each sub-store and aggregates.
  - Add `StorageConfig` struct for path customization.
  - Write integration test: create `SurrealStorage`, verify all sub-stores are accessible and isolated.
  - Verify `cargo test --lib storage` passes.
  - _Requirements: R1, R2, R7_

  - [ ] 9.1 Constructor, connection setup, path resolution
  - [ ] 9.2 Accessor methods (`files()`, `chunks()`, etc.)
  - [ ] 9.3 `count_all()` + `StorageStats`
  - [ ] 9.4 `StorageConfig` + path helpers
  - [ ] 9.5 Integration tests

- [ ] 10. **Implement transaction model**
  - Create `src/ocean_storage/transaction.rs`.
  - `TransactionStaging` struct with `Vec<StagedWrite>`.
  - Each `StagedWrite` records store name, table/ID, record data.
  - `begin_transaction()` — allocate empty staging.
  - All sub-store trait methods check `in_transaction()`; if true, writes go to staging instead of DB.
  - `commit()` — flush staged writes sequentially; on partial failure, mark files as `Failed` in StateStore.
  - `rollback()` — discard staging.
  - Thread safety: `RefCell<TransactionStaging>` + `Cell<u32>` for depth.
  - Write unit tests: commit succeeds, rollback discards, partial commit marks failed.
  - _Requirements: R3_

  - [ ] 10.1 `TransactionStaging` + `StagedWrite`
  - [ ] 10.2 Integration into `SurrealStorage` and sub-stores
  - [ ] 10.3 `commit()` — flush logic + error handling
  - [ ] 10.4 `rollback()` — discard
  - [ ] 10.5 Unit tests

- [ ] 11. **Migrate `QueryEngine` to optionally use `Storage` trait**
  - Add `Storage`-based constructor: `QueryEngine::from_storage(storage: Box<dyn Storage>)`.
  - Existing constructors remain unchanged (backwards compat).
  - Update `query()` method to delegate to `Storage` sub-stores when available.
  - Update `cmd_query`, `cmd_vector_search` in `ocean_cli::run` to construct `Storage` and pass it.
  - Add CLI flag `--storage-path` as alias for `--db-path`.
  - Verify existing tests pass without changes.
  - _Requirements: R6, R7_

  - [ ] 11.1 `from_storage()` constructor
  - [ ] 11.2 Update `execute()` to use `Storage` sub-stores
  - [ ] 11.3 Update CLI handlers to construct `Storage`
  - [ ] 11.4 Verify backwards compatibility — all tests green

- [ ] 12. **Migrate `IndexPipeline` to optionally use `Storage` trait**
  - Add `Storage`-based constructor: `IndexPipeline::from_storage(storage: Box<dyn Storage>)`.
  - Add `state()` step in pipeline: after all chunks are written, update `StateStore`.
  - Add `needs_update` check at start of indexing: skip files whose state matches.
  - Update `cmd_index` to use `Storage` when available.
  - Verify `ocean index --help` output is consistent.
  - _Requirements: R4, R5, R6, R7_

  - [ ] 12.1 `from_storage()` constructor for `IndexPipeline`
  - [ ] 12.2 Integrate `StateStore::needs_update` for incremental check
  - [ ] 12.3 Update `cmd_index` to use `Storage`
  - [ ] 12.4 Integration test: full index + reindex with `Storage`

- [ ] **Validation & Cleanup**
  - Run full test suite: `cargo test` — all tests must pass.
  - Verify `cargo build --release` succeeds.
  - Run `cargo clippy` (if configured) and fix warnings.
  - Update `AGENTS.md` with ocean_storage module conventions.
  - _Requirements: R6, R8_

## Notes

- **Task order**: 1→2→3→4→5→6→7→8→9→10→11→12. Tasks 6-7 can be done in parallel.
- **Dependencies**: Tasks 4-8 depend on task 2 (traits). Task 9 depends on 4-8. Task 10 depends on 9. Tasks 11-12 depend on 9-10.
- **Testing approach**: Each sub-store gets a `_test.rs` file. Integration tests live in `tests/storage_integration.rs`.
- **Backwards compatibility**: The existing `ocean_vector::store::VectorStore` and `ocean_graph::store::GraphStore` are never deleted — only wrapped.
- **Performance considerations**: Sub-store isolation prevents SurrealDB revision collisions. The shared `Runtime` reduces thread overhead vs. the current per-store runtime.
