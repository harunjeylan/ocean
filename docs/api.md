# Ocean API Reference

## Architecture Overview

Ocean is a single-crate Rust library (`ocean`, edition 2024) for document ingestion, chunking, embedding, graph construction, and semantic query. It transforms unstructured documents (PDF, DOCX, PPTX, XLSX, TXT, MD, HTML) into structured, searchable, and interconnected knowledge.

### Data Flow

```
Filesystem → Parser → Chunker → Vector/Graph → Query
   (raw)      (blocks)  (chunks)   (embeddings)
                                    (relationships)
```

### Derivation Chain

```
Files → Blocks → Chunks → Embeddings → Graph
```

Each step is one-way: metadata preserves the source chain so every derived unit is traceable back to its origin.

### Foundation Design Rules

1. **Filesystem is source of truth** — all data originates from files on disk. The scanner indexes what exists; changes to the filesystem propagate through re-indexing.
2. **Derivation chain is one-way** — Files → Blocks → Chunks → Embeddings → Graph. Each step adds structure; none modifies prior steps except through explicit re-indexing.
3. **No format awareness outside the parser** — The chunker, embedder, graph builder, and query engine operate on parsed blocks and chunks only. They never touch raw file formats.
4. **Every data unit is traceable** — Every chunk, embedding, and graph node records its source file id and location (page, slide, heading path, byte offset) so provenance is always preserved.

### 3-Layer Memory Model

| Layer | What | Technology | Module |
|-------|------|-----------|--------|
| Source | Physical filesystem | walkdir, SHA-256 | ocean_fs |
| Structure | Parsed document blocks | Format backends, ChunkBuffer | ocean_parser, ocean_chunk |
| Intelligence | Embedding vectors + Graph | Embedder traits, SurrealDB | ocean_vector, ocean_graph |

- **Source Layer**: File metadata, hashes, watcher events, scanner inventory.
- **Structure Layer**: ReadResult blocks (Text, Table, Slide, Page, etc.) and derived Chunks with heading hierarchy, token estimates, and overlapping splits.
- **Intelligence Layer**: Dense vector embeddings for semantic search and a property graph of structural, reference, and entity relationships for context expansion.

### Public Modules (in `lib.rs`)

```rust
pub mod ocean_fs;        // Source layer: scanning, hashing, watching, indexing
pub mod ocean_parser;    // Structure layer: document reading, format backends
pub mod ocean_cli;       // CLI dispatch, args, display, config, walk
pub mod ocean_chunk;     // Structure layer: semantic chunking
pub mod ocean_vector;    // Intelligence layer: embedding, vector store, search
pub mod ocean_graph;     // Intelligence layer: graph store, builder, expansion
pub mod ocean_query;     // Intelligence layer: unified query engine
pub mod ocean_storage;   // Core: PersistentKeyValueStore trait + SurrealDB impl
pub mod ocean_index;     // Intelligence layer: indexing pipeline orchestrator
pub mod ocean_api;       // Facade: high-level public API
pub mod ocean_cache;     // Intelligence layer: embedding cache (LRU)
```

---

## Module Reference

### ocean_fs

**Purpose**: Filesystem layer — file identity, scanning, hashing, watching, and file metadata persistence.

#### Key Types

**`FileMeta`**
```rust
pub struct FileMeta {
    pub file_id: Uuid,
    pub path: PathBuf,
    pub size: u64,
    pub sha256: Option<String>,
    pub extension: String,
    pub category: FileCategory,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub indexed_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}
```

**`FileCategory`**
```rust
pub enum FileCategory {
    Document,    // pdf, docx, doc, txt, md, html, htm, rtf, odt
    Spreadsheet, // xlsx, xls, csv, ods
    Presentation,// pptx, ppt, odp
    Image,       // png, jpg, jpeg, gif, bmp, svg, webp
    Unknown,
}
```

**`FileId`** — type alias for `Uuid` (UUIDv7 from `uuid::Uuid::now_v7()`).

**`PathResolver`** — wraps SurrealDB connection (in-memory or file-backed). Bridges sync-to-async via internal `tokio::runtime::Runtime`. Methods:
- `new(db_path: &str) -> Self` — file-backed store
- `in_memory() -> Self` — ephemeral in-memory store
- `store_file_meta(&self, meta: &FileMeta) -> Result<()>`
- `get_file_meta(&self, file_id: Uuid) -> Result<Option<FileMeta>>`
- `file_exists(&self, path: &Path) -> Result<bool>`
- `list_all_files(&self) -> Result<Vec<FileMeta>>`
- `delete_file(&self, file_id: Uuid) -> Result<()>`

**`FileScanner`** — uses `walkdir` + `rayon` parallel. Configuration:
- `max_depth: Option<usize>`
- `follow_links: bool`
- `skip_hidden: bool` (default true)
- `skip_dirs: Vec<String>` (default: `[".git", "node_modules", ".cache"]`)
- `extensions: Vec<String>` (default: supported document/image types)
Methods:
- `scan(&self, root: &Path) -> Result<Vec<PathBuf>>`
- `scan_with_meta(&self, root: &Path, hasher: &FileHasher) -> Result<Vec<FileMeta>>`

**`FileHasher`** — streaming SHA-256 with 64KB buffer. Rejects files > 4GB. Methods:
- `new() -> Self`
- `hash_file(&self, path: &Path) -> Result<String>` — returns hex string
- `hash_reader(&self, reader: impl Read) -> Result<String>`

**`FileWatcher`** — uses `notify` + `crossbeam_channel`. 100ms debounce, `MAX_BATCH_SIZE = 100`. Events:
- `FileWatcherEvent::Created(PathBuf)`
- `FileWatcherEvent::Modified(PathBuf)`
- `FileWatcherEvent::Removed(PathBuf)`
- `FileWatcherEvent::Error(String)`

#### Key Functions

- `generate_file_id() -> Uuid` — UUIDv7 from `uuid::Uuid::now_v7()`
- `normalize_file_category(ext: &str) -> FileCategory`
- `is_supported_extension(ext: &str) -> bool`

#### Re-exports (from `mod.rs`)

```rust
pub use types::*;
pub use scanner::*;
pub use hasher::*;
pub use watcher::*;
pub use resolver::*;
pub use normalizer::*;
```

---

### ocean_parser

**Purpose**: Document parsing — reads files into structured blocks (ReadResult). Backend-agnostic via the Document trait.

#### Key Traits

**`Document`** — object-safe trait for all format backends:
```rust
pub trait Document {
    fn metadata(&self) -> Result<HashMap<String, String>>;
    fn outline(&self) -> Result<Vec<OutlineEntry>>;
    fn page_count(&self) -> Result<usize>;
    fn search(&self, query: &str) -> Result<Vec<ReadResult>>;
    fn read(&self, selector: &Selector) -> Result<Vec<ReadResult>>;
}
```

**`DocumentFactory`** — trait for backend registry:
```rust
pub trait DocumentFactory: Send + Sync {
    fn open(&self, path: &Path) -> Result<Box<dyn Document>>;
}
```

#### Key Enums

**`Selector`** — 14 variants for reading document sections:
```rust
pub enum Selector {
    Page(usize),
    Pages(usize, usize),
    Heading(String),
    Paragraph(usize),
    Table(usize),
    Row(usize, usize),
    Column(usize, usize),
    Cell(usize, usize, usize),
    Sheet(usize),
    Slide(usize),
    Image(usize),
    Note(usize),
    Range(usize, usize),
    Slice(usize, usize),
}
```

**`ReadResult`** — 10 variants representing parsed content blocks:
```rust
pub enum ReadResult {
    Text(String, HashMap<String, String>),
    Table(Vec<Vec<String>>, HashMap<String, String>),
    Image(Vec<u8>, String, HashMap<String, String>),
    Metadata(HashMap<String, String>),
    Outline(Vec<OutlineEntry>),
    Page(usize, Vec<ReadResult>),
    Slide(usize, Vec<ReadResult>),
    Sheet(usize, String, Vec<ReadResult>),
    CellValue(String),
    MatchResult(String, usize, String),
}
```

**`OutlineEntry`**
```rust
pub struct OutlineEntry {
    pub level: usize,
    pub title: String,
    pub page: Option<usize>,
    pub children: Vec<OutlineEntry>,
}
```

#### Backend Registry

**`BackendRegistry`** — global `OnceLock<Arc<dyn DocumentFactory>>`. Auto-initialized on first `open()`. Registers all 7 backends.

#### Free Functions (in `read_api.rs`)

- `open(path: &Path) -> Result<Box<dyn Document>>` — auto-detect format from extension, open via registry
- `read(path: &Path, selector: &Selector) -> Result<Vec<ReadResult>>`
- `read_page(path: &Path, page: usize) -> Result<Vec<ReadResult>>`
- `read_heading(path: &Path, heading: &str) -> Result<Vec<ReadResult>>`
- `read_all_blocks(path: &Path) -> Result<Vec<ReadResult>>`

#### Format Backends (7)

| Backend | File | Key Library | Selectors |
|---------|------|-------------|-----------|
| TXT | `txt.rs` | std::fs lines | Paragraph, Range, Slice |
| Markdown | `markdown.rs` | std::fs lines | Heading, Paragraph, Range, Slice |
| HTML | `html.rs` | quick-xml | Heading, Paragraph, Table, Image, Range, Slice |
| DOCX | `docx.rs` | zip + quick-xml | Paragraph, Heading, Table, Image, Slice (page-break-aware) |
| PPTX | `pptx.rs` | zip + quick-xml | Slide, Image, Paragraph, Note, Slice |
| XLSX | `xlsx.rs` | calamine | Sheet, Cell, Table, Slice |
| PDF | `pdf.rs` | lopdf | Page, Pages, Heading, Range, Slice |

Each backend implements `DocumentFactory` and `Document`.

#### Re-exports

```rust
pub use backend::*;
pub use selector::*;
pub use read_result::*;
pub use document::*;
pub use registry::*;
pub use read_api::*;
pub use outline::*;
pub use txt::*;
pub use markdown::*;
pub use html::*;
pub use docx::*;
pub use pptx::*;
pub use xlsx::*;
pub use pdf::*;
```

---

### ocean_chunk

**Purpose**: Semantic chunking — splits parsed blocks into chunks suitable for embedding.

#### Key Types

**`Chunk`**
```rust
pub struct Chunk {
    pub id: Uuid,
    pub file_id: Uuid,
    pub chunk_type: ChunkType,
    pub content: String,
    pub heading_path: Vec<String>,
    pub page_number: Option<usize>,
    pub slide_number: Option<usize>,
    pub sheet_name: Option<String>,
    pub table_index: Option<usize>,
    pub image_data: Option<Vec<u8>>,
    pub image_mime: Option<String>,
    pub token_count: Option<usize>,
    pub byte_offset: u64,
    pub byte_length: u64,
    pub content_hash: String,
    pub created_at: DateTime<Utc>,
}
```

**`ChunkType`**
```rust
pub enum ChunkType {
    Text,
    Table,
    Page,
    Slide,
    Sheet,
    Cell,
    Image,
    Metadata,
    Heading,
}
```

**`ChunkConfig`**
```rust
pub struct ChunkConfig {
    pub min_tokens: usize,           // default 50
    pub max_tokens: usize,           // default 512
    pub overlap_sentences: usize,    // default 1
    pub include_images: bool,        // default false
    pub rows_per_sheet_chunk: usize, // default 50
    pub token_estimator: TokenEstimator, // default GPT4
}
```

**`TokenEstimator`**
```rust
pub enum TokenEstimator {
    GPT4,       // ~4 chars per token
    Claude,     // ~3.5 chars per token
    Exact(usize), // custom chars per token
}
```

#### Key Functions

**`chunk(blocks: Vec<ReadResult>, file_id: Uuid, config: &ChunkConfig) -> Vec<Chunk>`**

Main chunking function. Processes `ReadResult` blocks through:
1. Heading detection via `detect_heading()`
2. Text accumulation in `ChunkBuffer`
3. Sentence-boundary splitting via `split_with_overlap()`
4. Atomic emit for tables, slides, sheets
5. Post-processing: merge adjacent same-type chunks under same heading if combined tokens ≤ max_tokens

**`detect_heading(line: &str) -> Option<usize>`** — detects `# ` through `###### ` markdown headings with leading/trailing whitespace handling. Returns heading level (1-6) or None.

**`split_with_overlap(text: &str, config: &ChunkConfig) -> Vec<String>`** — sentence-boundary splitting using regex `[.!?]\s+` with configurable overlap (number of sentences).

**`ChunkBuffer`** — accumulates text chunks, flushes on heading/slide/sheet boundary, atomic emit for tables/slides/sheets. Methods:
- `new(file_id, config)`
- `add_block(block: ReadResult) -> Vec<Chunk>` — returns flushed chunks
- `flush() -> Vec<Chunk>` — final flush

#### Re-exports

```rust
pub use types::*;
pub use chunker::*;
pub use config::*;
pub use buffer::*;
pub use heading::*;
pub use split::*;
```

---

### ocean_vector

**Purpose**: Embedding generation and vector similarity search.

#### Key Traits

**`Embedder`** — trait for embedding backends:
```rust
pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError>;
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedderError>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> String;
}
```

Embedders auto-normalize returned vectors to unit length.

**`EmbedderError`**
```rust
pub enum EmbedderError {
    ApiError(String),
    RateLimited(String),
    AuthError(String),
    InvalidResponse(String),
    Timeout(String),
}
```

#### Embedder Backends

| Backend | Source | Endpoint |
|---------|--------|----------|
| `OllamaEmbedder` | `ollama.rs` | `POST /api/embed` |
| `OpenAIEmbedder` | `openai.rs` | OpenAI-compatible API |
| `AnthropicEmbedder` | `anthropic.rs` | x-api-key auth |
| `GeminiEmbedder` | `gemini.rs` | Google Generative Language API |

All constructors accept `(model: String, api_key: Option<String>, base_url: Option<String>)`.

#### Key Types

**`VectorStore`** — SurrealDB-backed vector store. Schema: `chunk` table with HNSW index on `embedding` field. Methods:
- `new(db_path: &str) -> Result<Self>`
- `in_memory() -> Result<Self>`
- `insert_chunk(chunk: &Chunk, embedding: &[f32]) -> Result<()>`
- `upsert_chunk(chunk: &Chunk, embedding: &[f32]) -> Result<()>`
- `insert_chunks_batch(chunks: &[IndexedChunk]) -> Result<()>`
- `get_chunk(chunk_id: Uuid) -> Result<Option<IndexedChunk>>`
- `delete_chunks_by_file(file_id: Uuid) -> Result<()>`
- `count() -> Result<u64>`
- `chunk_exists(content_hash: &str) -> Result<bool>`
- `vector_search(embedding: &[f32], top_k: usize, filter: Option<SearchFilter>) -> Result<Vec<RankedChunk>>`
- `fts_search(query: &str, top_k: usize, filter: Option<SearchFilter>) -> Result<Vec<RankedChunk>>`

**`IndexedChunk`**
```rust
pub struct IndexedChunk {
    pub chunk: Chunk,
    pub embedding: Vec<f32>,
    pub file_path: Option<String>,
}
```

**`RankedChunk`**
```rust
pub struct RankedChunk {
    pub chunk: Chunk,
    pub score: f32,
    pub file_path: Option<String>,
}
```

**`SearchEngine`** — orchestrates search strategies:
- `new(store: Arc<VectorStore>) -> Self`
- `search(embedding: &[f32], top_k: usize, filter: Option<SearchFilter>) -> Result<Vec<RankedChunk>>` — KNN only
- `hybrid_search(query: &str, embedding: &[f32], top_k: usize, filter: Option<SearchFilter>) -> Result<Vec<RankedChunk>>` — vector + FTS + RRF (k=60)
- `filtered_search(embedding: &[f32], filter: &SearchFilter, top_k: usize) -> Result<Vec<RankedChunk>>`
- `hybrid_filtered_search(query: &str, embedding: &[f32], filter: &SearchFilter, top_k: usize) -> Result<Vec<RankedChunk>>`

**`SearchFilter`**
```rust
pub struct SearchFilter {
    pub file_id: Option<Uuid>,
    pub heading_prefix: Option<String>,
    pub block_type: Option<ChunkType>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
}
```
Builder pattern: `SearchFilter::new().with_file_id(id).with_heading_prefix("Intro").with_block_type(ChunkType::Text)`

**`IndexPipeline`** — batches chunks for embedding + storage:
```rust
pub struct IndexPipeline {
    pub store: Arc<VectorStore>,
    pub batch_size: usize,
    pub reindex: bool,
}
```
- `index_chunks(&self, chunks: Vec<Chunk>, embedder: &dyn Embedder) -> Result<IndexReport>`
- Deduplicates by `content_hash` unless `reindex: true`

**`IndexReport`**
```rust
pub struct IndexReport {
    pub total: usize,
    pub indexed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub duration: Duration,
}
```

#### Error Types

```rust
pub enum StoreError { Connection(String), Query(String), Serialization(String), NotFound(String), ... }
pub enum SearchError { Store(StoreError), Embedding(EmbedderError), InvalidFilter(String), ... }
```

#### Re-exports

```rust
pub use embedder::*;
pub use ollama::*;
pub use openai::*;
pub use anthropic::*;
pub use gemini::*;
pub use store::*;
pub use search::*;
pub use index::*;
pub use error::*;
pub use mock::*;
```

---

### ocean_graph

**Purpose**: Knowledge graph — structural, reference, and entity relationships between chunks.

#### Key Types

**`Node`**
```rust
pub struct Node {
    pub id: Uuid,
    pub node_type: NodeType,
    pub ref_id: Uuid,
    pub label: String,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}
```

**`Edge`**
```rust
pub struct Edge {
    pub id: Uuid,
    pub from_node: Uuid,
    pub to_node: Uuid,
    pub relation: RelationType,
    pub weight: f64,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}
```

**`NodeType`**
```rust
pub enum NodeType {
    File,
    Chunk,
    Heading,
    Entity,
    Folder,
}
```

**`RelationType`**
```rust
pub enum RelationType {
    Contains,
    References,
    Mentions,
    BelongsTo,
    DerivedFrom,
    SimilarTo,
    CrossReference,
}
```

**`GraphStore`** — SurrealDB-backed (`graph_node` and `graph_edge` tables). Methods:
- `new(db_path: &str) -> Result<Self>`
- `in_memory() -> Result<Self>`
- `insert_node(node: &Node) -> Result<()>`
- `get_node(id: Uuid) -> Result<Option<Node>>`
- `delete_nodes_by_file(file_id: Uuid) -> Result<()>`
- `insert_edge(edge: &Edge) -> Result<()>`
- `get_neighbors(node_id: Uuid, relation: Option<RelationType>) -> Result<Vec<(Node, Edge)>>`
- `count_nodes() -> Result<u64>`
- `count_edges() -> Result<u64>`
- `clear() -> Result<()>`

**`GraphConfig`**
```rust
pub struct GraphConfig {
    pub extract_references: bool,      // default true
    pub extract_entities: bool,        // default true
    pub max_expansion_depth: usize,    // default 3
    pub entity_min_frequency: usize,   // default 3
    pub default_edge_weight: f64,      // default 1.0
}
```

**`GraphBuilder`** — builds graph from chunks:
- `from_chunks(chunks: &[Chunk], file_id: Uuid, config: &GraphConfig, store: &GraphStore) -> Result<BuildReport>`
- Structural edges: `File →Contains→ Chunk`, `Chunk →BelongsTo→ File/Heading`
- Reference edges (see/refer to/as per/per patterns)
- Entity edges via `EntityExtractor`

**`EntityExtractor`** — heuristic extraction:
- Capitalized phrases (3+ consecutive capitalized words, excluding sentence-start)
- Repeated words (frequency ≥ `entity_min_frequency`, excluding stopwords)

**`ExpansionEngine`** — BFS graph traversal:
- `new(store: Arc<GraphStore>) -> Self`
- `expand(node_id: Uuid, depth: usize, direction: Direction) -> Result<Vec<(Node, Edge, usize)>>`
- `expand_from_chunks(chunk_ids: &[Uuid], depth: usize) -> Result<Vec<(Node, Edge, usize)>>`
- `find_path(from: Uuid, to: Uuid, max_depth: usize) -> Result<Vec<Vec<(Node, Edge)>>>`
- `get_file_graph(file_id: Uuid) -> Result<Vec<(Node, Edge)>>`

```rust
pub enum Direction { Forward, Backward, Both }
```

**`BuildReport`**
```rust
pub struct BuildReport {
    pub nodes: usize,
    pub edges: usize,
    pub structural_edges: usize,
    pub reference_edges: usize,
    pub entity_edges: usize,
    pub errors: Vec<String>,
    pub duration: Duration,
}
```

#### Re-exports

```rust
pub use types::*;
pub use store::*;
pub use builder::*;
pub use extractor::*;
pub use expansion::*;
pub use config::*;
```

---

### ocean_query

**Purpose**: Unified query engine — wraps vector search, hybrid search, graph expansion, and context window building.

#### Key Types

**`QueryEngine`**
```rust
pub struct QueryEngine {
    store: Arc<VectorStore>,
    search: SearchEngine,
    graph: Option<Arc<ExpansionEngine>>,
}
```
- `new(db_path: &str) -> Result<Self>` — file-backed store
- `new_memory() -> Result<Self>` — in-memory for tests
- `query(&self, q: Query, embedder: &dyn Embedder) -> Result<QueryResult>`

**`Query`**
```rust
pub struct Query {
    pub mode: QueryMode,
    pub top_k: usize,
    pub expand_depth: Option<usize>,
    pub filter: Option<SearchFilter>,
    pub include_context: bool,
    pub context_chunks: usize,
    pub rerank_by_heading: bool,
    pub rerank_by_file: bool,
}
```

Builder: `Query::new().with_mode(QueryMode::Hybrid).with_top_k(10)`

**`QueryMode`**
```rust
pub enum QueryMode {
    Auto,    // heuristic select_mode()
    Vector,  // KNN only
    Hybrid,  // vector + FTS + RRF
    Expand,  // hybrid + graph expansion
}
```

**`select_mode(query: &str, expand_depth: Option<usize>) -> QueryMode`** — pure function:
- expand_depth > 0 → Expand
- query < 3 words → Vector
- cross-ref phrases (see also, refer to, etc.) → Expand
- 3+ words → Hybrid
- empty → Hybrid

**`QueryResult`**
```rust
pub struct QueryResult {
    pub results: Vec<RankedChunk>,
    pub context_windows: Vec<ContextWindow>,
    pub execution: ExecutionMeta,
}
```

**`ContextWindow`**
```rust
pub struct ContextWindow {
    pub source_chunk: RankedChunk,
    pub chunks: Vec<ContextChunk>,
}
```

**`ContextChunk`**
```rust
pub struct ContextChunk {
    pub chunk: Chunk,
    pub position: String, // "before", "after"
    pub distance: usize,
}
```

**`ExecutionMeta`**
```rust
pub struct ExecutionMeta {
    pub mode: QueryMode,
    pub plan: ExecutionPlan,
    pub duration: Duration,
    pub vector_hits: usize,
    pub fts_hits: usize,
    pub expansion_hits: usize,
    pub total_candidates: usize,
}
```

**`ExecutionPlan`** — ordered `Vec<SubQuery>`:
```rust
pub enum SubQuery {
    Vector,
    Fts,
    RrfFusion,
    GraphExpand,
    RerankByHeading,
    RerankByFile,
    BuildContext,
}
```

**`ContextWindowBuilder`** — fetches adjacent chunks in same file + heading scope. Never crosses heading boundaries. Clamps to `[1, 10]` chunks.

#### Public API Functions

```rust
pub fn query(engine: &QueryEngine, q: Query, embedder: &dyn Embedder) -> Result<QueryResult>
pub fn query_stream(
    engine: &QueryEngine,
    q: Query,
    embedder: &dyn Embedder,
) -> impl Stream<Item = Result<QueryEvent>>
```

#### Re-exports

```rust
pub use engine::*;
pub use types::*;
pub use api::*;
```

---

### ocean_storage

**Purpose**: Core storage abstraction — trait defining persistent key-value store operations, with SurrealDB implementation.

#### Key Trait

**`PersistentKeyValueStore`**
```rust
pub trait PersistentKeyValueStore: Send + Sync {
    fn get(&self, table: &str, key: &str) -> Result<Option<Vec<u8>>>;
    fn set(&self, table: &str, key: &str, value: &[u8]) -> Result<()>;
    fn delete(&self, table: &str, key: &str) -> Result<()>;
    fn exists(&self, table: &str, key: &str) -> Result<bool>;
    fn list_keys(&self, table: &str) -> Result<Vec<String>>;
    fn clear_table(&self, table: &str) -> Result<()>;
}
```

#### Key Types

**`DbConfig`**
```rust
pub struct DbConfig {
    pub db_path: String,
    pub in_memory: bool,
}
```

**`StorageResult<T>`** — type alias for `Result<T, StorageError>`.

**`StorageError`**
```rust
pub enum StorageError {
    Connection(String),
    Query(String),
    Serialization(String),
    Deserialization(String),
    NotFound(String),
    Duplicate(String),
    Transaction(String),
}
```

#### SurrealDB Implementation

Internally uses SurrealDB with `SurrealKv` (embedded RocksDB) or in-memory engine. Bridged via `tokio::runtime::Runtime` for sync-style API.

#### Re-exports

```rust
pub use store::*;
pub use error::*;
```

---

### ocean_index

**Purpose**: Indexing pipeline orchestrator — coordinates scanning, parsing, chunking, embedding, and graph building into a single runnable pipeline.

#### Key Types

**`IndexConfig`**
```rust
pub struct IndexConfig {
    pub batch_size: usize,           // default 50
    pub db_path: String,
    pub reindex: bool,
    pub no_graph: bool,
    pub no_references: bool,
    pub no_entities: bool,
    pub embedder: EmbedderConfig,
}
```

**`EmbedderConfig`** — provider/connection settings for embedding backends.

**`IndexPipeline`**
```rust
pub struct IndexPipeline {
    config: IndexConfig,
    store: Arc<VectorStore>,
    graph_store: Option<Arc<GraphStore>>,
}
```
- `new(config: IndexConfig) -> Result<Self>`
- `index_directory(&self, dir: &Path, embedder: &dyn Embedder) -> Result<IndexSummary>`
- `index_files(&self, files: &[PathBuf], embedder: &dyn Embedder) -> Result<IndexSummary>`

**`IndexSummary`**
```rust
pub struct IndexSummary {
    pub files_scanned: usize,
    pub files_indexed: usize,
    pub files_failed: usize,
    pub total_chunks: usize,
    pub total_embeddings: usize,
    pub graph_nodes: Option<usize>,
    pub graph_edges: Option<usize>,
    pub duration: Duration,
    pub errors: Vec<String>,
}
```

### Integration Flow

1. Scan directory for supported files
2. Parse each file via `ocean_parser::read_all_blocks()`
3. Chunk blocks via `ocean_chunk::chunk()`
4. Embed chunks via embedder in batches via `VectorStore::insert_chunks_batch()`
5. Optionally build graph structure, reference, and entity edges via `GraphBuilder::from_chunks()`
6. Return `IndexSummary`

#### Re-exports

```rust
pub use pipeline::*;
pub use config::*;
```

---

### ocean_api

**Purpose**: High-level facade — simplified public API that coordinates all subsystems (parser, chunker, vector, graph, query) with sensible defaults.

#### Key Types

**`Ocean`** — main application handle:
```rust
pub struct Ocean {
    config: OceanConfig,
    store: Arc<VectorStore>,
    graph: Option<Arc<GraphStore>>,
    cache: Arc<EmbeddingCache>,
}
```
- `new(config: OceanConfig) -> Result<Self>`
- `ingest_file(&self, path: &Path, embedder: &dyn Embedder) -> Result<IngestResult>`
- `ingest_directory(&self, dir: &Path, embedder: &dyn Embedder, reindex: bool) -> Result<Vec<IngestResult>>`
- `search(&self, query_text: &str, embedder: &dyn Embedder, options: SearchOptions) -> Result<QueryResult>`
- `query_engine(&self) -> Result<QueryEngine>`

**`IngestResult`**
```rust
pub struct IngestResult {
    pub file_path: PathBuf,
    pub file_id: Uuid,
    pub chunks: usize,
    pub graph_nodes: Option<usize>,
    pub graph_edges: Option<usize>,
    pub duration: Duration,
}
```

**`SearchOptions`**
```rust
pub struct SearchOptions {
    pub mode: QueryMode,
    pub top_k: usize,
    pub expand_depth: Option<usize>,
    pub include_context: bool,
    pub filter: Option<SearchFilter>,
}
```

#### Re-exports

```rust
pub use ocean::*;
```

---

### ocean_cache

**Purpose**: Embedding cache — LRU cache to avoid re-embedding identical text.

#### Key Types

**`EmbeddingCache`**
```rust
pub struct EmbeddingCache {
    capacity: usize,
    // internal: LruCache<u64, Vec<f32>>  — keyed by content_hash
}
```
- `new(capacity: usize) -> Self` (default: 1024)
- `get(content_hash: &str) -> Option<Vec<f32>>`
- `set(content_hash: &str, embedding: Vec<f32>)`
- `clear()`
- `len() -> usize`
- `hit_rate() -> f64` — cache hit ratio since creation

**`CachedEmbedder`** — wraps any `Embedder` with LRU caching:
```rust
pub struct CachedEmbedder<E: Embedder> {
    inner: E,
    cache: EmbeddingCache,
}
```
- Implements `Embedder` trait
- Delegates `embed()`: checks cache first, falls back to inner embedder, stores result
- Delegates `embed_batch()`: batch-aware — embeds only uncached texts

#### Re-exports

```rust
pub use cache::*;
pub use cached::*;
```

---

### ocean_cli

**Purpose**: Command-line interface — argument parsing, command dispatch, output display, configuration loading, file walking.

#### Key Types

**`Cli`** — clap argument struct:
```rust
pub struct Cli {
    pub command: Commands,
}
```

**`Commands`** — 12 subcommands:
```rust
pub enum Commands {
    Info { file: String },
    Metadata { file: String },
    Outline { file: String },
    PageCount { file: String },
    Search { file: String, query: String },
    Grep { dir: String, query: String },
    Read { file: String, args: ReadArgs },
    Scan { dir: String, no_hash: bool },
    Hash { file: String },
    Verify { file: String, hash: String },
    Watch { dir: String },
    Chunk { file: String, args: ChunkArgs },
    Index { dir: String, args: IndexArgs },
    Query { query: String, args: QueryArgs },
    VectorSearch { query: String, args: VectorArgs },
}
```

**`ReadArgs`** — selector arguments:
```rust
pub struct ReadArgs {
    pub page: Option<usize>,
    pub heading: Option<String>,
    pub paragraph: Option<usize>,
    pub table: Option<usize>,
    pub slide: Option<usize>,
    pub sheet: Option<usize>,
    pub cell: Option<String>,
    pub image: Option<usize>,
    pub range: Option<String>,
    pub skip: Option<usize>,
    pub take: Option<usize>,
}
```

**`ChunkArgs`**
```rust
pub struct ChunkArgs {
    pub min_size: Option<usize>,
    pub max_size: Option<usize>,
    pub overlap: Option<usize>,
    pub include_images: bool,
    pub rows_per_chunk: Option<usize>,
}
```

**`OceanConfig`** — serde-deserialized from `.ocean/config.json`:
```rust
pub struct OceanConfig {
    pub embedding: EmbeddingConfig,
    pub index: IndexSection,
    pub query: QuerySection,
}
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub dimension: Option<usize>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}
pub struct IndexSection {
    pub batch_size: Option<usize>,
    pub db_path: Option<String>,
    pub reindex: Option<bool>,
    pub no_graph: Option<bool>,
    pub no_references: Option<bool>,
    pub no_entities: Option<bool>,
}
pub struct QuerySection {
    pub top_k: Option<usize>,
    pub db_path: Option<String>,
    pub mode: Option<String>,
    pub expand_depth: Option<usize>,
    pub context: Option<bool>,
    pub context_chunks: Option<usize>,
    pub verbose: Option<bool>,
}
```

Config loaded from `CWD/.ocean/config.json` (local, priority) and `~/.ocean/config.json` / `%APPDATA%/ocean/config.json` (global), merged with local priority. Env vars in values resolved via `${VAR}` syntax. `.env` files loaded from `~/.ocean/.env` → `CWD/.env` → `CWD/.ocean/.env`.

Resolution order: CLI flags > config file > `.env` > hardcoded defaults.

Default DB path: `~/.ocean/database/{cwd-kebab-case}.db` (auto-computed from CWD directory name).

#### Key Functions

- `run()` — main entry point, dispatched from `src/main.rs` and `src/cli.rs`
- `cmd_info()`, `cmd_metadata()`, `cmd_outline()`, `cmd_page_count()` — document introspection
- `cmd_search()` — single-file full-text search
- `cmd_grep()` — recursive directory search
- `cmd_read()` — selector-based reading with skip/take
- `cmd_scan()` — list supported files with metadata
- `cmd_hash()` — compute SHA-256
- `cmd_verify()` — verify hash
- `cmd_watch()` — monitor directory
- `cmd_chunk()` — semantic chunking
- `cmd_index()` — full indexing pipeline
- `cmd_query()` — unified semantic query
- `cmd_vector_search()` — vector search (legacy, preserved for backwards compatibility)

#### Display Functions (in `display.rs`)

- `print_meta(meta: &HashMap<String, String>)`
- `print_outline(entries: &[OutlineEntry])`
- `print_read_result(results: &[ReadResult])`
- Formats output for terminal display

#### Walking (in `walk.rs`)

- `walk_supported_files(dir: &Path) -> Vec<PathBuf>` — recursive walk, filters by `SUPPORTED_EXTS`
- `SUPPORTED_EXTS: &[&str]` — `[pdf, docx, pptx, xlsx, txt, md, html, htm, png, jpg, jpeg]`

#### Re-exports

```rust
pub use args::*;
pub use display::*;
pub use walk::*;
pub use run::*;
pub use config::*;
```
