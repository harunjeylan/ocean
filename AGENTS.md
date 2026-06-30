# AGENTS.md — Ocean (DocTools)

## Project structure
- Single crate `ocean` (edition 2024), not a workspace.
- Library entrypoint: `src/lib.rs` exports `pub mod ocean_fs;`, `pub mod ocean_parser;`, `pub mod ocean_cli;`
- Two binary targets: `ocean` (default, `src/main.rs`) and `cli` (`src/cli.rs`, via `[[bin]]` in Cargo.toml)
- `src/main.rs` is a thin 5-line delegate calling `ocean::ocean_cli::run()`
- `src/cli.rs` is the explicit CLI entry point with same content
- Module code under `src/ocean_fs/`, `src/ocean_parser/`, `src/ocean_cli/` — each with `mod.rs` + `pub use` re-exports
- Integration tests: `tests/fs_integration.rs`, `tests/parser_integration.rs`, `tests/parser_real_files.rs`
- Real test fixtures in `tests/test-cwd/` (PDF, DOCX, PPTX, XLSX, TXT, MD, HTML files)
- Spec docs in `.specs/ocean-fs/` and `.specs/ocean-parser/`. Plan in `project-plan.md`.
- CLI docs in `cli-docs.md`.

## Module: ocean_fs (Phase 1)
- **File identity**: UUIDv7 (`uuid::Uuid::now_v7()`) via `generate_file_id()` in `types.rs`
- **Persistence**: SurrealDB (embedded RocksDb/in-memory). `PathResolver` wraps internal `tokio::runtime::Runtime` for sync-to-async bridging.
- **Scanner**: `walkdir` + `rayon` parallel. `WalkDir::filter_entry` for directory filtering, then separate file-level filters.
- **Hasher**: streaming SHA-256 with 64KB buffer, rejects files >4GB.
- **Watcher**: `notify` + `crossbeam_channel`. 100ms debounce, MAX_BATCH_SIZE=100.
- **Filter**: ignores `.git/`, `node_modules/`, `.cache/` + hidden files; supports pdf/docx/pptx/xlsx/txt/md/html/htm/png/jpg/jpeg.

## Module: ocean_chunk (Phase 3)
- **Chunk types**: Text, Table, Page, Slide, Sheet, Cell, Image, Metadata, Heading
- **chunker.rs**: `chunk(blocks, file_id, config)` — processes `Vec<ReadResult>` into `Vec<Chunk>` using heading detection, sentence-boundary split, and buffer management.
- **buffer.rs**: `ChunkBuffer` — accumulates text, flushes on heading/slide/sheet boundary, atomic emit for tables/slides/sheets.
- **heading.rs**: `detect_heading(line)` — detects `# ` through `###### ` markdown headings with leading/trailing whitespace handling.
- **split.rs**: `split_with_overlap(text, config)` — sentence-boundary splitting with configurable overlap.
- **config**: `ChunkConfig { min_tokens, max_tokens, overlap_sentences, include_images, rows_per_sheet_chunk, token_estimator }`
- **post-processing**: merges adjacent same-type chunks under same heading if combined tokens ≤ max_tokens.

## Module: ocean_parser (Phase 2)
- **Document trait** (object-safe, `Box<dyn Document>`): `metadata()`, `outline()`, `page_count()`, `search()`, `read()`
- **Selector enum** (14 variants): Page, Pages, Heading, Paragraph, Table, Row, Column, Cell, Sheet, Slide, Image, Note, Range, Slice
- **ReadResult enum** (10 variants): Text, Table, Image, Metadata, Outline, Page, Slide, Sheet, CellValue, MatchResult
- **7 format backends**:
  - TXT (`txt.rs`) — std::fs lines, Paragraph/Range/Slice selectors
  - Markdown (`markdown.rs`) — std::fs lines, Heading/Paragraph/Range/Slice
  - HTML (`html.rs`) — quick-xml parser, Heading/Paragraph/Table/Image/Range/Slice
  - DOCX (`docx.rs`) — zip + quick-xml, Paragraph/Heading/Table/Image/Slice (page-break-aware)
  - PPTX (`pptx.rs`) — zip + quick-xml, Slide/Image/Paragraph/Note/Slice
  - XLSX (`xlsx.rs`) — calamine, Sheet/Cell/Table/Slice
  - PDF (`pdf.rs`) — lopdf, Page/Pages/Heading/Range/Slice
- **BackendRegistry**: `Arc<dyn DocumentFactory>`, `OnceLock` for static global, auto-initialised on first `open()` call
- **Free functions**: `open()`, `read()`, `read_page()`, `read_heading()`, `read_all_blocks()`, etc. in `read_api.rs`
- **`read_all_blocks()`**: returns structured `Vec<ReadResult>` per format — pages for PDF, slides for PPTX, sheets for XLSX, full-text Slice for others.
- **Outline tree**: recursive `build_tree` for heading hierarchies (Markdown, HTML, DOCX, PDF)

## Module: ocean_cli (CLI)

### Files
- `args.rs` — `Cli` struct, `Commands` enum (12 commands), `ReadArgs` + `ChunkArgs` structs
- `display.rs` — `print_meta()`, `print_outline()`, `print_read_result()` — all output formatting
- `walk.rs` — `walk_supported_files()`, `SUPPORTED_EXTS` constant
- `run.rs` — `run()` dispatch + `cmd_*` handler functions
- `mod.rs` — re-exports

### 12 CLI commands
- `info <file>` — metadata + outline in one view
- `metadata <file>` — all metadata fields
- `outline <file>` — hierarchical table of contents
- `page-count <file>` — page/slide count
- `search <file> <query>` — full-text search in single file
- `grep <dir> <query>` — recursive search across all supported docs in a directory
- `read <file>` — read by selector (--page, --heading, --paragraph, --table, --slide, --sheet, --cell, --image, --range, --skip/--take)
- `scan <dir> [--no-hash]` — list supported files with size/hash/extension
- `hash <file>` — compute SHA-256 hex
- `verify <file> <hash>` — check file hash, prints true/false
- `watch <dir>` — monitor directory for file changes (Ctrl+C to stop)
- `chunk <file> [--min-size] [--max-size] [--overlap] [--include-images] [--rows-per-chunk]` — semantic chunking
- `index <dir> [--model] [--provider] [--ollama-url] [--openai-key] [--anthropic-key] [--gemini-key] [--db-path] [--batch-size] [--reindex]` — scan, parse, chunk, embed, and store in SurrealDB
- `vector-search <query> [--top-k] [--hybrid] [--file-id] [--heading] [--block-type] [--model] [--provider] [--ollama-url] [--openai-key] [--anthropic-key] [--gemini-key] [--db-path]` — semantic vector search over indexed documents

### Skip/take slicing
- `--skip <N>` — skip N units from start (pages for PDF/DOCX, slides for PPTX, lines for TXT/MD, paragraphs for HTML, sheets for XLSX)
- `--take <N>` — read N units after skip (defaults skip=0 if used alone)
- Implemented in all 7 backends
- DOCX uses `<w:br w:type="page"/>` detection for page-level slicing

## Module: ocean_vector (Phase 4)
- **Embedder trait**: `Embedder` with `embed()`, `embed_batch()`, `dimension()`, `model_name()` — auto-normalizes to unit length
- **Backends**: `OllamaEmbedder` (local, `POST /api/embed`), `OpenAIEmbedder` (OpenAI-compatible), `AnthropicEmbedder` (x-api-key auth), `GeminiEmbedder` (Google Generative Language API)
- **VectorStore**: SurrealDB-backed (in-memory for tests, SurrealKv for persistence). Schema: `chunk` table with HNSW index on `embedding` field. Methods: `insert_chunk`, `upsert_chunk`, `insert_chunks_batch`, `get_chunk`, `delete_chunks_by_file`, `count`, `chunk_exists`, `vector_search`, `fts_search`
- **IndexPipeline**: `index_chunks(chunks, embedder, config)` — batches, dedup by content_hash (unless `reindex: true`), produces `IndexReport`
- **SearchEngine**: `search()` (KNN), `hybrid_search()` (vector + FTS with RRF fusion, k=60), `filtered_search()`, `hybrid_filtered_search()`
- **SearchFilter**: builder pattern with `file_id`, `heading_prefix`, `block_type`, `created_after`/`created_before`
- **Error types**: `EmbedderError`, `StoreError`, `IndexError`, `SearchError` — all implement `Display` + `Error`
- **MockEmbedder** in tests: returns deterministic unit vectors of configurable dimension

## Module: ocean_graph (Phase 5)
- **Node/Edge types**: `Node` (id, node_type, ref_id, label), `Edge` (from, to, relation, weight, metadata), `NodeType` (File, Chunk, Heading, Entity, Folder), `RelationType` (Contains, References, Mentions, BelongsTo, DerivedFrom, SimilarTo, CrossReference)
- **GraphStore**: SurrealDB-backed (in-memory for tests, SurrealKv for persistence) with `graph_node` and `graph_edge` SCHEMAFULL tables. CRUD: insert/get/delete by file, neighbors query, type/relation queries, count, clear
- **GraphBuilder**: `from_chunks(chunks, file_id, config)` — builds structural edges (File→Contains→Chunk, Chunk→BelongsTo→File/Heading), reference edges (see/refer to/as per/per patterns), and entity edges (capitalized phrases + repeated words)
- **EntityExtractor**: heuristic extraction of capitalized phrases (3+ words), repeated words (configurable frequency threshold)
- **ExpansionEngine**: BFS traversal with `expand(node_id, depth, direction)`, `expand_from_chunks(chunk_ids, depth)`, `find_path(from, to, max_depth)`, `get_file_graph(file_id)`.
- **GraphConfig**: extract_references (default true), extract_entities (true), max_expansion_depth (3), entity_min_frequency (3), default_edge_weight (1.0)
- **Index integration**: graph built automatically after vector indexing in `ocean index` (opt-out via `--no-graph`)
- **Context expansion**: `SearchEngine::expand_results()` enriches vector search results with graph-connected chunks

## Commands
```
cargo test                         # all (200 lib/bin/integration tests)
cargo test --lib                   # unit tests only (ocean_fs + ocean_parser + ocean_chunk)
cargo test --test fs_integration   # ocean_fs integration
cargo test --test parser_integration   # ocean_parser integration
cargo test --test parser_real_files    # real file acceptance test
cargo test --lib <test_name>       # specific test (cargo test --lib path_resolver)
cargo build                        # debug
cargo build --release              # release
cargo run --bin ocean -- <args>    # run CLI (default binary)
cargo run --bin cli -- <args>      # run explicit CLI binary
```

## Patterns & conventions
- Error types defined as enums with `Display` + `Error` impls in their own module.
- No comments in production code.
- Tests live in `_test.rs` files alongside source, included via `src/tests.rs` with `#[path]` attributes and a single `#[cfg(test)] mod tests;` in `lib.rs`. No `#[cfg(test)]` blocks in production code.
- `use crate::ocean_fs::*` / `crate::ocean_parser::*` / `crate::ocean_cli::*` / `crate::ocean_chunk::*` for sibling module access.

### Writing unit tests (`_test.rs`)

1. **Create a `_test.rs` file** alongside the source it tests. E.g. `heading_test.rs` tests `heading.rs`.
2. **Import using `crate::` paths** — never `use super::*`:
   ```rust
   use crate::ocean_chunk::heading::detect_heading;
   ```
3. **Use `#[test]` attributes** on each test function (same as normal Rust tests).
4. **Register the test file** in `src/tests.rs` with `#[path]`:
   ```rust
   #[path = "ocean_chunk/heading_test.rs"]
   mod heading_spec;
   ```
5. **Do NOT add** any `#[cfg(test)]` or test code to production files (`mod.rs`, source files, etc.).
6. **Run tests**: `cargo test` — spec files are compiled only during `cargo test` via `#[cfg(test)]` on `mod tests;` in `lib.rs`.
- `PathResolver` has `in_memory()` and `new(db_path)` constructors.
- SeaORM entities in dedicated files with `DeriveEntityModel`, `DeriveRelation`, `ActiveModelBehavior`.
- `mime_guess` for MIME fallback in normalizer.
- `quick-xml 0.31`: use `reader.trim_text(true)`, `attr.value` (not `attr.unescape_value()`).
- `calamine 0.24`: use `Data` enum (not `DataType` — it's a trait).
- `lopdf 0.32`: `Object::as_str()` returns `&[u8]`; trailer info via `trailer.get(b"Info")`.

## Specs & design
- `.specs/ocean-fs/` — Phase 1 requirements (R1–R9), design, tasks.
- `.specs/ocean-parser/` — Phase 2 requirements, design, tasks (all implemented).
- `.specs/ocean-chunk/` — Phase 3 requirements, design, tasks (all implemented).
- `project-plan.md` — full architecture (parser, chunk, vector, graph, query — ocean-fs, ocean-parser, ocean-chunk done).
- `cli-docs.md` — full CLI command reference.
- Foundation constants in `foundation.rs`: filesystem is source of truth, derivation chain is one-way, no format awareness outside parser, every data unit traceable.

## What does NOT exist yet
- No CI/CD, no README, no opencode.json, no lint/format config, no pre-commit hooks.
- Vector index, graph index, storage, query engine from `project-plan.md` not implemented.
