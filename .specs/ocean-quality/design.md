# Design Document: Ocean Code Quality & Architectural Cleanup

## Overview

This phase addresses 13 distinct code quality and architectural issues identified in the analysis. Each fix is small, isolated, and independently testable. The changes are grouped into four categories: performance (regex lazy compilation, token estimator), dead code removal (max_retries, SubQuery::BuildContext, GraphRequest, proptest), dependency hygiene (tempfile, proptest, layer violations), and bug fixes (graph stats zeros, Gemini API key, mutex poisoning).

### Key Design Decisions

1. **OnceLock over lazy_static** — `std::sync::OnceLock` is stable since Rust 1.70 and avoids external dependencies. Preferred over `once_cell::lazy` or `lazy_static!`.
2. **No API-breaking changes** — All fixes preserve existing public APIs. Renames and moves are internal-only.
3. **Architectural fixes are optional** — R11 and R12 are the most invasive changes. They can be deferred if risk is too high. All other fixes are safe.
4. **Each fix is a separate commit** — Makes bisection easy if a fix causes regression.

---

## Category 1: Performance Fixes

### 1. Regex Lazy Compilation (R1)

**File:** `src/ocean_graph/builder.rs`

**Current code:**
```rust
pub fn extract_references(&self, text: &str) -> Vec<(String, String)> {
    let see_pattern = Regex::new(r"(?i)\bsee\s+([a-z\s]+)").unwrap();
    let refer_pattern = Regex::new(r"(?i)\brefer\s+to\s+([a-z\s]+)").unwrap();
    // ... 3 more patterns compiled every call
}
```

**Fixed code:**
```rust
use std::sync::OnceLock;

fn see_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\bsee\s+([a-z\s]+)").unwrap())
}

fn refer_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\brefer\s+to\s+([a-z\s]+)").unwrap())
}
// ... same for remaining 3 patterns

pub fn extract_references(&self, text: &str) -> Vec<(String, String)> {
    // use see_regex(), refer_regex(), etc.
}
```

### 2. Token Estimator Default (R8)

**File:** `src/ocean_chunk/types.rs`

**Current code:**
```rust
pub struct ChunkConfig {
    pub token_estimator: Option<fn(&str) -> usize>, // always None in production
}

// Fallback in chunker.rs:
let tokens = config.token_estimator.map_or_else(
    || text.len() / 4, // crude character-based estimate
    |f| f(text),
);
```

**Fixed code:**
```rust
/// Estimate tokens by counting whitespace-separated words + punctuation clumps.
/// More accurate than `text.len() / 4` for English text.
pub fn default_token_estimator(text: &str) -> usize {
    text.split_whitespace().count() + text.matches(|c: char| c.is_ascii_punctuation()).count() / 2
}

pub struct ChunkConfig {
    pub token_estimator: fn(&str) -> usize, // always has a default
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            // other fields...
            token_estimator: default_token_estimator,
        }
    }
}
```

---

## Category 2: Dead Code Removal

### 3. `max_retries` Field (R2)

**Files:** `src/ocean_index/config.rs`, `src/ocean_api/indexing.rs`

- Remove `max_retries: u32` from `IndexConfig`.
- Remove the hardcoded `max_retries: 3` in `ocean_api::indexing::index_directory()`.
- If `ocean-runtime` phase is implemented first, replace with `RetryPolicy`.

### 4. `SubQuery::BuildContext` (R3)

**File:** `src/ocean_query/engine.rs`

- Remove the `BuildContext` variant from `SubQuery` enum.
- Remove `#[allow(dead_code)]`.
- Remove match arm handling `BuildContext` in `execute_plan()`.

### 5. `GraphRequest` (R5)

**File:** `src/ocean_api/types.rs`

- Remove the `GraphRequest` struct definition.
- No other references exist.

### 6. `proptest` Dependency (R13)

**File:** `Cargo.toml`

- Remove `proptest` from `[dev-dependencies]`.

---

## Category 3: Dependency Hygiene

### 7. `tempfile` to Dev-Deps (R7)

**File:** `Cargo.toml`

- Move `tempfile` from `[dependencies]` to `[dev-dependencies]`.

### 8. `ocean_vector` → `ocean_graph` Layering (R11)

**Files involved:**
- `src/ocean_vector/search.rs` — remove `ExpansionEngine`, `EdgeDirection` imports
- `src/ocean_query/engine.rs` — add `expand_results()` as free function
- `src/ocean_graph/expansion.rs` — no changes (the engine stays here)

**New code in `ocean_query/engine.rs`:**
```rust
pub fn expand_results(
    search_results: Vec<SearchResult>,
    expansion_engine: &ExpansionEngine,
    max_depth: usize,
) -> Vec<(String, f32)> {
    let mut expanded = Vec::new();
    for result in &search_results {
        let neighbors = expansion_engine.expand(&result.chunk_id, max_depth);
        for node in neighbors {
            expanded.push((node.id.clone(), result.score));
        }
    }
    expanded
}
```

**Current code in `ocean_vector/search.rs` that moves:**
- `SearchEngine::expand_results()` → becomes free function in `ocean_query`
- `SearchEngine::hybrid_filtered_search()` → the graph-expansion parts move, the vector+FTS core stays

### 9. `ocean_storage` → `ocean_chunk` Layering (R12)

**Files involved:**
- `src/ocean_storage/chunk_store.rs` — define `ChunkData` struct inside storage
- `src/ocean_chunk/types.rs` — no changes (Chunk stays)
- `src/ocean_storage/chunk_store_impl.rs` — `from_chunk()` → `from_data()`
- `src/ocean_index/processor.rs` — convert `Chunk` → `ChunkData` before calling store

**New `ChunkData` struct:**
```rust
// In ocean_storage::chunk_store:
pub struct ChunkData {
    pub id: String,
    pub file_id: String,
    pub content: String,
    pub heading: Option<String>,
    pub chunk_type: String,
    pub page: Option<u32>,
    pub slide: Option<u32>,
    pub sheet: Option<String>,
    pub start_offset: Option<usize>,
    pub end_offset: Option<usize>,
    pub content_hash: String,
    pub model: String,
    pub embedding: Vec<f32>,
}

impl From<ChunkData> for ChunkRecord { ... }
```

**Conversion at call site (`ocean_index::processor`):**
```rust
let chunk_data = ChunkData {
    id: chunk.id.clone(),
    file_id: chunk.file_id.clone(),
    content: chunk.content.clone(),
    // ... map fields
};
chunk_store.insert_chunk(&chunk_data.into())?;
```

---

## Category 4: Bug Fixes

### 10. Graph Stats Zeros (R6)

**File:** `src/ocean_cli/run.rs`

**Current code (broken):**
```rust
let type_counts = vec![
    ("File".to_string(), 0u64),
    ("Chunk".to_string(), 0u64),
    ("Heading".to_string(), 0u64),
    ("Entity".to_string(), 0u64),
    ("Folder".to_string(), 0u64),
];
print_graph_stats(0, 0, type_counts);
```

**Fixed code:**
```rust
let total_nodes = store.count_nodes()?;
let total_edges = store.count_edges()?;
let type_counts = vec![
    ("File".to_string(), store.get_nodes_by_type(&NodeType::File)?.len() as u64),
    ("Chunk".to_string(), store.get_nodes_by_type(&NodeType::Chunk)?.len() as u64),
    ("Heading".to_string(), store.get_nodes_by_type(&NodeType::Heading)?.len() as u64),
    ("Entity".to_string(), store.get_nodes_by_type(&NodeType::Entity)?.len() as u64),
    ("Folder".to_string(), store.get_nodes_by_type(&NodeType::Folder)?.len() as u64),
];
print_graph_stats(total_nodes, total_edges, type_counts);
```

### 11. Gemini API Key Header (R10)

**File:** `src/ocean_vector/embedder.rs`

**Current code:**
```rust
let url = format!(
    "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
    model, api_key
);
```

**Fixed code:**
```rust
let url = format!(
    "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent",
    model,
);
// ... add header:
let response = client
    .post(&url)
    .header("X-Goog-Api-Key", &api_key)
    .json(&body)
    .send()?;
```

### 12. Mutex Poisoning Safety (R9)

**Files:** `src/ocean_fs/watcher.rs`, `src/ocean_storage/storage_impl.rs`

**Current code:**
```rust
self.inner.lock().unwrap()
```

**Fixed code:**
```rust
self.inner.lock().unwrap_or_else(|e| {
    log::warn!("Mutex was poisoned, recovering: {}", e);
    e.into_inner()
})
```

If `log` crate is not a dependency, use `eprintln!` instead.

---

## Correctness Properties

### Property 1: Behavior Preservation

*For any* existing test that passes before these changes, the same test SHALL pass after all changes are applied (no behavioral changes except where bugs are fixed).

**Validates:** R1–R13

### Property 2: Regex Compilation Once

*For any* N calls to `extract_references()`, the 5 regex patterns SHALL be compiled exactly once (first call), not N times.

**Validates:** R1

### Property 3: Dependency Isolation

*For any* module in the crate, `ocean_vector` SHALL NOT import from `ocean_graph`, and `ocean_storage` SHALL NOT import from `ocean_chunk`.

**Validates:** R11, R12

---

## Testing Strategy

### Per-Fix Tests

- R1: Unit test calls `extract_references()` twice, verify patterns work identically.
- R2: `cargo build` succeeds, no `max_retries` field anywhere.
- R3: `cargo build` succeeds, no `BuildContext` variant.
- R4: `cargo build` succeeds, no `IndexError` ambiguity.
- R5: `cargo build` succeeds, no `GraphRequest` struct.
- R6: Mock `GraphStore`, call `cmd_graph_stats`, verify non-zero counts appear.
- R7: `cargo build --lib` succeeds; `cargo test` succeeds.
- R8: Unit test `default_token_estimator` returns expected values.
- R9: `cargo build` succeeds, no `.lock().unwrap()` in affected files.
- R10: Mock HTTP server verifies `X-Goog-Api-Key` header is sent.
- R11: `cargo build` succeeds, `grep "ocean_graph" src/ocean_vector/` returns nothing.
- R12: `cargo build` succeeds, `grep "ocean_chunk" src/ocean_storage/` returns nothing.
- R13: `cargo build` succeeds, `cargo test` succeeds.

### Integration

Run full test suite: `cargo test` — all tests must pass.
