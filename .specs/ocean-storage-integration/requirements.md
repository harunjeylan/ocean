# Requirements Document: Ocean Storage Integration

## Introduction

The `ocean_storage` module (defined in `.specs/ocean-storage/`) provides a unified persistence layer with five sub-store traits (`FileStore`, `ChunkStore`, `VectorStore`, `GraphStore`, `StateStore`), transaction support, and a master `Storage` trait. The module has been implemented but **not yet integrated** into the existing consumers: `ocean_vector`, `ocean_graph`, `ocean_query`, and `ocean_cli`.

Currently, the codebase has two parallel sets of types (`ocean_graph::types::Node` vs `ocean_storage::graph_store::Node`), duplicate store logic, and no unified entry point. This spec covers the refactoring to make `ocean_storage` the canonical persistence layer, with existing modules re-exporting types from it.

## Scope

- **Make `ocean_storage` type definitions canonical** — `ocean_graph::types` re-exports from `ocean_storage::graph_store`
- **Augment `ocean_storage` traits** — add missing methods needed by consumers
- **Rewrite consumers** — `SearchEngine`, `ContextWindowBuilder`, `ExpansionEngine` accept `ocean_storage` traits
- **Rewrite orchestrators** — `QueryEngine`, `IndexPipeline` accept `Box<dyn Storage>`
- **Update CLI** — all commands use `SurrealStorage`
- **Fix tests** — use public re-exports; add missing test coverage
- **Keep backwards compatibility** — old APIs remain for migration window

## Glossary

- **Canonical type**: The single authoritative type definition; all other modules re-export or reference it
- **Consumer**: A module that uses storage types/traits (e.g., `SearchEngine`, `ExpansionEngine`, `QueryEngine`)
- **Orchestrator**: A high-level module that coordinates multiple stores (e.g., `QueryEngine`, `IndexPipeline`)
- **SurrealGraphStore wrapper**: The ocean_storage implementation that delegates to `ocean_graph::store::GraphStore`
- **Type conversion cost**: The overhead of converting between duplicate type definitions (Node→Node, Edge→Edge, etc.)

---

## Requirements

### R1: Canonical Type Definitions in ocean_storage

**User Story:** As a developer, I want a single source of truth for `Node`, `Edge`, `NodeType`, `RelationType`, and `EdgeDirection` types, so that I do not need to convert between identical types when crossing module boundaries.

#### Acceptance Criteria

1. THE `ocean_storage::graph_store` module SHALL define the canonical `Node`, `Edge`, `NodeType`, `RelationType`, and `EdgeDirection` types.
2. THE `ocean_graph::types` module SHALL re-export these types via `pub use crate::ocean_storage::graph_store::{...}`.
3. THE `ocean_graph::store::GraphStore` struct SHALL use the `ocean_storage::graph_store` types in its own public API.
4. THE `SurrealGraphStore` wrapper SHALL remove all manual `convert_node`/`convert_edge` functions since types are now identical.
5. ALL existing code that uses `ocean_graph::types::*` SHALL continue to compile without changes (since re-exports are API-compatible).

---

### R2: Complete Trait Interfaces

**User Story:** As the `ExpansionEngine` and `GraphBuilder`, I want the `ocean_storage::GraphStore` trait to expose all methods I need, so that I can switch to using the trait without losing functionality.

#### Acceptance Criteria

1. THE `ocean_storage::GraphStore` trait SHALL add: `get_node_by_ref()`, `get_nodes_by_type()`, `get_edges_by_relation()`, `get_nodes_by_file()`, `get_edges_by_file()`, `clear()`.
2. THE `ocean_storage::GraphStore::get_edges()` method SHALL accept an `EdgeDirection` parameter.
3. THE `ocean_storage::GraphStore` trait SHALL add `initialize_schema()`.
4. THE `ocean_storage::VectorStore` trait SHALL add `initialize_schema(dimension: usize)`.
5. THE `SurrealGraphStore` SHALL implement all new methods by delegating to the inner `ocean_graph::store::GraphStore`.

---

### R3: SearchEngine Uses ocean_storage::VectorStore

**User Story:** As the `QueryEngine`, I want `SearchEngine` to accept an `ocean_storage::VectorStore` trait, so that I can pass it a store obtained from `Storage::vectors()`.

#### Acceptance Criteria

1. THE `SearchEngine` struct SHALL accept `Arc<dyn ocean_storage::VectorStore>` as its store.
2. ALL existing `SearchEngine` methods (`search()`, `hybrid_search()`, `filtered_search()`, `hybrid_filtered_search()`) SHALL continue to work identically.
3. THE `SearchEngine` SHALL expose a constructor `new(store: Arc<dyn ocean_storage::VectorStore>)`.
4. The old `SearchEngine::new(store: VectorStore)` constructor SHALL remain available (wrap the old store in an adapter if needed).

---

### R4: ContextWindowBuilder Uses ocean_storage::ChunkStore

**User Story:** As the `QueryEngine`, I want `ContextWindowBuilder` to use `ocean_storage::ChunkStore` instead of directly depending on `ocean_vector::store::VectorStore`.

#### Acceptance Criteria

1. THE `ContextWindowBuilder` SHALL accept `Arc<dyn ocean_storage::ChunkStore>`.
2. THE `build()` method SHALL delegate to `chunk_store.get_chunk()` and `chunk_store.get_by_file_and_heading()`.
3. THE old constructor SHALL remain available for backwards compatibility.

---

### R5: ExpansionEngine Uses ocean_storage::GraphStore

**User Story:** As the `QueryEngine`, I want `ExpansionEngine` to use `ocean_storage::GraphStore` so that I can pass a store from `Storage::graph()`.

#### Acceptance Criteria

1. THE `ExpansionEngine` SHALL accept `Arc<dyn ocean_storage::GraphStore>`.
2. ALL existing methods (`expand()`, `expand_from_chunks()`, `find_path()`, `get_file_graph()`) SHALL continue to work identically.
3. THE old constructor SHALL remain available.

---

### R6: QueryEngine Accepts Storage Trait

**User Story:** As a CLI user, I want `QueryEngine` to be constructable from a single `Storage` trait, so that I don't need to manage separate `VectorStore` and `GraphStore` instances.

#### Acceptance Criteria

1. THE `QueryEngine` SHALL provide `from_storage(storage: Arc<dyn Storage>) -> Result<Self, QueryError>`.
2. The `from_storage()` constructor SHALL extract sub-stores from the `Storage` trait and wire them into `SearchEngine`, `ContextWindowBuilder`, and `ExpansionEngine`.
3. THE `QueryEngine::query()` method SHALL continue to work identically regardless of which constructor was used.
4. ALL existing constructors SHALL continue to work unchanged.

---

### R7: IndexPipeline Accepts Storage Trait

**User Story:** As a CLI user, I want `IndexPipeline` to be constructable from a single `Storage` trait, using `StateStore` for incremental indexing.

#### Acceptance Criteria

1. THE `IndexPipeline` SHALL provide `from_storage(storage: Arc<dyn Storage>) -> Self`.
2. WHEN `from_storage()` is used, the pipeline SHALL:
   - Use `ChunkStore` for content chunk persistence
   - Use `VectorStore` for embedding persistence
   - Use `StateStore` for tracking indexing state
   - Check `StateStore` for each file before re-indexing (incremental)
   - Update `StateStore` after each file is indexed
3. THE old `IndexPipeline::new(store: VectorStore)` SHALL continue to work unchanged.

---

### R8: CLI Uses SurrealStorage

**User Story:** As a CLI user, I want the `ocean index`, `ocean query`, `ocean vector-search`, and `ocean graph` commands to use `SurrealStorage` internally, so that all stores share a single base path and are managed uniformly.

#### Acceptance Criteria

1. THE `ocean index` command SHALL construct `SurrealStorage` and pass it to `IndexPipeline::from_storage()`.
2. THE `ocean query` command SHALL construct `SurrealStorage` and pass it to `QueryEngine::from_storage()`.
3. THE `ocean vector-search` command SHALL construct `SurrealStorage` and use `storage.vectors()` directly.
4. THE `ocean graph` subcommands SHALL construct `SurrealStorage` and use `storage.graph()`.
5. THE `--storage-path` flag SHALL be available as an alias for `--db-path` and map to `SurrealStorage::new()`.
6. THE default storage path SHALL follow the `~/.ocean/store/{cwd}/{files,chunks,vectors,graph,state}.db` convention.

---

### R9: Tests Pass

**User Story:** As a developer, I want all existing tests to pass after the refactoring, and new tests to cover the integration points.

#### Acceptance Criteria

1. ALL existing tests in `ocean_graph`, `ocean_vector`, `ocean_query` SHALL pass without modification.
2. ALL `ocean_storage` tests SHALL pass (fix test imports to use public re-exports).
3. NEW integration tests SHALL verify that `QueryEngine` and `IndexPipeline` produce identical results whether constructed via the old API or via `Storage::from_storage()`.
4. `cargo test` SHALL pass with zero failures before the refactoring is considered complete.
