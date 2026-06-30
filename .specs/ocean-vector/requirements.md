# Requirements Document: ocean-vector

## Introduction

ocean-vector is the semantic memory layer of the Ocean Document Runtime (DRT). It converts semantically chunked text into vector embeddings, stores them in SurrealDB alongside their source chunks and metadata, and provides similarity search with hybrid ranking. It is the first index layer built on top of the chunk output from ocean-chunk.

SurrealDB serves as the unified backend — a single multi-model engine that handles vector storage (HNSW indexes), document records, full-text search, graph relationships, and ACID transactions. This eliminates the need for separate vector, graph, and storage backends while keeping the embedder layer pluggable and model-agnostic.

**Scope:** This phase covers the embedder trait + backends, SurrealDB store layer, embedding pipeline, and search API. It does NOT cover graph indexing (ocean-graph) or the final query orchestrator (ocean-query).

---

## Glossary

- **Embedder**: A trait/component that converts text into a fixed-size vector (embedding) using a model.
- **Ollama embedder**: Local embedding backend that calls an Ollama server API (`http://localhost:11434/api/embed`).
- **OpenAI-compatible embedder**: Remote embedding backend that calls any OpenAI-compatible API (text-embedding-3-small/large, etc.).
- **Vector store**: The SurrealDB-backed storage layer that holds chunk records with vector fields and HNSW indexes.
- **KNN search**: k-nearest-neighbors vector search using SurrealDB's `<|K|>` operator with HNSW index.
- **Hybrid search**: Combined vector similarity + full-text search with reciprocal rank fusion (RRF) scoring.
- **HNSW**: Hierarchical Navigable Small World — the approximate nearest-neighbor index algorithm used by SurrealDB.
- **Chunk ID**: Stable UUIDv7 (or content-hash based) identifier for a chunk, matching the ID scheme from ocean-chunk.
- **Batch embedding**: Sending multiple texts to the embedder in one API call for throughput.
- **Document Runtime (DRT)**: The overall system vision — SurrealDB as a unified multi-model database powering vector, graph, full-text, and metadata operations in a single engine.

---

## Requirements

### Requirement 1: Embedder Trait

**User Story:** As a developer, I want a uniform interface for embedding models so that I can swap between local (Ollama) and remote (OpenAI) backends without changing calling code.

#### Acceptance Criteria

1. THE system SHALL define a public `Embedder` trait with at least `embed(text: &str) -> Vec<f32>` and `embed_batch(texts: &[&str]) -> Vec<Vec<f32>>` methods.
2. THE trait SHALL require `Send + Sync` so it can be shared across threads.
3. THE trait SHALL expose `dimension() -> usize` and `model_name() -> &str` for metadata.
4. THE `embed` method SHALL return `Result<Vec<f32>, EmbedderError>` (not panic).
5. THE `embed` method SHALL normalize the output vector to unit length.

---

### Requirement 2: Ollama Embedder Backend

**User Story:** As a user running local models, I want to use Ollama as the embedding provider so that I can keep all data and computation local.

#### Acceptance Criteria

1. THE system SHALL provide an `OllamaEmbedder` that implements the `Embedder` trait.
2. IT SHALL connect to `http://localhost:11434/api/embed` by default, with a configurable URL.
3. IT SHALL support configurable model name (default: `nomic-embed-text`).
4. IT SHALL handle connection errors gracefully with a descriptive `EmbedderError`.
5. IT SHALL support a configurable timeout for API calls (default: 30 seconds).

---

### Requirement 3: OpenAI-Compatible Embedder Backend

**User Story:** As a user with access to cloud models, I want to use OpenAI-compatible APIs so that I can leverage high-quality embedding models like `text-embedding-3-small`.

#### Acceptance Criteria

1. THE system SHALL provide an `OpenAIEmbedder` that implements the `Embedder` trait.
2. IT SHALL accept a base URL, API key, and model name via constructor.
3. IT SHALL default to `text-embedding-3-small` (dimension 1536) when model is not specified.
4. IT SHALL support dimension truncation for models that support it (e.g., `text-embedding-3-small` supports dimensions 512/1536).
5. IT SHALL handle HTTP errors, rate limits, and authentication failures with distinct error variants.

---

### Requirement 4: SurrealDB Vector Store

**User Story:** As the system, I need a persistent store that can hold chunk records with vector fields and perform fast ANN search, so that the DRT can retrieve semantically related chunks.

#### Acceptance Criteria

1. THE system SHALL use SurrealDB as the backing store for all vector + chunk data.
2. IT SHALL support SurrealDB in embedded mode (`Surreal::new::<Mem>(())` for tests, `Surreal::new::<RocksDb>("./path")` for persistence).
3. IT SHALL define a SurrealDB table `chunk` with fields: `id`, `file_id`, `content`, `heading`, `page`, `slide`, `sheet`, `block_type`, `embedding` (vector), `model`, `dimension`, `created_at`.
4. IT SHALL create an HNSW index on the `embedding` field with configurable distance metric (default: COSINE).
5. IT SHALL define a full-text search index on `content` using SurrealDB's `FULLTEXT` analyzer.
6. IT SHALL provide CRUD operations: `insert_chunk(chunk, embedding)`, `get_chunk(id)`, `delete_chunks(file_id)`, `update_embedding(id, embedding)`.

---

### Requirement 5: Embedding Pipeline

**User Story:** As the system, I want to take chunk output from ocean-chunk, embed each chunk, and store the result in SurrealDB in a single coordinated pipeline.

#### Acceptance Criteria

1. THE system SHALL provide `index_chunks(chunks: Vec<Chunk>, embedder: &dyn Embedder, store: &VectorStore, config: &IndexConfig) -> Result<IndexReport>`.
2. THE pipeline SHALL process chunks in configurable batches (default: 10 chunks per batch).
3. THE pipeline SHALL skip embedding for chunks that already have a stored embedding with the same model and content hash.
4. THE pipeline SHALL produce an `IndexReport` containing: `total`, `embedded`, `skipped`, `failed`, and `duration_ms`.
5. THE pipeline SHALL be idempotent — running it twice on the same input produces the same state.
6. IF the embedder returns an error for a chunk, THE pipeline SHALL record the failure, continue with remaining chunks, and return the partial result with failure details.

---

### Requirement 6: Vector Search API

**User Story:** As a user, I want to search my document collection by semantic meaning so that I can find relevant chunks even when exact keywords don't match.

#### Acceptance Criteria

1. THE system SHALL provide `search(query: &str, embedder: &dyn Embedder, store: &VectorStore, top_k: usize) -> Result<Vec<SearchResult>>`.
2. `SearchResult` SHALL contain: `chunk_id`, `file_id`, `content`, `heading`, `score`, `block_type`.
3. SEARCH SHALL embed the query, then run a SurrealDB KNN query using `<|K|>` operator on the HNSW index.
4. THE default `top_k` SHALL be 10 with a configurable range of 1–100.
5. RESULTS SHALL be returned in descending order of similarity score.
6. IF the embedder fails, IT SHALL return a descriptive error.

---

### Requirement 7: Hybrid (Vector + FTS) Search

**User Story:** As a user, I want to combine semantic similarity with keyword matching so that I get the best of both retrieval strategies.

#### Acceptance Criteria

1. THE system SHALL provide `hybrid_search(query: &str, embedder: &dyn Embedder, store: &VectorStore, top_k: usize) -> Result<Vec<SearchResult>>`.
2. HYBRID search SHALL run both vector KNN search AND full-text search (`WHERE content @@ $query`) in parallel.
3. THE results SHALL be merged using reciprocal rank fusion (RRF) with `k = 60`.
4. THE system MAY provide configurable weights for vector vs FTS contributions.
5. EACH result SHALL include its `score` (RRF fused score) and the individual vector/FTS scores.

---

### Requirement 8: Filtered Search

**User Story:** As a user, I want to constrain search results by file or metadata filters so that I can search within specific documents or sections.

#### Acceptance Criteria

1. THE system SHALL support adding SurrealQL `WHERE` conditions before the KNN operator.
2. FILTERS SHALL support: `file_id`, `heading` (exact or prefix), `block_type`, and `created_at` range.
3. FILTERS SHALL be composable — multiple filters ANDed together.
4. THE system SHALL provide a builder-pattern `SearchFilter` struct for constructing filters.

---

### Requirement 9: CLI Integration

**User Story:** As a user, I want to query my indexed documents from the command line so that I can use Ocean as a document search tool.

#### Acceptance Criteria

1. THE system SHALL add commands: `ocean index <dir>` (scan → parse → chunk → embed → store) and `ocean search <query>` (query the vector store).
2. `ocean index` SHALL accept `--model` (default: `nomic-embed-text`), `--url` (Ollama URL), and `--db-path` (SurrealDB storage path).
3. `ocean search` SHALL accept `--top-k`, `--hybrid` (enable FTS hybrid), `--file-id`, and `--model`.
4. OUTPUT for search SHALL show: rank, score, content (truncated), file path, heading.

---

### Requirement 10: Configuration and Error Handling

**User Story:** As a developer, I want clear error types and a consistent configuration system so that I can diagnose issues and tune the system.

#### Acceptance Criteria

1. THE system SHALL define a public `EmbedderError` enum with variants: `ConnectionFailed`, `AuthenticationFailed`, `RateLimited`, `ModelReturnedError(String)`, `Timeout`, `Unexpected(String)`.
2. THE system SHALL define a public `StoreError` enum with variants: `ConnectionFailed`, `QueryFailed(String)`, `RecordNotFound`, `SchemaError(String)`.
3. THE system SHALL define a public `IndexError` enum that combines `EmbedderError` and `StoreError` via `From` impls.
4. THE system SHALL provide an `IndexConfig` struct with fields: `batch_size` (default 10), `reindex` (bool, default false), `model` (String), `dimension` (usize), `ollama_url` (Option<String>), `openai_api_key` (Option<String>), `db_path` (String).
5. ALL public types SHALL implement `std::fmt::Debug` and `std::fmt::Display`.
