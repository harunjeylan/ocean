# Implementation Plan: ocean-query

## Overview

Implement the top-level query orchestrator: `QueryEngine`, query/result types, mode selection, context window builder, CLI `query` command, and public API. Ocean-query delegates to existing sub-engines (`SearchEngine`, `ExpansionEngine`, `VectorStore`) rather than duplicating their logic. All new code lives under `src/ocean_query/`.

## Dependencies

No new Cargo dependencies. `surrealdb`, `tokio`, `serde`, `serde_json`, `chrono`, `sha2`, `regex` are already present from ocean-vector and ocean-graph.

## Tasks

### Sub-Phase A: Foundation — Types and Module Structure

- [ ] 1. Create module structure (`src/ocean_query/`)
  - `mod.rs` — `pub mod engine;` + `pub mod types;` + `pub mod context;` + `pub mod api;` + `pub mod error;` + `pub use` re-exports
  - `types.rs` — `Query`, `QueryMode`, `QueryResult`, `RankedChunk`, `ContextWindow`, `ContextChunk`, `ExecutionMeta`
  - `error.rs` — `QueryError` enum
  - `engine.rs` — `QueryEngine`, `ExecutionPlan`, `SubQuery`
  - `context.rs` — `ContextWindowBuilder`
  - `api.rs` — public API functions (`query`, `query_stream`)
  - `engine_test.rs`, `context_test.rs`, `types_test.rs` — unit tests

  _Requirements: R1, R2, R5, R9_

  - [ ] 1.1 Define `QueryMode` enum: `Auto`, `Vector`, `Hybrid`, `Expand`
  - [ ] 1.2 Define `Query` struct with all fields from design.md, impl `Default`
  - [ ] 1.3 Define `QueryResult` struct with `results`, `context_windows`, `execution`
  - [ ] 1.4 Define `RankedChunk`, `ContextWindow`, `ContextChunk`, `ExecutionMeta` structs
  - [ ] 1.5 Define `QueryError` enum with all variants from R9
  - [ ] 1.6 Derive `Clone, Debug, PartialEq, Serialize, Deserialize` on data types where appropriate
  - [ ] 1.7 Write unit tests: type creation, `Query::default()`, `QueryError::Display`

  _Requirements: R1, R2, R9_

### Sub-Phase B: Mode Selection Logic

- [ ] 2. Implement `select_mode()` in `engine.rs`

  Pure function: `pub fn select_mode(text: &str, expand_depth: usize) -> QueryMode`

  Logic:
  - if `expand_depth > 0` → `Expand`
  - if `text` is empty → `Hybrid` (fallback)
  - if word count < 3 → `Vector` (short keyword/entity)
  - if text contains cross-ref trigger phrases ("related to", "connected to", "reference", "associated with") → `Expand`
  - else (3+ words, no trigger) → `Hybrid`

  _Requirements: R5_

  - [ ] 2.1 Implement word-count heuristic
  - [ ] 2.2 Implement trigger-phrase detection using `regex` or `str::contains`
  - [ ] 2.3 Write tests: short keyword, long phrase, trigger phrases, empty text, expand_depth override

  _Requirements: R5.4, R10.2_

### Sub-Phase C: QueryEngine Orchestrator

- [ ] 3. Implement `QueryEngine` in `engine.rs`

  ```rust
  pub struct QueryEngine {
      store: VectorStore,
      search: SearchEngine,
      graph: Option<ExpansionEngine>,
  }
  ```

  - [ ] 3.1 Implement `QueryEngine::new(db_path)` — create `VectorStore`, `SearchEngine`, `ExpansionEngine` from a shared db path. Use the same `tokio::runtime::Runtime` pattern as `ocean_vector::store::VectorStore`.
  - [ ] 3.2 Implement `QueryEngine::new_memory()` — in-memory SurrealDB for tests.
  - [ ] 3.3 Implement `build_execution_plan(&self, query: &Query) -> ExecutionPlan`:
    - Match on `query.mode` (after applying `select_mode` if `Auto`).
    - Return ordered `Vec<SubQuery>` steps.
  - [ ] 3.4 Implement `execute(&self, plan: &ExecutionPlan, query: &Query) -> Result<QueryResult, QueryError>`:
    - Execute each `SubQuery` step sequentially.
    - Collect timing via `std::time::Instant`.
    - Package results into `QueryResult`.
  - [ ] 3.5 Implement `query(&self, query: Query) -> Result<QueryResult, QueryError>` — the main public method. Calls `build_execution_plan` then `execute`.
  - [ ] 3.6 Implement `query_stream(...)` — iterate over `query()` results and yield one at a time.

  _Requirements: R1, R3, R8_

  - [ ] 3.7 Handle empty `ExecutionPlan` (should not happen, but return error gracefully).
  - [ ] 3.8 Handle `graph = None` in `expand` mode — log warning and fall back to `hybrid`.

  _Requirements: R9.3_

### Sub-Phase D: ContextWindowBuilder

- [ ] 4. Implement `ContextWindowBuilder` in `context.rs`

  ```rust
  pub struct ContextWindowBuilder {
      store: VectorStore,
  }
  ```

  - [ ] 4.1 Implement `ContextWindowBuilder::new(store)`.
  - [ ] 4.2 Implement `build(&self, anchor: &RankedChunk, context_chunks: usize) -> Result<ContextWindow, QueryError>`:
    - Fetch the anchor chunk from `VectorStore` by `chunk_id`.
    - Query `chunk` table for chunks with same `file_id` and same `heading`, ordered by chunk offset/created_at.
    - Find anchor position in the ordered list.
    - Take `context_chunks / 2` before and `context_chunks / 2` after (adjust for edges).
    - Clamp `context_chunks` to `[1, 10]`.
    - Compute `total_tokens` via token count estimation (same heuristic as ocean-chunk).
    - Assign `distance_from_anchor` (negative = before, 0 = anchor, positive = after).
  - [ ] 4.3 Handle empty results from `VectorStore.get_chunk` — return `ContextWindow` with only the anchor (use content from `RankedChunk` itself).
  - [ ] 4.4 Handle chunks with no heading — group all `heading = None` chunks together.

  _Requirements: R4_

  - [ ] 4.5 Write unit tests: heading boundary isolation, empty neighbors, single chunk, clamp behavior.

  _Requirements: R4, R10.3_

### Sub-Phase E: Execution Pipeline Integration

- [ ] 5. Wire up sub-engine calls in `QueryEngine::execute`

  Each `SubQuery` step maps to:

  | SubQuery | Action |
  |----------|--------|
  | `Vector { top_k }` | `search.search(query.text, top_k)` |
  | `Fts { top_k }` | `search.fts_search(query.text, top_k)` |
  | `RrfFusion { k }` | `ocean_vector::search::fuse_rrf(vec_results, fts_results, k)` |
  | `GraphExpand { depth }` | `search.expand_results(fused_results, depth)` (reuses existing `SearchEngine::expand_results`) |
  | `RerankByHeading` | Apply heading diversity penalty: multiply score by `1.0 / (1.0 + 0.15 * count_in_heading)` |
  | `RerankByFile` | Apply file diversity penalty: multiply score by `1.0 / (1.0 + 0.1 * count_in_file)` |
  | `BuildContext { n }` | Call `ContextWindowBuilder::build()` for each top-N result |

  - [ ] 5.1 Implement `SubQuery::Vector` — delegates to `SearchEngine::search()`
  - [ ] 5.2 Implement `SubQuery::Fts` — delegates to `SearchEngine` FTS
  - [ ] 5.3 Implement `SubQuery::RrfFusion` — calls `fuse_rrf()` from ocean_vector
  - [ ] 5.4 Implement `SubQuery::GraphExpand` — calls `SearchEngine::expand_results()`
  - [ ] 5.5 Implement `SubQuery::RerankByHeading` — heading diversity penalty
  - [ ] 5.6 Implement `SubQuery::RerankByFile` — file diversity penalty
  - [ ] 5.7 Implement `SubQuery::BuildContext` — context window construction
  - [ ] 5.8 Collect `ExecutionMeta` timing at each step

  _Requirements: R3, R6_

### Sub-Phase F: CLI Integration

- [ ] 6. Add `query` CLI command

  - [ ] 6.1 Add `Query(QueryArgs)` variant to `Commands` enum in `src/ocean_cli/args.rs`
  - [ ] 6.2 Add `QueryArgs` struct with fields:
    - `file: String` (positional)
    - `mode: Option<String>` (auto/vector/hybrid/expand)
    - `top_k: Option<usize>`
    - `expand_depth: Option<usize>`
    - `context: bool`
    - `context_chunks: Option<usize>`
    - `file_id`, `heading`, `block_type` filters
    - `rerank_by_heading: bool`, `rerank_by_file: bool`
    - `verbose: bool` — includes `ExecutionMeta` in output
    - embedder config: `model`, `provider`, `ollama_url`, `openai_key`, etc.
    - `db_path`
  - [ ] 6.3 Add `cmd_query()` handler in `src/ocean_cli/run.rs`:
    - Build `Query` from `QueryArgs`
    - Create `QueryEngine`
    - Call `engine.query(query)`
    - Display results using `print_query_result()` in display.rs
  - [ ] 6.4 Add `print_query_result()` in `src/ocean_cli/display.rs`:
    - Same format as current `vector-search` output for results
    - Add `---` separator before context windows
    - Show `ExecutionMeta` when `--verbose`
  - [ ] 6.5 Keep existing `vector-search` command unchanged (still uses `SearchEngine` directly)

  _Requirements: R7, R11_

### Sub-Phase G: Public API Module

- [ ] 7. Implement `src/ocean_query/api.rs`

  ```rust
  /// Execute a query and return structured results.
  pub fn query(engine: &QueryEngine, q: Query) -> Result<QueryResult, QueryError>;

  /// Execute a query and stream results one at a time.
  pub fn query_stream(
      engine: &QueryEngine,
      q: Query,
  ) -> impl Iterator<Item = Result<RankedChunk, QueryError>>;
  ```

  - [ ] 7.1 Implement `query()` — thin wrapper around `engine.query(q)`
  - [ ] 7.2 Implement `query_stream()` — buffer all results then yield sequentially
  - [ ] 7.3 Add doc comments with examples for both functions

  _Requirements: R8_

### Sub-Phase H: Module Registration

- [ ] 8. Register ocean-query module in the project

  - [ ] 8.1 Add `pub mod ocean_query;` to `src/lib.rs`
  - [ ] 8.2 Add `#[path]` test module entries in `src/tests.rs`:
    - `ocean_query/engine_test.rs`
    - `ocean_query/context_test.rs`
    - `ocean_query/types_test.rs`
  - [ ] 8.3 Update `AGENTS.md` with ocean-query module section:
    - Module structure, public API, CLI command, test count
  - [ ] 8.4 Update `cli-docs.md` with `query` command docs

  _Requirements: R7, R8_

### Sub-Phase I: Tests

- [ ] 9. Write unit and integration tests

  - [ ] 9.1 `types_test.rs` — 5+ tests: `Query::default()`, `QueryMode::Auto` default, `ExecutionMeta` population, `RankedChunk` creation, `ContextWindow` creation
  - [ ] 9.2 `engine_test.rs` — 8+ tests:
    - `query_vector_mode` — in-memory store, vector-only returns results
    - `query_hybrid_mode` — in-memory store, vector+FTS returns fused results
    - `query_expand_mode` — in-memory store + graph, returns expanded results
    - `query_auto_mode_short` — auto selects Vector for short text
    - `query_auto_mode_long` — auto selects Hybrid for long text
    - `query_auto_mode_expand_depth` — expand_depth > 0 forces Expand
    - `query_empty_text` — returns error
    - `query_no_results` — returns `QueryError::NoResults`
  - [ ] 9.3 `context_test.rs` — 4+ tests: heading boundary, empty neighbors, single chunk, context_chunks clamp
  - [ ] 9.4 Run full test suite: `cargo test` — all existing tests continue to pass

  _Requirements: R10_

### Sub-Phase J: Documentation and Polish

- [ ] 10. Finalize

  - [ ] 10.1 Update `AGENTS.md` with complete ocean-query module reference
  - [ ] 10.2 Update `cli-docs.md` with `query` command reference
  - [ ] 10.3 Verify `cargo build` has zero warnings
  - [ ] 10.4 Verify `cargo test` passes

  _Requirements: R7, R11_

## Notes

- **Backwards compatibility**: `vector-search` CLI command remains unchanged. The `query` command is additive.
- **No new storage**: Ocean-query reads from existing `chunk`, `graph_node`, `graph_edge` tables only.
- **Auto mode heuristics are pure functions**: No I/O, fully unit-testable.
- **ContextWindowBuilder** queries the chunk table by `file_id + heading` — ensure there's a SurrealDB index on `(file_id, heading)` for performance (this should already exist from ocean-vector's schema).
- **timing**: Use `std::time::Instant` for all `ExecutionMeta` measurements. Resolution is millisecond-precision.
