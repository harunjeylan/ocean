# Design Document: Ocean Storage Layer

## Overview

ocean-storage extracts and consolidates the persistence logic currently embedded in `ocean_vector::store` and `ocean_graph::store` into a single, cohesive module with a unified `Storage` trait, five sub-store traits, application-level transaction support, and file/state tracking that enables incremental indexing.

The current codebase has two independent SurrealDB-backed stores (`VectorStore` and `GraphStore`), each managing its own SurrealKv (RocksDB) directory and runtime. There is no shared interface, no file tracking, no indexing state, and no transaction model. ocean-storage wraps these into a `SurrealStorage` struct that owns all five sub-store connections behind a single public API, while keeping the existing store types available for backwards compatibility.

### Key Design Decisions

1. **Separate SurrealKv sub-directories per sub-store** — SurrealDB revision mismatches occur when multiple connections share the same RocksDB directory. Each sub-store gets its own sub-directory within a parent storage directory. This prevents corruption while keeping SurrealDB as the backend.
2. **Application-level transactions** — SurrealDB does not support multi-table ACID transactions across separate connections. Transactions are implemented as a write-ahead staging buffer: writes during a transaction go to an in-memory staging area; `commit()` flushes all staged writes sequentially; `rollback()` discards the staging buffer. Partial commit failures mark the affected files for re-index via `StateStore`.
3. **Trait-based sub-store access** — The `Storage` trait returns `&dyn FileStore`, `&dyn ChunkStore`, etc., allowing alternate backends (e.g. SQLite via `rusqlite`, in-memory for tests) to be swapped in without changing consumers.
4. **Backwards compatibility first** — `VectorStore` and `GraphStore` remain in their current locations with their current APIs. The new `SurrealStorage` struct wraps them internally. Consumers can migrate incrementally.
5. **Single tokio Runtime per Storage instance** — All sub-stores share one async runtime, created once in `SurrealStorage::new()`. This avoids the runtime-per-store overhead of the current design.

---

## Architecture

### High-level module structure

```text
src/ocean_storage/
├── mod.rs              — public re-exports, Storage trait, SurrealStorage
├── file_store.rs       — FileStore trait + SurrealFileStore
├── chunk_store.rs      — ChunkStore trait + SurrealChunkStore
├── vector_store.rs     — VectorStore trait + SurrealVectorStore (wraps ocean_vector::store)
├── graph_store.rs      — GraphStore trait + SurrealGraphStore (wraps ocean_graph::store)
├── state_store.rs      — StateStore trait + SurrealStateStore
├── error.rs            — StorageError enum
├── transaction.rs      — TransactionStaging, TransactionError
└── config.rs           — StorageConfig, path resolution helpers
```

### Data flow

```text
IndexPipeline / QueryEngine
        │
        ▼
  Storage trait (unified interface)
        │
        ├── files()    ──► FileStore    ──► SurrealFileStore    (SurrealKv: storage/files.db)
        ├── chunks()   ──► ChunkStore   ──► SurrealChunkStore   (SurrealKv: storage/chunks.db)
        ├── vectors()  ──► VectorStore  ──► SurrealVectorStore  (SurrealKv: storage/vectors.db)
        ├── graph()    ──► GraphStore   ──► SurrealGraphStore   (SurrealKv: storage/graph.db)
        └── state()    ──► StateStore   ──► SurrealStateStore   (SurrealKv: storage/state.db)
```

### Directory layout

```
~/.ocean/store/{cwd-kebab-case}/
├── files.db/       (SurrealKv directory — SurrealFileStore)
├── chunks.db/      (SurrealKv directory — SurrealChunkStore)
├── vectors.db/     (SurrealKv directory — SurrealVectorStore)
├── graph.db/       (SurrealKv directory — SurrealGraphStore)
└── state.db/       (SurrealKv directory — SurrealStateStore)
```

---

## Components and Interfaces

### 1. Storage Trait (Master Interface)

The central abstraction exposed to all consumers.

```rust
pub trait Storage: Send + Sync {
    fn files(&self) -> &dyn FileStore;
    fn chunks(&self) -> &dyn ChunkStore;
    fn vectors(&self) -> &dyn VectorStore;
    fn graph(&self) -> &dyn GraphStore;
    fn state(&self) -> &dyn StateStore;

    fn begin_transaction(&mut self) -> Result<(), StorageError>;
    fn commit(&mut self) -> Result<(), StorageError>;
    fn rollback(&mut self) -> Result<(), StorageError>;
    fn in_transaction(&self) -> bool;

    fn storage_path(&self) -> &str;
    fn count_all(&self) -> Result<StorageStats, StorageError>;
}

pub struct StorageStats {
    pub file_count: u64,
    pub chunk_count: u64,
    pub node_count: u64,
    pub edge_count: u64,
}
```

### 2. FileStore Sub-Store

```rust
pub trait FileStore: Send + Sync {
    fn upsert_file(&self, file: &FileMeta) -> Result<(), StorageError>;
    fn get_file(&self, id: &str) -> Result<Option<FileMeta>, StorageError>;
    fn get_file_by_path(&self, path: &str) -> Result<Option<FileMeta>, StorageError>;
    fn delete_file(&self, id: &str) -> Result<bool, StorageError>;
    fn list_files(&self) -> Result<Vec<FileMeta>, StorageError>;
    fn needs_update(&self, file: &FileMeta) -> Result<bool, StorageError>;
}

pub struct FileMeta {
    pub id: String,
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub modified: i64,
    pub extension: String,
    pub last_indexed: i64,
}
```

### 3. ChunkStore Sub-Store

```rust
pub trait ChunkStore: Send + Sync {
    fn insert_chunk(&self, chunk: &ChunkRecord) -> Result<(), StorageError>;
    fn upsert_chunk(&self, chunk: &ChunkRecord) -> Result<(), StorageError>;
    fn get_chunk(&self, chunk_id: &str) -> Result<Option<ChunkRecord>, StorageError>;
    fn delete_chunks_by_file(&self, file_id: &str) -> Result<u64, StorageError>;
    fn count(&self) -> Result<u64, StorageError>;
    fn chunk_exists(&self, content_hash: &str, model: &str) -> Result<bool, StorageError>;
    fn get_by_file_and_heading(
        &self,
        file_id: &str,
        heading: Option<&str>,
    ) -> Result<Vec<ChunkRecord>, StorageError>;
}
```

### 4. VectorStore Sub-Store

```rust
pub trait VectorStore: Send + Sync {
    fn insert(&self, record: &ChunkRecord) -> Result<(), StorageError>;
    fn vector_search(
        &self,
        query_vec: &[f32],
        top_k: usize,
        extra_where: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, StorageError>;
    fn fts_search(
        &self,
        query: &str,
        top_k: usize,
        extra_where: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, StorageError>;
    fn delete_by_file(&self, file_id: &str) -> Result<u64, StorageError>;
    fn count(&self) -> Result<u64, StorageError>;
}
```

### 5. GraphStore Sub-Store

```rust
pub trait GraphStore: Send + Sync {
    fn insert_node(&self, node: &Node, file_id: &str) -> Result<(), StorageError>;
    fn insert_edge(&self, edge: &Edge, file_id: &str) -> Result<(), StorageError>;
    fn insert_nodes_batch(&self, nodes: Vec<(Node, String)>) -> Result<(), StorageError>;
    fn insert_edges_batch(&self, edges: Vec<(Edge, String)>) -> Result<(), StorageError>;
    fn get_node(&self, id: &str) -> Result<Option<Node>, StorageError>;
    fn get_neighbors(&self, node_id: &str) -> Result<Vec<(Node, Edge)>, StorageError>;
    fn get_edges(&self, node_id: &str) -> Result<Vec<Edge>, StorageError>;
    fn delete_by_file(&self, file_id: &str) -> Result<u64, StorageError>;
    fn count_nodes(&self) -> Result<u64, StorageError>;
    fn count_edges(&self) -> Result<u64, StorageError>;
}
```

### 6. StateStore Sub-Store

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum IndexStatus {
    Pending,
    Indexed,
    Failed,
}

pub struct StateRecord {
    pub file_id: String,
    pub hash: String,
    pub last_indexed: i64,
    pub status: IndexStatus,
}

pub trait StateStore: Send + Sync {
    fn update_state(&self, file_id: &str, hash: &str, status: IndexStatus) -> Result<(), StorageError>;
    fn get_state(&self, file_id: &str) -> Result<Option<StateRecord>, StorageError>;
    fn delete_state(&self, file_id: &str) -> Result<bool, StorageError>;
    fn list_pending(&self) -> Result<Vec<StateRecord>, StorageError>;
    fn list_all(&self) -> Result<Vec<StateRecord>, StorageError>;
}
```

---

## SurrealStorage (Reference Implementation)

```rust
pub struct SurrealStorage {
    path: String,
    rt: tokio::runtime::Runtime,
    files: SurrealFileStore,
    chunks: SurrealChunkStore,
    vectors: SurrealVectorStore,
    graph: SurrealGraphStore,
    state: SurrealStateStore,
    transaction_depth: Cell<u32>,
    staging: RefCell<TransactionStaging>,
}

impl SurrealStorage {
    pub fn new(base_path: &str) -> Result<Self, StorageError> { ... }
    pub fn new_memory() -> Result<Self, StorageError> { ... }
}
```

Each sub-store opens its own SurrealKv connection at `{base_path}/files.db`, `{base_path}/chunks.db`, etc. The single `Runtime` is shared across all sub-stores via `Arc<Runtime>`.

---

## Data Models (SurrealDB Schema)

### FileStore — table: `file`

| Field | Type | Notes |
|-------|------|-------|
| `file_id` | `string` | UUIDv7 primary key |
| `path` | `string` | Absolute path |
| `hash` | `string` | SHA-256 hex |
| `size` | `int` | Bytes |
| `modified` | `int` | Unix timestamp |
| `extension` | `string` | Lowercase, no dot |
| `last_indexed` | `int` | Unix timestamp |

Indexes: `idx_file_id UNIQUE ON file_id`, `idx_file_path UNIQUE ON path`

### ChunkStore — table: `chunk`

Same schema as existing `ChunkRecord` in `ocean_vector::store`.

### StateStore — table: `index_state`

| Field | Type | Notes |
|-------|------|-------|
| `file_id` | `string` | UUIDv7 primary key |
| `hash` | `string` | SHA-256 hex at time of indexing |
| `last_indexed` | `int` | Unix timestamp |
| `status` | `string` | "Pending" | "Indexed" | "Failed" |

Indexes: `idx_state_file_id UNIQUE ON file_id`, `idx_state_status ON status`

---

## Transaction Model

SurrealDB does not support distributed transactions across separate SurrealKv connections. Ocean implements an application-level transaction:

```
begin_transaction()
  → creates empty TransactionStaging { staged_writes: Vec<StagedWrite> }

write to any sub-store during transaction
  → StagedWrite { store_name, table, record } appended to staging

commit()
  → for each StagedWrite in staging:
       write to actual SurrealDB table via sub-store connection
  → if any write fails:
       mark affected file_id as status=Failed in StateStore
       return TransactionError with details of failed writes
  → on success: clear staging

rollback()
  → clear staging (no writes were flushed)
```

For non-transactional writes (the common case), writes go directly to SurrealDB without staging.

---

## Correctness Properties

### Property 1: Sub-store Isolation

*For any* sub-store, writes to that sub-store SHALL NOT affect the data of any other sub-store (each has its own SurrealKv directory).

**Validates:** R2

### Property 2: Transaction Atomicity

*For any* sequence of writes within a `begin_transaction()` / `commit()` block, either ALL writes are persisted or NONE are persisted (best-effort — failed partial writes mark files as `Failed` for recovery).

**Validates:** R3

### Property 3: File Change Detection

*For any* `FileMeta` whose `hash` differs from the stored `hash` in `StateStore`, `needs_update()` SHALL return `true`.

**Validates:** R4, R5

### Property 4: Backwards Compatibility

*For any* test that passes against the current `VectorStore`/`GraphStore` APIs, the same test SHALL pass when those stores are constructed via `SurrealStorage`.

**Validates:** R6

### Property 5: Deterministic Path Resolution

*For any* given base storage path, the sub-store paths SHALL be deterministic: `{base}/files.db`, `{base}/chunks.db`, `{base}/vectors.db`, `{base}/graph.db`, `{base}/state.db`.

**Validates:** R2, R7

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| Sub-store connection fails | `StorageError::ConnectionFailed(sub_store_name, details)` |
| SurrealDB query returns error | `StorageError::QueryFailed(sub_store_name, details)` |
| Record not found by ID | `StorageError::RecordNotFound(sub_store_name, id)` |
| Schema definition fails | `StorageError::SchemaError(sub_store_name, details)` |
| Transaction commit partially fails | `StorageError::TransactionFailed { succeeded: Vec<String>, failed: Vec<(String, String)> }` |
| Batch insert partially fails | `StorageError::BatchFailed(sub_store_name, succeeded: u64, failed: u64, last_error: String)` |

---

## Testing Strategy

### Unit Tests

- `SurrealStorage::new()` creates all five sub-store directories and connections.
- Each sub-store trait method has CRUD tests (happy path + not-found).
- `needs_update()` returns correct boolean for same/different hash.
- `StateStore::list_pending()` returns only pending/failed/changed files.
- `Storage::count_all()` returns aggregated counts across sub-stores.

### Integration Tests

- `QueryEngine` using `SurrealStorage` (via `Storage` trait) produces same results as with separate `VectorStore` + `GraphStore`.
- `IndexPipeline` using `Storage` trait produces same index results.
- Transaction commit/rollback works across sub-stores.
- Concurrent reads during write do not block.

### Property-Based Tests

- Property 1 (sub-store isolation): write random data to each sub-store, verify other sub-stores remain empty.
- Property 3 (file change detection): generate random `FileMeta` with same/different hashes, verify `needs_update()` return value.
