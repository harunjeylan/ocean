# Design Document: Ocean Caching Layer

## Overview

Ocean currently has no caching. Every query and embedding operation hits the storage layer or embedder API fresh. This design introduces a 3-tier cache hierarchy (L1 in-memory LRU, L2 on-disk persistent), covering three domains: query results, query/text embeddings, and graph neighbors. The design emphasizes correctness (cache invalidation on index changes), graceful degradation (cache failures are non-fatal), and minimal dependencies (LRU via existing `std::collections::HashMap` + a simple linked-map, or the `lru` crate if already present).

### Key Design Decisions

1. **Separate caches per domain** — Query results, embeddings, and graph neighbors have different invalidation semantics and access patterns. A single unified cache would couple unrelated concerns.
2. **L1 LRU via `lru` crate** — Simple, well-tested. If the crate isn't a dependency, a minimal `LinkedHashMap`-based LRU is ~100 lines.
3. **L2 embedding cache on disk** — SurrealKv for consistency with existing storage, but a simple `bincode`-serialized file per key would also work. Use SurrealKv to reuse the existing runtime.
4. **Cache invalidation on index change** — Coarse invalidation (clear all) is correct and simple. File-level fine-grained invalidation is a future optimization.
5. **Non-fatal cache errors** — Every cache operation is wrapped in `catch_unwind` or returns `Option`/`Result` that is silently degraded.

---

## Architecture

```text
Query Engine / Index Pipeline
         │
         ├── EmbeddingCache (L1 + L2)
         │     key: (content_hash, model) → Vec<f32>
         │
         ├── QueryCache (L1 only)
         │     key: (query, mode, top_k, filter) → Vec<RankedChunk>
         │
         └── GraphCache (L1 only)
               key: node_id → Vec<(Node, Edge)>

Cache check order:
  1. L1 (in-memory LRU) — fastest, ~1μs
  2. L2 (disk) — ~1ms, only for embeddings
  3. Fall through to computation
```

---

## Components and Interfaces

### 1. LruCache (generic LRU container)

```rust
/// A simple generic LRU cache with fixed capacity.
/// Not thread-safe. Wrap in `Mutex` for shared access.
pub struct LruCache<K, V> {
    map: LinkedHashMap<K, V>,
    capacity: usize,
}

impl<K: Hash + Eq + Clone, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self;
    pub fn get(&mut self, key: &K) -> Option<&V>;
    pub fn put(&mut self, key: K, value: V);
    pub fn remove(&mut self, key: &K) -> Option<V>;
    pub fn clear(&mut self);
    pub fn len(&self) -> usize;
    pub fn is_full(&self) -> bool;
}
```

### 2. EmbeddingCache

```rust
pub struct EmbeddingCache {
    l1: Mutex<LruCache<(String, String), Vec<f32>>>, // key: (content_hash, model)
    l2: Option<SurrealEmbeddingStore>,                // on-disk persistent
}

impl EmbeddingCache {
    pub fn new(l1_capacity: usize, l2_path: Option<&str>) -> Self;

    /// Get from L1, then L2. Promotes L2 → L1 on hit.
    pub fn get(&self, content_hash: &str, model: &str) -> Option<Vec<f32>>;

    /// Set in both L1 and L2.
    pub fn set(&self, content_hash: &str, model: &str, embedding: Vec<f32>);

    /// Batch get — returns a map of found keys.
    pub fn get_batch(
        &self,
        keys: &[(&str, &str)],
    ) -> HashMap<(String, String), Vec<f32>>;

    /// Clear L1 only (L2 is persistent).
    pub fn clear_l1(&self);
}
```

### 3. QueryCache

```rust
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct QueryCacheKey {
    pub query_text: String,
    pub mode: QueryMode,
    pub top_k: usize,
    pub filter_hash: u64, // hash of active SearchFilter
}

pub struct QueryCache {
    l1: Mutex<LruCache<QueryCacheKey, Vec<RankedChunk>>>,
    ttl: Duration,
    last_invalidated: AtomicInstant, // or std::sync::atomic::AtomicU64
}

impl QueryCache {
    pub fn new(capacity: usize, ttl_secs: u64) -> Self;

    /// Get cached results if fresh (within TTL and not invalidated).
    pub fn get(&self, key: &QueryCacheKey) -> Option<Vec<RankedChunk>>;

    /// Set cached results.
    pub fn set(&self, key: QueryCacheKey, results: Vec<RankedChunk>);

    /// Invalidate all entries.
    pub fn invalidate(&self);

    /// Returns true if the cache was invalidated since `since`.
    pub fn was_invalidated_since(&self, since: Instant) -> bool;
}
```

### 4. GraphCache

```rust
pub struct GraphCache {
    l1: Mutex<LruCache<String, Vec<(Node, Edge)>>>, // key: node_id
}

impl GraphCache {
    pub fn new(capacity: usize) -> Self;

    /// Get cached neighbors for a node.
    pub fn get(&self, node_id: &str) -> Option<Vec<(Node, Edge)>>;

    /// Set cached neighbors.
    pub fn set(&self, node_id: String, neighbors: Vec<(Node, Edge)>);

    /// Remove cached entry for a specific node.
    pub fn invalidate_node(&self, node_id: &str);

    /// Clear entire cache.
    pub fn invalidate_all(&self);
}
```

### 5. CacheConfig

```rust
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub embedding_cache_size: usize,       // default 1000
    pub query_cache_size: usize,           // default 100
    pub graph_cache_size: usize,           // default 5000
    pub query_ttl_secs: u64,               // default 60
    pub embedding_cache_path: Option<String>, // default: ~/.ocean/cache/embeddings/
    pub enabled: bool,                     // default true; --no-cache sets false
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            embedding_cache_size: 1000,
            query_cache_size: 100,
            graph_cache_size: 5000,
            query_ttl_secs: 60,
            embedding_cache_path: None,
            enabled: true,
        }
    }
}
```

### 6. CacheManager (unified facade)

```rust
pub struct CacheManager {
    pub embeddings: EmbeddingCache,
    pub queries: QueryCache,
    pub graph: GraphCache,
    pub config: CacheConfig,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self;

    /// Invalidate all caches (called after index changes).
    pub fn invalidate_all(&self);

    /// Invalidate graph cache for a specific file's nodes.
    pub fn invalidate_file_graph(&self, file_id: &str);
}
```

---

## Integration Points

### EmbeddingCache → IndexPipeline

```rust
// In IndexPipeline::index_chunks():
let cache = EmbeddingCache::new(config.embedding_cache_size, l2_path);

for batch in chunks.chunks(config.batch_size) {
    let to_embed: Vec<&Chunk> = batch.iter()
        .filter(|c| cache.get(&content_hash, &model).is_none())
        .collect();

    let embeddings = embedder.embed_batch(&to_embed_texts)?;

    for (chunk, embedding) in to_embed.iter().zip(embeddings) {
        cache.set(&content_hash, &model, embedding);
    }
}
```

### EmbeddingCache → QueryEngine

```rust
// In QueryEngine::execute():
let cache_key = (query_text_hash, model);
if let Some(cached_vec) = embedding_cache.get(&query_text_hash, &model) {
    query_vec = cached_vec;
} else {
    query_vec = embedder.embed(query_text);
    embedding_cache.set(&query_text_hash, &model, query_vec.clone());
}
```

### QueryCache → QueryEngine

```rust
// In QueryEngine::execute():
let key = QueryCacheKey {
    query_text: q.text.clone(),
    mode: q.mode,
    top_k: q.top_k,
    filter_hash: q.filter.map(|f| hash(&f)).unwrap_or(0),
};

if let Some(cached) = query_cache.get(&key) {
    return Ok(QueryResult { chunks: cached, ... });
}

// ... full query execution ...
query_cache.set(key, ranked_chunks.clone());
```

### GraphCache → ExpansionEngine

```rust
// In ExpansionEngine::expand():
if let Some(neighbors) = graph_cache.get(node_id) {
    return neighbors;
}
let neighbors = store.get_neighbors(node_id)?;
graph_cache.set(node_id.to_string(), neighbors.clone());
```

---

## Data Models

### L2 Embedding Store Schema

```sql
-- SurrealDB table for persistent embedding cache
CREATE TABLE embedding_cache (
    content_hash STRING,
    model STRING,
    embedding ARRAY,
    created_at INT
);
CREATE UNIQUE INDEX idx_embedding_key ON embedding_cache (content_hash, model);
```

---

## Correctness Properties

### Property 1: Cache Transparency

*For any* query with `--no-cache` flag, the result SHALL be identical to the result produced without any cache layer (caches are bypassed, not used).

**Validates:** R6

### Property 2: Staleness Bound

*For any* query whose result was cached before the most recent index change, the cache SHALL return `None` (the cache is invalidated on index change).

**Validates:** R5

### Property 3: Embedding Determinism

*For any* `(content_hash, model)` pair, the cached embedding SHALL be identical to a freshly computed embedding from the same model (embeddings are deterministic).

**Validates:** R1

### Property 4: Graceful Degradation

*For any* cache failure (disk full, corrupt file, OOM), the system SHALL continue to operate correctly by falling through to the uncached path.

**Validates:** R6

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| L2 disk cache directory creation fails | Log warning, continue without L2 |
| L2 deserialization error (corrupt cache) | Log warning, remove corrupt entry, return `None` |
| L1 LRU at capacity | Evict LRU entry silently |
| `--no-cache` flag set | All `get()` calls return `None`, all `set()` calls are no-ops |
| Embedding cache `set()` fails | Log warning, continue (embedding already computed) |
| Query cache `set()` fails | Log warning, return result uncached |

---

## Testing Strategy

### Unit Tests

- `LruCache` evicts at capacity, preserves most-recently-used.
- `EmbeddingCache` L1 → L2 promotion works correctly.
- `QueryCache` TTL expiry returns `None`.
- `QueryCache::invalidate()` clears all entries.
- `GraphCache` stores and retrieves neighbors.

### Integration Tests

- `IndexPipeline` with cache skips embedding for previously indexed chunks.
- `QueryEngine` with cache returns cached results faster than uncached.
- Cache invalidation after index change: new query returns fresh results.
- `--no-cache` produces same results as with cache.

### Property-Based Tests

- Property 4 (graceful degradation): Simulate L2 disk failure, verify queries still succeed.
- Property 1 (cache transparency): Run query with and without `--no-cache`, verify identical results.

---

## Performance Considerations

- **L1 target latency**: `get()` < 1μs (HashMap lookup). `set()` < 1μs.
- **L2 target latency**: `get()` < 5ms (SurrealKv read). `set()` < 10ms.
- **Memory budget**: 1000 embeddings at 768 dimensions × 4 bytes = ~3MB L1. 5000 graph entries at ~200 bytes = ~1MB. 100 query results at ~10KB = ~1MB. Total < 10MB RAM.
- **Cache warming**: Not implemented in MVP. Future: preload common embeddings on startup.
