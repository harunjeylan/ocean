# SECTION 0 — SYSTEM FOUNDATION (Ocean CORE CONCEPTS)

This section defines the **non-negotiable rules of the entire system**. If this is wrong, everything after it becomes inconsistent.

---

# 0.1 What Ocean *is*

Ocean is a:

> **Local multi-index document runtime that turns unstructured files into a queryable knowledge system.**

Not:

* not a vector DB
* not a graph DB
* not a file manager
* not an AI system

It is:

```text id="s0_def"
Filesystem → Structured Understanding → Multi-Index Runtime → Query API
```

---

# 0.2 The Core Abstraction Problem

Traditional systems treat documents as:

```text id="s0_old"
file → bytes → text → search
```

or

```text id="s0_vector"
file → chunks → embeddings → vector search
```

or

```text id="s0_graph"
file → entities → graph → traversal
```

Ocean changes this:

## NEW MODEL

```text id="s0_new"
file
  → structured blocks
      → semantic chunks
          → embeddings (vector index)
          → relationships (graph index)
          → filesystem mapping
```

So EVERYTHING is derived from a **single source of truth pipeline**.

---

# 0.3 The 3-Layer Memory Model

Ocean has 3 layers of “memory”:

## (1) SOURCE LAYER (Truth)

```text id="s0_source"
Physical Filesystem
```

* PDFs
* DOCX
* PPTX
* XLSX
* Images

RULE:

> Never modify this layer directly.

---

## (2) STRUCTURE LAYER (Understanding)

```text id="s0_structure"
Parsed Document Blocks
```

Example:

```text id="s0_blocks"
Heading("Leave Policy")
Paragraph("Employees receive 20 days")
Table(...)
```

This layer is:

* ephemeral OR cached
* format-independent
* normalized

---

## (3) INTELLIGENCE LAYER (Indexes)

```text id="s0_intel"
Vector Index + Graph Index
```

This is where:

* meaning lives
* relationships live
* search happens

---

# 0.4 Key Design Rule (VERY IMPORTANT)

## Rule 1 — No index is authoritative

```text id="s0_rule1"
Vector DB ❌ NOT source of truth
Graph DB  ❌ NOT source of truth
Filesystem ✔ SOURCE OF TRUTH
```

Everything can be rebuilt.

---

## Rule 2 — Everything is derived

```text id="s0_rule2"
Files → Blocks → Chunks → Embeddings → Graph
```

Never reverse direction.

---

## Rule 3 — No format awareness outside parser layer

Outside `ocean-parser`:

❌ no PDF logic
❌ no DOCX logic
❌ no XLSX logic

Everything becomes:

```text id="s0_rule3"
Block / Chunk / Node
```

---

## Rule 4 — Every unit must be addressable

Everything must have:

```rust id="s0_addr"
id: String
source_file: FileId
location: Selector
```

So ANY answer can trace back to origin.

---

# 0.5 Core Data Flow Contract

This is the most important part of the entire system.

## INDEX FLOW

```text id="s0_flow"
1. Scan filesystem
2. Parse file → Blocks
3. Chunk blocks → Chunks
4. Embed chunks → Vector index
5. Extract relationships → Graph index
6. Store metadata → Storage layer
```

---

## QUERY FLOW

```text id="s0_query"
1. Vector search (recall)
2. Graph expansion (context)
3. Ranking fusion
4. Return structured result
```

---

# 0.6 The Mental Model (VERY IMPORTANT)

Ocean behaves like 3 systems working together:

## VECTOR SYSTEM = "Meaning"

```text id="s0_vector_role"
What is this about?
```

---

## GRAPH SYSTEM = "Context"

```text id="s0_graph_role"
How is this related?
```

---

## FILESYSTEM = "Reality"

```text id="s0_fs_role"
Where does this come from?
```

---

# 0.7 Unified Object Model

Everything in Ocean must eventually map to:

```rust id="s0_model"
Document → Blocks → Chunks → Nodes
```

## Block

Raw parsed structure

## Chunk

Search unit

## Node

Graph entity

---

# 0.8 System Boundaries

Ocean has strict boundaries:

## Inside Ocean

* parsing
* chunking
* indexing
* searching
* graph building

---

## Outside Ocean

* UI
* AI agents
* MCP tools
* CLI
* API server

Ocean does NOT care about these.

---

# 0.9 Core Guarantee

Ocean must guarantee:

## 1. Determinism

Same input → same index

---

## 2. Rebuildability

If everything is deleted:

```text id="s0_rebuild"
Filesystem → rebuild full system
```

---

## 3. Traceability

Every answer can be traced:

```text id="s0_trace"
Answer → Chunk → File → Path
```

---

## 0.10 What you are actually building

Not:

* search engine

Not:

* vector DB

Not:

* graph DB

---

You are building:

> **A local operating system for knowledge extraction over documents**

---

# END OF SECTION 0

---

If this foundation is correct, next we go to:

# SECTION 1 — FILESYSTEM LAYER (SCAN, WATCH, METADATA ENGINE)

Where we design:

* directory walker
* file identity system
* hash system
* incremental detection
* change tracking model

Say: **“next section”** and we continue.

# SECTION 1 — FILESYSTEM LAYER (Ocean-FS)

This is the **ground truth ingestion layer**.

Everything in Ocean starts here. If this layer is wrong or unstable, your entire indexing system becomes inconsistent.

---

# 1.1 Purpose of ocean-fs

The filesystem layer is responsible for:

```text id="fs1_purpose"
- discovering files
- tracking changes
- identifying uniqueness
- normalizing metadata
- feeding the indexing pipeline
```

It does NOT:

* parse documents
* understand content
* create embeddings
* build graphs

It is purely a **file intelligence layer**.

---

# 1.2 Core Design Principle

## Rule: Files are immutable identities (logically)

Even if a file changes:

```text id="fs1_rule"
Same path ≠ Same content
```

So we separate:

* **Identity (path-based)**
* **Content (hash-based)**

---

# 1.3 File Identity Model

Every file in Ocean is represented as:

```rust id="fs1_file"
pub struct FileMeta {
    pub id: FileId,          // stable internal ID
    pub path: String,        // current location
    pub hash: String,        // content fingerprint
    pub size: u64,
    pub modified: u64,
    pub extension: String,
}
```

---

## 1.3.1 FileId Strategy

You have 2 valid options:

### Option A (recommended): Hash-based ID

```text id="fs1_id_hash"
FileId = sha256(path + inode + created_time)
```

Pros:

* stable across moves (if tracked properly)
* deterministic

---

### Option B: UUID-based ID

```text id="fs1_id_uuid"
FileId = UUIDv7
```

Pros:

* simpler
* decoupled from filesystem

---

👉 Recommendation:
Use **UUIDv7 + path mapping table** (more flexible for graph + moves).

---

# 1.4 Directory Scanner

## Responsibility

Recursively traverse workspace and emit FileMeta list.

---

## 1.4.1 Core API

```rust id="fs1_scan_api"
pub fn scan_dir(path: &str) -> Vec<FileMeta>
```

---

## 1.4.2 Implementation Behavior

Scanner must:

* walk recursively
* ignore hidden/system files
* filter supported formats
* compute metadata

---

## 1.4.3 Supported formats

```text id="fs1_formats"
pdf
docx
pptx
xlsx
txt
md
html
png
jpg
```

---

## 1.4.4 Pseudocode

```rust id="fs1_scan_impl"
for entry in walkdir(path) {
    if entry.is_file() {

        let meta = FileMeta {
            id: generate_id(),
            path: entry.path(),
            hash: sha256(entry.content),
            size: entry.size,
            modified: entry.modified_time,
            extension: entry.extension,
        };

        files.push(meta);
    }
}
```

---

# 1.5 File Hashing System

## Why hashing matters

Hash is your **change detection engine**.

```text id="fs1_hash_role"
Hash = "Did this file actually change?"
```

---

## 1.5.1 Hash function

Use streaming SHA-256:

```rust id="fs1_hash"
pub fn hash_file(path: &str) -> String
```

---

## 1.5.2 Optimization rule

Never load full file into memory.

Use:

* buffered reader
* streaming hash

---

## 1.5.3 Change detection logic

```text id="fs1_change"
Old hash != New hash → reindex
```

---

# 1.6 File Watcher (REAL-TIME INDEXING)

## Responsibility

Detect changes in real-time.

---

## Events

```rust id="fs1_events"
pub enum FileEvent {
    Created,
    Modified,
    Deleted,
    Renamed,
    Moved,
}
```

---

## API

```rust id="fs1_watch_api"
pub fn watch(path: &str, callback: fn(FileEvent, FileMeta))
```

---

## Behavior

On event:

### Created

→ send to index pipeline

### Modified

→ re-hash → if changed → reindex

### Deleted

→ remove from indexes

### Renamed/Moved

→ update path mapping ONLY (no reparse if hash same)

---

# 1.7 Path Resolution Layer

This is important for graph consistency.

## Problem:

Files move:

```text id="fs1_move"
HR/Policy.pdf → Archive/Policy.pdf
```

Without path tracking → graph breaks.

---

## Solution: Path Mapping Table

```sql id="fs1_path_table"
CREATE TABLE file_paths (
    file_id TEXT,
    old_path TEXT,
    new_path TEXT,
    timestamp INTEGER
);
```

---

## Rule:

```text id="fs1_rule_move"
File identity follows content, not location
```

---

# 1.8 Metadata Normalization

Before passing to parser, normalize:

```rust id="fs1_normalize"
pub struct NormalizedFile {
    pub id: FileId,
    pub meta: FileMeta,
    pub mime_type: String,
    pub category: FileCategory,
}
```

---

## File categories

```text id="fs1_category"
Document
Spreadsheet
Presentation
Image
Text
Unknown
```

---

# 1.9 Filtering System

Avoid indexing noise:

### Ignore:

```text id="fs1_ignore"
node_modules/
.git/
.cache/
system files
temporary files
```

---

# 1.10 Output of ocean-fs

The output of this layer is:

```rust id="fs1_output"
Vec<FileMeta>
```

OR stream:

```rust id="fs1_stream"
FileEventStream
```

---

# 1.11 Integration Point

Filesystem layer feeds:

```text id="fs1_pipeline"
ocean-parser
```

ONLY.

Not vector.

Not graph.

Not storage.

---

# 1.12 Performance Requirements

## Must support:

* 10,000–100,000 files
* incremental updates
* parallel scanning

---

## Optimization strategies:

### 1. Parallel scan

```rust id="fs1_parallel"
rayon::par_iter()
```

---

### 2. Incremental hash cache

```text id="fs1_cache"
path → hash cache
```

---

### 3. Event batching

Avoid reindex storms:

```text id="fs1_batch"
100 file changes → 1 batch update
```

---

# 1.13 Final Mental Model

Filesystem layer answers ONLY:

```text id="fs1_model"
"What files exist and have they changed?"
```

NOTHING ELSE.

---

# 1.14 Output Contract (IMPORTANT)

ocean-fs guarantees:

```text id="fs1_contract"
Given a directory → produces stable, deduplicated, change-aware file metadata stream
```

---

# END OF SECTION 1

---

Next section:

# SECTION 2 — PARSER LAYER (ocean-parser)

We will design:

* PDF/DOCX/XLSX/PPTX parsing strategy
* unified block model
* abstraction over file formats
* streaming vs full parsing decisions
* memory control system

Say **“next section”** and we continue.

# SECTION 2 — PARSER LAYER (ocean-parser)

This is where Ocean starts turning **raw files into structured knowledge**.

If Section 1 is “what files exist”,
Section 2 is:

> “what does each file actually *mean structurally*?”

---

# 2.1 Purpose of ocean-parser

The parser layer converts:

```text id="p2_input"
PDF / DOCX / XLSX / PPTX / TXT
```

into a **unified intermediate representation**:

```text id="p2_output"
Blocks (format-independent structure)
```

---

## Key Rule (VERY IMPORTANT)

> Parser must NOT create chunks
> Parser must NOT create embeddings
> Parser must NOT build graphs

It only produces:

```text id="p2_rule"
Raw structured document blocks
```

---

# 2.2 Why we need a unified model

Every format is different:

```text id="p2_formats"
PDF      → layout + text streams
DOCX     → XML paragraphs + runs
XLSX     → cells + sheets
PPTX     → slides + shapes
```

If we don't normalize them, later layers become:

* complex
* duplicated
* fragile

So we introduce a **universal abstraction layer**

---

# 2.3 Core Output Model — BLOCKS

Everything becomes a Block.

## 2.3.1 Block Definition

```rust id="p2_block"
pub enum Block {
    Heading {
        level: u8,
        text: String,
    },

    Paragraph {
        text: String,
    },

    Table {
        rows: Vec<Vec<String>>,
    },

    List {
        items: Vec<String>,
    },

    Image {
        bytes: Vec<u8>,
        caption: Option<String>,
    },

    Metadata {
        key: String,
        value: String,
    },
}
```

---

## 2.3.2 Why this works

Because all documents can be reduced to:

```text id="p2_reduce"
structure + text + tables + media
```

This removes format complexity early.

---

# 2.4 Parser Interface

Every format must implement:

```rust id="p2_trait"
pub trait Parser {
    fn parse(path: &str) -> Vec<Block>;
}
```

---

# 2.5 PDF Parser (hardest case)

## 2.5.1 Tools

Use:

* `pdfium-render` (layout + rendering)
* fallback: `lopdf` (raw structure)

---

## 2.5.2 Strategy

PDF is NOT text.

It is:

```text id="p2_pdf_model"
text + coordinates + fonts + streams
```

So we reconstruct structure:

### Step 1: Extract text blocks

```text id="p2_pdf_step1"
page → text spans → lines → paragraphs
```

---

### Step 2: Detect headings

Heuristics:

* font size larger than normal
* bold text
* spacing before/after

---

### Step 3: Extract tables

Detect:

* aligned columns
* grid-like spacing
* repeated x-coordinates

---

## PDF Output Example

```text id="p2_pdf_out"
Heading("Leave Policy")
Paragraph("Employees are entitled to 20 days...")
Table(...)
```

---

# 2.6 DOCX Parser

DOCX = ZIP + XML

---

## 2.6.1 Internal structure

```text id="p2_docx"
word/document.xml
word/styles.xml
word/media/
```

---

## 2.6.2 Strategy

### Step 1: unzip

### Step 2: parse XML

### Step 3: map to blocks

---

## Mapping rules:

```text id="p2_docx_map"
<w:p> → Paragraph
<w:h> → Heading
<w:tbl> → Table
```

---

## DOCX Output

```text id="p2_docx_out"
Heading("Policy")
Paragraph("...")
Table(...)
```

---

# 2.7 XLSX Parser

Use `calamine`.

---

## 2.7.1 Structure

```text id="p2_xlsx"
Workbook
  ├── Sheet
  ├── Rows
  └── Cells
```

---

## 2.7.2 Conversion rule

Each sheet becomes:

```text id="p2_xlsx_block"
Block::Table
```

---

## Output

```text id="p2_xlsx_out"
Table {
    rows: Vec<Vec<String>>
}
```

---

# 2.8 PPTX Parser

PPTX = ZIP + XML slides

---

## 2.8.1 Structure

```text id="p2_pptx"
ppt/slides/slide1.xml
ppt/slides/slide2.xml
```

---

## 2.8.2 Mapping

```text id="p2_pptx_map"
Title → Heading
Text box → Paragraph
Table → Table
Image → Image
```

---

## Output

```text id="p2_pptx_out"
Heading("Slide Title")
Paragraph("Content")
```

---

# 2.9 TEXT / MARKDOWN Parser

Simplest case:

```rust id="p2_txt"
Paragraph(text)
```

Markdown:

```text id="p2_md"
# → Heading
- → List
text → Paragraph
```

---

# 2.10 IMAGE HANDLING (IMPORTANT)

Images are NOT interpreted yet.

They are stored as:

```rust id="p2_image"
Image {
    bytes,
    caption: None
}
```

Later OCR belongs to another layer.

---

# 2.11 Parsing Pipeline (CORE FLOW)

This is the most important part of Section 2.

```text id="p2_pipeline"
FileMeta
   ↓
detect format
   ↓
select parser
   ↓
extract raw structure
   ↓
normalize into Blocks
   ↓
return Vec<Block>
```

---

# 2.12 Streaming vs Full Parse

## Rule:

### Small files (<5MB)

→ full parse

### Large files (PDF, scans)

→ streaming parse

---

## Streaming model:

```text id="p2_stream"
page-by-page parsing
chunked XML parsing
incremental block emission
```

---

# 2.13 Memory Safety Rule

NEVER load:

* full PDF into memory
* full XLSX sheet at once
* large images unnecessarily

---

# 2.14 Parser Output Contract

Every parser must guarantee:

```text id="p2_contract"
1. Deterministic output
2. Format-agnostic structure
3. No indexing logic
4. No semantic interpretation
```

---

# 2.15 Error Model

```rust id="p2_error"
pub enum ParseError {
    UnsupportedFormat,
    CorruptedFile,
    InvalidEncoding,
    XmlError,
    PdfRenderError,
}
```

---

# 2.16 Performance Strategy

## Parallel parsing

```text id="p2_perf1"
files.par_iter().map(parse)
```

---

## Lazy parsing (optional later)

Only parse when indexing is needed.

---

# 2.17 Output of ocean-parser

The final output is:

```rust id="p2_final"
Vec<Block>
```

OR streaming:

```rust id="p2_stream_out"
Stream<Block>
```

---

# 2.18 Integration Boundary

ocean-parser feeds ONLY:

```text id="p2_boundary"
ocean-chunk
```

It does NOT:

* embed
* index
* store
* graph

---

# 2.19 Mental Model

Parser answers ONLY:

```text id="p2_model"
"What is inside this file structurally?"
```

NOT:

* meaning
* relevance
* relationships
* search

---

# END OF SECTION 2

---

Next section:

# SECTION 3 — CHUNK ENGINE (ocean-chunk)

This is where we convert structured blocks into **searchable semantic units**, which will later feed both vector + graph systems.

Say **“next section”** and we continue.

# SECTION 3 — CHUNK ENGINE (ocean-chunk)

This is where Ocean starts becoming “intelligent”.

Up to now you have:

* files (fs layer)
* structure (parser layer → Blocks)

Now we convert structure into:

> **Searchable + indexable + graph-ready units**

---

# 3.1 Purpose of ocean-chunk

ocean-chunk is responsible for:

```text id="c3_purpose"
Blocks → Chunks
```

A chunk is the **smallest meaningful unit of knowledge** used by:

* vector index (semantic search)
* graph index (relationships)
* query engine (context building)

---

## Key Rule

> Parser understands structure
> Chunker understands meaning boundaries

But NOT embeddings or graphs yet.

---

# 3.2 Why chunking is hard

Bad chunking = broken AI system.

Example problem:

```text id="c3_bad"
Chunk 1: "Employees are entitled"
Chunk 2: "to 20 days of leave per year"
```

→ loses meaning

---

Good chunk:

```text id="c3_good"
"Employees are entitled to 20 days of leave per year."
```

---

# 3.3 Chunk Definition

```rust id="c3_chunk"
pub struct Chunk {
    pub id: String,
    pub file_id: FileId,

    pub content: String,

    pub heading: Option<String>,
    pub page: Option<u32>,

    pub start_offset: Option<usize>,
    pub end_offset: Option<usize>,
}
```

---

# 3.4 Chunking Input

Chunker takes:

```text id="c3_input"
Vec<Block>
```

from parser.

---

# 3.5 Chunking Output

Returns:

```text id="c3_output"
Vec<Chunk>
```

---

# 3.6 Core Chunking Strategy

We use **3-level grouping strategy**

```text id="c3_strategy"
Level 1: Structural grouping (heading/table/paragraph)
Level 2: Semantic merging (context preservation)
Level 3: Size constraints (token limit)
```

---

# 3.7 Step-by-step Chunk Pipeline

## STEP 1 — Group by Heading Context

```text id="c3_step1"
Heading
  ↓
All paragraphs under heading
```

Example:

```text id="c3_ex1"
Heading: "Leave Policy"
Paragraph: "Employees are entitled..."
Paragraph: "Leave is approved by..."
```

---

## STEP 2 — Merge adjacent paragraphs

We merge until:

* topic changes OR
* size limit reached

---

## STEP 3 — Preserve tables as atomic chunks

```text id="c3_tables"
Tables MUST NOT be split
```

Because:

* structure matters
* rows lose meaning if split

---

## STEP 4 — Apply size limit

Default:

```text id="c3_limit"
~300–800 tokens per chunk
```

If exceeded:

* split by sentence boundaries

---

# 3.8 Chunking Algorithm (Core)

```rust id="c3_algo"
pub fn chunk(blocks: Vec<Block>, file_id: FileId) -> Vec<Chunk> {
    let mut chunks = vec![];
    let mut buffer = String::new();

    let mut current_heading: Option<String> = None;

    for block in blocks {

        match block {

            Block::Heading { text, .. } => {
                flush_buffer(&mut buffer, &mut chunks, file_id.clone(), current_heading.clone());
                current_heading = Some(text);
            }

            Block::Paragraph { text } => {
                buffer.push_str(&text);
                buffer.push(' ');
            }

            Block::Table { rows } => {
                flush_buffer(&mut buffer, &mut chunks, file_id.clone(), current_heading.clone());

                chunks.push(Chunk {
                    id: new_id(),
                    file_id: file_id.clone(),
                    content: format!("{:?}", rows),
                    heading: current_heading.clone(),
                    page: None,
                    start_offset: None,
                    end_offset: None,
                });
            }

            _ => {}
        }

        if buffer.len() > 2000 {
            flush_buffer(&mut buffer, &mut chunks, file_id.clone(), current_heading.clone());
        }
    }

    flush_buffer(&mut buffer, &mut chunks, file_id);

    chunks
}
```

---

# 3.9 Flush Function

```rust id="c3_flush"
fn flush_buffer(
    buffer: &mut String,
    chunks: &mut Vec<Chunk>,
    file_id: FileId,
    heading: Option<String>,
) {
    if buffer.trim().is_empty() {
        return;
    }

    chunks.push(Chunk {
        id: new_id(),
        file_id,
        content: buffer.trim().to_string(),
        heading,
        page: None,
        start_offset: None,
        end_offset: None,
    });

    buffer.clear();
}
```

---

# 3.10 Chunk Identity System

Each chunk must be stable:

```text id="c3_id"
chunk_id = hash(file_id + heading + offset)
```

or

```text id="c3_uuid"
UUIDv7
```

---

# 3.11 Chunk Context Enrichment

Each chunk carries:

```text id="c3_context"
- file_id
- heading
- page
- section
```

This is CRITICAL for graph building later.

---

# 3.12 Special Handling Rules

## Rule 1 — Tables

```text id="c3_rule1"
Never split tables
```

---

## Rule 2 — Lists

Lists stay together:

```text id="c3_rule2"
- item 1
- item 2
- item 3
```

---

## Rule 3 — Headings define boundaries

Chunk cannot cross heading boundary unless explicitly merged.

---

## Rule 4 — Images excluded (for now)

Images:

* NOT chunked
* stored separately

---

# 3.13 Chunk Quality Heuristics

We must ensure:

## 1. Self-contained meaning

Each chunk should answer:

> “If I read only this, do I understand something complete?”

---

## 2. Topic coherence

No mixing:

* finance + HR
* policy + code
* table + paragraph mismatch

---

## 3. Low fragmentation

Avoid:

```text
10 tiny chunks = bad
1 coherent chunk = good
```

---

# 3.14 Chunk Size Control

We enforce:

```text id="c3_size"
MIN: 100 tokens
MAX: 800 tokens
```

---

If too small:

* merge with neighbor

If too large:

* split by sentence boundary

---

# 3.15 Sentence Split Strategy

Use:

* punctuation
* NLP sentence splitter (later)

Rule:

```text id="c3_sentence"
Never split mid-sentence
```

---

# 3.16 Output Format Contract

Chunker MUST return:

```rust id="c3_contract"
Vec<Chunk>
```

Each chunk:

* independent
* self-contained
* traceable to file
* context-aware

---

# 3.17 Performance Strategy

## Parallel chunking

```rust id="c3_parallel"
blocks.par_iter().map(chunk)
```

---

## Streaming chunking (future)

Process large docs without full memory load.

---

# 3.18 Integration Boundary

ocean-chunk outputs ONLY to:

```text id="c3_boundary"
ocean-vector
ocean-graph
ocean-storage
```

It does NOT:

* parse files
* embed vectors
* build graph

---

# 3.19 Mental Model

Chunk layer answers:

```text id="c3_model"
"What is the smallest meaningful unit of knowledge in this document?"
```

NOT:

* what it means (vector layer)
* how it connects (graph layer)

---

# 3.20 Final Flow

```text id="c3_flow"
Blocks
  ↓
Group by heading
  ↓
Merge paragraphs
  ↓
Preserve tables
  ↓
Enforce size limit
  ↓
Chunks
```

---

# END OF SECTION 3

---

Next section:

# SECTION 4 — VECTOR INDEX (ocean-vector)

This is where we design:

* embedding pipeline
* ANN storage
* hybrid search scoring
* incremental updates
* performance design

Say **“next section”** and we continue.

Good—now we move into a **real implementation-grade design**, not just architecture.

---

# SECTION 4 — VECTOR INDEX (ocean-vector)

This is the **semantic memory layer** of Ocean.

It answers:

> “What does this chunk mean?”

---

# 4.1 Purpose of ocean-vector

ocean-vector is responsible for:

```text id="v4_purpose"
Chunks → Embeddings → Similarity Search
```

It provides:

* semantic search
* fuzzy recall
* meaning-based retrieval
* input for query engine

---

## Key Rule

> Vector index NEVER knows about files, folders, or graph

It only understands:

```text id="v4_rule"
Chunk → Vector → Similarity
```

---

# 4.2 Core Data Model

## 4.2.1 Embedding Record

```rust id="v4_embedding"
pub struct EmbeddingRecord {
    pub chunk_id: String,

    pub vector: Vec<f32>,

    pub model: String,

    pub dimension: usize,

    pub created_at: u64,
}
```

---

## 4.2.2 Why this structure matters

We must support:

* multiple embedding models (future-proof)
* reindexing
* versioning
* upgrades

---

# 4.3 Storage Schema (SQLite / Limbo)

## 4.3.1 Embeddings Table

```sql id="v4_sql_embeddings"
CREATE TABLE embeddings (
    chunk_id TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    dimension INTEGER NOT NULL,
    vector BLOB NOT NULL,
    created_at INTEGER
);
```

---

## 4.3.2 Vector encoding format

Store vector as:

* `f32` array → binary blob
* or compressed float16 (optimization later)

---

# 4.4 Embedder Interface (CORE ABSTRACTION)

This makes your system independent of any AI provider.

```rust id="v4_embedder"
pub trait Embedder {
    fn embed(&self, input: &str) -> Vec<f32>;

    fn dimension(&self) -> usize;

    fn model_name(&self) -> &str;
}
```

---

## Example implementation (future)

```rust id="v4_openai"
pub struct OpenAIEmbedder;

impl Embedder for OpenAIEmbedder {
    fn embed(&self, input: &str) -> Vec<f32> {
        // API call or local model
    }

    fn dimension(&self) -> usize {
        1536
    }

    fn model_name(&self) -> &str {
        "text-embedding-3-large"
    }
}
```

---

# 4.5 Indexing Pipeline (CORE FLOW)

This is the heart of vector system:

```text id="v4_pipeline"
Chunk → Embed → Store → Index
```

---

## 4.5.1 Function: index_chunks

```rust id="v4_index_fn"
pub fn index_chunks(
    chunks: Vec<Chunk>,
    embedder: &dyn Embedder,
    storage: &dyn VectorStorage
) -> Result<()> {

    for chunk in chunks {

        let vector = embedder.embed(&chunk.content);

        let record = EmbeddingRecord {
            chunk_id: chunk.id.clone(),
            vector,
            model: embedder.model_name().to_string(),
            dimension: embedder.dimension(),
            created_at: now(),
        };

        storage.insert(record)?;
    }

    Ok(())
}
```

---

# 4.6 Vector Storage Trait

This allows swapping SQLite → Limbo → custom ANN.

```rust id="v4_storage_trait"
pub trait VectorStorage {
    fn insert(&self, record: EmbeddingRecord) -> Result<()>;

    fn get(&self, chunk_id: &str) -> Option<EmbeddingRecord>;

    fn delete(&self, chunk_id: &str) -> Result<()>;

    fn search(&self, vector: &[f32], top_k: usize) -> Vec<SearchResult>;
}
```

---

## 4.6.1 Search Result

```rust id="v4_search_result"
pub struct SearchResult {
    pub chunk_id: String,
    pub score: f32,
}
```

---

# 4.7 Similarity Function

Core math:

```rust id="v4_cosine"
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x,y)| x*y).sum();
    let mag_a: f32 = a.iter().map(|x| x*x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x*x).sum::<f32>().sqrt();

    dot / (mag_a * mag_b)
}
```

---

# 4.8 Search Pipeline

## 4.8.1 Entry function

```rust id="v4_search"
pub fn search(
    query: &str,
    embedder: &dyn Embedder,
    storage: &dyn VectorStorage,
    top_k: usize
) -> Vec<SearchResult> {

    let query_vec = embedder.embed(query);

    storage.search(&query_vec, top_k)
}
```

---

# 4.9 ANN Index (Optional Upgrade)

For performance with 10k–1M chunks:

You will later replace linear scan with:

* HNSW (recommended)
* IVF
* Flat index (fallback)

---

## Option A: simple (start here)

```text id="v4_flat"
O(n) cosine similarity
```

---

## Option B: advanced (later)

Use:

* `hnsw_rs`
* or custom HNSW graph

---

# 4.10 Incremental Indexing

CRITICAL for your system.

## Rule:

> Never re-embed unchanged chunks

---

## 4.10.1 Change detection flow

```text id="v4_incremental"
File changed
   ↓
Re-chunk
   ↓
Compare chunk hashes
   ↓
Only embed new/changed chunks
```

---

# 4.11 Embedding Cache

Avoid re-calling model API.

```rust id="v4_cache"
pub struct EmbeddingCache {
    pub chunk_hash: String,
    pub vector: Vec<f32>,
}
```

---

# 4.12 Batch Embedding

For performance:

```rust id="v4_batch"
fn embed_batch(chunks: &[Chunk]) -> Vec<Vec<f32>> {
    // send multiple chunks at once
}
```

---

# 4.13 Vector Normalization (IMPORTANT)

Normalize all vectors:

```rust id="v4_norm"
fn normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x*x).sum::<f32>().sqrt();
    for x in v {
        *x /= norm;
    }
}
```

---

# 4.14 Multi-Model Support

Ocean must support multiple embedding models:

```text id="v4_models"
- local (bge, llama embeddings)
- cloud (OpenAI, Cohere)
- hybrid
```

Schema already supports:

```text id="v4_model_field"
model: String
```

---

# 4.15 Versioning Strategy

If model changes:

```text id="v4_versioning"
chunk_id + model_version → new embedding
```

Never overwrite silently.

---

# 4.16 Storage Layout

Example SQLite structure:

```text id="v4_layout"
embeddings table
    ├── chunk_id
    ├── model
    ├── vector
    ├── dimension
    ├── timestamp
```

---

# 4.17 Output Contract

Vector layer returns:

```rust id="v4_contract"
Vec<SearchResult>
```

OR:

```text id="v4_output"
Top-K chunk IDs ranked by similarity
```

---

# 4.18 Performance Strategy

## Must support:

* 10k–1M chunks
* sub-second search
* incremental updates

---

## Optimization techniques:

### 1. Pre-normalization

Vectors stored normalized → faster cosine

---

### 2. Cache query vectors

```text id="v4_cache2"
query → embedding cache
```

---

### 3. Batch indexing

Reduce embedding calls

---

### 4. ANN index (future upgrade)

Replace brute-force search

---

# 4.19 Integration Boundary

Vector layer ONLY talks to:

```text id="v4_boundary"
ocean-chunk (input)
ocean-query (output)
ocean-storage (persistence)
```

It does NOT:

* parse files
* build graph
* understand folders

---

# 4.20 Mental Model

Vector index answers:

```text id="v4_model"
"What chunks are semantically similar to this query?"
```

NOT:

* where file is (fs layer)
* how documents connect (graph layer)

---

# 4.21 Final Flow

```text id="v4_flow"
Chunk
  ↓
Embed
  ↓
Store Vector
  ↓
Query → Embed
  ↓
Similarity Search
  ↓
Top-K Results
```

---

# END OF SECTION 4

---

Next section:

# SECTION 5 — GRAPH INDEX (ocean-graph)

This is where we design:

* knowledge graph schema
* entity extraction
* relationship building
* traversal algorithms
* context expansion logic

Say **“next section”** and we continue.

# SECTION 5 — GRAPH INDEX (ocean-graph)

This is the **structural intelligence layer** of Ocean.

If vector index answers:

> “What is similar?”

Graph index answers:

> “What is connected, dependent, or related?”

---

# 5.1 Purpose of ocean-graph

ocean-graph is responsible for:

```text id="g5_purpose"
Chunks + Files + Entities → Relationships → Knowledge Graph
```

It enables:

* cross-document navigation
* context expansion
* dependency discovery
* multi-file reasoning
* topic clustering (structural, not semantic)

---

## Key Rule

> Graph index does NOT store meaning
> It stores **relationships**

---

# 5.2 Graph Mental Model

Everything becomes a node:

```text id="g5_nodes"
File
Chunk
Heading
Entity
Folder
Table
Sheet
Slide
```

Everything becomes an edge:

```text id="g5_edges"
contains
references
mentions
belongs_to
duplicates
same_topic
derived_from
```

---

# 5.3 Core Data Model

## 5.3.1 Node

```rust id="g5_node"
pub struct Node {
    pub id: String,
    pub node_type: NodeType,
    pub ref_id: String, // FileId or ChunkId
}
```

---

## 5.3.2 Node Types

```rust id="g5_node_type"
pub enum NodeType {
    File,
    Chunk,
    Heading,
    Entity,
    Folder,
}
```

---

## 5.3.3 Edge

```rust id="g5_edge"
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: RelationType,
}
```

---

## 5.3.4 Relation Types

```rust id="g5_relation"
pub enum RelationType {
    Contains,
    References,
    Mentions,
    BelongsTo,
    SimilarTo,
    DerivedFrom,
}
```

---

# 5.4 Storage Schema (SQLite / Limbo)

## 5.4.1 Nodes Table

```sql id="g5_nodes_sql"
CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    node_type TEXT NOT NULL,
    ref_id TEXT NOT NULL
);
```

---

## 5.4.2 Edges Table

```sql id="g5_edges_sql"
CREATE TABLE edges (
    from_id TEXT NOT NULL,
    to_id TEXT NOT NULL,
    relation TEXT NOT NULL
);
```

---

# 5.5 Graph Construction Pipeline

Graph is built AFTER chunking.

```text id="g5_pipeline"
Chunks → Entities → Nodes → Edges
```

---

## Step 1 — Create File Node

```text id="g5_step1"
File → Node(File)
```

---

## Step 2 — Create Chunk Nodes

```text id="g5_step2"
Each chunk → Node(Chunk)
```

---

## Step 3 — Extract structural edges

From chunk metadata:

```text id="g5_step3"
Chunk → belongs_to → File
Chunk → belongs_to → Heading
```

---

## Step 4 — Extract references

From text:

* "see policy document"
* "refer to contract"
* hyperlinks
* citations

---

## Step 5 — Entity extraction (basic version)

Simple heuristic first:

```text id="g5_entities"
Capitalized phrases
Repeated nouns
Known keywords (HR, Finance, etc.)
```

Later upgrade to NLP model.

---

# 5.6 Graph Builder Implementation

## 5.6.1 Main function

```rust id="g5_build"
pub fn build_graph(chunks: &[Chunk]) -> Vec<(Node, Edge)> {
    let mut nodes = vec![];
    let mut edges = vec![];

    for chunk in chunks {

        // create chunk node
        let chunk_node = Node {
            id: chunk.id.clone(),
            node_type: NodeType::Chunk,
            ref_id: chunk.id.clone(),
        };

        nodes.push(chunk_node.clone());

        // file relationship
        edges.push(Edge {
            from: chunk.file_id.clone(),
            to: chunk.id.clone(),
            relation: RelationType::Contains,
        });

        // heading relationship
        if let Some(heading) = &chunk.heading {
            let heading_id = format!("heading:{}", heading);

            edges.push(Edge {
                from: chunk.id.clone(),
                to: heading_id,
                relation: RelationType::BelongsTo,
            });
        }
    }

    (nodes, edges)
}
```

---

# 5.7 Graph Expansion Engine (CORE FEATURE)

This is what makes Ocean powerful.

## Input:

A chunk or node

## Output:

Connected knowledge subgraph

---

## 5.7.1 Expansion API

```rust id="g5_expand"
pub fn expand(node_id: &str, depth: usize) -> Vec<Node> {
    let mut visited = HashSet::new();
    let mut result = vec![];

    dfs(node_id, depth, &mut visited, &mut result);

    result
}
```

---

## 5.7.2 DFS traversal

```rust id="g5_dfs"
fn dfs(
    node: &str,
    depth: usize,
    visited: &mut HashSet<String>,
    result: &mut Vec<Node>,
) {
    if depth == 0 || visited.contains(node) {
        return;
    }

    visited.insert(node.to_string());

    let neighbors = get_neighbors(node);

    for n in neighbors {
        result.push(n.clone());
        dfs(&n.id, depth - 1, visited, result);
    }
}
```

---

# 5.8 Neighbor Retrieval

```rust id="g5_neighbors"
pub fn get_neighbors(node_id: &str) -> Vec<Node> {
    // SELECT * FROM edges WHERE from_id = node_id OR to_id = node_id
}
```

---

# 5.9 Graph Scoring (IMPORTANT)

Not all relationships are equal.

We assign weights:

```text id="g5_weights"
contains       = 1.0
belongs_to     = 0.8
references     = 0.7
mentions       = 0.5
similar_to     = 0.6
```

---

## Scoring function

```rust id="g5_score"
fn score(edge: &Edge) -> f32 {
    match edge.relation {
        Contains => 1.0,
        BelongsTo => 0.8,
        References => 0.7,
        Mentions => 0.5,
        SimilarTo => 0.6,
        _ => 0.3,
    }
}
```

---

# 5.10 Graph Query Types

## 1. Structural navigation

```text id="g5_q1"
"what is inside HR folder?"
```

→ uses contains edges

---

## 2. Dependency discovery

```text id="g5_q2"
"what documents does this refer to?"
```

→ uses references edges

---

## 3. Context expansion

```text id="g5_q3"
"expand leave policy context"
```

→ multi-hop traversal

---

# 5.11 Graph + Vector Integration (CRITICAL)

Graph is NOT independent.

It enhances vector search:

```text id="g5_hybrid"
Vector search → seed chunks
Graph expansion → context enrichment
```

---

## Example pipeline:

```text id="g5_hybrid_flow"
Query
  ↓
Vector Search (top 10 chunks)
  ↓
Graph expansion (neighbors)
  ↓
Re-rank
  ↓
Final context
```

---

# 5.12 Incremental Graph Updates

When file changes:

```text id="g5_incremental"
1. remove old nodes
2. remove old edges
3. rebuild only affected subtree
```

---

# 5.13 Graph Consistency Rule

> Every node must be traceable to a chunk or file

No orphan nodes.

---

# 5.14 Performance Strategy

## Must handle:

* 10k–100k nodes
* fast neighbor lookup
* multi-hop traversal

---

## Optimizations:

### 1. Index edges

```sql id="g5_index"
CREATE INDEX idx_from ON edges(from_id);
CREATE INDEX idx_to ON edges(to_id);
```

---

### 2. Cache adjacency list

```text id="g5_cache"
node_id → neighbors[]
```

---

### 3. Precompute frequent paths

Optional later

---

# 5.15 Output Contract

Graph layer returns:

```rust id="g5_output"
Vec<Node>
Vec<Edge>
```

OR:

```text id="g5_subgraph"
Subgraph(root, depth)
```

---

# 5.16 Mental Model

Graph index answers:

```text id="g5_model"
"What is connected to this knowledge?"
```

NOT:

* meaning (vector)
* structure parsing (parser)
* file location (fs)

---

# 5.17 Final Flow

```text id="g5_flow"
Chunks
  ↓
Nodes created
  ↓
Edges extracted
  ↓
Graph built
  ↓
Query expansion
```

---

# END OF SECTION 5

---

Next section:

# SECTION 6 — QUERY ENGINE (ocean-query)

This is where everything comes together:

* vector search
* graph expansion
* ranking fusion
* context window building

Say **“next section”** and we continue.


# SECTION 6 — QUERY ENGINE (ocean-query)

This is the **brain layer of Ocean**.

Everything you built so far converges here:

* filesystem (truth)
* parser (structure)
* chunker (units)
* vector index (meaning)
* graph index (relations)

Now we combine them into:

> **A single retrieval + reasoning system**

---

# 6.1 Purpose of ocean-query

ocean-query is responsible for:

```text id="q6_purpose"
User Query → Hybrid Retrieval → Ranked Context → Structured Output
```

It does NOT:

* parse files
* embed text
* build graph
* store data

It ONLY orchestrates.

---

# 6.2 Core Philosophy

Traditional search:

```text id="q6_trad"
query → search → results
```

Ocean query:

```text id="q6_ocean"
query → vector recall + graph expansion + ranking fusion → knowledge context
```

---

# 6.3 Query Engine Pipeline (CORE)

This is the most important system in Ocean.

```text id="q6_pipeline"
1. Embed query
2. Vector search (semantic recall)
3. Graph expansion (context growth)
4. Merge results
5. Deduplicate
6. Rank
7. Build context window
8. Return structured result
```

---

# 6.4 Core Data Types

## 6.4.1 Query Input

```rust id="q6_input"
pub struct Query {
    pub text: String,
    pub top_k: usize,
}
```

---

## 6.4.2 Query Result

```rust id="q6_result"
pub struct QueryResult {
    pub chunks: Vec<Chunk>,
    pub nodes: Vec<Node>,
    pub score: f32,
}
```

---

## 6.4.3 Final Response Context

```rust id="q6_context"
pub struct ContextWindow {
    pub text: String,
    pub sources: Vec<String>,
}
```

---

# 6.5 Main Query API

This is the public entrypoint.

```rust id="q6_api"
pub fn query(
    query: &str,
    vector: &dyn VectorStorage,
    graph: &dyn GraphStorage,
    embedder: &dyn Embedder,
) -> QueryResult {
```

---

# 6.6 Step 1 — Query Embedding

Convert query → vector space:

```rust id="q6_embed"
let q_vec = embedder.embed(query);
```

---

# 6.7 Step 2 — Vector Retrieval (Recall Layer)

Get semantically similar chunks:

```rust id="q6_vector"
let mut candidates = vector.search(&q_vec, 20);
```

This gives:

* raw semantic matches
* fuzzy relevance
* high recall

---

# 6.8 Step 3 — Graph Expansion (Context Layer)

For each vector result:

```rust id="q6_expand"
for chunk in candidates {
    let neighbors = graph.expand(&chunk.chunk_id, 2);
    expanded.push(neighbors);
}
```

This adds:

* related documents
* parent sections
* referenced policies
* sibling chunks

---

# 6.9 Step 4 — Merge Strategy

Now we combine:

* vector results
* graph results

```rust id="q6_merge"
let merged = merge(candidates, expanded);
```

---

## Merge rules:

* remove duplicates
* keep strongest signal
* preserve graph structure

---

# 6.10 Step 5 — Deduplication

```rust id="q6_dedup"
fn dedup(items: Vec<Chunk>) -> Vec<Chunk> {
    let mut seen = HashSet::new();

    items.into_iter()
        .filter(|c| seen.insert(c.id.clone()))
        .collect()
}
```

---

# 6.11 Step 6 — Ranking Fusion (VERY IMPORTANT)

We combine:

* vector similarity score
* graph relationship score
* structural relevance

---

## 6.11.1 Scoring model

```text id="q6_score"
final_score =
    (vector_score * 0.6) +
    (graph_score * 0.3) +
    (structure_score * 0.1)
```

---

## 6.11.2 Vector score

From cosine similarity.

---

## 6.11.3 Graph score

Based on edge weights:

```text id="q6_graph_score"
contains     = 1.0
references   = 0.7
mentions     = 0.5
belongs_to   = 0.8
```

---

## 6.11.4 Structure score

Boosts:

* same heading
* same file
* same section

---

# 6.12 Step 7 — Context Window Builder

Now we convert ranked chunks into LLM-ready context.

```rust id="q6_window"
pub fn build_context(chunks: Vec<Chunk>) -> ContextWindow {
```

---

## 6.12.1 Formatting rules

We structure output like:

```text id="q6_format"
[Source: file.pdf | Section: Leave Policy]

Employees are entitled to 20 days of leave...

---
```

---

## 6.12.2 Assembly logic

```rust id="q6_assemble"
let mut text = String::new();

for chunk in chunks {
    text.push_str(&format!(
        "[File: {} | Heading: {:?}]\n{}\n\n",
        chunk.file_id,
        chunk.heading,
        chunk.content
    ));
}
```

---

# 6.13 Step 8 — Source Tracking

Every output MUST include traceability:

```text id="q6_sources"
file_id → chunk_id → location
```

This enables:

* explainability
* debugging
* citation systems

---

# 6.14 Hybrid Retrieval Modes

Ocean query engine supports multiple modes:

---

## 1. FAST MODE (vector only)

```text id="q6_fast"
low latency Q&A
```

---

## 2. BALANCED MODE (vector + graph)

default mode

---

## 3. DEEP MODE (multi-hop graph expansion)

```text id="q6_deep"
research / exploration queries
```

---

# 6.15 Query Types Supported

## 6.15.1 Factual QA

> "What is leave policy?"

→ vector + top chunks

---

## 6.15.2 Exploration

> "Show all related HR documents"

→ graph traversal

---

## 6.15.3 Comparative

> "Compare policy A and B"

→ multi-chunk fusion

---

## 6.15.4 Root cause

> "Why was leave rejected?"

→ graph dependency trace

---

# 6.16 Performance Strategy

## 6.16.1 Cache query embeddings

```rust id="q6_cache"
query → vector cache
```

---

## 6.16.2 Cache frequent subgraphs

```text id="q6_subgraph"
node_id → neighbors
```

---

## 6.16.3 Limit graph depth

Prevent explosion:

```text id="q6_depth"
max_depth = 2 or 3
```

---

# 6.17 Query Engine Constraints

Hard rules:

* must be deterministic
* must be explainable
* must be stateless (except caches)
* must not modify data

---

# 6.18 Output Contract

```rust id="q6_contract"
pub struct QueryResult {
    pub chunks: Vec<Chunk>,
    pub context: ContextWindow,
    pub score: f32,
}
```

---

# 6.19 Mental Model

Query engine answers:

```text id="q6_model"
"What is the best structured knowledge context for this query?"
```

NOT:

* how data is stored
* how embeddings are computed
* how graph is built

---

# 6.20 Final System Flow

```text id="q6_flow"
User Query
   ↓
Embed Query
   ↓
Vector Search
   ↓
Graph Expansion
   ↓
Merge + Dedup
   ↓
Rank
   ↓
Build Context Window
   ↓
Return Result
```

---

# END OF SECTION 6

---

Next section:

# SECTION 7 — STORAGE LAYER (ocean-storage)

This is where we define:

* SQLite / Limbo schema design
* persistence model
* indexing durability
* transactional consistency
* cache strategy

Say **“next section”** and we continue.


# SECTION 7 — STORAGE LAYER (ocean-storage)

This is the **persistence backbone of Ocean**.

If everything else is the “brain”, storage is the **long-term memory + consistency layer**.

It ensures:

> everything you index can be rebuilt, queried, and updated reliably.

---

# 7.1 Purpose of ocean-storage

ocean-storage is responsible for:

```text id="s7_purpose"
- storing chunks metadata
- storing embeddings
- storing graph nodes/edges
- tracking file state
- enabling incremental updates
- ensuring durability
```

---

## Key Rule

> Storage is NOT intelligence
> Storage is NOT search
> Storage is ONLY persistence + retrieval

---

# 7.2 Storage Architecture Choice

You have 3 realistic options:

---

## Option A — SQLite (MVP)

* stable
* simple
* widely supported

---

## Option B — Limbo (Rust SQLite rewrite)

* modern Rust-native engine
* future-proof
* better integration for your system

---

## Option C — Hybrid (BEST DESIGN)

```text id="s7_hybrid"
SQLite/Limbo → structured data
plus
vector index → optimized storage layer
```

---

# 7.3 Core Storage Modules

```text id="s7_modules"
ocean-storage/
├── file_store
├── chunk_store
├── vector_store
├── graph_store
├── state_store
```

---

# 7.4 File Store (ocean-storage/file_store)

## Responsibility

Tracks file identity + changes.

---

## Schema

```sql id="s7_files"
CREATE TABLE files (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    hash TEXT NOT NULL,
    size INTEGER,
    modified INTEGER,
    extension TEXT,
    last_indexed INTEGER
);
```

---

## Purpose

* detect changes
* avoid reindexing unchanged files
* support incremental indexing

---

## API

```rust id="s7_file_api"
pub trait FileStore {
    fn upsert_file(&self, file: FileMeta);
    fn get_file(&self, id: &str) -> Option<FileMeta>;
    fn get_by_path(&self, path: &str) -> Option<FileMeta>;
}
```

---

# 7.5 Chunk Store (ocean-storage/chunk_store)

## Responsibility

Stores parsed + chunked content.

---

## Schema

```sql id="s7_chunks"
CREATE TABLE chunks (
    id TEXT PRIMARY KEY,
    file_id TEXT,
    content TEXT,
    heading TEXT,
    page INTEGER,
    start_offset INTEGER,
    end_offset INTEGER
);
```

---

## API

```rust id="s7_chunk_api"
pub trait ChunkStore {
    fn insert_chunk(&self, chunk: Chunk);
    fn get_chunk(&self, id: &str) -> Option<Chunk>;
    fn get_by_file(&self, file_id: &str) -> Vec<Chunk>;
}
```

---

# 7.6 Vector Store (ocean-storage/vector_store)

This connects storage to semantic search.

---

## Schema

```sql id="s7_vectors"
CREATE TABLE vectors (
    chunk_id TEXT PRIMARY KEY,
    model TEXT,
    dimension INTEGER,
    vector BLOB,
    created_at INTEGER
);
```

---

## Responsibilities

* store embeddings
* retrieve vectors
* support ANN index later

---

## API

```rust id="s7_vector_api"
pub trait VectorStore {
    fn insert(&self, record: EmbeddingRecord);
    fn get(&self, chunk_id: &str) -> Option<EmbeddingRecord>;
    fn delete(&self, chunk_id: &str);
}
```

---

# 7.7 Graph Store (ocean-storage/graph_store)

This is your knowledge network persistence.

---

## 7.7.1 Nodes Schema

```sql id="s7_nodes"
CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    node_type TEXT,
    ref_id TEXT
);
```

---

## 7.7.2 Edges Schema

```sql id="s7_edges"
CREATE TABLE edges (
    from_id TEXT,
    to_id TEXT,
    relation TEXT,
    weight REAL DEFAULT 1.0
);
```

---

## API

```rust id="s7_graph_api"
pub trait GraphStore {
    fn insert_node(&self, node: Node);
    fn insert_edge(&self, edge: Edge);

    fn get_neighbors(&self, node_id: &str) -> Vec<Node>;
    fn get_edges(&self, node_id: &str) -> Vec<Edge>;
}
```

---

# 7.8 State Store (ocean-storage/state_store)

This is critical for **incremental indexing correctness**.

---

## Schema

```sql id="s7_state"
CREATE TABLE index_state (
    file_id TEXT PRIMARY KEY,
    hash TEXT,
    last_indexed INTEGER,
    status TEXT
);
```

---

## Purpose

Tracks:

* last processed file state
* whether reindex is needed
* partial failures

---

## API

```rust id="s7_state_api"
pub trait StateStore {
    fn update_state(&self, file_id: &str, hash: &str);
    fn get_state(&self, file_id: &str) -> Option<FileState>;
}
```

---

# 7.9 Transaction Model (VERY IMPORTANT)

Ocean MUST guarantee consistency:

---

## Rule

> File → Chunk → Vector → Graph must be atomic

---

## Transaction Flow

```text id="s7_tx"
BEGIN
  insert chunks
  insert vectors
  insert graph nodes/edges
  update file state
COMMIT
```

---

## If failure occurs:

```text id="s7_fail"
ROLLBACK everything
```

---

# 7.10 Incremental Consistency Model

When file changes:

```text id="s7_incremental"
1. compare hash
2. if changed:
    - delete old chunks
    - delete old vectors
    - delete old graph nodes
    - reinsert fresh data
```

---

# 7.11 Indexing Strategy

Storage is optimized for:

* write-heavy indexing
* read-heavy querying

---

## Optimization 1 — Batch writes

```rust id="s7_batch"
insert_many(chunks)
```

---

## Optimization 2 — WAL mode (SQLite)

```sql id="s7_wal"
PRAGMA journal_mode=WAL;
```

---

## Optimization 3 — Prepared statements

Avoid repeated parsing.

---

# 7.12 Storage Abstraction Layer

## Core trait (master interface)

```rust id="s7_trait"
pub trait Storage {
    fn files(&self) -> &dyn FileStore;
    fn chunks(&self) -> &dyn ChunkStore;
    fn vectors(&self) -> &dyn VectorStore;
    fn graph(&self) -> &dyn GraphStore;
    fn state(&self) -> &dyn StateStore;
}
```

---

# 7.13 Limbo Integration (Future Upgrade)

If using Limbo:

* replace SQLite engine
* keep schema identical
* no logic change required

---

## Why this matters

Your architecture becomes:

```text id="s7_swap"
Storage Layer (pluggable)
   ↓
Ocean core unchanged
```

---

# 7.14 Performance Targets

Storage must support:

* 10k–1M chunks
* fast lookup by ID
* fast graph traversal
* fast vector retrieval

---

# 7.15 Index Design Strategy

## Indexes required:

```sql id="s7_indexes"
CREATE INDEX idx_file_id ON chunks(file_id);
CREATE INDEX idx_from ON edges(from_id);
CREATE INDEX idx_to ON edges(to_id);
CREATE INDEX idx_hash ON files(hash);
```

---

# 7.16 Cache Layer (Optional but recommended)

```text id="s7_cache"
hot chunks
hot vectors
hot graph neighbors
```

---

# 7.17 Storage Guarantees

Ocean storage guarantees:

## 1. Durability

No data loss after crash

---

## 2. Consistency

No partial indexing states

---

## 3. Rebuildability

Storage can be deleted and fully rebuilt from filesystem

---

# 7.18 Mental Model

Storage answers ONLY:

```text id="s7_model"
"Where is data stored and how do I retrieve it efficiently?"
```

NOT:

* meaning (vector layer)
* relationships reasoning (graph layer)
* parsing (parser layer)

---

# 7.19 Final Flow

```text id="s7_flow"
Index Pipeline
  ↓
Write chunks → storage
  ↓
Write vectors → storage
  ↓
Write graph → storage
  ↓
Update state → storage
  ↓
Commit
```

---

# END OF SECTION 7

---

Next section:

# SECTION 8 — INDEX ORCHESTRATOR (ocean-index)

This is where we combine EVERYTHING:

* fs → parser → chunk → vector → graph → storage

This is the **system controller layer**.

Say **“next section”** and we continue.


# SECTION 8 — INDEX ORCHESTRATOR (ocean-index)

This is the **control center of the entire Ocean system**.

If everything so far is components:

* fs = sensing
* parser = understanding structure
* chunk = segmentation
* vector = meaning
* graph = relationships
* storage = memory

Then ocean-index is:

> **the engine that runs the whole pipeline correctly, safely, and incrementally**

---

# 8.1 Purpose of ocean-index

ocean-index is responsible for:

```text id="i8_purpose"
Filesystem → Fully Indexed Knowledge System
```

It orchestrates:

* scanning
* parsing
* chunking
* vector indexing
* graph building
* storage writes
* incremental updates

---

## Key Rule

> ocean-index is the ONLY layer allowed to coordinate everything

No other module should “know the full pipeline”.

---

# 8.2 Core Design Principle

## Pipeline = deterministic state machine

```text id="i8_state"
File State → Parsed → Chunked → Embedded → Graphed → Stored
```

Every file must pass through this sequence.

---

# 8.3 High-Level Architecture

```text id="i8_arch"
           ocean-index
                │
 ┌──────────────┼────────────────┐
 │              │                │
fs scan     state check     update planner
 │              │                │
 └─────── pipeline executor ─────┘
                │
   ┌────────────┼────────────┐
   │            │            │
parser      chunker      vector/graph
   │            │            │
   └────────────┼────────────┘
                │
           storage layer
```

---

# 8.4 Core Entry Point

This is the main API:

```rust id="i8_index_api"
pub fn index_workspace(
    path: &str,
    fs: &dyn FileSystem,
    parser: &dyn ParserRegistry,
    chunker: &dyn Chunker,
    vector: &dyn VectorIndex,
    graph: &dyn GraphIndex,
    storage: &dyn Storage,
)
```

---

# 8.5 Indexing Pipeline (FULL FLOW)

## Step 1 — Scan filesystem

```rust id="i8_step1"
let files = fs.scan_dir(path);
```

---

## Step 2 — Filter changes (VERY IMPORTANT)

```rust id="i8_step2"
let changed_files = files
    .into_iter()
    .filter(|f| storage.state().needs_update(f))
    .collect();
```

---

## Step 3 — Process each file

```rust id="i8_step3"
for file in changed_files {
```

---

# 8.6 Step 4 — Parse File

```rust id="i8_parse"
let blocks = parser.parse(&file.path)?;
```

Output:

```text id="i8_blocks"
Vec<Block>
```

---

# 8.7 Step 5 — Chunking

```rust id="i8_chunk"
let chunks = chunker.chunk(blocks, file.id.clone());
```

Output:

```text id="i8_chunks"
Vec<Chunk>
```

---

# 8.8 Step 6 — Transaction Start (CRITICAL)

Everything must be atomic:

```rust id="i8_tx_start"
storage.begin_transaction()?;
```

---

# 8.9 Step 7 — Store Chunks

```rust id="i8_store_chunks"
for chunk in &chunks {
    storage.chunks().insert_chunk(chunk)?;
}
```

---

# 8.10 Step 8 — Vector Indexing

```rust id="i8_vectors"
vector.index_chunks(chunks.clone())?;
```

Internally:

* embed
* store
* normalize

---

# 8.11 Step 9 — Graph Building

```rust id="i8_graph"
let (nodes, edges) = graph.build(chunks.clone());

for node in nodes {
    storage.graph().insert_node(node)?;
}

for edge in edges {
    storage.graph().insert_edge(edge)?;
}
```

---

# 8.12 Step 10 — Update File State

```rust id="i8_state_update"
storage.state().update_state(
    &file.id,
    &file.hash
);
```

---

# 8.13 Step 11 — Commit Transaction

```rust id="i8_commit"
storage.commit_transaction()?;
```

---

# 8.14 Error Handling (VERY IMPORTANT)

If ANY step fails:

```text id="i8_fail"
ROLLBACK everything
```

---

## Example:

```rust id="i8_error"
if let Err(e) = process_file(file) {
    storage.rollback_transaction()?;
    log_error(e);
}
```

---

# 8.15 Incremental Indexing Engine

This is what makes Ocean powerful.

---

## Rule:

> Only reindex what changed

---

## Detection logic:

```rust id="i8_detect"
if old_hash != new_hash {
    reindex(file)
}
```

---

## Optimization:

Skip:

* unchanged files
* unchanged chunks
* unchanged embeddings

---

# 8.16 Parallel Execution (CRITICAL FOR SCALE)

Use parallel pipeline:

```rust id="i8_parallel"
files.par_iter().for_each(|file| {
    process(file);
});
```

---

## BUT with safety:

* chunking parallel OK
* embedding parallel OK
* storage writes must be controlled

---

# 8.17 Pipeline Stages (Formal Model)

Each file passes:

```text id="i8_stages"
SCAN
 → FILTER
 → PARSE
 → CHUNK
 → EMBED
 → GRAPH
 → STORE
 → STATE UPDATE
```

---

# 8.18 Indexing Modes

## Mode 1 — FULL INDEX

```text id="i8_full"
Rebuild entire workspace
```

Used for:

* first run
* corruption recovery

---

## Mode 2 — INCREMENTAL

```text id="i8_inc"
Only changed files
```

Default mode

---

## Mode 3 — WATCH MODE

```text id="i8_watch"
real-time file updates
```

---

# 8.19 File Processing Unit

We define a reusable pipeline unit:

```rust id="i8_unit"
pub fn process_file(file: FileMeta) -> Result<()> {
```

Encapsulates full lifecycle.

---

# 8.20 Backpressure Strategy (IMPORTANT)

If system is overloaded:

* throttle file ingestion
* batch indexing
* queue updates

---

```text id="i8_queue"
file queue → worker pool → pipeline
```

---

# 8.21 Logging & Observability

Every step emits events:

```rust id="i8_events"
IndexEvent::FileParsed
IndexEvent::ChunksCreated
IndexEvent::VectorsIndexed
IndexEvent::GraphBuilt
```

---

# 8.22 Index Consistency Guarantees

Ocean-index guarantees:

## 1. Atomicity

File is either fully indexed or not indexed at all

---

## 2. Idempotency

Re-running indexing produces same result

---

## 3. Determinism

Same file → same chunks → same graph

---

# 8.23 Performance Targets

Must handle:

* 10k–100k files
* nested directories
* incremental updates
* parallel execution

---

# 8.24 Mental Model

ocean-index answers:

```text id="i8_model"
"How do I transform raw filesystem into fully indexed knowledge system safely and efficiently?"
```

---

# 8.25 Final System Flow

```text id="i8_flow"
Filesystem Scan
   ↓
Change Detection
   ↓
Parse
   ↓
Chunk
   ↓
Embed
   ↓
Graph Build
   ↓
Storage Write
   ↓
State Update
   ↓
Commit
```

---

# END OF SECTION 8

---

Next section:

# SECTION 9 — QUERY + STORAGE INTEGRATION LAYER

This is where we:

* connect query engine to storage
* optimize retrieval paths
* build fast read pipelines
* implement caching strategy for runtime

Say **“next section”** and we continue.


# SECTION 9 — QUERY + STORAGE INTEGRATION LAYER (ocean-query ↔ ocean-storage)

This section is where Ocean becomes **fast in practice**, not just correct in design.

You already have:

* storage (Section 7)
* query engine (Section 6)
* vector index
* graph index

Now we remove abstraction gaps and make them work as **one read system**.

---

# 9.1 Purpose of this layer

This layer is responsible for:

```text id="q9_purpose"
Query Engine → Efficient Reads from Storage → Hybrid Retrieval → Context Assembly
```

It defines:

* how queries actually hit SQLite/Limbo
* how vector + graph are combined in real runtime
* how caching works
* how to avoid slow multi-table queries

---

## Key Rule

> Query layer must NEVER recompute data
> It only reads + combines + ranks

---

# 9.2 System Bottleneck Reality

Without this layer:

* vector search is fast
* graph traversal is slow
* joins become expensive
* context building is redundant

So we introduce:

> **Read Optimization Layer (ROL)**

---

# 9.3 Core Architecture

```text id="q9_arch"
                Query Engine
                     │
        ┌────────────┼────────────┐
        │            │            │
   Vector Store   Graph Store   Chunk Store
        │            │            │
        └─────── Optimized Read Layer ───────┘
                     │
                Storage (SQLite/Limbo)
```

---

# 9.4 Core Design Principle

## Instead of:

* multiple DB queries per request

## We use:

> **single-pass retrieval + in-memory fusion**

---

# 9.5 Query Execution Pipeline (REAL SYSTEM)

```text id="q9_pipeline"
1. Embed query
2. Vector lookup (top K chunks)
3. Batch fetch chunks from storage
4. Graph expansion (batched edges)
5. Fetch neighbors in bulk
6. Merge + score
7. Build context
```

---

# 9.6 Storage Access Patterns (CRITICAL)

We optimize around 3 patterns:

---

## Pattern 1 — Chunk batch fetch

```rust id="q9_chunk_fetch"
SELECT * FROM chunks WHERE id IN (...)
```

NEVER:

```text id="q9_bad"
SELECT * per chunk (N+1 problem)
```

---

## Pattern 2 — Edge batch fetch

```sql id="q9_edges"
SELECT * FROM edges
WHERE from_id IN (...) OR to_id IN (...)
```

---

## Pattern 3 — Vector lookup (already indexed)

Direct primary key or ANN result.

---

# 9.7 Integrated Query Struct

We define a runtime query context:

```rust id="q9_context"
pub struct QueryContext {
    pub query: String,
    pub query_vector: Vec<f32>,

    pub vector_hits: Vec<SearchResult>,
    pub chunks: Vec<Chunk>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
```

---

# 9.8 Main Integrated Query Function

```rust id="q9_query_fn"
pub fn execute_query(
    query: &str,
    embedder: &dyn Embedder,
    storage: &dyn Storage,
    graph: &dyn GraphStore,
    vector: &dyn VectorStore,
) -> QueryContext {
```

---

## Step 1 — Embed query

```rust id="q9_step1"
let q_vec = embedder.embed(query);
```

---

## Step 2 — Vector search (fast recall)

```rust id="q9_step2"
let hits = vector.search(&q_vec, 20);
```

---

## Step 3 — Batch fetch chunks

```rust id="q9_step3"
let chunk_ids: Vec<_> = hits.iter().map(|h| &h.chunk_id).collect();
let chunks = storage.chunks().get_batch(chunk_ids);
```

---

## Step 4 — Graph expansion (batch optimized)

Instead of per-node expansion:

```rust id="q9_step4"
let mut node_ids = chunks.iter().map(|c| c.id.clone()).collect();

let edges = graph.get_edges_batch(&node_ids);
```

---

## Step 5 — Neighbor extraction

```rust id="q9_step5"
let neighbor_nodes = graph.get_nodes_from_edges(&edges);
```

---

## Step 6 — Merge everything

```rust id="q9_step6"
let mut all_chunks = chunks.clone();
all_chunks.extend(fetch_chunks_from_nodes(&neighbor_nodes));
```

---

# 9.9 Deduplication Strategy

```rust id="q9_dedup"
fn dedup_chunks(mut chunks: Vec<Chunk>) -> Vec<Chunk> {
    let mut seen = HashSet::new();

    chunks.retain(|c| seen.insert(c.id.clone()));

    chunks
}
```

---

# 9.10 Ranking Fusion (FINAL STAGE)

We compute final relevance:

```text id="q9_score"
final_score =
    vector_score * 0.55 +
    graph_score  * 0.30 +
    locality     * 0.15
```

---

## Vector score

From cosine similarity.

---

## Graph score

Based on:

* edge type
* hop distance
* relationship strength

---

## Locality score

Boost if:

* same file
* same heading
* same section

---

# 9.11 Context Builder (LLM READY OUTPUT)

```rust id="q9_builder"
pub fn build_context(chunks: Vec<Chunk>) -> String {
```

---

## Output format:

```text id="q9_format"
[File: policy.pdf | Section: Leave Policy]

Employees are entitled to 20 days of leave...

---

[File: hr_manual.pdf | Section: Benefits]

Leave must be approved by manager...
```

---

# 9.12 Context Window Optimization

We enforce:

```text id="q9_limit"
Max tokens: 4k–16k depending on config
```

---

## Strategy:

* rank chunks
* include top N
* truncate low-score chunks

---

# 9.13 Caching Layer (VERY IMPORTANT)

We introduce runtime cache:

---

## 9.13.1 Query cache

```text id="q9_cache_query"
query → results
```

---

## 9.13.2 Vector cache

```text id="q9_cache_vec"
query → embedding
```

---

## 9.13.3 Graph cache

```text id="q9_cache_graph"
node → neighbors
```

---

# 9.14 Hot Path Optimization

Critical hot path:

```text id="q9_hot"
query → vector → chunk fetch → return
```

Graph expansion is optional.

---

# 9.15 Query Modes

## Mode 1 — FAST

```text id="q9_fast"
vector only
```

---

## Mode 2 — BALANCED (default)

```text id="q9_balanced"
vector + graph
```

---

## Mode 3 — DEEP

```text id="q9_deep"
multi-hop graph expansion
```

---

# 9.16 Performance Bottlenecks Solved

| Problem              | Solution          |
| -------------------- | ----------------- |
| N+1 queries          | batch fetch       |
| slow graph traversal | adjacency caching |
| repeated embeddings  | cache             |
| large context        | ranking cutoff    |

---

# 9.17 Final Query Contract

```rust id="q9_contract"
pub struct QueryResponse {
    pub chunks: Vec<Chunk>,
    pub context: String,
    pub score: f32,
}
```

---

# 9.18 Mental Model

This layer answers:

```text id="q9_model"
"How do I efficiently retrieve and combine all relevant knowledge from storage?"
```

---

NOT:

* how data is indexed
* how embeddings are computed
* how graph is built

---

# 9.19 Final Integrated Flow

```text id="q9_flow"
Query
  ↓
Embed
  ↓
Vector search
  ↓
Batch chunk fetch
  ↓
Graph expansion
  ↓
Batch neighbor fetch
  ↓
Merge
  ↓
Rank
  ↓
Build context
  ↓
Return result
```

---

# END OF SECTION 9

---

Next section:

# SECTION 10 — PERFORMANCE, SCALING & RUNTIME ARCHITECTURE

This is where we design:

* concurrency model
* memory management
* caching hierarchy
* indexing throughput
* 100k–1M file scaling strategy

Say **“next section”** and we continue.


# SECTION 10 — PERFORMANCE, SCALING & RUNTIME ARCHITECTURE

This section is where Ocean becomes a **real production-grade system**, not just a correct design.

We focus on:

> how to make it fast, scalable, memory-safe, and stable under 10k–1M files

---

# 10.1 Purpose of this layer

This layer defines:

```text id="s10_purpose"
- concurrency model
- memory control
- throughput strategy
- caching hierarchy
- backpressure handling
- system limits
```

---

## Key Rule

> Correctness comes from earlier layers
> This layer ensures **speed + stability under load**

---

# 10.2 System Reality: Where performance breaks

Ocean will break in 4 places:

1. parsing large PDFs
2. embedding batches
3. graph expansion explosion
4. disk IO saturation

So we design around these explicitly.

---

# 10.3 Global Runtime Model

We use a **pipeline worker architecture**:

```text id="s10_pipeline"
Filesystem Events
      ↓
   Job Queue
      ↓
 Worker Pool
      ↓
 Pipeline Stages
 (parse → chunk → embed → graph → store)
```

---

# 10.4 Concurrency Model (CORE DESIGN)

We use 3-tier concurrency:

---

## Tier 1 — IO concurrency (high)

* filesystem scan
* disk reads
* DB queries

```text id="s10_io"
high parallelism (rayon / async)
```

---

## Tier 2 — CPU concurrency (medium)

* parsing
* chunking
* graph building

```text id="s10_cpu"
bounded thread pool
```

---

## Tier 3 — AI/Embedding concurrency (limited)

* API calls
* model inference

```text id="s10_ai"
strict rate-limited pool
```

---

# 10.5 Worker Pool Architecture

```rust id="s10_worker"
pub struct WorkerPool {
    pub cpu_pool: ThreadPool,
    pub io_pool: ThreadPool,
    pub ai_semaphore: Semaphore,
}
```

---

## Why semaphore?

To avoid:

* API rate limit crashes
* memory spikes
* GPU overload

---

# 10.6 Pipeline Execution Model

Each file is a **job unit**:

```text id="s10_job"
FileJob {
    file_id,
    path,
    priority,
    retry_count
}
```

---

## Execution stages:

```text id="s10_stages"
SCAN → PARSE → CHUNK → EMBED → GRAPH → STORE
```

Each stage is independent worker step.

---

# 10.7 Backpressure System (CRITICAL)

When system is overloaded:

---

## Strategy 1 — Queue limit

```text id="s10_queue"
max_queue_size = 10,000
```

---

## Strategy 2 — Drop or delay low priority jobs

* recent file changes = high priority
* bulk indexing = low priority

---

## Strategy 3 — Adaptive throttling

```text id="s10_throttle"
if CPU > 80% → slow workers
if memory > 75% → pause ingestion
```

---

# 10.8 Memory Management Strategy

Ocean must avoid full file loading.

---

## Rule 1 — Streaming parsing

```text id="s10_stream"
never load full PDF into memory
```

---

## Rule 2 — Chunk-level processing

Process one chunk batch at a time.

---

## Rule 3 — Drop intermediate data

After each stage:

* discard raw blocks
* keep only chunks + metadata

---

# 10.9 Caching Hierarchy (VERY IMPORTANT)

We define 4 cache layers:

---

## L1 — In-memory hot cache

```text id="s10_l1"
recent queries
recent chunks
recent embeddings
```

---

## L2 — Local disk cache

```text id="s10_l2"
embedding cache
parsed file cache
```

---

## L3 — DB cache (SQLite/Limbo)

```text id="s10_l3"
persistent chunks + vectors
```

---

## L4 — recomputation fallback

If cache fails → recompute pipeline

---

# 10.10 Query Performance Optimization

We optimize query path:

---

## Hot path:

```text id="s10_hot"
query → vector search → chunk fetch → return
```

Must be:

* < 50ms (cached)
* < 200ms (cold)

---

## Graph expansion lazy load

Graph is NOT always loaded:

```text id="s10_lazy"
only expand if needed
```

---

# 10.11 Indexing Throughput Strategy

We want:

> 10k–100k files indexed efficiently

---

## Strategy 1 — Batch indexing

```text id="s10_batch"
process 100–500 files per batch
```

---

## Strategy 2 — Parallel chunking

Split CPU workload across threads.

---

## Strategy 3 — Async embedding

Pipeline decouples embedding step.

---

# 10.12 Bottleneck Controls

---

## 1. Parser bottleneck

Fix:

* streaming parsing
* format-specific optimization

---

## 2. Embedding bottleneck

Fix:

* batching
* caching
* async queue

---

## 3. Graph explosion

Fix:

```text id="s10_graph"
limit depth = 2–3
limit neighbors per node
```

---

## 4. Storage IO bottleneck

Fix:

* WAL mode
* batch writes
* prepared statements

---

# 10.13 Thread Model (RECOMMENDED)

```text id="s10_threads"
1 thread → filesystem watcher
N threads → parsing
M threads → chunking
1–N threads → embedding queue
1 thread → DB writer
```

---

# 10.14 Queue System Design

We use priority queue:

```rust id="s10_queue_struct"
pub struct JobQueue {
    pub high_priority: VecDeque<FileJob>,
    pub normal: VecDeque<FileJob>,
    pub low: VecDeque<FileJob>,
}
```

---

# 10.15 Retry & Fault Tolerance

If pipeline fails:

---

## Retry policy:

```text id="s10_retry"
max retries = 3
backoff = exponential
```

---

## Failure handling:

* log failure
* mark file as "partial"
* retry later

---

# 10.16 Scalability Targets

Ocean must support:

| Scale      | Capability           |
| ---------- | -------------------- |
| 10k files  | smooth               |
| 100k files | optimized            |
| 1M files   | partitioned indexing |

---

# 10.17 Partition Strategy (for large systems)

Split by:

* folder
* file type
* hash range

---

```text id="s10_partition"
workspace → shards → independent indexes
```

---

# 10.18 Runtime Modes

## Mode 1 — Desktop mode

* single thread pool
* low memory

---

## Mode 2 — Server mode

* distributed workers
* high throughput

---

## Mode 3 — Embedded mode

* minimal resources
* lazy indexing

---

# 10.19 Observability Layer

We track:

* indexing speed
* query latency
* cache hit rate
* memory usage

---

## Event system:

```rust id="s10_events"
IndexStarted
ChunkCreated
VectorIndexed
GraphUpdated
QueryExecuted
```

---

# 10.20 Mental Model

This layer answers:

```text id="s10_model"
"How do we make Ocean fast, stable, and scalable under real-world load?"
```

---

NOT:

* how data is represented
* how search works
* how graph is built

---

# 10.21 Final Runtime Flow

```text id="s10_flow"
Filesystem Events
      ↓
Job Queue
      ↓
Worker Pool
      ↓
Parse → Chunk → Embed → Graph → Store
      ↓
Cache Update
      ↓
Query Engine Reads Optimized Storage
```

---

# END OF SECTION 10

---

Next section:

# SECTION 11 — FINAL SYSTEM INTEGRATION & PRODUCTION ARCHITECTURE

This is where we:

* connect all modules into one system
* define full lifecycle
* API boundaries
* deployment model
* runtime topology

Say **“next section”** and we finish the full architecture.


# SECTION 11 — FINAL SYSTEM INTEGRATION & PRODUCTION ARCHITECTURE (Ocean COMPLETE SYSTEM)

This is the final layer where Ocean becomes a **deployable, real-world system**.

We now connect everything into a single coherent product:

> filesystem → indexing → storage → query → runtime → scaling

---

# 11.1 Purpose of this section

This section defines:

```text id="p11_purpose"
- full system wiring
- module boundaries
- runtime topology
- deployment model
- lifecycle management
- production readiness
```

---

## Key Rule

> No new logic is introduced here
> Only integration of all previous systems

---

# 11.2 Full System Architecture

```text id="p11_arch"
                 ┌──────────────────────┐
                 │     ocean-query        │
                 └─────────┬────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
  ocean-vector         ocean-graph        ocean-storage
        │                  │                  │
        └──────────────────┼──────────────────┘
                           │
                    ocean-index
                           │
                    ocean-chunk
                           │
                    ocean-parser
                           │
                    ocean-fs
```

---

# 11.3 System Lifecycle (END-TO-END)

## Phase 1 — Initialization

```text id="p11_init"
start system
load storage
restore state
load caches
```

---

## Phase 2 — Indexing

```text id="p11_index"
scan filesystem
detect changes
parse files
chunk content
embed vectors
build graph
store everything
```

---

## Phase 3 — Query runtime

```text id="p11_query"
receive query
vector search
graph expansion
merge results
rank
return context
```

---

## Phase 4 — Continuous sync

```text id="p11_sync"
watch filesystem
trigger incremental indexing
update storage
invalidate caches
```

---

# 11.4 Module Boundaries (STRICT CONTRACTS)

Each module is isolated:

---

## ocean-fs

```text id="p11_fs"
ONLY:
- file scanning
- file watching
```

---

## ocean-parser

```text id="p11_parser"
ONLY:
- convert file → blocks
```

---

## ocean-chunk

```text id="p11_chunk"
ONLY:
- blocks → chunks
```

---

## ocean-vector

```text id="p11_vector"
ONLY:
- chunks → embeddings
- similarity search
```

---

## ocean-graph

```text id="p11_graph"
ONLY:
- relationships
- traversal
```

---

## ocean-storage

```text id="p11_storage"
ONLY:
- persistence
- retrieval
```

---

## ocean-index

```text id="p11_index"
ONLY:
- orchestration of indexing pipeline
```

---

## ocean-query

```text id="p11_query"
ONLY:
- retrieval + ranking + context building
```

---

# 11.5 Deployment Architecture

We support 3 deployment modes:

---

## 1. Local Desktop Mode

```text id="p11_local"
single binary
local SQLite/Limbo
local filesystem watch
```

Use case:

* personal AI knowledge system

---

## 2. Server Mode

```text id="p11_server"
API service
multi-user
shared index
worker pools
```

Use case:

* enterprise knowledge base

---

## 3. Distributed Mode (future)

```text id="p11_distributed"
sharded indexes
remote vector nodes
graph partitioning
```

Use case:

* massive datasets (millions of docs)

---

# 11.6 Runtime Topology

## Core runtime components:

```text id="p11_runtime"
- Index Worker Pool
- Query Engine
- Storage Engine
- Cache Layer
- File Watcher
```

---

## Execution model:

```text id="p11_exec"
event-driven + pipeline-based
```

---

# 11.7 System API Layer (FINAL INTERFACE)

This is what external apps will use:

---

## 11.7.1 Index API

```rust id="p11_api_index"
fn index_workspace(path: &str)
```

---

## 11.7.2 Query API

```rust id="p11_api_query"
fn query(text: &str) -> QueryResponse
```

---

## 11.7.3 Watch API

```rust id="p11_api_watch"
fn watch(path: &str)
```

---

## 11.7.4 Rebuild API

```rust id="p11_api_rebuild"
fn rebuild_index()
```

---

# 11.8 Data Lifecycle Guarantee

Every file follows:

```text id="p11_lifecycle"
File → Parsed → Chunked → Indexed → Stored → Queryable
```

And always reversible:

```text id="p11_rebuild"
Storage deleted → full rebuild from filesystem
```

---

# 11.9 Consistency Model

We guarantee:

## 1. Eventual consistency

* updates propagate asynchronously

---

## 2. Deterministic rebuild

* same input → same index output

---

## 3. Atomic indexing per file

* no partial corruption

---

# 11.10 Performance Envelope

Expected production scale:

| Scale      | Behavior         |
| ---------- | ---------------- |
| 10k files  | instant          |
| 100k files | optimized batch  |
| 1M files   | sharded indexing |

---

# 11.11 Failure Recovery Model

If system crashes:

```text id="p11_recovery"
resume from last file state
rebuild only incomplete jobs
```

No full reindex needed.

---

# 11.12 Cache System Final Design

## 3-tier cache:

```text id="p11_cache"
L1: RAM cache (hot queries)
L2: disk cache (embeddings)
L3: DB (persistent storage)
```

---

# 11.13 Security Model (optional but important)

If used in enterprise:

* sandbox filesystem access
* read-only index mode
* query isolation per tenant

---

# 11.14 Observability Stack

System emits:

```text id="p11_obs"
Index events
Query events
Cache hits/misses
Latency metrics
Graph traversal stats
```

---

# 11.15 Mental Model (FINAL)

Ocean is:

```text id="p11_model"
A deterministic knowledge compiler:
Filesystem → Structured Knowledge Graph + Vector Memory → Queryable Intelligence Layer
```

---

NOT:

* a chatbot
* a database
* a search engine

It is:

> a **knowledge operating system**

---

# 11.16 FINAL END-TO-END FLOW

```text id="p11_flow"
FILESYSTEM
    ↓
INDEX ENGINE (ocean-index)
    ↓
PARSE → CHUNK → VECTOR → GRAPH
    ↓
STORAGE LAYER (ocean-storage)
    ↓
QUERY ENGINE (ocean-query)
    ↓
VECTOR + GRAPH + CHUNK FUSION
    ↓
CONTEXT WINDOW
    ↓
OUTPUT (AI / API / UI)
```

---

# 11.17 FINAL SYSTEM SUMMARY

You now have:

### Core engine

* deterministic indexing pipeline
* incremental updates

### Memory system

* vector semantic layer
* graph structural layer

### Storage system

* SQLite/Limbo persistence
* transactional consistency

### Runtime system

* worker pools
* caching layers
* backpressure control

### Query system

* hybrid retrieval
* ranking fusion
* context builder

---

# END OF FULL ARCHITECTURE
