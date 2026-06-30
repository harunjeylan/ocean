# ocean — Document Reader

A unified CLI for reading, inspecting, and searching documents across all major formats (PDF, DOCX, XLSX, PPTX, TXT, Markdown, HTML).

## Build

```powershell
cargo build --release
```

Produces two binaries: `ocean.exe` (default, via `src/main.rs`) and `cli.exe` (explicit, via `src/cli.rs`).

```powershell
target\release\ocean.exe
target\release\cli.exe
```

## Usage

```
ocean <COMMAND> [OPTIONS] <FILE>
```

Or with the explicit binary: `cli <COMMAND> [OPTIONS] <FILE>`. During development, use `cargo run -- <args>` (default) or `cargo run --bin cli -- <args>`.

### Global flags

| Flag | Description |
|------|-------------|
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

---

## Configuration

Ocean automatically loads settings from multiple locations (merged, local takes priority):

| Path | Scope |
|------|-------|
| `CWD/.ocean/config.json` | Project / current directory (highest file priority) |
| `~/.ocean/config.json` | Global user-level (Unix) |
| `%APPDATA%/ocean/config.json` | Global user-level (Windows) |

**Environment files** are loaded at startup (last file loaded wins):

| Path | Priority |
|------|----------|
| `~/.ocean/.env` | Lowest (global defaults) |
| `CWD/.env` | Medium |
| `CWD/.ocean/.env` | Highest (project overrides) |

**Example `.ocean/config.json`:**

```json
{
  "embedding": {
    "provider": "gemini",
    "model": "gemini-embedding-001",
    "dimension": 3072,
    "api_key": "${GOOGLE_GEMINI}",
    "base_url": ""
  },
  "index": {
    "batch_size": 10
  },
  "query": {
    "top_k": 10,
    "mode": "auto"
  }
}
```

**Default database path:** `~/.ocean/database/{cwd-kebab-case}.db` — auto-computed from current working directory name.

**Resolution order** (highest to lowest):
1. CLI flags (explicit per-invocation)
2. `.ocean/config.json` (project-level)
3. `~/.ocean/config.json` (global fallback)
4. `.env` files (in priority order above)
5. Hardcoded defaults

The `${VARIABLE}` syntax resolves from environment variables (including those loaded from `.env`).
Settings are merged — a local config only needs to specify the fields that differ from defaults.

---

## Commands

### info — Document summary

Show metadata + outline in one view.

```
ocean info <file>
```

```
ocean info report.pdf
ocean info presentation.pptx
ocean info "meeting notes.docx"
```

### metadata — Document metadata

Show path, format, size, title, author, created/modified dates, page count.

```
ocean metadata <file>
```

```
ocean metadata article.pdf
```

### outline — Table of contents

Show hierarchical heading outline (PDF/DOCX/MD/HTML), flat slide list (PPTX), or flat sheet list (XLSX).

```
ocean outline <file>
```

```
ocean outline presentation.pptx
  - [L1] Slide 1 Title  (Slide(1))
  - [L1] Slide 2 Title  (Slide(2))
```

```
ocean outline chapter.md
  - [L1] Introduction  (Heading("Introduction"))
    - [L2] Background  (Heading("Background"))
  - [L1] Conclusion  (Heading("Conclusion"))
```

### page-count — Page/slide count

```
ocean page-count <file>
```

```
ocean page-count book.pdf
  342
```

Returns `(none)` for formats without pages (DOCX, XLSX, TXT, MD, HTML).

### search — Full-text search (single file)

Case-insensitive search across a single document.

```
ocean search <file> <query>
```

```
ocean search report.pdf "budget"
  3 match(es) for 'budget':
    Page(2): "...annual budget for 2025..."
    Page(5): "...budget allocation..."
    Page(7): "...budget review..."
```

### grep — Full-text search (all files in a directory)

Recursively search all supported documents in a directory for a query. Scans PDF, DOCX, XLSX, PPTX, TXT, MD, HTML files.

```
ocean grep <dir> <query>
```

```
ocean grep ./documents "budget"
  ./documents/report.pdf:
    Page(2): "...annual budget..."
    Page(5): "...budget allocation..."
  ./documents/notes.docx:
    Paragraph(12): "...budget review..."
  Total: 3 match(es) in 42 file(s) for 'budget'
```

### read — Read content by selector

Read a specific part of the document using one of the selector flags.

```
ocean read <file> <selector>
```

**Selector flags** (exactly one required):

| Flag | Type | Example | Formats |
|------|------|---------|---------|
| `--page <N>` | u32 | `--page 1` | PDF |
| `--heading <TEXT>` | string | `--heading "Introduction"` | PDF, DOCX, MD, HTML |
| `--paragraph <N>` | u32 | `--paragraph 3` | TXT, MD, HTML, DOCX |
| `--table <N>` | u32 | `--table 0` | DOCX, HTML, XLSX |
| `--slide <N>` | u32 | `--slide 1` | PPTX |
| `--sheet <NAME>` | string | `--sheet "Data"` | XLSX |
| `--cell <REF>` | string | `--cell "B12"` | XLSX |
| `--image <N>` | u32 | `--image 0` | DOCX, PPTX, HTML |
| `--range <S-E>` | string | `--range "0-100"` | All formats |
| `--skip <N>` | u32 | `--skip 200` (with `--take`) | PDF, DOCX |
| `--take <N>` | u32 | `--take 10` (with `--skip`) | All formats |

**Examples:**

```
ocean read report.pdf --page 1
ocean read chapter.md --heading "Getting Started"
ocean read notes.txt --paragraph 0
ocean read presentation.pptx --slide 3
ocean read data.xlsx --sheet "Sheet1"
ocean read data.xlsx --cell "C5"
ocean read document.docx --table 0
ocean read report.pdf --skip 200 --take 10      # pages 201-210
ocean read notes.txt --take 5                    # first 5 lines
ocean read document.docx --skip 2 --take 3       # pages 3-5
ocean read slides.pptx --skip 0 --take 2         # slides 1-2
ocean read data.xlsx --skip 1 --take 2           # sheets 2-3
```

---

## Supported formats

| Format | Extension | Backend | Read selectors |
|--------|-----------|---------|----------------|
| PDF | `.pdf` | lopdf | Page, Heading, Range |
| DOCX | `.docx` | zip + quick-xml | Paragraph, Heading, Table, Image, Range |
| XLSX | `.xlsx` | calamine | Sheet, Cell, Table, Range |
| PPTX | `.pptx` | zip + quick-xml | Slide, Image, Paragraph, Note, Range |
| TXT | `.txt` | std::fs | Paragraph, Range |
| Markdown | `.md` | std::fs | Heading, Paragraph, Range |
| HTML | `.html`, `.htm` | quick-xml | Heading, Paragraph, Table, Image, Range |

Unsupported extensions return `Error: unsupported format`.

---

## File System Commands

### scan — Scan directory for supported documents

List all supported files with metadata (ID, size, extension, path).

```
ocean scan <dir> [--no-hash]
```

By default scans compute SHA-256 hashes for each file. Use `--no-hash` to skip hashing for faster scans on large directories.

```
ocean scan ./documents
  Found 43 file(s) in './documents':
    019f14f7       0.1 KB  html  ./documents/index.html
    019f18f2     12.3 KB  pdf   ./documents/report.pdf
    019f1a45      4.2 KB  docx  ./documents/notes.docx
```

```
ocean scan ./documents --no-hash
  Found 43 file(s) in './documents':
        0.1 KB  html  ./documents/index.html
       12.3 KB  pdf   ./documents/report.pdf
```

### hash — Compute file hash

Compute the SHA-256 hash of a file.

```
ocean hash <file>
```

```
ocean hash report.pdf
  dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f
```

### verify — Verify file hash

Check whether a file matches an expected SHA-256 hash. Prints `true` or `false`.

```
ocean verify <file> <expected-hash>
```

```
ocean verify report.pdf dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f
  true
```

### watch — Watch directory for file changes

Monitor a directory for file creation, modification, deletion, and renaming. Runs until Ctrl+C.

```
ocean watch <dir>
```

```
ocean watch ./documents
  Watching './documents'... Press Ctrl+C to stop.
  [CREATED]  ./documents/new_notes.txt
  [MODIFIED] ./documents/report.pdf
  [DELETED]  id=019f14f7...
  [RENAMED]  ./documents/old.txt -> ./documents/new.txt
```

---

## Chunk Command

### chunk — Semantic chunking of document content

Parse a document and split it into semantic chunks (text blocks, tables, slides, sheets) based on token-size limits. Each chunk is displayed with its ID, type, heading context, and token estimate.

```
ocean chunk <file> [--min-size <N>] [--max-size <N>] [--overlap <N>] [--include-images] [--rows-per-chunk <N>]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--min-size` | 100 | Minimum tokens per chunk |
| `--max-size` | 800 | Maximum tokens per chunk |
| `--overlap` | 1 | Overlap sentences between consecutive chunks |
| `--include-images` | false | Include image blocks in chunks |
| `--rows-per-chunk` | 50 | Rows per spreadsheet chunk |

**Chunk types:** Text, Heading, Table, Slide, Sheet, Cell, Image, Page

```
ocean chunk chapter.md
  5 chunks from 'chapter.md':
    [019f21a1] Text      h="Introduction"  342 tokens
    [019f21a2] Heading   h="Background"      0 tokens
    [019f21a3] Text      h="Background"    567 tokens
    [019f21a4] Heading   h="Conclusion"      0 tokens
    [019f21a5] Text      h="Conclusion"    123 tokens
```

```
ocean chunk presentation.pptx --max-size 400
  12 chunks from 'presentation.pptx':
    [019f21b1] Slide     h="Slide 1"       89 tokens
    [019f21b2] Slide     h="Overview"     234 tokens
    ...
```

```
ocean chunk data.xlsx --rows-per-chunk 100
  3 chunks from 'data.xlsx':
    [019f21c1] Sheet     h="Sheet1"      450 tokens
    [019f21c2] Sheet     h="Sheet2"      210 tokens
    [019f21c3] Sheet     h="Summary"      90 tokens
```

---

## Graph Commands

### graph info — Show graph info for a file

Show node count, edge count, and node-type breakdown for a file's subgraph.

```
ocean graph info <file> [--db-path <path>]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--db-path` | `ocean.db` | Path to SurrealDB database |

```
ocean graph info report.pdf
  Graph Info:
    Total nodes: 5
    Total edges: 7
    Breakdown by type:
      File: 1
      Chunk: 3
      Heading: 1
```

### graph expand — Expand from a node

Traverse the graph starting from a seed node up to a given depth.

```
ocean graph expand <node-id> [--depth <N>] [--direction <dir>] [--db-path <path>]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--depth` | 2 | Max expansion depth (1–5) |
| `--direction` | `both` | `forward`, `backward`, or `both` |
| `--db-path` | `ocean.db` | Path to SurrealDB database |

```
ocean graph expand chunk:019f21a1 --depth 2 --direction both
  Expanded from 'chunk:019f21a1' (depth: 2):
    [File] file:f1  "-"
    [Chunk] chunk:c1  "-"
    [Heading] heading:h1  "Intro"
    [Chunk] chunk:c2  "-"
  Edges:
    chunk:c1  --BelongsTo-->  heading:h1  (w: 1.0)
    file:f1  --Contains-->  chunk:c1  (w: 1.0)
    chunk:c1  --BelongsTo-->  file:f1  (w: 1.0)
    chunk:c1  --References-->  chunk:c2  (w: 0.7)
```

### graph path — Find shortest path between two nodes

```
ocean graph path <from-id> <to-id> [--max-depth <N>] [--db-path <path>]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--max-depth` | 5 | Max search depth (1–10) |
| `--db-path` | `ocean.db` | Path to SurrealDB database |

```
ocean graph path chunk:c1 heading:h1
  Path (2 hops):
    1. chunk:c1 --BelongsTo--> heading:h1 (w: 1.0)
```

### graph stats — Show global graph statistics

```
ocean graph stats [--db-path <path>]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--db-path` | `ocean.db` | Path to SurrealDB database |

```
ocean graph stats
  Graph Stats:
    Total nodes: 150
    Total edges: 320
    By type:
      File: 10
      Chunk: 100
      Heading: 40
```

---

## Index Command

### index — Scan, parse, chunk, embed, and index documents for vector search

Recursively scan a directory for supported documents, parse each file, split into semantic chunks, compute embeddings, and store in SurrealDB with HNSW vector index.

```
ocean index <dir> [--model <name>] [--provider <name>] [--ollama-url <url>]
                [--api-key <key>] [--dimension <N>]
                [--db-path <path>] [--batch-size <N>] [--reindex]
                [--no-graph] [--no-references] [--no-entities]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--model` | `nomic-embed-text` | Embedding model name |
| `--provider` | `ollama` | Embedding provider (`ollama`, `openai`, `anthropic`, `gemini`) |
| `--ollama-url` | config / heuristics | Override base URL for Ollama (falls back to config `base_url` or `http://localhost:11434`) |
| `--api-key` | config / env / — | API key (required for openai/anthropic/gemini) |
| `--dimension` | auto | Embedding dimension (overrides config & heuristics) |
| `--db-path` | auto (`~/.ocean/database/{cwd}.db`) | SurrealDB database path |
| `--batch-size` | 10 | Chunks per embedding batch |
| `--reindex` | false | Re-index existing files (update chunks) |
| `--no-graph` | false | Skip graph building |
| `--no-references` | false | Skip reference edge extraction during graph build |
| `--no-entities` | false | Skip entity extraction during graph build |

```
ocean index ./documents
  Found 5 supported file(s) in './documents'. Indexing...
  [1/5] Processing: report.pdf
    Indexed: 12 embedded, 0 skipped, 0 failed (342ms)
    Graph: 5 nodes, 8 edges
  [2/5] Processing: notes.docx
    Indexed: 8 embedded, 0 skipped, 0 failed (215ms)
    Graph: 4 nodes, 6 edges
  ...
  Graph total: 25 nodes, 40 edges
  Indexing complete.
```

---

## Query Command

### query — Unified query over indexed documents

Unified query command supporting auto mode selection, vector-only, hybrid (vector + FTS), and graph-expanded search with context windows and execution metadata.

```
ocean query <query> [--mode <mode>] [--top-k <N>] [--expand-depth <N>]
                     [--context] [--context-chunks <N>]
                     [--rerank-by-heading] [--rerank-by-file]
                     [--file-id <id>] [--heading <prefix>] [--block-type <type>]
                     [--model <name>] [--provider <name>]
                     [--ollama-url <url>] [--api-key <key>] [--dimension <N>]
                     [--db-path <path>] [--verbose]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--mode` | `auto` | Query mode: `auto`, `vector`, `hybrid`, `expand` |
| `--top-k` | 10 | Max results |
| `--expand-depth` | 0 | Graph expansion depth (0 = disabled) |
| `--context` | false | Include context windows around matched chunks |
| `--context-chunks` | 3 | Max chunks per context window (auto-clamped 1–10) |
| `--rerank-by-heading` | false | Penalize results from same heading for diversity |
| `--rerank-by-file` | false | Penalize results from same file for diversity |
| `--file-id` | — | Filter by file ID |
| `--heading` | — | Filter by heading prefix |
| `--block-type` | — | Filter by block type (Text, Heading, Table, etc.) |
| `--model` | `nomic-embed-text` | Embedding model name |
| `--provider` | `ollama` | Embedding provider |
| `--api-key` | config / env / — | API key (required for openai/anthropic/gemini) |
| `--dimension` | auto | Embedding dimension (overrides config & heuristics) |
| `--db-path` | auto (`~/.ocean/database/{cwd}.db`) | SurrealDB database path |
| `--verbose` | false | Show ExecutionMeta timing info |

**Auto mode heuristics:**
- `expand-depth > 0` → Expand
- Short query (<3 words) → Vector
- Contains cross-ref phrases ("related to", "reference") → Expand
- Natural language phrase (3+ words) → Hybrid

```
ocean query "budget allocation" --mode hybrid --top-k 5 --context --verbose
  Top 5 results for 'budget allocation':
    1. score=0.8521 (vec=0.8521, fts=0.7234)  file=019f14f7  heading="Budget"
       "...the annual budget allocation for 2025..."

  --- Context Windows ---
  Window 1 (anchor: chunk:abc, tokens: 685):
    [↑] ...previous section content... (dist=-1)
    [*] ...the annual budget allocation for 2025... (dist=0)
    [↓] ...next section content... (dist=1)

  --- Execution ---
  Mode: Hybrid
  Total: 5 results in 342ms
  Vector search: 120ms
  Fusion: 5ms
```

```
ocean query "budget" --mode auto --rerank-by-file
```

## Vector Search Command

### vector-search — Semantic vector search over indexed documents

Search across all indexed documents using cosine similarity, optionally with hybrid (vector + FTS) search, filtering, and graph context expansion.

```
ocean vector-search <query> [--top-k <N>] [--hybrid] [--file-id <id>]
                             [--heading <prefix>] [--block-type <type>]
                             [--model <name>] [--provider <name>]
                             [--ollama-url <url>] [--api-key <key>]
                             [--dimension <N>] [--db-path <path>]
                             [--expand-depth <N>]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--top-k` | 10 | Max results |
| `--hybrid` | false | Combine vector + FTS search with RRF fusion |
| `--file-id` | — | Filter by file ID |
| `--heading` | — | Filter by heading prefix |
| `--block-type` | — | Filter by block type (Text, Heading, Table, etc.) |
| `--model` | `nomic-embed-text` | Embedding model name |
| `--provider` | `ollama` | Embedding provider |
| `--ollama-url` | config / heuristics | Override base URL for Ollama (falls back to config `base_url` or `http://localhost:11434`) |
| `--api-key` | config / env / — | API key (required for openai/anthropic/gemini) |
| `--dimension` | auto | Embedding dimension (overrides config & heuristics) |
| `--db-path` | auto (`~/.ocean/database/{cwd}.db`) | SurrealDB database path |
| `--expand-depth` | 0 | Graph expansion depth (0 = disabled) |

```
ocean vector-search "budget allocation" --top-k 5 --expand-depth 1
  Top 5 expanded results for 'budget allocation':
    1. score=0.8521 (vec=0.8521)  file=019f14f7  heading="Budget"
       "...the annual budget allocation for 2025..."
    2. score=0.7234  file=019f14f7  heading="Budget"
       "...related budget planning document (see Finance Report)..."
    ...
```

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (file not found, unsupported format, invalid selector, parse failure) |
