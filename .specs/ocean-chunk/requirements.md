# Requirements Document: ocean-chunk — Semantic Chunk Engine

## Introduction

Phase 3 of Ocean introduces the **semantic chunk engine (ocean-chunk)** — the bridge between document structure and searchable intelligence. The parser layer (`ocean-parser`) produces `ReadResult` values representing a document's structural blocks. The chunker converts these blocks into **Chunks**: self-contained, context-aware, size-bounded meaningful units that form the atomic input to both the vector index (semantic search) and graph index (relationship building).

The chunker answers only: "What is the smallest meaningful unit of knowledge in this document?" It must respect structural boundaries (headings, tables), enforce size constraints (100–800 tokens), and produce chunks that are independently useful for retrieval.

---

## Glossary

- **Chunk**: The fundamental search unit — a self-contained piece of content with traceability to its source file and structural context (heading, page, offset).
- **Block**: A `ReadResult` variant from `ocean-parser` — `Text`, `Table`, `Page`, `Slide`, `Sheet`, `CellValue`, `Image`, `Metadata`, `Outline`, `MatchResult`.
- **Structural Boundary**: A natural division point in a document — heading change, table boundary, page break, slide boundary, sheet boundary.
- **Heading Context**: The nearest preceding heading in document order, used to enrich every chunk's semantic context.
- **Token Budget**: The approximate token count of a chunk's content string, estimated at ~4 characters per token for size limit enforcement.
- **Flush**: The operation of finalizing a chunk buffer, creating a `Chunk` record, and resetting the buffer for new content.
- **Atomic Chunk**: A block type (e.g., `Table`, `Image`) that must not be split across multiple chunks.
- **Overlap Strategy**: A sliding window approach where consecutive chunks share a small amount of boundary content to prevent information loss at split points.

---

## Requirements

### R1: Chunk Definition

**User Story:** As the vector and graph indexing layers, I want a well-defined chunk structure so that I can embed, store, and trace chunks independently.

#### Acceptance Criteria

1. THE system SHALL define a `Chunk` struct with fields:
   - `id: String` — UUIDv7 unique identifier
   - `file_id: String` — UUIDv7 from `ocean_fs::FileMeta`
   - `content: String` — the textual content of the chunk
   - `heading: Option<String>` — nearest preceding heading text
   - `page: Option<u32>` — page number if applicable (PDF, DOCX page-break-aware)
   - `slide: Option<u32>` — slide number if applicable (PPTX)
   - `sheet: Option<String>` — sheet name if applicable (XLSX)
   - `block_type: ChunkType` — the original block type this chunk was derived from
   - `start_offset: Option<usize>` — character offset in original content
   - `end_offset: Option<usize>` — character offset in original content
2. THE system SHALL define `ChunkType` enum with variants: `Text`, `Table`, `Page`, `Slide`, `Sheet`, `Cell`, `Image`, `Metadata`, `Heading`.

---

### R2: Chunking Input

**User Story:** As a consumer of ocean-chunk, I want to pass parser output directly to the chunker so that the pipeline is seamless.

#### Acceptance Criteria

1. THE system SHALL accept `ReadResult` values from `ocean-parser` as input to the chunking process.
2. THE system SHALL accept a flat `Vec<ReadResult>` representing the full document in document order.
3. THE system SHALL also accept individual `ReadResult` values for incremental/streaming chunking.
4. THE `chunk()` function SHALL take `file_id: String` and `blocks: Vec<ReadResult>` as parameters.

---

### R3: Heading-Boundary Chunking

**User Story:** As a user querying documents, I want chunks to respect heading boundaries so that content under different headings is not mixed.

#### Acceptance Criteria

1. THE system SHALL track the current heading context as it iterates through blocks in document order.
2. ON encountering a `ReadResult::Text` or `ReadResult::Page` whose content starts with a known heading pattern, the system SHALL flush the current buffer and start a new heading context.
3. WHEN a new `ReadResult` appears under a different heading than the current buffer, the system SHALL flush the buffer before appending.
4. Headings themselves SHALL be chunked as atomic `ChunkType::Heading` chunks.
5. The heading text SHALL be stored in every chunk's `heading` field that falls under that heading.

---

### R4: Table Atomicity

**User Story:** As a user querying tabular data, I want tables to remain intact so that row-column relationships are preserved.

#### Acceptance Criteria

1. THE system SHALL NOT split a `ReadResult::Table` across multiple chunks.
2. EACH `ReadResult::Table` SHALL be emitted as a single atomic chunk with `block_type: ChunkType::Table`.
3. The table content SHALL be serialized to a textual representation (pipe-delimited or similar) for embedding compatibility.
4. Large tables exceeding the maximum chunk size SHALL trigger a warning but SHALL still be emitted as single chunks (structural integrity over size limits).

---

### R5: Size Constraints and Overflow Splitting

**User Story:** As the embedding system, I want chunks within a predictable size range so that embedding models receive consistent input lengths.

#### Acceptance Criteria

1. THE default minimum chunk size SHALL be 100 tokens (~400 characters).
2. THE default maximum chunk size SHALL be 800 tokens (~3200 characters).
3. THE minimum and maximum SHALL be configurable via function parameters or a `ChunkConfig` struct.
4. WHEN cumulative buffer content exceeds the maximum size, the system SHALL flush the buffer into a chunk and start a new buffer.
5. WHEN a single paragraph/text block exceeds the maximum size, the system SHALL split it at the nearest sentence boundary (`.`, `!`, `?`, `\n\n`) before the limit.
6. Overflow splitting SHALL NOT split mid-sentence.
7. WHEN overflow splitting occurs, consecutive chunks SHALL overlap by 1–2 sentences (configurable) to prevent information loss at boundaries.

---

### R6: Slide and Sheet Handling

**User Story:** As a user indexing presentations and spreadsheets, I want slides and sheets treated as atomic or semi-atomic chunks.

#### Acceptance Criteria

1. EACH `ReadResult::Slide` SHALL be emitted as a chunk with `block_type: ChunkType::Slide` and `slide: Some(n)`.
2. Slide content exceeding max size SHALL be split by paragraph boundaries within the slide.
3. EACH `ReadResult::Sheet` SHALL be emitted as a chunk with `block_type: ChunkType::Sheet` and `sheet: Some(name)`.
4. Sheet content exceeding max size SHALL be split by row-group boundaries (configurable rows per chunk).

---

### R7: Image Handling

**User Story:** As a system designer, I want image handling to be explicit so that downstream layers know whether images are included or excluded.

#### Acceptance Criteria

1. BY DEFAULT, `ReadResult::Image` blocks SHALL be skipped during chunking (not included in any chunk).
2. THE system SHALL provide an option `include_images: bool` in `ChunkConfig` to enable image inclusion.
3. WHEN images are included, each image SHALL become an atomic chunk with `block_type: ChunkType::Image` containing the image metadata (caption, format) but NOT the raw bytes.
4. Image raw bytes SHALL NOT be stored in chunks — they belong to a separate media store.

---

### R8: Chunk Identity and Stability

**User Story:** As the indexing pipeline, I want chunk IDs to be deterministic so that incremental indexing can detect changes.

#### Acceptance Criteria

1. EACH chunk SHALL receive a UUIDv7 generated at chunk-creation time.
2. WHEN the same document is chunked twice with identical content, the chunk content SHALL be identical (deterministic).
3. Chunk IDs MAY differ between runs (UUIDv7 includes timestamp) — content equality, not ID equality, determines changes.
4. The chunker MUST NOT assume that chunk IDs persist across re-chunking.

---

### R9: Chunk Quality Heuristics

**User Story:** As a downstream system, I want chunks to be meaningful units that can stand alone.

#### Acceptance Criteria

1. EACH chunk SHALL be self-contained — reading only that chunk SHALL convey a complete idea.
2. Chunks SHALL NOT cross heading boundaries (exception: explicit merge configuration).
3. Chunks SHALL NOT mix unrelated structural types (e.g., table content with paragraph text in the same chunk, unless under the same heading and below size limit).
4. Adjacent text blocks under the same heading SHALL be merged into a single chunk (up to the size limit) to reduce fragmentation.

---

### R10: Output Contract

**User Story:** As the orchestrator (`ocean-index`), I want a predictable output from ocean-chunk so that I can feed vector and graph layers reliably.

#### Acceptance Criteria

1. THE primary function `chunk(blocks: Vec<ReadResult>, file_id: &str, config: Option<ChunkConfig>) -> Vec<Chunk>` SHALL return all chunks for a document.
2. THE output SHALL be deterministically ordered (document order).
3. EACH chunk SHALL be independently usable — no chunk depends on another for context.
4. THE chunker SHALL be a pure function — no side effects, no I/O, no mutable state beyond local buffers.

---

### R11: Token Estimation

**User Story:** As the chunker enforcing size limits, I want a reliable token estimation strategy without an external tokenizer.

#### Acceptance Criteria

1. THE system SHALL estimate token count as `content.len() / 4` (approximate for English text).
2. THE estimation function SHALL be `fn estimate_tokens(text: &str) -> usize`.
3. Users MAY provide a custom token estimation function via `ChunkConfig`.
4. The estimation SHALL be used only for size-limit enforcement, not for exact token counting.

---

### R12: Configurability

**User Story:** As a developer integrating ocean-chunk, I want configurable chunking behavior so that I can tune for different use cases.

#### Acceptance Criteria

1. THE system SHALL define a `ChunkConfig` struct with fields:
   - `min_tokens: usize` (default 100)
   - `max_tokens: usize` (default 800)
   - `overlap_sentences: usize` (default 1)
   - `include_images: bool` (default false)
   - `rows_per_sheet_chunk: usize` (default 50)
   - `token_estimator: Option<fn(&str) -> usize>`
2. WHEN `ChunkConfig` is `None`, sensible defaults SHALL be used.
3. All config fields SHALL have documented defaults and bounds.

---

### R13: Error Handling

**User Story:** As a caller, I want typed errors so that I can handle chunking failures gracefully.

#### Acceptance Criteria

1. THE system SHALL define a `ChunkError` enum with variants:
   - `EmptyInput` — no blocks provided
   - `InvalidConfig` — configuration validation failed (e.g., min > max)
   - `ContentTooLarge` — single block exceeds maximum size and cannot be split
2. ALL public functions SHALL return `Result<_, ChunkError>` — never panic.
3. `ChunkError` SHALL implement `Display`, `Error`, `Send`, and `Sync`.

---

### R14: Streaming Chunking

**User Story:** As the pipeline processing large documents, I want the chunker to support streaming input so that I don't need to hold all blocks in memory.

#### Acceptance Criteria

1. THE system SHALL provide `chunk_stream(blocks: impl Iterator<Item = ReadResult>, file_id: &str, config: Option<ChunkConfig>) -> impl Iterator<Item = Result<Chunk, ChunkError>>`.
2. Streaming chunking SHALL produce chunks incrementally as blocks are consumed.
3. Streaming chunking SHALL respect all structural and size constraints as batch chunking.
4. The streaming API SHALL be usable with both sync and async iterators.
