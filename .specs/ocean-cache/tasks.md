# Implementation Plan: Ocean Caching Layer

## Overview

Introduce a 3-domain, 2-tier caching system (L1 in-memory LRU, L2 on-disk for embeddings) covering query results, text embeddings, and graph neighbors. Work is structured as 6 tasks, each 2–4 hours, with testing embedded in each task.

## Pre-requisites

- `ocean_vector::embedder::Embedder` and `ocean_vector::pipeline::IndexPipeline` are stable.
- `ocean_query::engine::QueryEngine` and `ocean_graph::expansion::ExpansionEngine` are stable.
- `lru` crate is available (or add as dependency — lightweight, no transitive deps).

## Tasks

- [ ] 1. **Create `LruCache` generic container + `CacheConfig`**
  - Create `src/ocean_cache/` directory with `mod.rs`.
  - Register `pub mod ocean_cache;` in `src/lib.rs`.
  - Register test module in `src/tests.rs`.
  - Implement `LruCache<K, V>` using `LinkedHashMap` (from `std::collections` via `linked_hash_map` crate, or use the `lru` crate).
  - Methods: `new(capacity)`, `get()`, `put()`, `remove()`, `clear()`, `len()`, `is_full()`.
  - Define `CacheConfig` struct with all tuning fields + sensible defaults.
  - Write unit tests for LRU eviction, capacity enforcement, clear.
  - _Requirements: R1, R4_

  - [ ] 1.1 Create `ocean_cache` module skeleton
  - [ ] 1.2 Implement `LruCache<K, V>` generic container
  - [ ] 1.3 Define `CacheConfig` struct
  - [ ] 1.4 Unit tests for LRU eviction
  - [ ] 1.5 Verify `cargo build` succeeds

- [ ] 2. **Implement `EmbeddingCache` (L1 + L2)**
  - Create `src/ocean_cache/embedding_cache.rs` with `EmbeddingCache` struct.
  - L1: `Mutex<LruCache<(String, String), Vec<f32>>>` keyed by (content_hash, model).
  - L2 (optional): `Option<SurrealEmbeddingStore>` backed by SurrealKv at `{cache_path}/embeddings.db`.
  - `get()`: check L1 → L2 → promote L2→L1 → return `Option`.
  - `set()`: write to L1 + L2.
  - `get_batch()`: batch lookup for `IndexPipeline`.
  - `clear_l1()`: drop L1 entries only.
  - All errors are logged and swallowed (graceful degradation).
  - Write unit tests: L1 hit, L2 hit with promotion, miss, batch get.
  - _Requirements: R1, R6_

  - [ ] 2.1 `EmbeddingCache` struct + constructor
  - [ ] 2.2 L1 get/set with LRU
  - [ ] 2.3 L2 SurrealKv backend (get/set)
  - [ ] 2.4 L2→L1 promotion on hit
  - [ ] 2.5 `get_batch()` for pipeline integration
  - [ ] 2.6 Error logging (non-fatal)
  - [ ] 2.7 Unit tests

- [ ] 3. **Implement `QueryCache` (L1 only)**
  - Create `src/ocean_cache/query_cache.rs` with `QueryCache` + `QueryCacheKey`.
  - `QueryCacheKey` includes `query_text`, `mode`, `top_k`, `filter_hash`.
  - L1: `Mutex<LruCache<QueryCacheKey, Vec<RankedChunk>>>`.
  - TTL: store `Instant::now()` on set; on get, check elapsed < TTL.
  - `invalidate()`: clear all entries + record invalidation timestamp.
  - `was_invalidated_since()`: quick check without locking.
  - Write unit tests: hit within TTL, miss after TTL, invalidation clears.
  - _Requirements: R2, R5, R6_

  - [ ] 3.1 `QueryCacheKey` struct with `Hash` + `Eq`
  - [ ] 3.2 `QueryCache` struct + constructor
  - [ ] 3.3 TTL-aware get/set
  - [ ] 3.4 `invalidate()` + invalidation timestamp
  - [ ] 3.5 Unit tests

- [ ] 4. **Implement `GraphCache` (L1 only)**
  - Create `src/ocean_cache/graph_cache.rs` with `GraphCache` struct.
  - L1: `Mutex<LruCache<String, Vec<(Node, Edge)>>>` keyed by node_id.
  - Methods: `get()`, `set()`, `invalidate_node()`, `invalidate_all()`.
  - Write unit tests: store/retrieve, invalidation of single node.
  - _Requirements: R3, R5, R6_

  - [ ] 4.1 `GraphCache` struct + constructor
  - [ ] 4.2 get/set for node neighbors
  - [ ] 4.3 `invalidate_node()` + `invalidate_all()`
  - [ ] 4.4 Unit tests

- [ ] 5. **Create `CacheManager` facade + integrate into system**
  - Create `src/ocean_cache/cache_manager.rs` with `CacheManager` struct holding all three caches + config.
  - Implement `invalidate_all()` that clears all caches.
  - Integrate `EmbeddingCache` into `IndexPipeline::index_chunks()` — check cache before embedding.
  - Integrate `EmbeddingCache` into `QueryEngine::execute()` — cache query embedding.
  - Integrate `QueryCache` into `QueryEngine::execute()` — cache full query results.
  - Integrate `GraphCache` into `ExpansionEngine::expand()` — cache neighbor lookups.
  - Add `invalidate_all()` call at end of `IndexOrchestrator::run()`.
  - Add `--no-cache` flag support in `QueryEngine` (bypass all caches).
  - Write integration tests: verify caches are populated and used.
  - _Requirements: all_

  - [ ] 5.1 `CacheManager` struct
  - [ ] 5.2 Integrate `EmbeddingCache` into `IndexPipeline`
  - [ ] 5.3 Integrate `EmbeddingCache` into `QueryEngine`
  - [ ] 5.4 Integrate `QueryCache` into `QueryEngine`
  - [ ] 5.5 Integrate `GraphCache` into `ExpansionEngine`
  - [ ] 5.6 `invalidate_all()` orchestration in `IndexOrchestrator`
  - [ ] 5.7 `--no-cache` bypass
  - [ ] 5.8 Integration tests

- [ ] 6. **CLI + Config integration**
  - Add `cache` section to `OceanConfig` serde struct.
  - Add `--no-cache` flag to `ocean query` command.
  - Add `--cache-embedding-size`, `--cache-query-size`, `--cache-graph-size`, `--cache-query-ttl` flags to `ocean query` and `ocean index`.
  - Resolution order: CLI flag > config > defaults.
  - Document all new flags in `--help`.
  - _Requirements: R4_

  - [ ] 6.1 `cache` section in `OceanConfig`
  - [ ] 6.2 CLI flags in `IndexArgs` and `QueryArgs`
  - [ ] 6.3 Resolution logic in `cmd_index` and `cmd_query`
  - [ ] 6.4 Update `--help` strings

- [ ] **Validation & Cleanup**
  - Run full test suite: `cargo test` — all tests must pass.
  - Verify `cargo build --release` succeeds.
  - Manual smoke test: `ocean index ./docs` then `ocean query "test"` twice — second call should be faster.
  - _Requirements: R7_

## Notes

- **Task order**: 1→(2,3,4 in parallel)→5→6.
- **Dependencies**: Tasks 2, 3, 4 depend on 1. Task 5 depends on 2+3+4.
- **`lru` crate**: Add `lru = "0.12"` to Cargo.toml under `[dependencies]`. It's lightweight, pure Rust, no unsafe code. Alternative: use `hashlink` crate for `LinkedHashMap`.
- **No new dependencies for SurrealKv L2**: Reuse the existing `surrealdb` dependency and the `tokio::runtime::Runtime` pattern from `ocean_storage`.
- **Graceful degradation critical**: Every cache `get()` returns `Option`, every `set()` is fire-and-forget. Cache failures must never propagate to the caller.
