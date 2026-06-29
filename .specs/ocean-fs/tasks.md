# Implementation Plan: ocean-fs — Filesystem Layer

## Overview

Implement Phase 1 of Ocean in four sub-phases: (1) core types, data models, and foundation rules, (2) directory scanner + hashing system, (3) file watcher + path resolution, (4) metadata normalizer + filtering + integration wiring. Each sub-phase includes tests.

---

## Tasks

- [ ] 1. Create core types, data models, and foundation module
  - Define all structs, enums, and traits from design.md
  - Create the foundation module enforcing system rules (no index authority, derivation chain, traceability)
  - Export all public types from a `lib.rs` entry point
  - _Requirements: R1, R8, R9_

  - [ ] 1.1 Define `FileMeta` struct with all fields
  - [ ] 1.2 Define `FileId` type alias (UUIDv7 string) and generation function
  - [ ] 1.3 Define `FileEvent` enum with all variants
  - [ ] 1.4 Define `FileCategory` enum and extension-to-category mapping
  - [ ] 1.5 Define `NormalizedFile` struct
  - [ ] 1.6 Define `PathMove` struct
  - [ ] 1.7 Create foundation module with doc constants for system rules

- [ ] 2. Implement Directory Scanner
  - Implement recursive walk with rayon parallel iteration
  - Apply ignore list filtering (hidden files, system dirs)
  - Filter to supported extensions only
  - Compute `FileMeta` for each discovered file
  - _Requirements: R2, R7, R8_

  - [ ] 2.1 Implement `scan_dir(path) -> Vec<FileMeta>` using `walkdir` + `rayon`
  - [ ] 2.2 Implement default ignore list (`.git/`, `node_modules/`, `.cache/`, hidden files)
  - [ ] 2.3 Implement supported extension filter
  - [ ] 2.4 Implement `scandir_filtered(path, filter)` variant with custom callback
  - [ ] 2.5 Write unit tests for scanner (empty dir, nested dirs, hidden files, symlinks)

- [ ] 3. Implement File Hasher
  - Implement streaming SHA-256 via buffered reader
  - Ensure bounded memory usage (max 64KB buffer)
  - Handle edge cases: empty file, very large file, permission denied, binary content
  - _Requirements: R3_

  - [ ] 3.1 Implement `hash_file(path) -> Result<String, HashError>` with streaming reader
  - [ ] 3.2 Implement `verify_hash(path, expected) -> bool`
  - [ ] 3.3 Define `HashError` enum with `IoError` and `FileTooLarge` variants
  - [ ] 3.4 Write unit tests for hasher (empty, large, binary, permission-denied files)

- [ ] 4. Implement File Watcher
  - Use `notify` crate for cross-platform filesystem events
  - Implement event batching (debounce 100ms window, batch up to 100 events)
  - Emit typed `FileEvent` variants with correct metadata
  - Support recursive directory watching
  - _Requirements: R4_

  - [ ] 4.1 Implement `watch(path, callback) -> Result<WatchHandle>` using `notify`
  - [ ] 4.2 Implement `unwatch(handle) -> Result<()>`
  - [ ] 4.3 Implement event debouncing and batching logic
  - [ ] 4.4 Map native notify events to Ocean `FileEvent` enum
  - [ ] 4.5 Write unit tests for watcher (create, modify, delete, rename, move)

- [ ] 5. Implement Path Resolution Layer
  - Implement SQLite-backed path mapping table
  - Support insert, query, and history retrieval
  - Resolve `FileId` → current path
  - _Requirements: R5_

  - [ ] 5.1 Create `path_moves` table schema and indexes
  - [ ] 5.2 Implement `record_move(file_id, old_path, new_path)`
  - [ ] 5.3 Implement `resolve_path(file_id) -> Option<String>`
  - [ ] 5.4 Implement `get_move_history(file_id) -> Vec<PathMove>`
  - [ ] 5.5 Write unit tests for path resolution (single move, chain of moves, unknown id)

- [ ] 6. Implement Metadata Normalizer
  - Map file extension to `FileCategory` and MIME type
  - Construct `NormalizedFile` from `FileMeta`
  - Handle unknown extensions gracefully
  - _Requirements: R6_

  - [ ] 6.1 Implement extension → category mapping table
  - [ ] 6.2 Implement extension → MIME type mapping
  - [ ] 6.3 Implement `normalize(meta) -> NormalizedFile`
  - [ ] 6.4 Write unit tests for all supported and unsupported formats

- [ ] 7. Implement Filtering System
  - Enforce ignore list and extension whitelist
  - Provide configurable ignore patterns via builder or config struct
  - _Requirements: R7_

  - [ ] 7.1 Implement default filter (ignore list + supported extensions)
  - [ ] 7.2 Implement configurable filter with custom ignore patterns
  - [ ] 7.3 Write unit tests for filter edge cases

- [ ] 8. Write integration tests and benchmark
  - End-to-end pipeline: scan → hash → filter → normalize → output
  - Watcher + scanner consistency test
  - 10,000 file scan benchmark
  - _Validates: Properties 1, 2, 3, 5_

  - [ ] 8.1 End-to-end test with temp directory of mixed file types
  - [ ] 8.2 Watcher → scanner consistency test (modify files via watcher, verify scan matches)
  - [ ] 8.3 10,000 file scan benchmark (measure time and memory)
  - [ ] 8.4 Property-based tests for deterministic scan, identity stability, change detection

- [ ] 9. Documentation and module wiring
  - Document all public APIs with doc comments
  - Create `lib.rs` re-exports
  - Wire up ocean-fs as a standalone crate

  - [ ] 9.1 Add doc comments to all public items
  - [ ] 9.2 Create crate-level documentation
  - [ ] 9.3 Wire up `Cargo.toml` with required dependencies (walkdir, rayon, notify, sha2, uuid, rusqlite)

---

## Notes

### Dependencies
- Task 2, 3 can run in parallel after Task 1
- Task 5 depends on SQLite integration from Task 1 (or can use in-memory HashMap for initial MVP)
- Task 8 depends on Tasks 2, 3, 4, 6, 7
- Task 9 is final and depends on everything else

### Crate Dependencies
- `walkdir` — recursive directory traversal
- `rayon` — parallel iteration
- `sha2` — SHA-256 hashing
- `uuid` (v7 feature) — FileId generation
- `notify` — cross-platform filesystem watcher
- `rusqlite` — SQLite for path mapping table (use `bundled` feature)
- `mime_guess` — MIME type detection from extension

### Testing Approach
- Use `tempfile` crate for temporary directories in tests
- Property-based tests via `proptest` crate (100 iterations per property)
- Benchmark with `criterion` for scan performance

### Implementation Tips
- Start with an in-memory path resolver (HashMap) for MVP, upgrade to SQLite later
- Use `crossbeam-channel` for watcher event queue before batching
- Make the ignore list a `Vec<GlobPattern>` for configurable filtering

### Risk Mitigation
- [ ] Symlink loop detection verified early
- [ ] Permission-denied files handled without panic
- [ ] Watcher buffer overflow recovery implemented
- [ ] Memory usage verified for 100M+ files during scan
