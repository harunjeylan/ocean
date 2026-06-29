# Implementation Plan: ocean-chunk — Semantic Chunk Engine

## Overview

Implement Phase 3 of Ocean in four sub-phases: (1) core types, config, and error model, (2) buffer-and-flush chunking algorithm (batch), (3) sentence-boundary split + overlap + streaming, (4) integration into the module tree, CLI stubs, and tests. Each sub-phase includes tests.

---

## Tasks

- [ ] 1. Create core types, config, and error module
  - Define `Chunk`, `ChunkType`, `ChunkConfig`, `ChunkError` in `types.rs`
  - Implement `Display` + `Error` for `ChunkError`
  - Implement `Default` for `ChunkConfig`
  - Implement `estimate_tokens()` with `len / 4` default
  - Create `mod.rs` re-exporting all public items
  - Wire `ocean_chunk` module into `src/lib.rs`
  - _Requirements: R1, R11, R12, R13_

  - [ ] 1.1 Define `Chunk` struct with all fields
  - [ ] 1.2 Define `ChunkType` enum with all variants
  - [ ] 1.3 Define `ChunkConfig` struct with documented defaults
  - [ ] 1.4 Define `ChunkError` enum with `Display` + `Error` impls
  - [ ] 1.5 Implement `estimate_tokens()` function
  - [ ] 1.6 Create `mod.rs` with `pub use` re-exports
  - [ ] 1.7 Register `pub mod ocean_chunk` in `src/lib.rs`

- [ ] 2. Implement `ChunkBuffer` internal struct
  - Buffer state management (content, heading, page, slide, sheet context)
  - `append()`, `flush()`, `reset()`, `is_empty()`, `estimate_token_count()`
  - Flush produces `Vec<Chunk>` (handles both single and multi-chunk flush for oversize)
  - _Requirements: R3, R5, R9_

  - [ ] 2.1 Implement `ChunkBuffer::new()` with initialized state
  - [ ] 2.2 Implement `ChunkBuffer::append()` — accumulate text + track block_type
  - [ ] 2.3 Implement `ChunkBuffer::flush()` — drain buffer into `Vec<Chunk>`
  - [ ] 2.4 Implement `ChunkBuffer::reset()` — clear state for new heading context
  - [ ] 2.5 Implement token count estimation in buffer
  - [ ] 2.6 Write unit tests for buffer append, flush, empty, reset

- [ ] 3. Implement heading detection utility
  - Detect heading patterns in `ReadResult::Text` content: `# `, `## `, `### `, etc.
  - Also detect block that arrives as heading via outline context
  - Return extracted heading text and level
  - _Requirements: R3_

  - [ ] 3.1 Implement `fn detect_heading(text: &str) -> Option<(u8, String)>`
  - [ ] 3.2 Write unit tests for heading detection (markdown, plaintext false positives)

- [ ] 4. Implement batch `chunk()` function (core algorithm)
  - Iterate blocks in document order
  - Track heading context — flush on heading change
  - Emit tables, slides, sheets as atomic chunks
  - Merge adjacent text under same heading up to size limit
  - Flush remaining buffer at end
  - Post-process: merge small adjacent chunks, drop orphan fragments
  - _Requirements: R2, R3, R4, R5, R6, R7, R9, R10_

  - [ ] 4.1 Implement main `chunk()` function
  - [ ] 4.2 Implement heading-context tracking and boundary flush
  - [ ] 4.3 Implement table atomic emission
  - [ ] 4.4 Implement slide handling (atomic + paragraph split if oversize)
  - [ ] 4.5 Implement sheet handling (row-group split)
  - [ ] 4.6 Implement image handling (skip or emit metadata-only)
  - [ ] 4.7 Implement post-processing (merge < min_tokens, drop orphans)
  - [ ] 4.8 Write unit tests for end-to-end chunking with mock ReadResult input

- [ ] 5. Implement sentence-boundary split with overlap
  - Detect sentence endings: `. `, `! `, `? `, `\n\n`
  - Split at last sentence boundary before `max_tokens`
  - Generate overlap: prepend last N sentences from previous segment
  - Handle edge case: no sentence boundary before limit (hard split)
  - _Requirements: R5_

  - [ ] 5.1 Implement `fn find_sentence_boundary(text: &str, limit: usize) -> Option<usize>`
  - [ ] 5.2 Implement `fn split_with_overlap(text: &str, config: &ChunkConfig) -> Vec<String>`
  - [ ] 5.3 Implement `fn extract_last_sentences(text: &str, n: usize) -> String`
  - [ ] 5.4 Write unit tests for sentence-boundary split (normal, no-boundary, exact-boundary)
  - [ ] 5.5 Write unit tests for overlap generation

- [ ] 6. Implement streaming `chunk_stream()` function
  - Wrap core algorithm as an iterator over blocks
  - Yield chunks as they are flushed (no need to wait for all blocks)
  - Ensure same logical behavior as batch mode
  - _Requirements: R14_

  - [ ] 6.1 Implement `ChunkStream` struct implementing `Iterator`
  - [ ] 6.2 Implement `chunk_stream()` free function returning `ChunkStream`
  - [ ] 6.3 Write tests proving streaming equivalence to batch

- [ ] 7. Write integration tests
  - Real-parser integration: parse a real PDF/DOCX/TXT/MD with `ocean-parser`, chunk the output
  - Verify chunks are self-contained, correctly attributed to headings
  - Verify size constraints are respected
  - Verify determinism across multiple chunk runs
  - _Validates: Properties 1, 2, 3, 4_

  - [ ] 7.1 Integration test: parse TXT file, chunk, verify heading boundaries
  - [ ] 7.2 Integration test: parse DOCX with tables, verify table atomicity
  - [ ] 7.3 Integration test: parse PPTX, verify slide chunks
  - [ ] 7.4 Integration test: parse PDF with multiple pages, verify page context
  - [ ] 7.5 Integration test: determinism — chunk same document 3 times, verify identical output
  - [ ] 7.6 Integration test: streaming vs batch equivalence on mixed document

- [ ] 8. Add CLI stubs for ocean-chunk commands (optional, within `ocean_cli`)
  - `chunk <file>` — chunk a single file and display chunk summary (count, sizes)
  - `chunk <file> --verbose` — display all chunk content with metadata
  - _Requirements: R10_

  - [ ] 8.1 Add `Chunk` variant to `ocean_cli::Commands` enum
  - [ ] 8.2 Implement `cmd_chunk()` handler in `run.rs`
  - [ ] 8.3 Implement `print_chunks()` display function in `display.rs`
  - [ ] 8.4 Write CLI integration test via `cargo run -- chunk <file>`

- [ ] 9. Property-based tests
  - Use `proptest` to generate random block sequences and verify correctness properties
  - _Validates: Properties 1, 3, 4, 5_

  - [ ] 9.1 Property test: no heading crossing with random heading/text sequences
  - [ ] 9.2 Property test: all chunks within size bounds (atomic types exempted)
  - [ ] 9.3 Property test: deterministic output across repeated runs
  - [ ] 9.4 Property test: no mid-sentence split on overflow

---

## Notes

### Dependencies
- Internal: `ocean_parser` for `ReadResult` type — already in the crate
- External: `uuid` (v7 feature) for chunk IDs — already in the crate
- Dev: `proptest` for property-based tests (100 iterations per property)
- Dev: `tempfile` for test fixtures

### File Structure
```
src/ocean_chunk/
├── mod.rs          # pub use re-exports
├── types.rs        # Chunk, ChunkType, ChunkConfig, ChunkError
├── buffer.rs       # ChunkBuffer internal struct
├── split.rs        # sentence-boundary split + overlap logic
├── chunker.rs      # chunk() + chunk_stream() implementations
└── heading.rs      # heading detection utility
```

### Implementation Tips
- Start with `ChunkBuffer` as the core state machine — everything else wraps around it.
- Sentence-boundary detection can be simple regex or manual scan — avoid NLP dependencies.
- UUIDv7 for chunk IDs uses `uuid::Uuid::now_v7()` (same as ocean-fs).
- Heading detection should match markdown-style `# ` prefixes first, then fall back to outline context.
- The buffer should track `char_count` for O(1) size checks instead of recomputing on every append.
- Stream chunking can reuse the same `ChunkBuffer` — just yield on each flush instead of collecting.

### Risk Mitigation
- [ ] Single block with no sentence boundary before max limit: hard-split at limit to avoid OOM
- [ ] Extremely large table beyond reasonable chunk size: emit warning + still emit as single chunk
- [ ] Empty document (0 blocks): return `Err(ChunkError::EmptyInput)` — never empty `Vec<Chunk>`
- [ ] Very deeply nested headings: treat as flat heading sequence — no hierarchy needed in chunker
