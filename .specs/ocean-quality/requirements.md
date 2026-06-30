# Requirements Document: Ocean Code Quality & Architectural Cleanup

## Introduction

The Ocean codebase has accumulated several quality issues: dead code (`max_retries` field never read, `SubQuery::BuildContext` unused, `GraphRequest` unused), wasteful runtime behavior (regex recompilation on every file), duplicate type names (`IndexError` in two modules), architectural layering violations (`ocean_vector` importing from `ocean_graph`), and a broken CLI output (`graph stats` returning all zeros). This phase fixes these issues to improve maintainability, correctness, and performance.

The scope is purely cleanup — no new features. Each fix is small and isolated.

---

## Glossary

- **Dead code**: Fields, variants, or functions that are defined but never used in production.
- **OnceLock**: `std::sync::OnceLock<T>` — a thread-safe one-time initializer for static values. Used to compile regex patterns once instead of on every call.
- **Layering violation**: A dependency between modules that goes against the intended one-way derivation chain (fs → parser → chunk → vector → graph → query).
- **`IndexError` name collision**: Two distinct error types in `ocean_vector::pipeline` and `ocean_index::error` share the same name, causing ambiguity when both are in scope.

---

## Requirements

### R1: Regex Lazy Compilation

**User Story:** As a developer, I want regex patterns in `GraphBuilder::extract_references()` to be compiled once at startup, not on every `from_chunks()` call, so that indexing performance is not degraded by repeated regex compilation.

#### Acceptance Criteria

1. THE five hardcoded regex patterns in `ocean_graph::builder::extract_references()` SHALL be compiled using `std::sync::OnceLock<Regex>` (or `once_cell::sync::Lazy<Regex>`) at module level.
2. EACH `OnceLock<Regex>` SHALL be initialized on first access and reused for all subsequent calls.
3. THE regex patterns SHALL remain unchanged (same matching behavior).
4. A unit test SHALL verify that all patterns compile successfully (compile-once does not panic).

---

### R2: Remove `max_retries` Dead Code

**User Story:** As a developer, I want the `max_retries: u32` field that is declared but never used to either be wired up correctly or removed, so that the codebase does not contain misleading dead code.

#### Acceptance Criteria

1. THE `max_retries: u32` field in `IndexConfig` (and any other location) SHALL be removed OR replaced with the `RetryPolicy` struct from the ocean-runtime phase.
2. IF the ocean-runtime phase is implemented first, `RetryPolicy` SHALL be used instead.
3. IF the ocean-runtime phase is NOT yet implemented, the dead field SHALL be removed with no replacement.
4. All references to the removed field SHALL be removed from constructors, tests, and call sites.
5. `cargo build` SHALL succeed with no warnings about unused fields.

---

### R3: Remove `SubQuery::BuildContext` Dead Variant

**User Story:** As a developer, I want the `SubQuery::BuildContext` variant that is marked `#[allow(dead_code)]` to be removed, so that the codebase is cleaner.

#### Acceptance Criteria

1. THE `SubQuery::BuildContext` variant in `ocean_query::engine` SHALL be removed.
2. THE `#[allow(dead_code)]` attribute on the variant SHALL be removed.
3. Any match arms that handle `BuildContext` SHALL be removed.
4. The context building logic already implemented directly in `execute()` SHALL remain unchanged.
5. `cargo build` SHALL succeed with no dead_code warnings.

---

### R4: Rename Duplicate `IndexError`

**User Story:** As a developer, I want the two distinct `IndexError` types (`ocean_vector::pipeline::IndexError` and `ocean_index::error::IndexError`) to have distinct names so that they do not collide when both modules are imported.

#### Acceptance Criteria

1. THE `ocean_vector::pipeline::IndexError` SHALL be renamed to `PipelineError` (or `VectorIndexError`).
2. All references to `ocean_vector::pipeline::IndexError` within `ocean_vector` and external consumers SHALL be updated.
3. The `ocean_index::error::IndexError` SHALL remain unchanged (it is the "primary" index error type).
4. `cargo build` SHALL succeed with no naming ambiguities.

---

### R5: Remove `GraphRequest` Dead Struct

**User Story:** As a developer, I want the `GraphRequest` struct in `ocean_api::types` that is never used to be removed.

#### Acceptance Criteria

1. THE `GraphRequest` struct SHALL be removed from `ocean_api::types`.
2. No other code SHALL reference `GraphRequest`.
3. The graph API functions (`graph_info`, `graph_expand`, `graph_path`, `graph_stats`) take raw parameters and SHALL remain unchanged.
4. `cargo build` SHALL succeed.

---

### R6: Fix Graph Stats Returning Zeros

**User Story:** As a CLI user, I want `ocean graph stats` to display actual node type counts from the graph store instead of hardcoded zeros.

#### Acceptance Criteria

1. THE `cmd_graph_stats` function in `ocean_cli::run` SHALL call `GraphStore::count_nodes()` and `GraphStore::get_nodes_by_type()` (or equivalent) to get real counts.
2. THE hardcoded `type_counts` vector of zeros SHALL be removed.
3. THE displayed output SHALL show actual counts: number of File nodes, Chunk nodes, Heading nodes, Entity nodes, Folder nodes.
4. IF the graph store has no data, the output SHALL show zeros (not error).
5. Unit tests SHALL verify the output format with a mock graph store.

---

### R7: Move `tempfile` to Dev-Dependencies

**User Story:** As a developer, I want `tempfile` to be a dev-dependency since it is only used in tests, so that production builds are smaller and dependency audits are cleaner.

#### Acceptance Criteria

1. THE `tempfile` crate SHALL be moved from `[dependencies]` to `[dev-dependencies]` in `Cargo.toml`.
2. `cargo build --lib` SHALL succeed (no test-only dependency in production build).
3. `cargo test` SHALL still run all tests that use `tempfile`.

---

### R8: Add Token Estimator Implementation

**User Story:** As a developer, I want the `ChunkConfig::token_estimator` field to have a working default implementation instead of always being `None`, so that token estimation is more accurate than the fallback `text.len() / 4`.

#### Acceptance Criteria

1. THE `token_estimator: Option<fn(&str) -> usize>` field in `ChunkConfig` SHALL default to a built-in `default_token_estimator` function.
2. THE default estimator SHALL use a simple heuristic: count whitespace-separated tokens (words + punctuation clumps), which is more accurate than `text.len() / 4`.
3. ALL production code that constructs `ChunkConfig` SHALL use the default (no change to public API).
4. Tests that explicitly set `token_estimator: None` SHALL continue to work.

---

### R9: Fix Mutex Poisoning Safety

**User Story:** As a developer, I want mutex locks in `watcher.rs` and `storage_impl.rs` to handle potential poisoning gracefully instead of panicking with `.unwrap()`.

#### Acceptance Criteria

1. ALL `.lock().unwrap()` calls on `Mutex` in `ocean_fs::watcher` SHALL be replaced with `.lock().map_err(|_| ...)` or `.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoning.
2. ALL `.lock().unwrap()` calls on `Mutex` in `ocean_storage::storage_impl` SHALL be similarly fixed.
3. For poisoning recovery, the lock's poisoned value SHALL be extracted via `into_inner()` and the error SHALL be logged.
4. `cargo build` SHALL succeed with no `unwrap()` on mutex locks in these files.

---

### R10: Fix Gemini API Key in URL

**User Story:** As a security-conscious user, I want the Gemini embedder to pass the API key via an HTTP header instead of a URL query parameter, so that the key is not exposed in server logs.

#### Acceptance Criteria

1. THE `GeminiEmbedder::embed()` and `GeminiEmbedder::embed_batch()` methods SHALL pass the API key via the `X-Goog-Api-Key` HTTP header instead of the `?key=` query parameter.
2. THE endpoint URL SHALL no longer contain the API key.
3. All existing functionality SHALL remain unchanged.
4. Unit tests (with mock server) SHALL verify the header is sent correctly.

---

### R11: Architectural — Remove `ocean_vector` → `ocean_graph` Dependency

**User Story:** As a system architect, I want `ocean_vector::search` to stop importing from `ocean_graph`, so that the dependency chain remains one-way (vector → graph is a lateral dependency).

#### Acceptance Criteria

1. THE `ExpansionEngine` and `EdgeDirection` imports in `ocean_vector::search.rs` SHALL be removed.
2. THE `expand_results()` method in `ocean_vector::search.rs` SHALL be moved to `ocean_query::engine` or a new `ocean_query::expand` module.
3. THE `SearchEngine` struct SHALL no longer hold a reference to `ExpansionEngine`.
4. The `hybrid_filtered_search()` and `expand_results()` methods SHALL become free functions in `ocean_query` that take a `SearchEngine` + `ExpansionEngine` as parameters.
5. All existing consumers of `SearchEngine::expand_results()` SHALL be updated to use the new location.
6. `cargo build` SHALL succeed with no `ocean_graph` imports in `ocean_vector`.

---

### R12: Architectural — Fix `ocean_storage` → `ocean_chunk` Dependency

**User Story:** As a system architect, I want `ocean_storage` to stop importing from `ocean_chunk`, so that the storage layer does not depend on higher-level chunk types.

#### Acceptance Criteria

1. THE `ChunkRecord::from_chunk()` method in `ocean_storage::chunk_store` SHALL be refactored so that `ocean_storage` does not directly import `ocean_chunk::Chunk`.
2. OPTION A: Define a `ChunkData` struct in `ocean_storage` that contains the fields needed from `ocean_chunk::Chunk`, and add a `From<ChunkData> for ChunkRecord` impl. The conversion happens at the call site in `ocean_index::processor`.
3. OPTION B: Move the `ChunkRecord::from_chunk()` logic to `ocean_index::processor` or `ocean_api::indexing`, passing pre-extracted fields to `ocean_storage`.
4. `cargo build` SHALL succeed with no `crate::ocean_chunk` imports in `ocean_storage`.

---

### R13: Remove `proptest` Unused Dev-Dependency

**User Story:** As a developer, I want the `proptest` dev-dependency that is declared but never imported to be removed, so that `cargo update` does not pull unnecessary crates.

#### Acceptance Criteria

1. THE `proptest` entry SHALL be removed from `[dev-dependencies]` in `Cargo.toml`.
2. `cargo build` SHALL succeed.
3. `cargo test` SHALL succeed.
