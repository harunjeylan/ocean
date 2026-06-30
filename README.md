# ocean

[![CI](https://github.com/harunjeylan/ocean/actions/workflows/ci.yml/badge.svg)](https://github.com/harunjeylan/ocean/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ocean-doc.svg)](https://crates.io/crates/ocean-doc)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/ocean-doc)](https://crates.io/crates/ocean-doc)
[![Rust](https://img.shields.io/badge/rust-2024-dea584.svg)](https://www.rust-lang.org)

A local multi-index document runtime that turns unstructured files into a queryable knowledge system.

ocean is a document intelligence CLI that scans directories, parses documents into structured blocks, splits them into semantic chunks, generates embeddings via Ollama/OpenAI/Anthropic/Gemini, builds knowledge graphs, stores everything in SurrealDB, and provides unified vector + hybrid + graph-expanded search.

---

## Quick Install

### Linux / macOS

```bash
curl -fsSL https://github.com/harunjeylan/ocean/releases/latest/download/install.sh | bash
```

Downloads the pre-built binary for your platform (Linux x86_64 or macOS ARM64), places it in `~/.ocean/bin/`, and adds it to your PATH via `.bashrc` / `.zshrc` / `.profile`.

### Windows

```powershell
powershell -c "irm https://github.com/harunjeylan/ocean/releases/latest/download/install.ps1 | iex"
```

Downloads the pre-built binary, places it in `%LOCALAPPDATA%\ocean\bin\`, and adds it to the user PATH.

## Build from Source

Requires the Rust toolchain (edition 2024).

```sh
cargo build --release
```

Produces `target/release/ocean.exe`.

---

## CLI Commands

### Initialization

Initialize ocean in the current directory with an interactive setup that prompts for embedding configuration, writes `.ocean/config.json`, and appends integration notes to CLAUDE.md / AGENTS.md.

```
ocean init
ocean init --dir <path>
```

### File Commands

| Command | Description |
|---------|-------------|
| `ocean info <file>` | Metadata + outline in one view |
| `ocean metadata <file>` | All metadata fields |
| `ocean outline <file>` | Hierarchical table of contents |
| `ocean page-count <file>` | Page or slide count |
| `ocean search <file> <query>` | Full-text search within a single file |
| `ocean grep <dir> <query>` | Recursive full-text search across all supported files |
| `ocean hash <file>` | Compute SHA-256 hex digest |
| `ocean verify <file> <hash>` | Verify file hash (prints `true` / `false`) |
| `ocean scan <dir> [--no-hash]` | List supported files with size, hash, and extension |
| `ocean watch <dir>` | Monitor a directory for file changes (Ctrl+C to stop) |

### Read Command

Read structured content from documents using selectors.

```
ocean read <file> --page <N>
ocean read <file> --heading <text>
ocean read <file> --paragraph <N>
ocean read <file> --table <N>
ocean read <file> --slide <N>
ocean read <file> --sheet <name>
ocean read <file> --cell <ref>
ocean read <file> --image <N>
ocean read <file> --range <S-E>
ocean read <file> --skip <N> --take <N>
```

Selectors are format-aware — `--page` works on PDF/DOCX, `--slide` on PPTX, `--sheet`/`--cell` on XLSX, `--heading` on Markdown/HTML/DOCX/PDF, `--paragraph` on TXT/MD/HTML/DOCX, `--table` on DOCX/HTML/XLSX, `--image` on DOCX/PPTX/HTML. The `--skip`/`--take` pair provides generic slicing across all formats.

### Chunk Command

Split a document into semantic chunks with configurable token bounds.

```
ocean chunk <file> [--min-size N] [--max-size N] [--overlap N] [--include-images] [--rows-per-chunk N]
```

Produces chunks of types: Text, Table, Page, Slide, Sheet, Cell, Image, Metadata, Heading. Defaults are min=100, max=800 tokens, overlap=1 sentence.

### Index Command

Recursively scan, parse, chunk, embed, and store documents in SurrealDB with an HNSW vector index.

```
ocean index <dir> [--provider] [--model] [--db-path] [--reindex] [--no-graph] [--mode]
```

Supports Ollama (default), OpenAI, Anthropic, and Gemini embedding providers. Optional graph building with reference and entity extraction.

### Query Command

Unified semantic search over indexed documents.

```
ocean query <query> [--mode auto|vector|hybrid|expand] [--top-k N] [--context] [--expand-depth N] [--provider]
```

**Mode selection (auto):**
- `expand-depth > 0` → Expand
- Fewer than 3 words → Vector
- Cross-reference phrases → Expand
- 3 or more words → Hybrid
- Empty query → Hybrid

**Hybrid** combines vector KNN with full-text search using RRF fusion (k=60). **Expand** runs hybrid search then enriches results with graph traversal. Context windows pull adjacent chunks within the same file and heading scope (clamped to 1–10). Results can be reranked by heading or file for diversity.

### Vector-Search Command

Legacy command preserved for backwards compatibility.

```
ocean vector-search <query> [--top-k] [--hybrid] [--expand-depth]
```

### Graph Commands

Explore and inspect the knowledge graph built during indexing.

```
ocean graph info <file>          # Node/edge counts by type
ocean graph expand <node-id>     # BFS traversal (--depth, --direction)
ocean graph path <from> <to>     # Shortest path (--max-depth)
ocean graph stats                # Global graph statistics
```

### Config Commands

```
ocean config show                # Display current config as JSON
ocean config validate            # Validate config values
```

### Global Flags

Available on every command:

- `--help`, `--version`
- `--log-format text|json`
- `--log-file <path>`

---

## Supported Formats

| Format | Extension | Backend |
|--------|-----------|---------|
| PDF | `.pdf` | lopdf |
| DOCX | `.docx` | zip + quick-xml |
| PPTX | `.pptx` | zip + quick-xml |
| XLSX | `.xlsx` | calamine |
| Text | `.txt` | std::fs |
| Markdown | `.md` | std::fs |
| HTML | `.html`, `.htm` | quick-xml |
| Image | `.png`, `.jpg`, `.jpeg` | metadata only |

---

## Configuration

ocean is configured through a layered system of CLI flags, config files, and environment variables.

### Load Order

1. CLI flags (highest priority)
2. `.ocean/config.json` (local, per-project)
3. `~/.ocean/config.json` (global, user-level)
4. `.env` files (`~/.ocean/.env` → `CWD/.env` → `CWD/.ocean/.env`, last wins)
5. Hardcoded defaults

### Config Sections

| Section | Keys | Description |
|---------|------|-------------|
| `embedding` | `provider`, `model`, `dimension`, `api_key`, `base_url` | Embedding provider settings |
| `index` | `batch_size`, `db_path`, `reindex`, `no_graph` | Indexing pipeline settings |
| `query` | `top_k`, `db_path`, `mode`, `expand_depth`, `context`, `context_chunks`, `verbose` | Search defaults |
| `runtime` | `mode` | Runtime mode (desktop/server/embedded) |
| `security` | `sandbox`, `read_only` | Security settings |
| `observability` | `log_format` | Logging output format |

Config values support `${VAR}` environment variable syntax. The default database path is `~/.ocean/database/{cwd-kebab-case}.db`, computed automatically from the current working directory name.

### Example `.ocean/config.json`

```json
{
  "embedding": {
    "provider": "ollama",
    "model": "nomic-embed-text",
    "dimension": 768,
    "base_url": "http://localhost:11434"
  },
  "index": {
    "batch_size": 50,
    "db_path": "~/.ocean/database/my-project.db",
    "reindex": false,
    "no_graph": false
  },
  "query": {
    "top_k": 10,
    "mode": "auto",
    "expand_depth": 0,
    "context": true,
    "context_chunks": 3,
    "verbose": false
  },
  "runtime": {
    "mode": "desktop"
  },
  "security": {
    "sandbox": true,
    "read_only": false
  },
  "observability": {
    "log_format": "text"
  }
}
```

---

## Embedding Providers

| Provider | Default Model | Default Dim | Default URL |
|----------|---------------|-------------|-------------|
| Ollama | nomic-embed-text | 768 | http://localhost:11434 |
| OpenAI | text-embedding-3-small | 1536 | https://api.openai.com/v1 |
| Anthropic | cohere-embed-multilingual-v3 | 768 | https://api.anthropic.com/v1 |
| Gemini | gemini-embedding-001 | 3072 | built-in |

All embedders auto-normalize vectors to unit length. API keys can be provided via CLI flags, config file, or environment variables.

---

## Architecture

The processing pipeline follows a strict one-way derivation chain where the filesystem is always the source of truth.

```
Filesystem
  → ocean_fs       scan, hash, filter, watch, normalize
    → ocean_parser   parse 7 formats into normalized ReadResult blocks
      → ocean_chunk   block → semantic chunks with heading detection
                      and sentence-boundary splitting
        → ocean_vector  embed + HNSW vector index
        → ocean_graph   node/edge extraction, entity detection, BFS expansion
        → ocean_storage SurrealDB persistence
          → ocean_query  unified vector/hybrid/expand/graph search
```

### Key Design Rules

1. **Filesystem is the source of truth** — all indexes and caches can be rebuilt from files at any time.
2. **Everything is derived** — Files → Blocks → Chunks → Embeddings → Graph. Each stage transforms the output of the previous stage.
3. **Format isolation** — No format awareness exists outside the parser layer. All downstream code works with normalized `ReadResult` and `Chunk` types.
4. **Traceability** — Every data unit is traceable back to its source file and location within that file.
5. **Deterministic** — The same set of input files always produces the same index output.
6. **Observability** — File-level progress, indexing metrics (elapsed, chunks indexed, errors), and query execution metadata are surfaced to the user.

### Runtime Modes

| Mode | Threads | Cache TTL | Use Case |
|------|---------|-----------|----------|
| `desktop` | 4 | 50ms | Interactive use (default) |
| `server` | 8 | 200ms | High-throughput, larger batches |
| `embedded` | 2 | low memory | Minimal footprint, aggressive freshness |

Mode is auto-detected from the environment and can be overridden via the `--mode` flag or `runtime.mode` in the config file.

### Security

- **Filesystem sandbox** — Validates paths against the workspace root, detects symlink escape attempts, and whitelists supported extensions.
- **Read-only mode** — Blocks write operations (index, scan, watch) for query-only deployments.

Both are configurable via CLI flags and the `security.sandbox` / `security.read_only` config keys.

---

## Module Map

| Crate Module | Phase | Responsibility |
|-------------|-------|----------------|
| `ocean_fs` | Phase 1 | File scanning, identity (UUIDv7), hashing (SHA-256), filtering, watching (notify + crossbeam), path resolution, SurrealDB persistence |
| `ocean_parser` | Phase 2 | 7 format backends (PDF, DOCX, PPTX, XLSX, TXT, MD, HTML) with outline, read, search, and skip/take slicing |
| `ocean_chunk` | Phase 3 | Semantic chunking with heading detection, sentence-boundary splitting, overlap, buffer management, post-processing merge |
| `ocean_vector` | Phase 4 | Embedder abstraction (Ollama/OpenAI/Anthropic/Gemini), vector store (SurrealDB HNSW), index pipeline, hybrid search (RRF fusion) |
| `ocean_graph` | Phase 5 | Node/edge extraction, entity extraction (capitalized phrases, repeated words), graph store, BFS expansion, path finding |
| `ocean_query` | Phase 6 | Unified query engine with auto/hybrid/vector/expand modes, context window builder, reranking, execution metadata |
| `ocean_storage` | — | SurrealDB connection management, schema definitions |
| `ocean_index` | — | Index pipeline orchestration |
| `ocean_api` | — | Public API layer |
| `ocean_cache` | — | Caching layer |
| `ocean_cli` | — | CLI argument parsing, display formatting, command dispatch, config loading |

---

## Development

### Running Tests

```sh
cargo test                          # 345+ unit tests + 30 integration tests
cargo test --lib                    # Unit tests only
cargo test --test fs_integration    # Filesystem integration tests
cargo test --test parser_integration    # Parser integration tests
cargo test --test parser_real_files     # Real-file acceptance tests
cargo test --lib <test_name>        # Specific test
```

### Building

```sh
cargo build                         # Debug build
cargo build --release               # Optimized release build
cargo run -- ocean <args>           # Run default binary
cargo run --bin cli -- <args>       # Run explicit CLI binary
```

### Project Structure

```
src/
├── main.rs                        # Entry point, delegates to ocean_cli::run()
├── cli.rs                         # Explicit CLI binary entry point
├── lib.rs                         # Crate root, pub mod declarations
├── tests.rs                       # #[path] test module registry
├── ocean_fs/                      # Phase 1 — filesystem operations
├── ocean_parser/                  # Phase 2 — 7 format backends
├── ocean_chunk/                   # Phase 3 — semantic chunking
├── ocean_vector/                  # Phase 4 — embeddings + vector store
├── ocean_graph/                   # Phase 5 — knowledge graph
├── ocean_query/                   # Phase 6 — unified search
├── ocean_storage/                 # SurrealDB store
├── ocean_index/                   # Index pipeline
├── ocean_api/                     # Public API
├── ocean_cache/                   # Caching
└── ocean_cli/                     # CLI args, display, config, command dispatch
tests/
├── fs_integration.rs              # ocean_fs integration tests
├── parser_integration.rs          # ocean_parser integration tests
├── parser_real_files.rs           # Real-file acceptance tests
└── test-cwd/                      # Test fixtures (PDF, DOCX, PPTX, XLSX, TXT, MD, HTML)
.specs/
├── ocean-fs/                      # Phase 1 requirements and design
├── ocean-parser/                  # Phase 2 requirements and design
├── ocean-chunk/                   # Phase 3 requirements and design
└── project-plan.md                # Full architecture plan
```

---

## Project Status

| Phase | Module | Status |
|-------|--------|--------|
| Phase 1 | ocean_fs | Complete — scan, hash, filter, watch, normalize, path resolution, SurrealDB persistence |
| Phase 2 | ocean_parser | Complete — all 7 format backends with outline, read, search, skip/take |
| Phase 3 | ocean_chunk | Complete — semantic chunking with heading detection, sentence-boundary split, overlap, post-processing |
| Phase 4 | ocean_vector | Complete — embedder abstraction (4 providers), vector store (HNSW), index pipeline, hybrid search (RRF) |
| Phase 5 | ocean_graph | Complete — node/edge extraction, entity detection, BFS expansion, path finding |
| Phase 6 | ocean_query | Complete — unified query engine, auto mode, context windows, reranking, execution metadata |

---

## License

MIT
