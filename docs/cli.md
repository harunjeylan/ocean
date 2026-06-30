# ocean — Document Intelligence CLI

A unified command-line tool for parsing, inspecting, chunking, indexing, searching, and
graph-analysis of documents across all major formats.

## Install

### Linux / macOS

```bash
curl -fsSL https://github.com/harunjeylan/ocean/releases/latest/download/install.sh | bash
```

Downloads the pre-built binary to `~/.ocean/bin/ocean` and adds it to PATH.

### Windows

```powershell
powershell -c "irm https://github.com/harunjeylan/ocean/releases/latest/download/install.ps1 | iex"
```

Downloads the pre-built binary to `%LOCALAPPDATA%\ocean\bin\ocean.exe` and adds it to the user PATH.

### Build from Source

```sh
cargo build --release
```

Produces `target/release/ocean` (or `ocean.exe` on Windows).

During development: `cargo run -- <args>` or `cargo run --bin cli -- <args>`.

---

## Global Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-h`, `--help` | — | — | Print help |
| `-V`, `--version` | — | — | Print version |
| `--log-format` | `String` | `console` | Log output format: `console` or `json` |
| `--log-file` | `String` | — | Path to write JSON log output (appended). When combined with `--log-format console`, console logs go to stderr and JSON logs to the file |

`--log-format` and `--log-file` are global flags available on every invocation. They also
appear as per-command flags on `index` and `query` (command-level takes priority).

---

## Syntax

```
ocean <COMMAND> [OPTIONS] [ARGS]
```

---

## Configuration System

Ocean loads settings from multiple locations, merged with local priority. Every
setting can be overridden by CLI flags at invocation time.

### File Locations

| Path | Scope |
|------|-------|
| `CWD/.ocean/config.json` | Project / current directory (highest file priority) |
| `~/.ocean/config.json` | Global user-level (Unix) |
| `%APPDATA%/ocean/config.json` | Global user-level (Windows) |

### Environment Files

Loaded at startup in order (last wins):

| Path | Priority |
|------|----------|
| `~/.ocean/.env` | Lowest (global defaults) |
| `CWD/.env` | Medium |
| `CWD/.ocean/.env` | Highest (project overrides) |

### Resolution Order (highest to lowest)

1. CLI flags (explicit per-invocation)
2. `CWD/.ocean/config.json` (project-level config)
3. `~/.ocean/config.json` (global fallback)
4. `.env` files (in priority order above)
5. Hardcoded defaults

### `${VAR}` Environment Variable Syntax

Config values support `${VARIABLE}` placeholders that are resolved from the
environment (including variables loaded from `.env` files). Used for sensitive
values like API keys and database paths.

Example: `"api_key": "${OPENAI_API_KEY}"`

### Default Database Path

Auto-computed from the current working directory name:

```
~/.ocean/database/{cwd-kebab-case}.db
```

Non-alphanumeric characters are converted to hyphens, consecutive hyphens are
collapsed, and trailing hyphens are stripped.

### Config JSON Schema

```json
{
  "embedding": {
    "provider": "ollama",
    "model": "nomic-embed-text",
    "dimension": 768,
    "api_key": "${API_KEY}",
    "base_url": "http://localhost:11434"
  },
  "index": {
    "batch_size": 10,
    "db_path": null,
    "reindex": false,
    "no_graph": false,
    "no_references": false,
    "no_entities": false
  },
  "query": {
    "top_k": 10,
    "db_path": null,
    "mode": "auto",
    "expand_depth": 0,
    "context": false,
    "context_chunks": 3,
    "verbose": false
  },
  "runtime": {
    "mode": "desktop",
    "io_threads": null,
    "cpu_threads": null,
    "max_ai_concurrent": null,
    "max_retries": null,
    "retry_backoff_ms": null,
    "max_queue_size": null,
    "max_in_flight": null
  },
  "cache": {
    "embedding_cache_size": 1000,
    "query_cache_size": 100,
    "graph_cache_size": 100,
    "query_ttl_secs": 300,
    "embedding_cache_path": null,
    "enabled": true
  },
  "security": {
    "sandbox": true,
    "read_only": false
  },
  "observability": {
    "log_format": "console",
    "log_file": null
  }
}
```

All sections are optional. A local config only needs to specify the fields that
differ from defaults. Values marked `null` use their hardcoded defaults or CLI
fallbacks.

### Config Validation

Validates:
- `runtime.mode` — must be `desktop`, `server`, or `embedded`
- `observability.log_format` — must be `console` or `json`

---

## Supported Formats

| Format | Extension | Backend | Read Selectors |
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

## Runtime Modes

Three runtime modes control default resource allocation for indexing and query
operations. Auto-detected based on available parallelism (CPU count).

| Mode | Auto-Detect | AI Concurrent | Embedding Batch | Embedding Cache | Queue Size | In-Flight |
|------|-------------|---------------|-----------------|-----------------|------------|-----------|
| `desktop` | 4–15 CPUs | 2 | 10 | 1,000 | 10,000 | 10 |
| `server` | 16+ CPUs | 4 | 32 | 5,000 | 100,000 | 50 |
| `embedded` | ≤2 CPUs | 1 | 4 | 100 | 1,000 | 3 |

Set via `--mode` flag on `index` or via `runtime.mode` in config:

```
ocean index ./docs --mode server
```

Or in config:
```json
{ "runtime": { "mode": "server" } }
```

---

## Security Features

### Sandbox

The `watch` command uses a `Sandbox` that validates file access:
- Resolves paths to canonical form
- Rejects paths outside the workspace root
- Rejects symlinks pointing outside the workspace
- Only allows supported extensions (pdf, docx, pptx, xlsx, txt, md, html, htm, png, jpg, jpeg)

By default the sandbox is enabled. Disable with `--no-sandbox`:

```
ocean watch ./docs --no-sandbox
```

### Read-Only Mode

When `security.read_only` is `true` in config, the following commands are
disabled:
- `scan` — returns `scan is disabled in read-only mode`
- `watch` — returns `watch is disabled in read-only mode`
- `index` — returns `indexing is disabled in read-only mode`

Read-only mode also applies when `query --read-only` is set.

---

## Embedding Providers

| Provider | Default Model | Default Dim | Default Base URL | API Key |
|----------|---------------|-------------|------------------|---------|
| Ollama | `nomic-embed-text` | 768 | `http://localhost:11434` | Optional |
| OpenAI | `text-embedding-3-small` | 1536 | `https://api.openai.com/v1` | Required |
| Anthropic | `cohere-embed-multilingual-v3` | 768 | `https://api.anthropic.com/v1` | Required |
| Gemini | `gemini-embedding-001` | 3072 | (built-in, empty string) | Required |

Provider resolution order: CLI `--provider` > config `embedding.provider` > `ollama`.

Dimension resolution order: CLI `--dimension` > config `embedding.dimension` >
provider/model heuristics.

API key resolution order: CLI `--api-key` > config `embedding.api_key` >
(no env fallback).

Base URL resolution:
- For Ollama: `--ollama-url` > config `base_url` > `http://localhost:11434`
- For non-Ollama providers: config `base_url` > provider default

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (file not found, unsupported format, invalid selector, parse failure, config validation failure, etc.) |

---

## Document Commands

### info — Document summary

Show metadata + outline in one view.

**Syntax:**
```
ocean info <file> [--metrics]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the document file |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--metrics` | `bool` | `false` | Display global usage metrics instead of document info |

**Examples:**

```
ocean info report.pdf
ocean info presentation.pptx
ocean info "meeting notes.docx"
ocean info --metrics
```

**Sample output:**
```
Metadata:
  Path:    report.pdf
  Format:  Pdf
  Size:    1245678 bytes
  Title:   Annual Report 2025
  Author:  Jane Doe
  Created: 2025-01-15
  Modified: 2025-03-20
  Pages:   42

Outline:
  - [L1] Executive Summary  (Heading("Executive Summary"))
    - [L2] Key Findings  (Heading("Key Findings"))
  - [L1] Financial Overview  (Heading("Financial Overview"))
```

---

### metadata — Document metadata

Show path, format, size, title, author, created/modified dates, and page count.

**Syntax:**
```
ocean metadata <file>
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the document file |

**Examples:**

```
ocean metadata article.pdf
```

**Sample output:**
```
Metadata:
  Path:    article.pdf
  Format:  Pdf
  Size:    234567 bytes
  Title:   Machine Learning Trends
  Author:  John Smith
  Created: 2025-02-10
  Modified: 2025-02-12
  Pages:   15
```

---

### outline — Table of contents

Show hierarchical heading outline or flat list of slides/sheets.

**Syntax:**
```
ocean outline <file>
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the document file |

**Behavior by format:**
- PDF, DOCX, MD, HTML — hierarchical heading outline
- PPTX — flat slide list
- XLSX — flat sheet list
- TXT — empty outline

**Examples:**

```
ocean outline presentation.pptx
ocean outline chapter.md
ocean outline book.pdf
```

**Sample output (markdown):**
```
  - [L1] Introduction  (Heading("Introduction"))
    - [L2] Background  (Heading("Background"))
  - [L1] Conclusion  (Heading("Conclusion"))
```

**Sample output (PPTX):**
```
  - [L1] Slide 1 Title  (Slide(1))
  - [L1] Slide 2 Title  (Slide(2))
```

---

### page-count — Page/slide count

**Syntax:**
```
ocean page-count <file>
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the document file |

Returns `(none)` for formats without pages (DOCX, XLSX, TXT, MD, HTML).

**Examples:**

```
ocean page-count book.pdf
   342
```

---

### search — Full-text search (single file)

Case-insensitive full-text search within a single document.

**Syntax:**
```
ocean search <file> <query>
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the document file |
| `query` | `String` | Yes | Search term or phrase |

**Examples:**

```
ocean search report.pdf "budget"
ocean search notes.docx "quarterly results"
```

**Sample output:**
```
3 match(es) for 'budget':
  Page(2): "...annual budget for 2025..."
  Page(5): "...budget allocation..."
  Page(7): "...budget review..."
```

---

### grep — Full-text search (all files in a directory)

Recursively search all supported documents in a directory.

**Syntax:**
```
ocean grep <dir> <query>
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `dir` | `String` | Yes | Directory to search recursively |
| `query` | `String` | Yes | Search term or phrase |

**Examples:**

```
ocean grep ./documents "budget"
```

**Sample output:**
```
./documents/report.pdf:
  Page(2): "...annual budget..."
  Page(5): "...budget allocation..."

./documents/notes.docx:
  Paragraph(12): "...budget review..."

Total: 3 match(es) in 42 file(s) for 'budget'
```

---

### read — Read content by selector

Read a specific part of a document using one of the selector flags. Exactly
one selector must be specified (unless using `--skip`/`--take`).

**Syntax:**
```
ocean read <file> [--page <N>] [--heading <TEXT>] [--paragraph <N>]
                 [--table <N>] [--slide <N>] [--sheet <NAME>]
                 [--cell <REF>] [--image <N>] [--range <S-E>]
                 [--skip <N> --take <N>]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the document file |

**Selector Flags:**

| Flag | Type | Default | Description | Applicable Formats |
|------|------|---------|-------------|-------------------|
| `--page <N>` | `u32` | — | Read page number N (1-indexed) | PDF |
| `--heading <TEXT>` | `String` | — | Read content under named heading | PDF, DOCX, MD, HTML |
| `--paragraph <N>` | `u32` | — | Read paragraph number N (0-indexed) | TXT, MD, HTML, DOCX |
| `--table <N>` | `u32` | — | Read table number N (0-indexed) | DOCX, HTML, XLSX |
| `--slide <N>` | `u32` | — | Read slide number N (1-indexed) | PPTX |
| `--sheet <NAME>` | `String` | — | Read sheet by name | XLSX |
| `--cell <REF>` | `String` | — | Read a single cell value (e.g. `B12`) | XLSX |
| `--image <N>` | `u32` | — | Read image number N (0-indexed) | DOCX, PPTX, HTML |
| `--range <S-E>` | `String` | — | Read range of units (e.g. `0-100`) | All formats |
| `--skip <N>` | `u32` | — | Skip N units from start (used with `--take`) | All formats |
| `--take <N>` | `u32` | — | Read N units after skip (required with `--skip`) | All formats |

**Slice mode** (`--skip`/`--take`):
- Pages for PDF, DOCX
- Slides for PPTX
- Lines for TXT, MD
- Paragraphs for HTML
- Sheets for XLSX
- `--take` defaults to 1 when `--skip` is used alone (actually required)

**Examples:**

```
ocean read report.pdf --page 1
ocean read chapter.md --heading "Getting Started"
ocean read notes.txt --paragraph 0
ocean read presentation.pptx --slide 3
ocean read data.xlsx --sheet "Sheet1"
ocean read data.xlsx --cell "C5"
ocean read document.docx --table 0
ocean read report.pdf --skip 200 --take 10
ocean read notes.txt --take 5
ocean read document.docx --skip 2 --take 3
ocean read slides.pptx --skip 0 --take 2
ocean read data.xlsx --skip 1 --take 2
```

**Sample output (page):**
```
--- Page 1 ---
The quick brown fox jumps over the lazy dog...
```

**Sample output (table):**
```
Name | Age | City
--- | --- | ---
Alice | 30 | New York
Bob | 25 | London
```

**Sample output (slide):**
```
--- Slide 3 ---
Title: Q4 Results
Revenue grew 15% year-over-year...
```

---

## File System Commands

### scan — Scan directory for supported documents

List all supported files with metadata. Uses the PathResolver (SurrealDB-backed)
to generate persistent UUIDv7 file IDs.

**Syntax:**
```
ocean scan <dir> [--no-hash]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `dir` | `String` | Yes | Directory to scan recursively |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--no-hash` | `bool` | `false` | Skip SHA-256 hashing for faster scans |

**Sample output:**
```
Found 43 file(s) in './documents':
  019f14f7       0.1 KB  html  ./documents/index.html
  019f18f2     12.3 KB  pdf   ./documents/report.pdf
  019f1a45      4.2 KB  docx  ./documents/notes.docx
```

**Sample output (--no-hash):**
```
Found 43 file(s) in './documents':
        0.1 KB  html  ./documents/index.html
       12.3 KB  pdf   ./documents/report.pdf
```

**Note:** Disabled in read-only mode.

---

### hash — Compute file hash

Compute SHA-256 hash of a file using streaming 64KB buffer.

**Syntax:**
```
ocean hash <file>
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the file |

Files larger than 4GB are rejected.

**Sample output:**
```
dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f
```

---

### verify — Verify file hash

Check whether a file matches an expected SHA-256 hash.

**Syntax:**
```
ocean verify <file> <expected-hash>
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the file |
| `hash` | `String` | Yes | Expected SHA-256 hex string |

**Output:** Prints `true` or `false`.

**Sample output:**
```
ocean verify report.pdf dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f
  true
```

---

### watch — Watch directory for file changes

Monitor a directory for file creation, modification, deletion, and renaming.
Uses `notify` with 100ms debounce and MAX_BATCH_SIZE=100.

**Syntax:**
```
ocean watch <dir> [--no-sandbox]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `dir` | `String` | Yes | Directory to watch |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--no-sandbox` | `bool` | `false` | Disable filesystem sandbox validation |

Runs until Ctrl+C.

**Note:** Disabled in read-only mode.

**Sample output:**
```
Watching './documents'... Press Ctrl+C to stop.
[CREATED]  ./documents/new_notes.txt
[MODIFIED] ./documents/report.pdf
[DELETED]  id=019f14f7...
[RENAMED]  ./documents/old.txt -> ./documents/new.txt
```

---

## Chunk Command

### chunk — Semantic chunking

Parse a document and split it into semantic chunks (Text, Heading, Table,
Slide, Sheet, Cell, Image, Page) based on token-size limits. Uses sentence-boundary
splitting with configurable overlap and heading detection.

**Syntax:**
```
ocean chunk <file> [--min-size <N>] [--max-size <N>] [--overlap <N>]
                   [--include-images] [--rows-per-chunk <N>]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the document file |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--min-size` | `usize` | `100` | Minimum tokens per chunk |
| `--max-size` | `usize` | `800` | Maximum tokens per chunk |
| `--overlap` | `usize` | `1` | Overlap sentences between consecutive chunks |
| `--include-images` | `bool` | `false` | Include image blocks in chunks |
| `--rows-per-chunk` | `usize` | `50` | Rows per spreadsheet chunk |

**Chunk types:** Text, Heading, Table, Slide, Sheet, Cell, Image, Page

**Behavior:**
- Heading boundaries force chunk splits
- Adjacent same-type chunks under the same heading are merged if combined tokens ≤ `--max-size`
- Tables/slides/sheets are emitted atomically (never split)

**Examples:**

```
ocean chunk chapter.md
```

**Sample output:**
```
5 chunks from 'chapter.md':
  [019f21a1] Text      h="Introduction"    342 tokens
  [019f21a2] Heading   h="Background"       0 tokens
  [019f21a3] Text      h="Background"     567 tokens
  [019f21a4] Heading   h="Conclusion"       0 tokens
  [019f21a5] Text      h="Conclusion"     123 tokens
```

```
ocean chunk presentation.pptx --max-size 400
```

**Sample output:**
```
12 chunks from 'presentation.pptx':
  [019f21b1] Slide     h="Slide 1"        89 tokens
  [019f21b2] Slide     h="Overview"      234 tokens
  ...
```

```
ocean chunk data.xlsx --rows-per-chunk 100
```

**Sample output:**
```
3 chunks from 'data.xlsx':
  [019f21c1] Sheet     h="Sheet1"       450 tokens
  [019f21c2] Sheet     h="Sheet2"       210 tokens
  [019f21c3] Sheet     h="Summary"       90 tokens
```

---

## Index Command

### index — Scan, parse, chunk, embed, and index documents

Recursively scan a directory for supported documents, parse each file, split
into semantic chunks, compute embeddings, and store in SurrealDB with HNSW
vector index. Optionally builds a knowledge graph.

**Syntax:**
```
ocean index <dir> [--model <name>] [--provider <name>]
                  [--ollama-url <url>] [--api-key <key>]
                  [--dimension <N>] [--db-path <path>]
                  [--batch-size <N>] [--reindex] [--no-graph]
                  [--no-references] [--no-entities] [--watch]
                  [--mode <mode>] [--no-sandbox]
                  [--io-threads <N>] [--cpu-threads <N>]
                  [--max-ai-concurrent <N>] [--max-retries <N>]
                  [--retry-backoff-ms <N>] [--max-queue-size <N>]
                  [--max-in-flight <N>]
                  [--log-format <fmt>] [--log-file <path>]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `dir` | `String` | Yes | Directory to scan and index |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--model` | `String` | `nomic-embed-text` | Embedding model name |
| `--provider` | `String` | `ollama` | Embedding provider (`ollama`, `openai`, `anthropic`, `gemini`) |
| `--ollama-url` | `String` | config / heuristics | Override base URL for Ollama |
| `--api-key` | `String` | config / (none) | API key for embedding provider |
| `--dimension` | `usize` | auto | Embedding dimension (overrides config & heuristics) |
| `--db-path` | `String` | auto (`~/.ocean/database/{cwd}.db`) | SurrealDB database base path |
| `--batch-size` | `usize` | `10` | Chunks per embedding batch |
| `--reindex` | `bool` | `false` | Re-index existing files (update chunks) |
| `--no-graph` | `bool` | `false` | Skip graph building during index |
| `--no-references` | `bool` | `false` | Skip reference edge extraction during graph build |
| `--no-entities` | `bool` | `false` | Skip entity extraction during graph build |
| `--watch` | `bool` | `false` | Watch directory for changes after initial index |
| `--mode` | `String` | auto-detected | Runtime mode: `desktop`, `server`, `embedded` |
| `--no-sandbox` | `bool` | `false` | Disable filesystem sandbox |
| `--io-threads` | `usize` | CPUs × 2 | Number of I/O threads |
| `--cpu-threads` | `usize` | CPUs | Number of CPU threads |
| `--max-ai-concurrent` | `usize` | mode-dependent | Max concurrent AI/embedding requests |
| `--max-retries` | `u32` | — | Max retries for failed embedding calls |
| `--retry-backoff-ms` | `u64` | — | Initial retry backoff in milliseconds |
| `--max-queue-size` | `usize` | mode-dependent | Max queue size for backpressure |
| `--max-in-flight` | `usize` | mode-dependent | Max in-flight embedding requests |
| `--log-format` | `String` | `console` | Log format (`console` or `json`) |
| `--log-file` | `String` | — | JSON log output file path |

**Sample output:**
```
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

**Note:** Disabled in read-only mode.

---

## Query Command

### query — Unified semantic search over indexed documents

Unified query command supporting auto mode selection, vector-only, hybrid
(vector + full-text search), and graph-expanded search with context windows
and execution metadata.

**Syntax:**
```
ocean query <query> [--mode <mode>] [--top-k <N>]
                    [--expand-depth <N>] [--context]
                    [--context-chunks <N>] [--no-cache]
                    [--rerank-by-heading] [--rerank-by-file]
                    [--file-id <id>] [--heading <prefix>]
                    [--block-type <type>] [--read-only]
                    [--model <name>] [--provider <name>]
                    [--ollama-url <url>] [--api-key <key>]
                    [--dimension <N>] [--db-path <path>]
                    [--verbose]
                    [--log-format <fmt>] [--log-file <path>]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `query` | `String` | Yes | Search query text |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--mode` | `String` | `auto` | Query mode: `auto`, `vector`, `hybrid`, `expand` |
| `--top-k` | `usize` | `10` | Max number of results |
| `--expand-depth` | `usize` | `0` | Graph expansion depth (0 = disabled) |
| `--context` | `bool` | `false` | Include context windows around matched chunks |
| `--context-chunks` | `usize` | `3` | Max chunks per context window (auto-clamped 1–10) |
| `--no-cache` | `bool` | `false` | Bypass query cache |
| `--rerank-by-heading` | `bool` | `false` | Penalize results from same heading for diversity |
| `--rerank-by-file` | `bool` | `false` | Penalize results from same file for diversity |
| `--file-id` | `String` | — | Filter by file ID |
| `--heading` | `String` | — | Filter by heading prefix |
| `--block-type` | `String` | — | Filter by block type (Text, Heading, Table, etc.) |
| `--read-only` | `bool` | `false` | Enable read-only mode for this query |
| `--model` | `String` | `nomic-embed-text` | Embedding model name |
| `--provider` | `String` | `ollama` | Embedding provider |
| `--ollama-url` | `String` | config / heuristics | Override base URL for Ollama |
| `--api-key` | `String` | config / (none) | API key for embedding provider |
| `--dimension` | `usize` | auto | Embedding dimension |
| `--db-path` | `String` | auto | SurrealDB database base path |
| `--verbose` | `bool` | `false` | Show ExecutionMeta timing info |
| `--log-format` | `String` | `console` | Log format (`console` or `json`) |
| `--log-file` | `String` | — | JSON log output file path |

**Auto Mode Heuristics:**

| Condition | Selected Mode |
|-----------|---------------|
| `expand-depth > 0` | Expand |
| Query < 3 words | Vector |
| Contains cross-ref phrases (`related to`, `reference`, etc.) | Expand |
| Natural language phrase (3+ words) | Hybrid |
| Empty query | Hybrid |

**Examples:**

```
ocean query "budget allocation" --mode hybrid --top-k 5 --context --verbose
```

**Sample output:**
```
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
ocean query "Q4 results" --file-id 019f14f7 --context
ocean query --mode expand --expand-depth 2 "financial overview"
```

---

## Vector Search Command

### vector-search — Semantic vector search (legacy)

Backwards-compatible semantic search across indexed documents. Supports hybrid
(vector + FTS) search, filtering, and graph context expansion.

**Syntax:**
```
ocean vector-search <query> [--top-k <N>] [--hybrid]
                            [--file-id <id>] [--heading <prefix>]
                            [--block-type <type>]
                            [--model <name>] [--provider <name>]
                            [--ollama-url <url>] [--api-key <key>]
                            [--dimension <N>] [--db-path <path>]
                            [--expand-depth <N>]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `query` | `String` | Yes | Search query text |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--top-k` | `usize` | `10` | Max number of results |
| `--hybrid` | `bool` | `false` | Combine vector + FTS search with RRF fusion |
| `--file-id` | `String` | — | Filter by file ID |
| `--heading` | `String` | — | Filter by heading prefix |
| `--block-type` | `String` | — | Filter by block type (Text, Heading, Table, etc.) |
| `--model` | `String` | `nomic-embed-text` | Embedding model name |
| `--provider` | `String` | `ollama` | Embedding provider |
| `--ollama-url` | `String` | config / heuristics | Override base URL for Ollama |
| `--api-key` | `String` | config / (none) | API key for embedding provider |
| `--dimension` | `usize` | auto | Embedding dimension |
| `--db-path` | `String` | auto | SurrealDB database base path |
| `--expand-depth` | `usize` | `0` | Graph expansion depth (0 = disabled) |

**Sample output:**
```
Top 5 expanded results for 'budget allocation':
  1. score=0.8521 (vec=0.8521, fts=0.7234)  file=019f14f7  heading="Budget"
     "...the annual budget allocation for 2025..."
  2. score=0.7234  file=019f14f7  heading="Budget"
     "...related budget planning document (see Finance Report)..."
```

---

## Graph Commands

The knowledge graph stores nodes (File, Chunk, Heading, Entity, Folder) and
edges (Contains, References, Mentions, BelongsTo, DerivedFrom, SimilarTo,
CrossReference) in a SurrealDB-backed graph store.

### graph info — Show graph info for a file

Display node count, edge count, and type breakdown for a file's subgraph.

**Syntax:**
```
ocean graph info <file> [--db-path <path>]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | `String` | Yes | Path to the document file |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--db-path` | `String` | auto | SurrealDB database base path |

**Sample output:**
```
Graph Info:
  Total nodes: 5
  Total edges: 7
  Breakdown by type:
    File: 1
    Chunk: 3
    Heading: 1
```

---

### graph expand — Expand from a node

Traverse the graph starting from a seed node up to a given depth using BFS.

**Syntax:**
```
ocean graph expand <node-id> [--depth <N>] [--direction <dir>]
                             [--db-path <path>]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `node-id` | `String` | Yes | Seed node ID to expand from |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--depth` | `usize` | `2` | Max expansion depth (1–5) |
| `--direction` | `String` | `both` | Traversal direction: `forward`, `backward`, or `both` |
| `--db-path` | `String` | auto | SurrealDB database base path |

**Sample output:**
```
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

---

### graph path — Find shortest path between two nodes

**Syntax:**
```
ocean graph path <from-id> <to-id> [--max-depth <N>]
                                   [--db-path <path>]
```

**Arguments:**

| Arg | Type | Required | Description |
|-----|------|----------|-------------|
| `from-id` | `String` | Yes | Start node ID |
| `to-id` | `String` | Yes | Target node ID |

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--max-depth` | `usize` | `5` | Max search depth (1–10) |
| `--db-path` | `String` | auto | SurrealDB database base path |

**Sample output:**
```
Path (2 hops):
  1. chunk:c1 --BelongsTo--> heading:h1 (w: 1.0)
```

---

### graph stats — Show global graph statistics

**Syntax:**
```
ocean graph stats [--db-path <path>]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--db-path` | `String` | auto | SurrealDB database base path |

**Sample output:**
```
Graph Stats:
  Total nodes: 150
  Total edges: 320
  By type:
    File: 10
    Chunk: 100
    Heading: 40
    Entity: 0
    Folder: 0
```

---

## Config Commands

### config show — Display current configuration

Show the merged configuration as pretty-printed JSON. If no config file is
found, displays the default configuration.

**Syntax:**
```
ocean config show
```

**Sample output:**
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

---

### config validate — Validate configuration

Run validation rules against the loaded configuration.

**Syntax:**
```
ocean config validate
```

**Validation Rules:**
- `runtime.mode` — must be `desktop`, `server`, or `embedded`
- `observability.log_format` — must be `console` or `json`

**Sample output (valid):**
```
config OK
```

**Sample output (invalid):**
```
  error: runtime.mode: invalid value 'production'. Use: desktop, server, embedded
Error: config has 1 error(s)
```

**Sample output (no config):**
```
No config file found. Using defaults — no validation needed.
```

---

## Init Command

### init — Interactive project initialization

Interactive wizard that prompts for embedding provider, model, dimension, API
key, and base URL, then creates:

- `.ocean/config.json` — project configuration
- Appends Ocean CLI section to `CLAUDE.md`
- Appends Ocean CLI Usage section to `AGENTS.md`
- Creates `.agents/skills/ocean-cli/SKILL.md` — reusable skill definition

**Syntax:**
```
ocean init [--dir <path>]
```

**Flags:**

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--dir` | `String` | CWD | Target directory to initialize (creates `.ocean` subdirectory) |

**Default Model by Provider:**

| Provider | Default Model | Default Dim |
|----------|---------------|-------------|
| `ollama` | `nomic-embed-text` | 768 |
| `openai` | `text-embedding-3-small` | 1536 |
| `anthropic` | `cohere-embed-multilingual-v3` | 768 |
| `gemini` | `gemini-embedding-001` | 3072 |

**Default Base URL by Provider:**

| Provider | Default Base URL |
|----------|------------------|
| `ollama` | `http://localhost:11434` |
| `openai` | `https://api.openai.com/v1` |
| `anthropic` | `https://api.anthropic.com/v1` |
| `gemini` | (empty, built-in) |

**Sample session:**
```
> ocean init
Initializing ocean in C:\Users\user\project

Embedding provider (ollama/openai/anthropic/gemini) [ollama]: ollama
Embedding model [nomic-embed-text]:
Embedding dimension [768]:
(API key is optional for ollama, recommended for others)
API key (leave empty for none):
Base URL [http://localhost:11434]:

Configuration:
  Provider:  ollama
  Model:     nomic-embed-text
  Dimension: 768
  API key:   (none)
  Base URL:  http://localhost:11434

Write configuration? [y]: y
  Created .ocean/config.json
  Appended Ocean CLI section to CLAUDE.md
  Appended Ocean CLI Usage section to AGENTS.md
  Created .agents/skills/ocean-cli/SKILL.md

Ocean initialized in C:\Users\user\project

Next steps:
  1. Place supported documents in this directory
  2. Run: ocean scan .
  3. Run: ocean index .
  4. Run: ocean query "your question"
```

---

## Events & Observability

### System Events

Ocean emits structured events during indexing and query operations. These are
output as console lines (stderr) or JSON lines depending on `--log-format`.

**Event Types:**

| Event | Trigger | Fields |
|-------|---------|--------|
| `IndexStarted` | Start of `index` command | `timestamp`, `dir`, `total_files` |
| `IndexComplete` | Completion of `index` | `timestamp`, `duration_ms`, `indexed`, `skipped`, `failed` |
| `FileProcessed` | Each file during index | `timestamp`, `path`, `status`, `duration_ms` |
| `QueryExecuted` | Completion of `query` | `timestamp`, `query`, `mode`, `num_results`, `duration_ms`, `cached` |
| `BackpressureEvent` | Queue backpressure applied | `timestamp`, `action`, `queue_len`, `in_flight` |
| `ErrorEvent` | Runtime errors | `timestamp`, `severity`, `module`, `message` |

JSON format uses a tagged schema:
```json
{"event":"IndexStarted","data":{"timestamp":1700000000000,"dir":"./docs","total_files":5}}
```

### Global Metrics

Available via `ocean info --metrics`. Displays runtime counters:

| Metric | Description |
|--------|-------------|
| Uptime | Seconds since process start |
| Queries total | Total queries executed |
| Queries cached | Queries served from cache |
| Files indexed | Files successfully indexed |
| Files skipped | Files skipped during index |
| Files failed | Files that failed during index |
| Embedding calls | Total embedding API calls |
| Embedding cached | Embeddings served from cache |
| Graph expansions | Total graph expansion operations |
| Cache hits | Total cache hits |
| Cache misses | Total cache misses |
| Cache hit rate | Hit rate percentage |

---

## Database Path Resolution

Ocean uses a three-tier database layout under the resolved base path:

| Database | Path | Used By |
|----------|------|---------|
| Ocean | `{base_path}/ocean.db` | File metadata, chunk storage |
| Vector | `{base_path}/vector.db` | Embedding vectors with HNSW index |
| Graph | `{base_path}/graph.db` | Knowledge graph nodes and edges |

Base path resolution:
1. CLI `--db-path` flag
2. Config `index.db_path` or `query.db_path`
3. Auto-computed: `~/.ocean/database/{cwd-kebab-case}.db`
