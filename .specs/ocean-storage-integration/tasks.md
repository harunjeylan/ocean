# Implementation Plan: Ocean Storage Integration

## Overview

Refactor the codebase in 6 phases to make `ocean_storage` the canonical persistence layer. Each phase is independent and testable. Total estimated effort: 8-12 hours.

## Dependencies

- Phase 0 must be done first (fixes build)
- Phase 1 → Phase 2 → Phases 3-5 (parallel after type consolidation)
- Phase 6 is final validation

---

## Tasks

- [ ] 0. **Fix test imports and register missing tests**
  - Fix test files to use public re-exports instead of private sub-modules
  - Create vector_store_test.rs and graph_store_test.rs in ocean_storage
  - Register all new test files in `src/tests.rs`
  - `cargo test --lib` passes for all ocean_storage tests
  - _Requirements: R9_

  - [ ] 0.1 Fix file_store_test.rs imports (`SurrealFileStore` via `crate::ocean_storage::SurrealFileStore`)
  - [ ] 0.2 Fix chunk_store_test.rs imports
  - [ ] 0.3 Fix state_store_test.rs imports (also remove unused `StateRecord`)
  - [ ] 0.4 Create `vector_store_test.rs` — smoke test for SurrealVectorStore
  - [ ] 0.5 Create `graph_store_test.rs` — smoke test for SurrealGraphStore
  - [ ] 0.6 Register new tests in `src/tests.rs`
  - [ ] 0.7 Verify `cargo test --lib storage_*` passes

- [ ] 1. **Consolidate types in ocean_graph::types**
  - Change `ocean_graph::types` to re-export from `ocean_storage::graph_store`
  - Keep `Subgraph` and `GraphConfig` in `ocean_graph::types` (not storage types)
  - Remove manual conversion code from `SurrealGraphStore` (types are now identical)
  - Verify `cargo build` succeeds
  - _Requirements: R1_

  - [ ] 1.1 Rewrite `ocean_graph/types.rs` to `pub use` from `ocean_storage::graph_store`
  - [ ] 1.2 Remove `convert_node`, `convert_edge`, `convert_node_back`, `convert_edge_back` from `graph_store_impl.rs`
  - [ ] 1.3 Update `SurrealGraphStore` to pass types directly to inner store
  - [ ] 1.4 Verify `cargo build` succeeds

- [ ] 2. **Add missing methods to ocean_storage traits**
  - Augment `GraphStore` trait with methods needed by `ExpansionEngine`, `GraphBuilder`
  - Augment `VectorStore` trait with `initialize_schema`
  - Implement new methods in `SurrealGraphStore` and `SurrealVectorStore`
  - Verify `cargo build` succeeds
  - _Requirements: R2_

  - [ ] 2.1 Add to `ocean_storage::graph_store::GraphStore`: `get_node_by_ref`, `get_nodes_by_type`, `get_edges_by_relation`, `get_nodes_by_file`, `get_edges_by_file`, `clear`, `initialize_schema`, and add `direction` param to `get_edges`
  - [ ] 2.2 Implement new methods in `SurrealGraphStore` (delegate to `inner.*`)
  - [ ] 2.3 Add `initialize_schema` to `ocean_storage::vector_store::VectorStore` trait
  - [ ] 2.4 Implement `initialize_schema` in `SurrealVectorStore`
  - [ ] 2.5 Verify `cargo build` succeeds

- [ ] 3. **Rewrite SearchEngine to use ocean_storage::VectorStore trait**
  - `ocean_vector::search.rs`: accept `Arc<dyn ocean_storage::VectorStore>`
  - Keep old `new(store: VectorStore)` as adapter
  - All methods continue to work identically
  - _Requirements: R3_

  - [ ] 3.1 Change `SearchEngine` struct to hold `Arc<dyn ocean_storage::VectorStore>`
  - [ ] 3.2 Update constructor(s)
  - [ ] 3.3 Update all method implementations to use trait methods
  - [ ] 3.4 Verify `cargo build` succeeds

- [ ] 4. **Rewrite ContextWindowBuilder to use ocean_storage::ChunkStore**
  - `ocean_query/context.rs`: accept `Arc<dyn ocean_storage::ChunkStore>`
  - Update `build()` to call `chunk_store.get_chunk()` and `chunk_store.get_by_file_and_heading()`
  - _Requirements: R4_

  - [ ] 4.1 Change `ContextWindowBuilder` struct to hold `Arc<dyn ocean_storage::ChunkStore>`
  - [ ] 4.2 Update `build()` method
  - [ ] 4.3 Verify `cargo build` succeeds

- [ ] 5. **Rewrite ExpansionEngine to use ocean_storage::GraphStore**
  - `ocean_graph/expansion.rs`: accept `Arc<dyn ocean_storage::GraphStore>`
  - Update all method implementations to use trait methods
  - _Requirements: R5_

  - [ ] 5.1 Change `ExpansionEngine` struct to hold `Arc<dyn ocean_storage::GraphStore>`
  - [ ] 5.2 Update constructor
  - [ ] 5.3 Update `expand()`, `expand_from_chunks()`, `find_path()`, `get_file_graph()`
  - [ ] 5.4 Verify `cargo build` succeeds

- [ ] 6. **Rewrite QueryEngine with from_storage()**
  - Add `from_storage(storage: Arc<dyn Storage>)` constructor
  - Extract sub-stores, wire into SearchEngine, ContextWindowBuilder, ExpansionEngine
  - Keep all existing constructors unchanged
  - _Requirements: R6_

  - [ ] 6.1 Add `from_storage()` constructor to `QueryEngine`
  - [ ] 6.2 Update internal fields to hold `Arc<dyn VectorStore>` and `Arc<dyn ChunkStore>`
  - [ ] 6.3 Update legacy constructors to wrap old stores in adapters
  - [ ] 6.4 Verify `cargo build` succeeds

- [ ] 7. **Rewrite IndexPipeline with from_storage()**
  - Add `from_storage(storage: Arc<dyn Storage>)` constructor
  - Use `ChunkStore` for content, `VectorStore` for embeddings, `StateStore` for state
  - Add incremental indexing via `StateStore::needs_update` (via FileStore)
  - _Requirements: R7_

  - [ ] 7.1 Add `from_storage()` constructor to `IndexPipeline`
  - [ ] 7.2 Add incremental check using state_store
  - [ ] 7.3 Add state update after indexing
  - [ ] 7.4 Update `index_chunks()` to use chunk_store + vector_store
  - [ ] 7.5 Verify `cargo build` succeeds

- [ ] 8. **Update CLI to use SurrealStorage**
  - `ocean_cli/args.rs`: add `--storage-path` alias
  - `ocean_cli/run.rs`: construct `SurrealStorage` in cmd_index, cmd_query, cmd_vector_search, cmd_graph
  - Pass to `from_storage()` constructors
  - _Requirements: R8_

  - [ ] 8.1 Add `--storage-path` to `IndexArgs`, `QueryArgs`, `VectorSearchArgs`
  - [ ] 8.2 Update `cmd_index` to create `SurrealStorage`, use `IndexPipeline::from_storage()`
  - [ ] 8.3 Update `cmd_query` to create `SurrealStorage`, use `QueryEngine::from_storage()`
  - [ ] 8.4 Update `cmd_vector_search` to create `SurrealStorage`, use `storage.vectors()` directly
  - [ ] 8.5 Update `cmd_graph_*` to create `SurrealStorage`, use `storage.graph()`
  - [ ] 8.6 Verify `cargo build` succeeds

- [ ] 9. **Validation & Cleanup**
  - Run full test suite: `cargo test` — all tests must pass
  - Verify `cargo build --release` succeeds
  - Update `AGENTS.md` with ocean_storage integration notes
  - _Requirements: R9_

  - [ ] 9.1 Run `cargo test` — fix any failures
  - [ ] 9.2 Run `cargo build --release` — fix any release-mode issues
  - [ ] 9.3 Update `AGENTS.md`

## Notes

- **Phase order**: 0 → 1 → 2 → (3,4,5 in parallel) → 6 → 7 → 8 → 9
- **Backwards compatibility**: Old constructors and old type paths (`ocean_graph::types::Node`, etc.) continue to compile. No existing test should need modification.
- **Testing approach**: After each phase, run `cargo build` and relevant unit tests before moving on. Phase 9 runs the full suite.
- **Type adaptation**: Where old concrete store types need to be passed where `Arc<dyn Trait>` is expected, use a thin wrapper struct that implements the trait by delegating to the concrete type.
