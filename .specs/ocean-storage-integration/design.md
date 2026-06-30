# Design Document: Ocean Storage Integration

## Overview

Refactor the codebase to make `ocean_storage` the canonical persistence layer. The existing `ocean_graph::types` types become thin re-exports of `ocean_storage::graph_store` types. Consumer modules (`SearchEngine`, `ExpansionEngine`, `ContextWindowBuilder`) accept `Arc<dyn ocean_storage::*>` traits instead of concrete store types. Orchestrators (`QueryEngine`, `IndexPipeline`) gain `from_storage()` constructors. The CLI handles all storage through `SurrealStorage`.

### Key Design Decisions

1. **Types migrate to ocean_storage, existing modules re-export** — Since `ocean_storage` wraps `ocean_graph::store::GraphStore` and `ocean_vector::store::VectorStore`, the direction of dependency flows from consumers → ocean_storage → old modules. Making `ocean_storage` the canonical type owner avoids circular dependencies (everything is in the same crate).

2. **Arc<dyn Trait> for store references** — Consumer modules need shared ownership of stores (e.g., `SearchEngine` and `ContextWindowBuilder` both reference the same store). `Arc<dyn Trait>` provides reference-counted shared ownership with trait-object dispatch.

3. **Dual constructors for backwards compatibility** — Old constructors that take concrete types stay. New constructors that take `Arc<dyn Trait>` are added. Internally, the struct stores the trait object, and the old constructors wrap the concrete type in an adapter.

4. **SurrealGraphStore removes conversion layer** — Once `ocean_graph::types` re-exports from `ocean_storage::graph_store`, the types are identical (`std::mem::discriminant` and layout match). `SurrealGraphStore` can pass `ocean_storage::graph_store` types directly to `ocean_graph::store::GraphStore` without conversion.

5. **SurrealStorage as the single entry point** — CLI code constructs one `SurrealStorage`, then passes `Arc::new(storage)` to `QueryEngine` or `IndexPipeline`. Sub-stores are extracted via `storage.files()`, `storage.chunks()`, etc.

---

## Architecture

### Before (current state)

```
ocean_graph::types          ocean_vector::store
  Node, Edge (duplicate)      ChunkRecord (with embeddings)
        │                              │
        ▼                              ▼
ocean_graph::store:GraphStore    ocean_vector::store:VectorStore
        │                              │
        ▼                              ▼
  SurrealGraphStore              SurrealVectorStore
  (has convert_node/edge)        (creates surrogate ChunkRecord)
        │                              │
        └──────────┬───────────────────┘
                   ▼
            SurrealStorage
         (mod.rs + storage_impl.rs)
```

### After (target state)

```
       ocean_storage::graph_store (canonical types)
         Node, Edge, NodeType, RelationType
                ▲
                │ re-exports
                │
         ocean_graph::types
       (pub use crate::ocean_storage::graph_store::*)


  Arc<dyn ocean_storage::ChunkStore>
          │                ▲
          │                │
  ContextWindowBuilder ─────┘

  Arc<dyn ocean_storage::VectorStore>
          │                ▲
          │                │
  SearchEngine ─────────────┘

  Arc<dyn ocean_storage::GraphStore>
          │                ▲
          │                │
  ExpansionEngine ─────────┘

  SurrealStorage (implements Storage trait)
    files()    → &dyn FileStore
    chunks()   → &dyn ChunkStore
    vectors()  → &dyn VectorStore
    graph()    → &dyn GraphStore
    state()    → &dyn StateStore
          │
          ├── QueryEngine::from_storage()
          └── IndexPipeline::from_storage()
```

---

## Components and Interfaces

### 1. Type Consolidation (ocean_graph::types → re-export)

```rust
// src/ocean_graph/types.rs — AFTER refactoring
pub use crate::ocean_storage::graph_store::{
    Edge, EdgeDirection, Node, NodeType, RelationType,
};
```

The `Subgraph` struct and `GraphConfig` struct remain defined in `ocean_graph::types` since they are not storage-layer types.

### 2. Enhanced GraphStore Trait

```rust
// src/ocean_storage/graph_store.rs — AFTER adding missing methods
pub trait GraphStore: Send + Sync {
    // Existing methods...
    fn insert_node(&self, node: &Node, file_id: &str) -> Result<(), StorageError>;
    fn insert_edge(&self, edge: &Edge, file_id: &str) -> Result<(), StorageError>;
    fn insert_nodes_batch(&self, nodes: Vec<(Node, String)>) -> Result<(), StorageError>;
    fn insert_edges_batch(&self, edges: Vec<(Edge, String)>) -> Result<(), StorageError>;
    fn get_node(&self, id: &str) -> Result<Option<Node>, StorageError>;
    fn get_neighbors(&self, node_id: &str) -> Result<Vec<(Node, Edge)>, StorageError>;
    fn get_edges(&self, node_id: &str, direction: EdgeDirection) -> Result<Vec<Edge>, StorageError>;
    fn delete_by_file(&self, file_id: &str) -> Result<u64, StorageError>;
    fn count_nodes(&self) -> Result<u64, StorageError>;
    fn count_edges(&self) -> Result<u64, StorageError>;

    // New methods
    fn get_node_by_ref(&self, ref_id: &str) -> Result<Option<Node>, StorageError>;
    fn get_nodes_by_type(&self, node_type: NodeType) -> Result<Vec<Node>, StorageError>;
    fn get_edges_by_relation(&self, relation: RelationType) -> Result<Vec<Edge>, StorageError>;
    fn get_nodes_by_file(&self, file_id: &str) -> Result<Vec<Node>, StorageError>;
    fn get_edges_by_file(&self, file_id: &str) -> Result<Vec<Edge>, StorageError>;
    fn clear(&self) -> Result<(), StorageError>;
    fn initialize_schema(&self) -> Result<(), StorageError>;
}
```

### 3. Enhanced VectorStore Trait

```rust
// src/ocean_storage/vector_store.rs — AFTER adding initialize_schema
pub trait VectorStore: Send + Sync {
    fn insert(&self, record: &ChunkRecord) -> Result<(), StorageError>;
    fn vector_search(&self, query_vec: &[f32], top_k: usize, extra_where: Option<&str>) -> Result<Vec<serde_json::Value>, StorageError>;
    fn fts_search(&self, query: &str, top_k: usize, extra_where: Option<&str>) -> Result<Vec<serde_json::Value>, StorageError>;
    fn delete_by_file(&self, file_id: &str) -> Result<u64, StorageError>;
    fn count(&self) -> Result<u64, StorageError>;
    fn initialize_schema(&self, dimension: usize) -> Result<(), StorageError>;
}
```

### 4. SearchEngine (updated)

```rust
// src/ocean_vector/search.rs — AFTER refactoring
pub struct SearchEngine {
    store: Arc<dyn ocean_storage::VectorStore>,
}

impl SearchEngine {
    pub fn new(store: Arc<dyn ocean_storage::VectorStore>) -> Self;
    // Old constructor remains as a convenience:
    pub fn from_legacy(store: ocean_vector::store::VectorStore) -> Self {
        Self { store: Arc::new(LegacyVectorStoreAdapter(store)) }
    }

    pub fn search(&self, query: &str, embedder: &dyn Embedder, top_k: usize) -> Result<Vec<SearchResult>, SearchError>;
    pub fn hybrid_search(&self, query: &str, embedder: &dyn Embedder, top_k: usize) -> Result<Vec<SearchResult>, SearchError>;
    pub fn filtered_search(&self, query: &str, embedder: &dyn Embedder, filter: &SearchFilter, top_k: usize) -> Result<Vec<SearchResult>, SearchError>;
    pub fn hybrid_filtered_search(&self, query: &str, embedder: &dyn Embedder, filter: &SearchFilter, top_k: usize) -> Result<Vec<SearchResult>, SearchError>;
    pub fn expand_results(&self, results: &[SearchResult], expansion: &ExpansionEngine, depth: usize) -> Result<Vec<SearchResult>, SearchError>;
}
```

### 5. ContextWindowBuilder (updated)

```rust
// src/ocean_query/context.rs — AFTER refactoring
pub struct ContextWindowBuilder {
    store: Arc<dyn ocean_storage::ChunkStore>,
}

impl ContextWindowBuilder {
    pub fn new(store: Arc<dyn ocean_storage::ChunkStore>) -> Self;
    pub fn build(&self, anchor: &RankedChunk, context_chunks: usize) -> Result<ContextWindow, QueryError>;
}
```

### 6. ExpansionEngine (updated)

```rust
// src/ocean_graph/expansion.rs — AFTER refactoring
pub struct ExpansionEngine {
    store: Arc<dyn ocean_storage::GraphStore>,
}

impl ExpansionEngine {
    pub fn new(store: Arc<dyn ocean_storage::GraphStore>) -> Self;
    pub fn expand(&self, node_id: &str, depth: usize, direction: EdgeDirection) -> Result<Subgraph, GraphError>;
    pub fn expand_from_chunks(&self, chunk_ids: &[String], depth: usize) -> Result<Subgraph, GraphError>;
    pub fn find_path(&self, from_id: &str, to_id: &str, max_depth: usize) -> Result<Option<Vec<Edge>>, GraphError>;
    pub fn get_file_graph(&self, file_id: &str) -> Result<Subgraph, GraphError>;
}
```

### 7. QueryEngine (updated)

```rust
// src/ocean_query/engine.rs — AFTER refactoring
pub struct QueryEngine {
    store: Arc<dyn ocean_storage::VectorStore>,
    chunk_store: Arc<dyn ocean_storage::ChunkStore>,
    search: SearchEngine,
    graph: Option<ExpansionEngine>,
}

impl QueryEngine {
    pub fn from_storage(storage: Arc<dyn ocean_storage::Storage>) -> Result<Self, QueryError>;
    pub fn new(db_path: &str) -> Result<Self, QueryError>;            // legacy — unchanged
    pub fn new_with_paths(...) -> Result<Self, QueryError>;           // legacy — unchanged
    pub fn new_memory() -> Result<Self, QueryError>;                  // legacy — unchanged
    pub fn query(&self, query: Query, embedder: &dyn Embedder) -> Result<QueryResult, QueryError>;
    pub fn query_stream<'a>(...) -> Result<impl Iterator<...>, QueryError>;
}
```

### 8. IndexPipeline (updated)

```rust
// src/ocean_vector/pipeline.rs — AFTER refactoring
pub struct IndexPipeline {
    store: Arc<dyn ocean_storage::VectorStore>,
    chunk_store: Arc<dyn ocean_storage::ChunkStore>,
    state_store: Option<Arc<dyn ocean_storage::StateStore>>,
}

impl IndexPipeline {
    pub fn from_storage(storage: Arc<dyn ocean_storage::Storage>) -> Self;
    pub fn new(store: VectorStore) -> Self;    // legacy — unchanged
    pub fn index_chunks(&self, chunks: Vec<Chunk>, embedder: &dyn Embedder, config: &IndexConfig) -> Result<IndexReport, IndexError>;
}
```

---

## Data Models

### No new data models — types consolidate into ocean_storage

The canonical types are already defined in:
- `ocean_storage::graph_store::{Node, Edge, NodeType, RelationType, EdgeDirection}`
- `ocean_storage::chunk_store::ChunkRecord`
- `ocean_storage::file_store::FileMeta`
- `ocean_storage::state_store::{StateRecord, IndexStatus}`

These remain unchanged in shape. The only changes are:
- `ocean_graph::types` becomes a re-export module
- `ocean_storage::graph_store::GraphStore` gains new methods
- `ocean_storage::vector_store::VectorStore` gains `initialize_schema`

---

## Correctness Properties

### Property 1: Type Identity

*For any* `Node`, `Edge`, `NodeType`, `RelationType`, or `EdgeDirection` value, the type accessed via `ocean_graph::types::*` SHALL be the same type (`std::any::TypeId`) as the one accessed via `ocean_storage::graph_store::*`.

**Validates:** R1

### Property 2: Query Result Equivalence

*For any* query text and configuration, the `QueryResult` returned by `QueryEngine::from_storage()` SHALL be identical (same results, same scores, same metadata) to the result returned by `QueryEngine::new_with_paths()` when both are backed by the same data.

**Validates:** R6, R9

### Property 3: Index Result Equivalence

*For any* set of chunks and embedder, the `IndexReport` returned by `IndexPipeline::from_storage()` SHALL have the same total/embedded/skipped/failed counts as `IndexPipeline::new()` when both are backed by the same data.

**Validates:** R7, R9

### Property 4: Backwards Compatibility

*For any* test in the existing test suite that passes on the current codebase, the same test SHALL pass after the refactoring, without modifying the test.

**Validates:** R9

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| `Storage::vectors().initialize_schema()` called twice | Second call is a no-op (SurrealDB `IF NOT EXISTS`) |
| `GraphStore` trait method called on store missing the method | `StorageError::QueryFailed` with descriptive message |
| `From<StorageError>` conversion fails | Returns `QueryError` or `IndexError` wrapping the original error |
| Legacy constructor used after migration | Continues to work, wraps old store in adapter internally |

---

## Testing Strategy

### Unit Tests
- `ocean_storage::graph_store` — verify all new trait methods work (CRUD, by-type queries, clear)
- `ocean_storage::vector_store` — verify `initialize_schema` works and is idempotent
- `ocean_graph::types` — verify re-exports compile and match type identity

### Integration Tests
- `QueryEngine::from_storage()` vs `QueryEngine::new()` — same results on same data
- `IndexPipeline::from_storage()` vs `IndexPipeline::new()` — same index report
- `SurrealStorage` — verify all 5 sub-stores accessible and isolated

### Regression Tests
- Run full `cargo test` suite before merging — all tests must pass
- Run `cargo build --release` to verify release build
