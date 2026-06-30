# Implementation Plan: Ocean Code Quality & Architectural Cleanup

## Overview

13 independent fixes grouped into 4 categories. Each task is 15–60 minutes. Tasks can be done in any order within their category, and categories are independent.

## Pre-requisites

- Full test suite passes (`cargo test`).
- `cargo build` succeeds with no warnings.

## Tasks

### Category A: Performance Fixes

- [ ] 1. **Lazy-compile regex patterns in `GraphBuilder`**
  - In `src/ocean_graph/builder.rs`, replace 5 `Regex::new(...)` calls inside `extract_references()` with 5 `OnceLock<Regex>` statics.
  - Add `use std::sync::OnceLock;` at top of file.
  - Wrap each regex in a getter function: `fn see_regex() -> &'static Regex { static RE: OnceLock<Regex> = OnceLock::new(); RE.get_or_init(|| Regex::new(...).unwrap()) }`.
  - Update `extract_references()` to use the getter functions.
  - Add test: call `extract_references()` twice, verify output is identical.
  - _Requirements: R1_
  - ⏱ 30 min

- [ ] 2. **Add default token estimator**
  - In `src/ocean_chunk/types.rs`, add `pub fn default_token_estimator(text: &str) -> usize` using word-count approach.
  - Change `token_estimator` field type from `Option<fn(&str) -> usize>` to `fn(&str) -> usize`.
  - Update `Default` impl to use `default_token_estimator`.
  - Update all call sites that pass `token_estimator: None` to pass `token_estimator: default_token_estimator` or remove the explicit field.
  - Update `chunker.rs` to remove the `map_or_else` fallback branch.
  - Add test: verify estimator returns >0 for non-empty text.
  - _Requirements: R8_
  - ⏱ 30 min

### Category B: Dead Code Removal

- [ ] 3. **Remove `max_retries` dead field**
  - Remove `max_retries: u32` from `IndexConfig` in `src/ocean_index/config.rs`.
  - Remove `max_retries: 3` in `src/ocean_api/indexing.rs`.
  - Remove any other references across the codebase (grep for `max_retries`).
  - If `RetryPolicy` from ocean-runtime exists, use it instead.
  - _Requirements: R2_
  - ⏱ 15 min

- [ ] 4. **Remove `SubQuery::BuildContext` dead variant**
  - Remove `BuildContext` from `SubQuery` enum in `src/ocean_query/engine.rs`.
  - Remove `#[allow(dead_code)]` attribute.
  - Remove match arm for `BuildContext` in `execute_plan()`.
  - _Requirements: R3_
  - ⏱ 15 min

- [ ] 5. **Remove `GraphRequest` dead struct**
  - Remove `GraphRequest` struct from `src/ocean_api/types.rs`.
  - _Requirements: R5_
  - ⏱ 5 min

- [ ] 6. **Remove `proptest` unused dev-dependency**
  - Remove `proptest` line from `[dev-dependencies]` in `Cargo.toml`.
  - _Requirements: R13_
  - ⏱ 5 min

### Category C: Dependency Hygiene

- [ ] 7. **Move `tempfile` to dev-dependencies**
  - Move `tempfile` from `[dependencies]` to `[dev-dependencies]` in `Cargo.toml`.
  - Verify `cargo build --lib` succeeds (no test-only deps in production).
  - _Requirements: R7_
  - ⏱ 10 min

- [ ] 8. **Rename duplicate `IndexError`**
  - Rename `ocean_vector::pipeline::IndexError` to `PipelineError`.
  - Update all references in `pipeline.rs` itself.
  - Update `ocean_api::indexing.rs` if it references `ocean_vector::pipeline::IndexError`.
  - Update `ocean_index::error` if it uses match/from for the old name.
  - _Requirements: R4_
  - ⏱ 20 min

- [ ] 9. **Remove `ocean_vector` → `ocean_graph` dependency**
  - In `src/ocean_vector/search.rs`:
    - Remove `use crate::ocean_graph::expansion::ExpansionEngine`.
    - Remove `use crate::ocean_graph::types::EdgeDirection`.
    - Remove `expand_results()` method from `SearchEngine`.
    - Remove `expansion_engine` field from `SearchEngine` (if present).
  - In `src/ocean_query/engine.rs` or new `src/ocean_query/expand.rs`:
    - Add `pub fn expand_results(...)` free function.
  - Update all callers of `SearchEngine::expand_results()` to use new location.
  - _Requirements: R11_
  - ⏱ 45 min

- [ ] 10. **Fix `ocean_storage` → `ocean_chunk` layering**
  - Add `ChunkData` struct to `src/ocean_storage/chunk_store.rs` with fields mirroring `Chunk`.
  - Add `From<ChunkData> for ChunkRecord` impl.
  - Remove `use crate::ocean_chunk::Chunk` from `chunk_store.rs` and `chunk_store_impl.rs`.
  - In `src/ocean_index/processor.rs`, add conversion from `Chunk` → `ChunkData` at the store call site.
  - _Requirements: R12_
  - ⏱ 45 min

### Category D: Bug Fixes

- [ ] 11. **Fix graph stats returning zeros**
  - In `src/ocean_cli/run.rs`, update `cmd_graph_stats` to call `store.count_nodes()`, `store.count_edges()`, `store.get_nodes_by_type()`.
  - Remove hardcoded `type_counts` vector with zeros.
  - Add error handling for store operations.
  - Add test with mock `GraphStore`.
  - _Requirements: R6_
  - ⏱ 30 min

- [ ] 12. **Fix Gemini API key in URL**
  - In `src/ocean_vector/embedder.rs`, modify `GeminiEmbedder::embed()` and `embed_batch()`:
    - Remove `?key={}` from URL.
    - Add `header("X-Goog-Api-Key", api_key)` to request.
  - Verify existing mock tests still pass (update mocks to check header).
  - _Requirements: R10_
  - ⏱ 20 min

- [ ] 13. **Fix mutex poisoning safety**
  - In `src/ocean_fs/watcher.rs`, replace `.lock().unwrap()` with `.lock().unwrap_or_else(|e| { eprintln!(...); e.into_inner() })`.
  - In `src/ocean_storage/storage_impl.rs`, same replacement for all `.lock().unwrap()` calls.
  - _Requirements: R9_
  - ⏱ 15 min

- [ ] **Validation & Cleanup**
  - Run full test suite: `cargo test` — all tests must pass.
  - Verify `cargo build` succeeds with no warnings.
  - Verify `cargo build --release` succeeds.
  - Grep for remaining `max_retries`, `BuildContext`, `GraphRequest`, `proptest` — none should exist.
  - _Requirements: All_

## Notes

- **Task order**: All tasks are independent. Category D (bugs) has highest priority.
- **Architectural fixes (9, 10)**: These are the highest-risk changes. Run full test suite immediately after each.
- **No API breaking changes**: All fixes preserve existing public API signatures.
- **Each fix = one commit**: Makes bisection easy.
- **Estimated total**: ~4–5 hours for all 13 fixes.
