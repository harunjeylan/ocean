# Implementation Plan: ocean-vector

## Overview

Implement the semantic memory layer: embedder trait + Ollama/OpenAI backends, SurrealDB vector store, indexing pipeline, and search API. Integrate with the existing CLI via `index` and `search` commands. All new code lives under `src/ocean_vector/`.

## Dependency: Cargo.toml additions

```toml
reqwest = { version = "0.12", features = ["json"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
```

`serde`, `surrealdb`, and `tokio` are already present after the SeaORM/SQLite migration (Task 0).

## Tasks

### Pre-Step: Migrate PathResolver to SurrealDB

- [x] 0. Replace SeaORM + SQLite PathResolver with SurrealDB-backed version
  - Remove `sea-orm` from Cargo.toml
  - Add `surrealdb` (features: `kv-mem`, `kv-surrealkv`) and `serde` (features: `derive`)
  - Delete `src/ocean_fs/path_entities.rs` (SeaORM entity)
  - Rewrite `src/ocean_fs/path_resolver.rs` to use SurrealDB embedded engine
  - Add `Serialize`/`Deserialize` derives to `PathMove` in `types.rs`
  - Remove `pub mod path_entities;` from `ocean_fs/mod.rs`
  - Update `path_resolver_test.rs` for RocksDB directory path convention
  - Update `AGENTS.md` persistence line
  - All 124 tests pass

### Sub-Phase A: Foundation — Embedder Trait + Backends

- [ ] 1. Create module structure (`src/ocean_vector/`)
  - `mod.rs` — `pub mod embedder;` + `pub mod store;` + `pub mod pipeline;` + `pub mod search;` + `pub use` re-exports
  - `embedder.rs` — `Embedder` trait + error types
  - `store.rs` — `VectorStore` + `ChunkRecord` + SurrealDB schema
  - `pipeline.rs` — `IndexPipeline` + `IndexConfig` + `IndexReport`
  - `search.rs` — `SearchEngine` + `SearchResult` + `SearchFilter` + `SearchError`

  _Requirements: R1, R4, R5, R6_

- [ ] 2. Implement `Embedder` trait in `embedder.rs`
  - `embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError>`
  - `embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedderError>`
  - `dimension(&self) -> usize`
  - `model_name(&self) -> &str`
  - Auto-normalize output vectors to unit length
  - `EmbedderError` enum with all variants from R10

  _Requirements: R1, R10_

  - [ ] 2.1 Write unit tests: trait contract, vector normalization, error types impl Display/Debug

- [ ] 3. Implement `OllamaEmbedder`
  - Constructor: `new(model, url)` with configurable timeout
  - `POST /api/embed` with batch input
  - Parse response: extract embeddings, validate dimension
  - Map HTTP errors to `EmbedderError` variants

  _Requirements: R2_

  - [ ] 3.1 Write unit tests with mock HTTP server (wiremock or custom)
  - [ ] 3.2 Test: connection error, invalid response, dimension mismatch

- [ ] 4. Implement `OpenAIEmbedder`
  - Constructor: `new(model, base_url, api_key)` with dimension truncation option
  - `POST /v1/embeddings` with batch input
  - Parse response: extract embeddings from `data[].embedding`
  - Support dimension parameter for models that allow it
  - Map HTTP/auth/rate-limit errors to `EmbedderError` variants

  _Requirements: R3_

  - [ ] 4.1 Write unit tests with mock HTTP server
  - [ ] 4.2 Test: auth failure, rate limit, dimension truncation

### Sub-Phase B: SurrealDB Vector Store

- [ ] 5. Set up SurrealDB dependency and feature flags
  - Add `surrealdb` with `kv-mem` (for tests) and `kv-rocksdb` (for persistence)
  - Add `tokio` with `rt-multi-thread` for async runtime
  - Add `chrono` for datetime fields

  _Requirements: R4_

- [ ] 6. Implement `VectorStore` in `store.rs`
  - `new_memory()` — in-memory SurrealDB for tests
  - `new_persistent(path)` — RocksDB-backed SurrealDB
  - `initialize_schema()` — run SurrealQL to define table, fields, indexes
  - `insert_chunk(chunk, embedding)` — single record insert
  - `insert_chunks_batch(records)` — batched insert
  - `get_chunk(chunk_id)` — fetch by record ID
  - `delete_chunks_by_file(file_id)` — cascade delete all chunks for a file
  - `count()` — total chunk count

  _Requirements: R4_

  - [ ] 6.1 SurrealDB schema: `chunk` table with SCHEMAFULL, all fields, HNSW index on embedding, FTS index on content
  - [ ] 6.2 Write tests: insert + get roundtrip, batch insert, delete by file, count, schema idempotency
  - [ ] 6.3 Test: HNSW index creation, FTS index creation

- [ ] 7. Implement `ChunkRecord` data model
  - Convert from `ocean_chunk::Chunk` (map fields)
  - Implement `Serialize`/`Deserialize` for SurrealDB
  - Compute `content_hash` as SHA-256 of chunk content

  _Requirements: R4_

### Sub-Phase C: Indexing Pipeline

- [ ] 8. Implement `IndexPipeline` in `pipeline.rs`
  - `index_chunks(chunks, embedder, config)` — main entry point
  - Batch processing: group chunks into batches of `config.batch_size`
  - Change detection: skip chunks whose `content_hash` matches existing record for the same model
  - Error resilience: continue on per-chunk embed failures, record in report
  - Build `IndexReport` with totals + duration

  _Requirements: R5, R10_

  - [ ] 8.1 Write tests: pipeline with in-memory SurrealDB + mock embedder
  - [ ] 8.2 Test: idempotency (index twice, same state), skip logic
  - [ ] 8.3 Test: partial batch failure, report accuracy

### Sub-D Phase: Search API

- [ ] 9. Implement `SearchEngine` in `search.rs`
  - `search(query, embedder, top_k)` — embed query, SurrealQL KNN query, return results
  - `hybrid_search(query, embedder, top_k)` — parallel vector + FTS, RRF fusion in Rust
  - `filtered_search(query, embedder, filter, top_k)` — KNN with WHERE conditions prepended
  - Build `SearchResult` from SurrealDB response rows

  _Requirements: R6, R7, R8_

  - [ ] 9.1 SurrealQL: KNN with `<|K|>` operator on HNSW index
  - [ ] 9.2 SurrealQL: FTS with `@@` operator on content field
  - [ ] 9.3 RRF fusion: `score = sum(1 / (k + rank))` for each result in merged set
  - [ ] 9.4 Write tests: search round-trip, filtered search, hybrid search superset property

- [ ] 10. Implement `SearchFilter` builder
  - Builder pattern methods: `with_file_id()`, `with_heading()`, `with_block_type()`, `with_created_range()`
  - Render to SurrealQL WHERE clause fragment
  - Empty filter renders as no condition

  _Requirements: R8_

### Sub-Phase E: CLI Integration

- [ ] 11. Add `ocean index` CLI command
  - Register in `ocean_cli::args.rs`: `IndexArgs` struct
  - Handler in `ocean_cli::run.rs`: `cmd_index`
  - Pipeline: scan dir → parse files → chunk → embed → store
  - Flags: `--model`, `--ollama-url`, `--db-path`, `--batch-size`

  _Requirements: R9_

- [ ] 12. Add `ocean search` CLI command
  - Register in `ocean_cli::args.rs`: `SearchArgs` struct
  - Handler in `ocean_cli::run.rs`: `cmd_search`
  - Display: rank, score, content (truncated), heading, file_id
  - Flags: `--top-k`, `--hybrid`, `--file-id`, `--model`, `--ollama-url`

  _Requirements: R9_

- [ ] 13. Update `cli-docs.md` with `index` and `search` commands

### Sub-Phase F: Integration + Real File Tests

- [ ] 14. Integration tests: full pipeline with in-memory SurrealDB
  - Scan a test directory → parse (reads fixtures from `tests/test-cwd/`) → chunk → embed (use mock embedder that returns fixed-size unit vectors) → store → search
  - Verify search returns expected chunks for known queries

  _Requirements: R5, R6, R9_

- [ ] 15. Property-based tests with `proptest`
  - Embedder contract: for any string, embed returns correct dimension, all finite
  - Store roundtrip: for any valid ChunkRecord, insert + get returns equivalent record
  - Filter: for any SearchFilter, all results satisfy filter predicates

  _Requirements: R1, R4, R8_

- [ ] 16. Add `pub mod ocean_vector;` to `src/lib.rs`
- [ ] 17. Update `AGENTS.md` with ocean-vector module overview

## Notes

- SurrealDB can be embedded directly (`Surreal::new::<Mem>(())`), no server process needed.
- Use `tokio` runtime inside `VectorStore` (sync-to-async bridge, same pattern as `PathResolver`).
- HNSW index dimension in `DEFINE INDEX` must match the actual embedding dimension. Use a default of 768 (nomic-embed-text) but make it configurable.
- The FTS analyzer `ocean_fts` handles lowercase + ASCII folding for case-insensitive search.
- For testing without a real Ollama/OpenAI server, use a `MockEmbedder` that returns fixed unit vectors of the configured dimension.
- RRF k-value of 60 is the SurrealDB `search::rrf()` default and well-tested in information retrieval literature.

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| SurrealDB embedded mode has unexpected behavior on Windows | Start with `kv-mem` for development; test `kv-rocksdb` early in a CI-like local check |
| HNSW index dimension mismatch at runtime | Validate dimension when inserting; panic/error early with clear message |
| reqwest TLS issues on Windows | Use `rustls-tls` feature instead of `native-tls` |
| Ollama API format changes | Pin to known working API version; document compatibility |
| Large batch embedding OOM | Cap batch size per IndexConfig (default 10); document memory implications |
