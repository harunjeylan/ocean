# AGENTS.md — Ocean (DocTools)

## Project structure
- Single crate `ocean` (edition 2024), not a workspace. Binary in `src/main.rs`, library in `src/lib.rs`.
- All module code under `src/ocean_fs/`. Modules registered in `src/ocean_fs/mod.rs` with `pub use` re-exports.
- Library entrypoint: `src/lib.rs` exports `pub mod ocean_fs;`
- Integration tests: `tests/fs_integration.rs` (calls through `ocean::ocean_fs::*`)
- Spec docs in `.specs/ocean-fs/` (design.md, requirements.md, tasks.md). Plan in `project-plan.md`.

## Key architecture
- **File identity**: UUIDv7 (`uuid::Uuid::now_v7()`) via `generate_file_id()` in `types.rs`
- **Persistence**: SeaORM + SQLite (via sqlx-sqlite, runtime-tokio). `PathResolver` wraps an internal `tokio::runtime::Runtime` for sync-to-async bridging — no outside async needed.
- **Scanner**: `walkdir` + `rayon` parallel. Uses `WalkDir::filter_entry` for directory-level filtering, then separate file-level filters.
- **Hasher**: streaming SHA-256 with 64KB buffer, rejects files >4GB.
- **Watcher**: `notify` crate + `crossbeam_channel` (not std mpsc). 100ms debounce, MAX_BATCH_SIZE=100.
- **Filter**: ignores `.git/`, `node_modules/`, `.cache/` + hidden files; supports pdf/docx/pptx/xlsx/txt/md/html/htm/png/jpg/jpeg.

## Commands
```
cargo test                         # all (unit + integration)
cargo test --lib                   # unit tests only
cargo test --test fs_integration   # integration tests only
cargo test --lib <test_name>       # specific test (cargo test --lib path_resolver)
cargo build                        # debug
cargo build --release              # release
```

## Patterns & conventions
- Error types defined as enums with `Display` + `Error` impls in their own module (e.g., `HashError`, `ScanError`, `WatchError`, `ResolverError`).
- No comments in production code. Test modules use `#[cfg(test)]` in-file.
- `use crate::ocean_fs::*` for sibling module access.
- `PathResolver` has `in_memory()` and `new(db_path)` constructors.
- SeaORM entities go in dedicated files (e.g., `path_entities.rs`) with `DeriveEntityModel`, `DeriveRelation`, `ActiveModelBehavior`.
- `mime_guess` crate doubles as fallback for MIME lookup in normalizer.

## Specs & design
- `.specs/ocean-fs/requirements.md` — acceptance criteria tagged `R1`–`R9`.
- `.specs/ocean-fs/design.md` — component interfaces and data flow.
- `.specs/ocean-fs/tasks.md` — task checklist (not yet fully checked off).
- `project-plan.md` — full system architecture across all phases (including parser, chunk, vector, graph — not yet implemented).
- Foundation constants in `foundation.rs` enforce: filesystem is source of truth, derivation chain is one-way, no format awareness outside parser, every data unit traceable.

## What does NOT exist yet
- No CI/CD, no README, no opencode.json, no lint/format config, no pre-commit hooks.
- Parser, chunker, vector index, graph index, storage, query engine sections from `project-plan.md` are not implemented — only ocean-fs (Phase 1) exists.
