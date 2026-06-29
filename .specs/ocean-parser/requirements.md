# Requirements Document: ocean-parser — Document Abstraction Layer

## Introduction

Phase 2 of Ocean introduces the **document abstraction layer (ocean-parser)** — a unified API over all document formats. Every document, regardless of underlying format (PDF, DOCX, XLSX, PPTX, TXT, MD, HTML), implements a single `Document` trait and exposes the same reading, navigation, and inspection capabilities through **Unified Selectors**.

The goal: reading a page, a heading, a table, a cell, or a slide feels like using a standard library — `read(document, selector)` — with no builders, no framework, just functions.

Phase 2 covers the **Document trait** and the **Read API**. Search, navigation, editing, tables, spreadsheet operations, images, formatting, comparison, validation, export, rendering, and batch operations are planned for subsequent phases (3–15).

---

## Glossary

- **Document**: The core trait — `metadata()`, `outline()`, `page_count()`, `search()`, `read()` — implemented once per format.
- **Selector**: A unified enum (`Page(u32)`, `Heading(String)`, `Slide(u32)`, `Cell("B12")`, etc.) that identifies any addressable element in any document.
- **Read API**: Free functions (`read()`, `read_page()`, `read_heading()`, etc.) that accept a `&dyn Document` and a `Selector`.
- **Backend**: A format-specific implementation of `Document` (e.g., `PdfDocument`, `DocxDocument`).
- **DocumentError**: Typed error enum — never panics.
- **Outline**: Hierarchical table of contents (headings, slides, sheets) returned by `Document::outline()`.
- **Unified Coordinate Model**: Page numbers, slide numbers, sheet names, cell references — all normalized to the same selector system regardless of format.

---

## Requirements

### R1: Document Trait

**User Story:** As a consumer of ocean-parser, I want every document to expose the same interface so that I can write format-agnostic code.

#### Acceptance Criteria

1. THE system SHALL define a `Document` trait with methods:
   - `fn metadata(&self) -> DocumentMetadata`
   - `fn outline(&self) -> Outline`
   - `fn page_count(&self) -> Option<u32>`
   - `fn search(&self, query: &str) -> Vec<Match>`
   - `fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError>`
2. THE trait SHALL be object-safe (`&self`, no generics) so documents can be stored as `Box<dyn Document>`.
3. EVERY format backend SHALL implement `Document` — there SHALL be no format-specific public API.
4. THE trait SHALL be in the crate root and re-exported publicly.

---

### R2: Unified Selectors

**User Story:** As a caller, I want to address any document element with a single selector type so that read, search, and navigation share the same addressing model.

#### Acceptance Criteria

1. THE system SHALL define a `Selector` enum with variants:
   - `Page(u32)`, `Pages(Vec<u32>)` — page ranges
   - `Heading(String)` — by heading text (first match)
   - `Paragraph(u32)` — by paragraph index
   - `Table(u32)` — by table index
   - `Row(u32)`, `Column(u32)` — row/column within context
   - `Cell(String)` — e.g., `"B12"`, `"A1:C10"`
   - `Sheet(String)` — by sheet name
   - `Slide(u32)` — by slide number
   - `Image(u32)` — by image index
   - `Note(u32)` — by note/footnote index
   - `Range { start: usize, end: usize }` — arbitrary byte/offset range
2. ALL read operations SHALL accept `&Selector` as the addressing mechanism.
3. Selectors SHALL be composable — `read(doc, &Selector::Cell("B12"))` inside a sheet context.

---

### R3: Read API

**User Story:** As a caller, I want to read any part of a document with simple functions so that I don't need to learn format-specific libraries.

#### Acceptance Criteria

1. THE system SHALL provide free functions for common read operations:
   - `fn read(document: &dyn Document, selector: &Selector) -> Result<ReadResult, DocumentError>`
   - `fn read_page(document: &dyn Document, page: u32) -> Result<ReadResult, DocumentError>`
   - `fn read_heading(document: &dyn Document, heading: &str) -> Result<ReadResult, DocumentError>`
   - `fn read_slide(document: &dyn Document, slide: u32) -> Result<ReadResult, DocumentError>`
   - `fn read_sheet(document: &dyn Document, sheet: &str) -> Result<ReadResult, DocumentError>`
   - `fn read_table(document: &dyn Document, table: u32) -> Result<ReadResult, DocumentError>`
   - `fn read_cell(document: &dyn Document, cell: &str) -> Result<ReadResult, DocumentError>`
   - `fn read_paragraph(document: &dyn Document, paragraph: u32) -> Result<ReadResult, DocumentError>`
   - `fn read_image(document: &dyn Document, image: u32) -> Result<ReadResult, DocumentError>`
   - `fn read_notes(document: &dyn Document) -> Result<ReadResult, DocumentError>`
2. ALL read convenience functions SHALL delegate to `Document::read()` with the appropriate `Selector`.
3. `read()` SHALL return `Err(DocumentError::InvalidSelector)` when the selector is invalid for the document type (e.g., `Selector::Slide` on a TXT file).
4. `ReadResult` SHALL be an enum or struct that carries the extracted content in a structured way.

---

### R4: Document Metadata

**User Story:** As a caller, I want consistent metadata from any document.

#### Acceptance Criteria

1. `Document::metadata()` SHALL return a `DocumentMetadata` struct with:
   - `path: PathBuf` — source file path
   - `format: DocumentFormat` — enum (Pdf, Docx, Xlsx, Pptx, Text, Markdown, Html)
   - `title: Option<String>`
   - `author: Option<String>`
   - `created: Option<u64>` — Unix timestamp
   - `modified: Option<u64>` — Unix timestamp
   - `page_count: Option<u32>`
   - `size: u64` — file size in bytes
2. The metadata extraction SHALL be format-specific (PDF metadata, DOCX properties, etc.) but the return type SHALL be universal.

---

### R5: Outline

**User Story:** As a caller, I want a hierarchical table of contents from any document without parsing the full content.

#### Acceptance Criteria

1. `Document::outline()` SHALL return an `Outline` — a tree of `OutlineEntry` nodes:
   - `label: String` — heading text, slide title, sheet name
   - `level: u8` — nesting depth (1 = top)
   - `selector: Selector` — the selector to read this entry
   - `children: Vec<OutlineEntry>` — nested entries
2. For PDF/DOCX: outline mirrors heading hierarchy.
3. For PPTX: outline is a flat list of slide titles (level 1).
4. For XLSX: outline is a flat list of sheet names (level 1).
5. For TXT/MD: outline follows heading levels (`#`, `##`, etc.).

---

### R6: Error Model

**User Story:** As a caller, I want typed errors so that I can handle failures gracefully without panics.

#### Acceptance Criteria

1. THE system SHALL define a `DocumentError` enum with variants:
   - `UnsupportedFormat` — file extension has no backend
   - `InvalidSelector` — selector doesn't apply to this document
   - `CorruptedFile` — file is invalid for its format
   - `PermissionDenied` — OS permission error
   - `ReadOnly` — mutation attempted on read-only document (future)
   - `InvalidEncoding` — text file is not valid UTF-8
   - `OCRFailed` — OCR extraction failed (future)
   - `ParseFailed` — general parsing failure
   - `SaveFailed` — write failure (future)
2. ALL public functions SHALL return `Result<_, DocumentError>` — never panic.
3. `DocumentError` SHALL implement `Display`, `Error`, `Send`, and `Sync`.
4. Each variant SHOULD carry a description string and the source file path for context.

---

### R7: Format Backend Registry

**User Story:** As a developer, I want to open any supported document by file path and get the correct backend automatically.

#### Acceptance Criteria

1. THE system SHALL provide `fn open(path: &str) -> Result<Box<dyn Document>, DocumentError>`.
2. `open()` SHALL detect format by file extension and dispatch to the correct backend.
3. THE system SHALL support at minimum: PDF, DOCX, XLSX, PPTX, TXT, MD, HTML.
4. Unknown extensions SHALL return `Err(DocumentError::UnsupportedFormat)`.
5. The registry SHALL be extensible — new backends can be registered at runtime.

---

### R8: Format-Specific Backends

**User Story:** As a user, I want to open real-world documents in every common format.

#### Acceptance Criteria

1. THE **PDF backend** SHALL use `pdfium-render` (primary, read+render) with `lopdf` (low-level PDF object editing).
2. THE **DOCX backend** SHALL use `document_tree` or `docx-rs` for reading and writing Word documents.
3. THE **XLSX backend** SHALL use `calamine` for reading and `rust_xlsxwriter` for creating/updating Excel files.
4. THE **PPTX backend** SHALL use `zip` + `quick-xml` to parse the PowerPoint Open XML format.
5. THE **TXT backend** SHALL read the file as plain UTF-8 text.
6. THE **Markdown backend** SHALL detect `# `, `## `, `- ` patterns line-by-line.
7. THE **HTML backend** SHALL parse via `quick-xml` or `ego-tree` (upgradable later).
8. THE **Image backend** (for image extraction/thumbnails/OCR) SHALL use the `image` crate for decode/manipulation and `tesseract-rs` for OCR of scanned documents.
9. ALL backends SHALL bound memory usage — files larger than 500MB SHALL return `DocumentError::CorruptedFile` or `DocumentError::ParseFailed`.
10. ALL backends SHALL be deterministic — same file always produces the same `Document` output.

---

### R9: Search

**User Story:** As a user, I want to search within a document so that I can find relevant content quickly.

#### Acceptance Criteria

1. `Document::search(&self, query: &str) -> Vec<Match>` SHALL return all occurrences as `Match` structs:
   - `selector: Selector` — location of the match
   - `text: String` — matched text
   - `context: String` — surrounding text (50 chars before/after)
   - `score: f64` — relevance score (1.0 for exact match)
2. `search()` SHALL be case-insensitive by default.
3. For structured formats (DOCX, PDF), `search()` SHALL respect heading boundaries — a match inside a heading context includes the heading in the context field.
4. `search()` SHALL NOT load the entire document into memory — streaming where possible.

---

### R10: Public API — Functions over Builders

**User Story:** As a developer, I want the API to feel like a standard library — no builders, no configuration objects, just functions.

#### Acceptance Criteria

1. THE public API SHALL be free functions, not builder chains:
   - `let doc = open(path)?;`
   - `let page = read(&doc, Selector::Page(1))?;`
   - `let results = search_text(&doc, "annual leave")?;`
   - `let outline = doc.outline();`
2. No configuration structs, no builder pattern, no method chaining for core operations.
3. All functions SHALL be `pub` in the crate root.
