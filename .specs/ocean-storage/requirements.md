# Requirements Document: Ocean Storage Layer

## Introduction

ocean-storage is the **persistence backbone of Ocean**. It provides a unified, transactional storage layer that consolidates file metadata, chunk content, vector embeddings, graph nodes/edges, and indexing state into a single coherent interface.

Currently, persistence logic is split across `ocean_vector::store::VectorStore` and `ocean_graph::store::GraphStore`, each managing its own SurrealDB/SurrealKv connection. There is no central `Storage` trait, no `FileStore` or `StateStore`, and no transaction model. This phase extracts a new `ocean_storage` module that provides a unified `Storage` trait with five sub-store accessors (`files()`, `chunks()`, `vectors()`, `graph()`, `state()`), wrapping the existing SurrealDB-backed implementations while adding file tracking, indexing state, and transaction support.

The scope is MVP: unify what exists, add missing stores (file, state), add a master trait, add transaction scaffolding, and provide a migration path for existing consumers (`QueryEngine`, `IndexPipeline`, CLI handlers).

---

## Glossary

- **FileStore**: Sub-store responsible for persisting file identity and metadata (path, hash, size, modified time, last indexed time).
- **ChunkStore**: Sub-store responsible for persisting chunk content and structural metadata (heading, page, slide, block type).
- **VectorStore**: Sub-store responsible for persisting embeddings and enabling similarity search (HNSW-backed in SurrealDB).
- **GraphStore**: Sub-store responsible for persisting knowledge graph nodes and edges with CRUD and neighbor traversal.
- **StateStore**: Sub-store responsible for tracking per-file indexing state (hash, last indexed timestamp, status) to enable incremental indexing.
- **Storage trait**: Master interface that provides accessors for all five sub-stores and a transaction API.
- **SurrealKv**: SurrealDB's embedded key-value storage engine (RocksDB-based), used as the persistence backend by all sub-stores.
- **Transactional consistency**: The guarantee that a multi-file indexing operation either fully commits or fully rolls back across all sub-stores.
- **Sub-store**: A component within the storage layer that handles a specific domain of data (e.g. files, chunks, vectors).

---

## Requirements

### R1: Unified Storage Trait

**User Story:** As a system integrator (ocean-index, ocean-query), I want a single `Storage` trait that provides access to all sub-stores, so that I do not need to manage separate VectorStore and GraphStore instances.

#### Acceptance Criteria

1. THE `Storage` trait SHALL define accessor methods `files()`, `chunks()`, `vectors()`, `graph()`, `state()`, each returning a `&dyn` reference to the corresponding sub-store trait.
2. THE `Storage` trait SHALL provide `begin_transaction()`, `commit()`, and `rollback()` methods.
3. A `SurrealStorage` struct SHALL implement `Storage`, wrapping all five sub-store implementations behind a single SurrealDB connection (single namespace + database).
4. WHEN all sub-stores share the same SurrealDB connection, THEN they SHALL use the same SurrealKv directory (single RocksDB instance).
5. THE `Storage` trait SHALL be object-safe OR provide a `Box<dyn Storage>` wrapper type.
6. THE `Storage` trait SHALL be constructable via `Storage::new(db_path)` and `Storage::new_memory()`.
7. THE `ChunkStore` trait SHALL provide `insert_chunk`, `upsert_chunk`, `get_chunk`, `delete_chunks_by_file`, `count`, `chunk_exists`, `get_by_file_and_heading`.
8. THE `FileStore` trait SHALL provide `upsert_file`, `get_file`, `get_file_by_path`, `delete_file`, `list_files`.
9. THE `StateStore` trait SHALL provide `update_state`, `get_state`, `delete_state`, `list_pending`.
10. THE `VectorStore` trait SHALL provide `vector_search`, `fts_search`, `insert`, `delete_by_file`.
11. THE `GraphStore` trait SHALL provide `insert_node`, `insert_edge`, `insert_nodes_batch`, `insert_edges_batch`, `get_node`, `get_neighbors`, `get_edges`, `delete_by_file`, `count_nodes`, `count_edges`.

---

### R2: Separate Directory Per Store (Prevent Revision Collisions)

**User Story:** As a developer, I want each sub-store to use a physically separate SurrealKv directory (within a parent storage directory), so that SurrealDB revision mismatches do not occur when multiple tables are defined in different connections.

#### Acceptance Criteria

1. THE default storage path SHALL be `~/.ocean/store/{cwd-kebab-case}/` with sub-directories `files/`, `chunks/`, `vectors/`, `graph/`, `state/`.
2. EACH sub-store SHALL open its own SurrealKv connection to its own sub-directory.
3. THE `Storage` struct SHALL manage all five sub-connections and expose them through the `Storage` trait.
4. WHEN the user provides a custom `--db-path`, THEN the storage root SHALL be `{provided}/` with the same sub-directory convention.
5. THE existing `VectorStore` and `GraphStore` constructors SHALL remain unchanged for backwards compatibility, but SHALL delegate to the unified `Storage` when constructed through `Storage::new`.

---

### R3: Transaction Model

**User Story:** As the indexing pipeline, I want to wrap a batch of file operations (delete old chunks, insert new chunks, update vectors, rebuild graph, update state) in a single transaction, so that partial failures do not leave storage in an inconsistent state.

#### Acceptance Criteria

1. THE `Storage` trait SHALL expose `begin_transaction()`, `commit()`, and `rollback()`.
2. SurrealDB does not support multi-table ACID transactions across separate SurrealKv connections; THEREFORE transaction support SHALL be best-effort at the application level using a write-ahead approach (write to a staging area, then flush on commit; on rollback, discard staging).
3. WHEN `commit()` succeeds, ALL sub-store writes since `begin_transaction()` SHALL be persisted.
4. WHEN `rollback()` is called, ALL sub-store writes since `begin_transaction()` SHALL be discarded.
5. IF `commit()` fails partway, the storage SHALL be in a recoverable state (re-index affected files).
6. THE transaction model SHALL be optional — single-write operations SHALL work without an explicit transaction.

---

### R4: File Metadata Store

**User Story:** As the index orchestrator, I want to persist file metadata (path, hash, size, extension, last indexed) in the storage layer, so that I can detect changes and skip unmodified files during incremental indexing.

#### Acceptance Criteria

1. THE `FileStore` SHALL persist `FileMeta` records with fields: `id` (UUIDv7), `path`, `hash` (SHA-256), `size`, `modified` (Unix timestamp), `extension`, `last_indexed` (Unix timestamp).
2. THE `FileStore` SHALL support upsert by file ID.
3. THE `FileStore` SHALL support lookup by file ID and by absolute path.
4. THE `FileStore` SHALL support deletion by file ID.
5. THE `FileStore` SHALL support listing all tracked files.
6. THE `FileStore` SHALL support a `needs_update(file: &FileMeta) -> bool` method that compares current hash + modified time against stored values.

---

### R5: Indexing State Store

**User Story:** As the index orchestrator, I want to persist per-file indexing state (hash, last indexed timestamp, status), so that I can resume interrupted indexing sessions and detect which files need reprocessing.

#### Acceptance Criteria

1. THE `StateStore` SHALL persist records with fields: `file_id`, `hash`, `last_indexed`, `status` (one of `Pending`, `Indexed`, `Failed`).
2. THE `StateStore` SHALL support `update_state(file_id, hash, status)` — upsert by file_id.
3. THE `StateStore` SHALL support `get_state(file_id) -> Option<StateRecord>`.
4. THE `StateStore` SHALL support `delete_state(file_id)`.
5. THE `StateStore` SHALL support `list_pending() -> Vec<StateRecord>` for files that are `Pending` or whose hash differs from stored state.

---

### R6: Backwards Compatibility

**User Story:** As an existing consumer of `VectorStore` and `GraphStore`, I want the existing APIs to continue working without modification after the ocean-storage module is introduced.

#### Acceptance Criteria

1. THE existing `VectorStore` in `ocean_vector::store` SHALL continue to exist and work identically.
2. THE existing `GraphStore` in `ocean_graph::store` SHALL continue to exist and work identically.
3. THE `QueryEngine` SHALL be updated to optionally accept a `Storage` trait instead of separate `VectorStore` + optional `ExpansionEngine`.
4. THE `IndexPipeline` SHALL be updated to optionally use `Storage::chunks()`, `Storage::vectors()`, `Storage::graph()`, `Storage::state()` instead of direct store calls.
5. ALL existing tests SHALL continue to pass without modification.

---

### R7: CLI Integration

**User Story:** As a CLI user, I want the existing index/query/vector-search/graph commands to continue working with the unified storage layer.

#### Acceptance Criteria

1. THE `ocean index` command SHALL use `Storage` internally when the ocean-storage module is available, falling back to the existing separate-store approach for backwards compatibility.
2. THE `ocean query` and `ocean vector-search` commands SHALL accept `--storage-path` as an alias for `--db-path`, with identical behavior.
3. THE default storage paths SHALL follow the `~/.ocean/store/{cwd}/{files,chunks,vectors,graph,state}.db` convention.
4. THE `ocean info` command SHALL display storage statistics (file count, chunk count, node count, edge count) from the unified storage layer.

---

### R8: Error Handling

**User Story:** As a system operator, I want clear, typed error variants for all storage operations, so that I can handle failures appropriately in the indexing pipeline and CLI.

#### Acceptance Criteria

1. A `StorageError` enum SHALL exist with variants: `ConnectionFailed`, `QueryFailed`, `RecordNotFound`, `SchemaError`, `TransactionFailed`, `BatchFailed`.
2. EACH sub-store trait method SHALL return `Result<T, StorageError>`.
3. THE `StorageError` SHALL implement `Display`, `Error`, `Send`, and `Sync`.
4. WHEN a sub-store operation fails, the error SHALL include the sub-store name (e.g. "FileStore::upsert_file: ...").
5. WHEN a transaction commit fails, the error SHALL include how many sub-stores succeeded and how many failed.
