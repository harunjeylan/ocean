# Requirements Document: ocean-fs — Filesystem Layer

## Introduction

This project builds Ocean, a **local multi-index document runtime** that transforms unstructured files into a queryable knowledge system. Phase 1 establishes the **system foundation** (core design rules, object model, guarantees) and the **filesystem layer (ocean-fs)** — the ground truth ingestion layer responsible for discovering files, tracking changes, identifying uniqueness, normalizing metadata, and feeding the downstream indexing pipeline.

The filesystem layer answers only: "What files exist and have they changed?" It must handle 10,000–100,000 files, support incremental updates, and perform parallel scanning.

---

## Glossary

- **FileMeta**: The canonical metadata record for every file, containing identity, path, hash, size, and timestamps.
- **FileId**: A stable internal identifier for a file, decoupled from its filesystem path.
- **Ocean**: Document Runtime — the overall system being built.
- **Index Pipeline**: The downstream processing chain (parse → chunk → embed → graph) that consumes file metadata.
- **FileEvent**: A real-time notification of filesystem changes (Created, Modified, Deleted, Renamed, Moved).
- **Path Mapping Table**: A persistent log of file moves tracking old → new paths to preserve graph consistency.
- **FileCategory**: Normalized classification of file type (Document, Spreadsheet, Presentation, Image, Text, Unknown).
- **Streaming Hash**: SHA-256 computed via buffered reader without loading the entire file into memory.

---

## Requirements

### R1: File Identity Model

**User Story:** As the indexing pipeline, I want every file to have a stable identity that survives moves and renames, so that graph relationships remain consistent.

#### Acceptance Criteria

1. THE system SHALL assign a UUIDv7 to every discovered file as its `FileId`.
2. THE system SHALL maintain a path mapping table that tracks file identity independent of current location.
3. THE `FileMeta` struct SHALL include fields: `id: String`, `path: String`, `hash: String`, `size: u64`, `modified: u64`, `extension: String`.
4. THE system SHALL guarantee that a file retains the same `FileId` as long as its content is unchanged, even if moved.

---

### R2: Directory Scanner

**User Story:** As a user, I want to point Ocean at any directory and have it recursively discover all supported files, so that indexing can begin.

#### Acceptance Criteria

1. THE system SHALL recursively traverse all subdirectories starting from a given root path.
2. THE system SHALL ignore hidden/system files (starting with `.`) and common ignore directories (`node_modules/`, `.git/`, `.cache/`).
3. THE system SHALL only emit files with supported extensions: `pdf`, `docx`, `pptx`, `xlsx`, `txt`, `md`, `html`, `png`, `jpg`.
4. THE system SHALL compute and attach `FileMeta` (id, hash, size, modified, extension) for each discovered file.
5. THE system SHALL support scanning 10,000–100,000 files efficiently using parallel iteration.

---

### R3: File Hashing System

**User Story:** As the indexing pipeline, I want a content fingerprint for each file so that I can detect changes without reparsing unchanged files.

#### Acceptance Criteria

1. THE system SHALL use streaming SHA-256 to hash file contents.
2. THE system SHALL NOT load the full file into memory during hashing — only buffered streaming SHALL be used.
3. THE hash SHALL be a hex-encoded string (64 characters).
4. WHen `old_hash != new_hash`, the system SHALL flag the file as changed and requiring reindex.

---

### R4: File Watcher (Real-Time Indexing)

**User Story:** As a user, I want Ocean to detect filesystem changes in real time so that the index stays up to date without manual rescans.

#### Acceptance Criteria

1. THE system SHALL watch a directory recursively for filesystem events.
2. THE system SHALL emit a `FileEvent` enum with variants: `Created`, `Modified`, `Deleted`, `Renamed`, `Moved`.
3. ON `Created`, the system SHALL send the file to the indexing pipeline.
4. ON `Modified`, the system SHALL re-hash the file and only reindex if the hash changed.
5. ON `Deleted`, the system SHALL remove the file from all indexes.
6. ON `Renamed`/`Moved`, the system SHALL update the path mapping table only (no reparsing if hash is same).
7. THE watcher SHALL batch multiple rapid events to avoid reindex storms (100 file changes → 1 batch update).

---

### R5: Path Resolution Layer

**User Story:** As the graph system, I want file moves to be tracked so that graph edges remain valid when files are relocated.

#### Acceptance Criteria

1. THE system SHALL maintain a path mapping table with columns: `file_id`, `old_path`, `new_path`, `timestamp`.
2. ON file move, THE system SHALL insert a new row recording the path change.
3. THE system SHALL resolve queries by `file_id` regardless of current file path.
4. THE system SHALL follow the rule: file identity follows content, not location.

---

### R6: Metadata Normalization

**User Story:** As downstream layers, I want consistent metadata from every file so that I don't need format-specific handling.

#### Acceptance Criteria

1. THE system SHALL produce a `NormalizedFile` struct containing: `id: FileId`, `meta: FileMeta`, `mime_type: String`, `category: FileCategory`.
2. THE `FileCategory` enum SHALL include: `Document`, `Spreadsheet`, `Presentation`, `Image`, `Text`, `Unknown`.
3. THE system SHALL determine `FileCategory` from file extension.
4. Unsupported or unrecognized file types SHALL be categorized as `Unknown` and skipped during indexing, not errored.

---

### R7: Filtering System

**User Story:** As a user, I want Ocean to avoid indexing noise so that the index stays relevant and performant.

#### Acceptance Criteria

1. THE system SHALL ignore files matching the ignore list: `node_modules/`, `.git/`, `.cache/`, system files, temporary files.
2. THE ignore list SHALL be configurable by the user.
3. Only files with supported extensions SHALL be emitted by the scanner.

---

### R8: Output Contract

**User Story:** As the pipeline orchestrator (`ocean-index`), I want a predictable interface from ocean-fs so that I can reliably feed the parser layer.

#### Acceptance Criteria

1. THE scanner SHALL return `Vec<FileMeta>` for full scans.
2. THE watcher SHALL produce a `Stream<FileEvent>` for real-time changes.
3. THE output SHALL contain only files that passed filtering.
4. THE output SHALL guarantee no duplicate `FileId` values.

---

### R9: System Foundation Rules

**User Story:** As a developer, I want the system design rules codified so that all layers remain consistent.

#### Acceptance Criteria

1. THE system SHALL treat the filesystem as the sole source of truth — no index is authoritative.
2. THE system SHALL follow the derivation chain: Files → Blocks → Chunks → Embeddings → Graph, never the reverse.
3. THE system SHALL ensure no format-awareness leaks outside the parser layer.
4. EVERY data unit SHALL have an `id: String`, `source_file: FileId`, and `location: Selector` for traceability.
5. THE system SHALL guarantee determinism (same input → same output).
6. THE system SHALL guarantee rebuildability (filesystem deletion → full rebuild from filesystem).
