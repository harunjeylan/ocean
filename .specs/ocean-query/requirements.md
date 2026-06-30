# Requirements Document: ocean-query

## Introduction

ocean-query is the top-level query orchestrator of the Ocean DRT. It unifies vector search (semantic recall), graph expansion (structural context), full-text search (keyword precision), and metadata filtering into a single, coherent query API. It produces structured, traceable results with context windows that link back to the original source files.

Currently, vector search (KNN), hybrid (vector + FTS), filtering, and graph expansion are implemented inside `ocean_vector::search::SearchEngine`. ocean-query lifts these capabilities into their own module, provides a unified `QueryEngine` with proper query/result types, context window construction, ranking improvements, and a public API that serves as the single entry point for all external consumers (CLI, MCP, API server, AI agents).

**Scope:** This phase covers the standalone `ocean_query` module, query types, the orchestrator engine, context window builder, CLI integration (replacing the current `vector-search` command with a unified `query` command), and the public API. It does NOT cover AI agent integration, MCP tools, or web UI.

---

## Glossary

- **QueryEngine**: The orchestrator that accepts a `Query`, executes against sub-engines (vector, graph, FTS), fuses results, and returns a `QueryResult`.
- **Query**: A structured request with text, optional filters, mode (auto/vector/hybrid/expand), and parameters (top_k, expand_depth, etc.).
- **QueryResult**: A structured response with ranked results, context windows, execution metadata, and source traceability.
- **Context window**: A contiguous block of chunks surrounding a matched chunk, providing surrounding context for LLM consumption.
- **Ranking fusion**: The algorithm that merges multiple ranked lists (vector, FTS, graph) into a single ranked output.
- **RRF**: Reciprocal Rank Fusion — the fusion algorithm used for hybrid search.
- **Sub-query**: An individual query executed against a single index type (vector only, FTS only, graph only).
- **Execution plan**: The ordered sequence of sub-queries the QueryEngine will execute, determined by the Query mode and parameters.
- **Anchor chunk**: The highest-scoring chunk in a result group, used as the center of a context window.

---

## Requirements

### Requirement 1: Unified QueryEngine

**User Story:** As a developer, I want a single entry point for all queries so that I don't need to know whether to call vector search, graph expansion, or FTS separately.

#### Acceptance Criteria

1. THE system SHALL provide a `QueryEngine` struct that wraps `VectorStore`, `SearchEngine`, `GraphStore`, and `ExpansionEngine`.
2. THE `QueryEngine` SHALL accept a `Query` struct and return a `QueryResult`.
3. THE `QueryEngine` SHALL support `auto` mode (determines best strategy), `vector` mode (KNN only), `hybrid` mode (vector + FTS + RRF), and `expand` mode (hybrid + graph context expansion).
4. THE `QueryEngine` SHALL be constructable with `new(store_path: &str)` for production and `new_memory()` for testing.
5. THE `QueryEngine` SHALL implement the public API surface defined in R8.

---

### Requirement 2: Query and QueryResult Types

**User Story:** As a caller, I want well-typed query inputs and structured query outputs so that I can integrate with the system programmatically.

#### Acceptance Criteria

1. THE system SHALL define a `Query` struct with fields: `text: String`, `mode: QueryMode` (auto/vector/hybrid/expand), `top_k: usize`, `expand_depth: usize`, `filter: Option<SearchFilter>`, `include_context: bool`, `context_chunks: usize`.
2. THE `Query` struct SHALL implement `Default` with sensible defaults (mode=auto, top_k=10, expand_depth=0, include_context=false, context_chunks=3).
3. THE system SHALL define a `QueryResult` struct with fields: `results: Vec<RankedChunk>`, `context_windows: Vec<ContextWindow>`, `execution: ExecutionMeta`.
4. THE system SHALL define a `RankedChunk` struct with fields: `chunk_id: String`, `file_id: String`, `content: String`, `heading: Option<String>`, `score: f32`, `vector_score: Option<f32>`, `fts_score: Option<f32>`, `graph_score: Option<f32>`, `block_type: Option<String>`.
5. THE system SHALL define a `ContextWindow` struct with fields: `anchor_chunk_id: String`, `chunks: Vec<ContextChunk>`, `total_tokens: usize`.
6. THE system SHALL define an `ExecutionMeta` struct with fields: `query_mode: QueryMode`, `total_results: usize`, `vector_search_time_ms: u64`, `graph_expand_time_ms: Option<u64>`, `fusion_time_ms: u64`, `total_time_ms: u64`.

---

### Requirement 3: Query Engine Execution Pipeline

**User Story:** As the system, I want a deterministic query execution pipeline that follows a defined plan so that results are consistent and performant.

#### Acceptance Criteria

1. THE `QueryEngine` SHALL build an `ExecutionPlan` from the `Query` before executing.
2. THE execution plan for `vector` mode SHALL be: embed → vector search → return.
3. THE execution plan for `hybrid` mode SHALL be: embed → vector search + FTS search (parallel) → RRF fusion → return.
4. THE execution plan for `expand` mode SHALL be: embed → vector search + FTS search (parallel) → RRF fusion → graph expand → rerank → return.
5. THE execution plan for `auto` mode SHALL inspect the query: if `expand_depth > 0` use `expand`, else if filters are present use `hybrid`, else use `vector`.
6. ALL execution plans SHALL be observable via `ExecutionMeta` timing.

---

### Requirement 4: Context Window Builder

**User Story:** As an AI agent, I want surrounding context around matched chunks so that I can understand the full document section without fetching the entire file.

#### Acceptance Criteria

1. THE system SHALL provide a `ContextWindowBuilder` that, given an anchor chunk and a file, fetches adjacent chunks (before and after within the same heading scope).
2. THE `ContextWindowBuilder` SHALL return a `ContextWindow` containing up to `context_chunks` total chunks (anchor ± N neighbors).
3. THE context window SHALL NOT cross heading boundaries (a chunk under `# Section A` should not include chunks from `# Section B`).
4. THE `ContextWindowBuilder` SHALL compute `total_tokens` as the sum of token estimates for all chunks in the window.
5. IF a chunk has no neighbors in the same heading scope, the context window SHALL contain only the anchor chunk.

---

### Requirement 5: Auto Mode Heuristics

**User Story:** As a user, I want the system to automatically choose the best query strategy so that I get good results without configuring search parameters.

#### Acceptance Criteria

1. THE `auto` mode SHALL use `vector` search when the query is a short keyword or entity name (<3 words).
2. THE `auto` mode SHALL use `hybrid` search when the query is a natural-language phrase (3+ words).
3. THE `auto` mode SHALL use `expand` when the query contains explicit cross-reference keywords ("related to", "connected to", "reference").
4. THE `auto` switching logic SHALL be implemented as a pure function `fn select_mode(query: &str, expand_depth: usize) -> QueryMode` for testability.

---

### Requirement 6: Ranking Improvements

**User Story:** As a user, I want well-ranked results that properly balance vector similarity, keyword matching, and graph connectivity.

#### Acceptance Criteria

1. THE RRF fusion implementation SHALL use the same `k=60` constant from ocean-vector for consistency.
2. THE graph-expanded results SHALL be scored as `0.7 * original_score + 0.3 * graph_score` (same as current `SearchEngine::expand_results`).
3. THE system SHALL add a `rerank_by_heading_diversity` option to `Query` that penalizes results from the same heading to increase coverage.
4. THE system SHALL add a `rerank_by_file_diversity` option to `Query` that penalizes results from the same file to increase cross-document coverage.

---

### Requirement 7: CLI Integration

**User Story:** As a CLI user, I want a unified `query` command and the existing `vector-search` command to remain working for backwards compatibility.

#### Acceptance Criteria

1. THE system SHALL add a `query` subcommand to the CLI with the same interface as `vector-search` plus `--mode` (auto/vector/hybrid/expand), `--context`, and `--context-chunks` flags.
2. THE `query` command SHALL use `QueryEngine` under the hood.
3. THE existing `vector-search` command SHALL continue to work unchanged (delegating to `SearchEngine` for backwards compatibility).
4. THE `query` command output SHALL include `ExecutionMeta` timing when `--verbose` is passed.

---

### Requirement 8: Public Query API

**User Story:** As an external system (MCP server, API server), I want a stable public API for querying so that I can integrate Ocean into larger workflows.

#### Acceptance Criteria

1. THE system SHALL expose `pub async fn query(engine: &QueryEngine, q: Query) -> Result<QueryResult, QueryError>`.
2. THE system SHALL expose `pub async fn query_stream(engine: &QueryEngine, q: Query) -> impl Stream<Item = Result<RankedChunk, QueryError>>` for streaming results.
3. THE public API SHALL be in `src/ocean_query/api.rs` or re-exported from `src/ocean_query/mod.rs`.
4. ALL public API functions SHALL have doc comments with examples.

---

### Requirement 9: Error Handling

**User Story:** As a developer, I want clear, typed errors so that I can handle failures appropriately.

#### Acceptance Criteria

1. THE system SHALL define a `QueryError` enum with variants: `NoResults`, `EmbeddingFailed(EmbedderError)`, `VectorSearchFailed(StoreError)`, `GraphExpandFailed(GraphError)`, `ContextBuildFailed(String)`, `InvalidQuery(String)`.
2. ALL `QueryError` variants SHALL implement `Display` and `Error`.
3. THE `QueryEngine` SHALL never panic on invalid input.

---

### Requirement 10: Testability

**User Story:** As a developer, I want the query engine to be testable with mock/stub stores so that CI tests run without SurrealDB.

#### Acceptance Criteria

1. THE `QueryEngine` SHALL accept `VectorStore` and `GraphStore` via generic parameters or trait bounds for testability.
2. THE `QueryMode` selection logic SHALL be a standalone pure function.
3. THE `ContextWindowBuilder` SHALL be testable with a mock chunk store.
4. THE `ExecutionMeta` SHALL be populated correctly in tests with zero-dependency mock engines.

---

### Requirement 11: Backwards Compatibility

**User Story:** As an existing user, I want my `vector-search` usage to continue working so that I can migrate to `query` at my own pace.

#### Acceptance Criteria

1. THE `vector-search` CLI command SHALL remain unchanged (same args, same output format).
2. THE `SearchEngine` in `ocean_vector::search` SHALL remain unchanged.
3. THE `ocean_query` module SHALL depend on `ocean_vector::search::SearchEngine` and `ocean_graph` — not extract or duplicate their code.
