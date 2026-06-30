# Requirements Document: Ocean Caching Layer

## Introduction

Ocean currently has **zero caching**. Every query re-embeds the query text, re-searches the vector store, re-fetches chunks from storage, and re-computes graph expansions. For repeated or similar queries (common in interactive use), this is wasteful and slow. This phase introduces a 3-tier caching hierarchy — L1 (in-memory hot cache), L2 (disk cache for embeddings), L3 (database as persistent cache) — to reduce latency and embedding API costs.

The scope covers three cache domains: query results, query embeddings, and graph neighbors. Each has different cacheability characteristics.

---

## Glossary

- **L1 Cache**: In-memory `HashMap`/`HashMap` with LRU eviction. Fastest, smallest capacity. Stores hot query results and recently used embeddings.
- **L2 Cache**: On-disk persistent cache (using SurrealDB or flat files). Stores embedding vectors keyed by content hash + model name. Survives process restarts.
- **L3 Cache**: The existing SurrealDB storage layer (not modified by this phase). Serves as the slowest cache tier (always consistent, always available).
- **LRU**: Least-Recently-Used eviction policy. When the cache is full, the oldest accessed entry is dropped.
- **Query Result Cache**: Caches `(query_text, mode, top_k)` → `Vec<RankedChunk>`. Invalidated when new files are indexed.
- **Embedding Cache**: Caches `(content_hash, model_name)` → `Vec<f32>`. Never invalidated (deterministic for same model).
- **Graph Neighbor Cache**: Caches `(node_id, depth)` → `Vec<(Node, Edge)>`. Invalidated when graph structure changes.
- **TTL**: Time-To-Live. Maximum time an entry stays in cache before being considered stale.

---

## Requirements

### R1: Embedding Cache (L1 + L2)

**User Story:** As a query engine consumer, I want previously computed embeddings to be cached so that re-embedding the same or similar content is avoided.

#### Acceptance Criteria

1. AN `EmbeddingCache` struct SHALL exist with two tiers: `L1Memory` (in-memory LRU) and `L2Disk` (on-disk persistent).
2. THE cache key SHALL be `(content_hash: String, model_name: String)`.
3. THE cached value SHALL be `Vec<f32>` (the normalized embedding vector).
4. THE L1 cache SHALL have a configurable max capacity (default 1,000 entries) with LRU eviction.
5. THE L2 cache SHALL persist embeddings in a SurrealKv-backed store or flat file, indexed by `(content_hash, model_name)`.
6. `get(key) -> Option<Vec<f32>>` SHALL check L1 first, then L2. On L2 hit, THE entry SHALL be promoted to L1.
7. `set(key, value)` SHALL write to both L1 and L2.
8. THE existing `IndexPipeline::index_chunks()` SHALL check the embedding cache before calling `embedder.embed_batch()`.
9. THE existing `QueryEngine` SHALL check the embedding cache before calling `embedder.embed()` for query embedding.

---

### R2: Query Result Cache (L1 only)

**User Story:** As a CLI user, I want repeated queries to return instantly from cache instead of re-executing the full pipeline.

#### Acceptance Criteria

1. A `QueryCache` struct SHALL exist with an in-memory LRU cache.
2. THE cache key SHALL be `(query_text: String, mode: QueryMode, top_k: usize, filter_hash: u64)`.
3. THE cached value SHALL be `Vec<RankedChunk>` (the top results).
4. THE cache SHALL have a configurable max capacity (default 100 entries) with LRU eviction.
5. THE cache SHALL have a configurable TTL (default 60 seconds) — entries older than TTL are considered stale and re-fetched.
6. `invalidate()` SHALL clear all entries — called when new files are indexed.
7. `invalidate_file(file_id)` SHALL remove entries that reference the given file (if the cache tracks per-file dependencies).
8. Simple strategy: `invalidate()` clears the entire cache on any index change (coarse but correct).

---

### R3: Graph Neighbor Cache (L1 only)

**User Story:** As the query engine, I want graph neighbor lookups to be cached so that repeated expansions of the same node do not hit the storage layer.

#### Acceptance Criteria

1. A `GraphCache` struct SHALL exist with an in-memory LRU cache.
2. THE cache key SHALL be `node_id: String`.
3. THE cached value SHALL be `Vec<(Node, Edge)>` (neighbors + connecting edges).
4. THE cache SHALL have a configurable max capacity (default 5,000 entries) with LRU eviction.
5. THE `ExpansionEngine::expand()` SHALL check `GraphCache` before querying the store.
6. ON store write (graph update), `invalidate_node(node_id)` SHALL remove cached neighbors for affected nodes.

---

### R4: Cache Configuration

**User Story:** As a power user, I want to configure cache sizes, TTL, and persistence via CLI flags and config file.

#### Acceptance Criteria

1. A `CacheConfig` struct SHALL exist with fields: `embedding_cache_size: usize` (default 1000), `query_cache_size: usize` (default 100), `graph_cache_size: usize` (default 5000), `query_ttl_secs: u64` (default 60), `embedding_cache_path: Option<String>` (default: `~/.ocean/cache/embeddings/`).
2. THE `ocean query` command SHALL accept `--no-cache` flag to bypass all caches for a single query.
3. THE config file (`~/.ocean/config.json`) SHALL support a `cache` section.
4. Resolution order: CLI flag > config file > defaults.

---

### R5: Invalidation Semantics

**User Story:** As a user, I want cached data to be automatically invalidated when the underlying data changes so that I never see stale results.

#### Acceptance Criteria

1. WHEN any file is indexed or reindexed, the query cache SHALL be fully invalidated.
2. WHEN a file is deleted from the index, the query cache SHALL be fully invalidated.
3. THE embedding cache SHALL NOT be invalidated on index changes (embeddings are deterministic for same content + model).
4. THE graph cache SHALL be invalidated for affected nodes when a file's graph is rebuilt.
5. THE `IndexOrchestrator` SHALL call `Cache::invalidate_all()` at the end of `run()`.

---

### R6: Error Handling & Degradation

**User Story:** As a system operator, I want cache failures to be non-fatal — if the cache is unavailable, the system SHALL fall through to the uncached path.

#### Acceptance Criteria

1. ALL cache operations (`get`, `set`, `invalidate`) SHALL be infallible — errors are logged but never propagated.
2. IF the L2 disk cache fails to open or write, THE system SHALL log a warning and continue without the L2 tier.
3. IF the L1 cache is full, LRU eviction SHALL drop the oldest entry silently.
4. THE `--no-cache` flag SHALL bypass all cache tiers for the current operation.

---

### R7: Performance Targets

**User Story:** As a user, I want measurable latency improvements from caching.

#### Acceptance Criteria

1. Cached query results SHALL return in <5ms (vs. 50–500ms uncached).
2. Cached embedding lookup SHALL be <1ms (L1) or <10ms (L2) vs. 100ms–5s for API embedding call.
3. Cached graph neighbor lookup SHALL be <1ms (L1) vs. 5–50ms for storage query.
