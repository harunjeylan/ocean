# Implementation Plan: ocean-parser — Document Abstraction Layer

## Overview

Phase 2 implements the Document abstraction layer in four sub-phases: (1) shared types — Document trait, Selector, DocumentError, data models, (2) BackendRegistry + `open()` + text/markdown/html backends (simple, validates the architecture), (3) DOCX backend (`document_tree`/`docx-rs`) + PPTX backend (`zip`+`quick-xml`), (4) XLSX backend (`calamine`) + PDF backend (`pdfium-render`, hardest last). The Read API free functions are wired as convenience wrappers throughout.

Each sub-phase includes tests. At the end, `open("file.pdf")` → `Box<dyn Document>` → `read()` / `search()` / `metadata()` / `outline()` works for every supported format.

---

## Tasks

- [ ] 1. Create shared types and data models
  - Define all core types: `Document` trait, `Selector`, `ReadResult`, `DocumentMetadata`, `DocumentFormat`, `Outline`, `OutlineEntry`, `Match`, `DocumentError`
  - Define module structure: `types.rs`, `error.rs`, `traits.rs`, `registry.rs`
  - Set up `src/ocean_parser/` module directory
  - _Requirements: R1, R2, R4, R5, R6_

  - [ ] 1.1 Define `Document` trait with `metadata()`, `outline()`, `page_count()`, `search()`, `read()`
  - [ ] 1.2 Define `Selector` enum with all 14 variants
  - [ ] 1.3 Define `ReadResult` enum with all variants
  - [ ] 1.4 Define `DocumentMetadata` struct and `DocumentFormat` enum
  - [ ] 1.5 Define `Outline` and `OutlineEntry` structs
  - [ ] 1.6 Define `Match` struct (selector, text, context, score)
  - [ ] 1.7 Define `DocumentError` enum with all variants, implementing `Display` + `Error` + `Send` + `Sync`
  - [ ] 1.8 Create module structure, wire `mod.rs` re-exports
  - [ ] 1.9 Add `pub mod ocean_parser;` to `src/lib.rs`
  - [ ] 1.10 Write unit tests for all type construction, Debug/Display formatting

- [ ] 2. Implement Backend Registry and `open()`
  - Build extensible dispatch table mapping extensions to document factories
  - Implement `open(path)` free function
  - Wire up all backends as they become available
  - _Requirements: R7_

  - [ ] 2.1 Define `DocumentFactory` trait: `fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError>`
  - [ ] 2.2 Implement `BackendRegistry` with `HashMap<String, Box<dyn DocumentFactory>>`
  - [ ] 2.3 Implement `register(format: DocumentFormat, factory)` and dispatch by extension
  - [ ] 2.4 Implement `open(path)` — extract extension, look up factory, delegate
  - [ ] 2.5 Write unit tests for registry (register, lookup, missing extension, duplicate register)

- [ ] 3. Implement Text/Markdown/HTML backends
  - Simple backends that validate the architecture before tackling complex formats
  - _Requirements: R8_

  - [ ] 3.1 Implement `TxtDocument`:
    - `metadata()` — path, format=Text, size
    - `outline()` — empty
    - `page_count()` — None
    - `search()` — substring scan with case-insensitive matching
    - `read(Selector::Paragraph(0))` — entire file as text
    - `read(Selector::Range{start,end})` — substring slice
    - Any other selector → `DocumentError::InvalidSelector`

  - [ ] 3.2 Implement `MarkdownDocument`:
    - `outline()` — parse `#` / `##` / `###` lines into hierarchy
    - `read(Selector::Heading(s))` — find heading, return text until next heading at same/higher level
    - `read(Selector::Paragraph(n))` — nth non-heading line group
    - Other selectors as supported

  - [ ] 3.3 Implement `HtmlDocument`:
    - Simple parser: extract `<h1>`–`<h6>`, `<p>`, `<table>`, `<ul>`/`<ol>`, `<img>`
    - `outline()` — from heading tags
    - `read(Selector::Table(n))` — nth `<table>` as rows
    - `read(Selector::Image(n))` — nth `<img>` with src and alt

  - [ ] 3.4 Handle UTF-8 validation for all text backends → `DocumentError::InvalidEncoding`
  - [ ] 3.5 Register all three backends in the default registry
  - [ ] 3.6 Write unit tests for each backend: valid, empty, non-UTF-8, corrupted

- [ ] 4. Implement Read API free functions
  - Convenience wrappers that delegate to `Document::read()` with constructed selectors
  - _Requirements: R3_

  - [ ] 4.1 Implement `read(document, selector)` — direct delegation
  - [ ] 4.2 Implement `read_page`, `read_pages` — constructs `Selector::Page(n)` / `Selector::Pages(v)`
  - [ ] 4.3 Implement `read_heading` — constructs `Selector::Heading(s)`
  - [ ] 4.4 Implement `read_paragraph` — constructs `Selector::Paragraph(n)`
  - [ ] 4.5 Implement `read_table` — constructs `Selector::Table(n)`
  - [ ] 4.6 Implement `read_sheet` — constructs `Selector::Sheet(s)`
  - [ ] 4.7 Implement `read_slide` — constructs `Selector::Slide(n)`
  - [ ] 4.8 Implement `read_cell` — constructs `Selector::Cell(r)`
  - [ ] 4.9 Implement `read_image` — constructs `Selector::Image(n)`
  - [ ] 4.10 Implement `read_notes` — constructs `Selector::Note(n)` or iterates all notes
  - [ ] 4.11 Implement `read_range` — constructs `Selector::Range{start,end}`
  - [ ] 4.12 Write unit tests — verify each convenience function constructs the correct selector

- [ ] 5. Implement DOCX Backend
  - Use `document_tree` or `docx-rs` for reading/writing Word documents
  - _Requirements: R8_

  - [ ] 5.1 Scaffold `DocxDocument` implementing `Document`
  - [ ] 5.2 Open document via `document_tree` or `docx-rs` — lazy load
  - [ ] 5.3 Cache parsed document structure (paragraphs, tables, images) in memory
  - [ ] 5.4 `metadata()` — from document properties
  - [ ] 5.5 `outline()` — build from heading paragraphs (Heading1/2/3 styles)
  - [ ] 5.6 `page_count()` — None
  - [ ] 5.7 `search()` — linear scan of cached paragraph text
  - [ ] 5.8 `read(Selector::Heading(s))` — find heading paragraph, collect text until next heading
  - [ ] 5.9 `read(Selector::Table(n))` — nth table parsed into rows/cells
  - [ ] 5.10 `read(Selector::Image(n))` — extract image from document
  - [ ] 5.11 Handle corrupted document → `DocumentError::CorruptedFile`
  - [ ] 5.12 Register `DocxDocument` in the default registry
  - [ ] 5.13 Write unit tests: simple docx, docx with headings/tables/images, corrupted docx

- [ ] 6. Implement PPTX Backend
  - Unzip archive, parse `ppt/slides/slideN.xml`, map shapes to content
  - _Requirements: R8_

  - [ ] 6.1 Scaffold `PptxDocument` implementing `Document`
  - [ ] 6.2 Enumerate and parse `ppt/slides/slideN.xml` files in order
  - [ ] 6.3 `metadata()` — from `docProps/core.xml`
  - [ ] 6.4 `outline()` — flat list of slide titles
  - [ ] 6.5 `page_count()` — number of slides (Some)
  - [ ] 6.6 `search()` — scan all slide text
  - [ ] 6.7 `read(Selector::Slide(n))` — extract all shapes from slide n
  - [ ] 6.8 `read(Selector::Image(n))` — nth image from slide media
  - [ ] 6.9 Handle slides with no title gracefully
  - [ ] 6.10 Register `PptxDocument` in the default registry
  - [ ] 6.11 Write unit tests: single slide, multi-slide, with images, corrupted

- [ ] 7. Implement XLSX Backend
  - Use `calamine` for workbook reading
  - _Requirements: R8_

  - [ ] 7.1 Scaffold `XlsxDocument` implementing `Document`
  - [ ] 7.2 Open workbook via `calamine`, enumerate sheet names
  - [ ] 7.3 `metadata()` — workbook properties
  - [ ] 7.4 `outline()` — flat list of sheet names
  - [ ] 7.5 `page_count()` — None
  - [ ] 7.6 `search()` — scan all cells across all sheets
  - [ ] 7.7 `read(Selector::Sheet(s))` — return all rows for sheet `s`
  - [ ] 7.8 `read(Selector::Cell("B12"))` — single cell value
  - [ ] 7.9 `read(Selector::Table(n))` — nth sheet as table
  - [ ] 7.10 Handle formula cells (use computed values from `calamine`)
  - [ ] 7.11 Register `XlsxDocument` in the default registry
  - [ ] 7.12 Write unit tests: single sheet, multi-sheet, formulas, corrupted

- [ ] 8. Implement PDF Backend
  - Hardest backend — use `pdfium-render` (primary) + `lopdf` (fallback)
  - Streaming page-by-page processing
  - _Requirements: R8_

  - [ ] 8.1 Scaffold `PdfDocument` implementing `Document`
  - [ ] 8.2 Implement PDF opening with `pdfium-render` — lazy page iteration
  - [ ] 8.3 `metadata()` — extract from PDF info dict (title, author, created)
  - [ ] 8.4 `outline()` — try PDF bookmarks first; fall back to heading detection via font size/weight heuristics
  - [ ] 8.5 `page_count()` — direct from PDF catalog
  - [ ] 8.6 `search()` — per-page text extraction with context from surrounding text
  - [ ] 8.7 `read(Selector::Page(n))` — extract all text from page n, group into paragraphs
  - [ ] 8.8 `read(Selector::Heading(s))` — find heading by text, return content until next heading
  - [ ] 8.9 `read(Selector::Table(n))` — detect tables via coordinate analysis (aligned columns)
  - [ ] 8.10 Handle scanned PDFs — detect no text, return empty paragraphs (not an error)
  - [ ] 8.11 Handle corrupted PDFs → `DocumentError::CorruptedFile`
  - [ ] 8.12 Handle files >500MB → `DocumentError::ParseFailed`
  - [ ] 8.13 Register `PdfDocument` in the default registry
  - [ ] 8.14 Write unit tests: text PDF, multi-page, with headings/tables, scanned PDF, corrupted PDF

- [ ] 9. Write integration tests
  - End-to-end: `open()` → `metadata()` → `outline()` → `read()` → `search()` for every format
  - Real-world documents (5+ per format, committed to `tests/fixtures/`)
  - Cross-format consistency checks
  - _Validates: Properties 1–4_

  - [ ] 9.1 Integration test: every format — open, metadata, outline, read page/slide/sheet, search, verify non-empty
  - [ ] 9.2 Integration test: unsupported extension → `DocumentError::UnsupportedFormat`
  - [ ] 9.3 Integration test: corrupted file per format → typed error (never panic)
  - [ ] 9.4 Integration test: invalid selector per format → `DocumentError::InvalidSelector`
  - [ ] 9.5 Integration test: missing file → `DocumentError::PermissionDenied` (or IoError wrapped)
  - [ ] 9.6 Property-based test: deterministic output — open same file 10×, assert all methods return identical results
  - [ ] 9.7 Property-based fuzz test: random byte slices → catch panics (100 iterations per format)

- [ ] 10. Performance benchmarks
  - Measure open/read/search time and peak memory per format
  - _Validates: Property 3 (memory bound), Property 4 (no panic under load)_

  - [ ] 10.1 Benchmark `open()` + `read(Selector::Page(1))` for PDF (small, medium, large files)
  - [ ] 10.2 Benchmark `search()` across all formats with varying query length
  - [ ] 10.3 Verify peak heap < 256MB for files up to 500MB
  - [ ] 10.4 Record results in a benchmark log

---

## Notes

### Dependencies
- Task 1 blocks everything
- Tasks 3, 4 are independent and can run in parallel after Task 1
- Tasks 5 (DOCX) and Task 6 (PPTX) are independent and can run in parallel
- Task 7 (XLSX) is independent of 5/6
- Task 8 (PDF) is independent of 5/6/7 (or run in parallel with them)
- Task 9 depends on Tasks 3, 5, 6, 7, 8 (all backends)
- Task 10 is optional

### Crate Dependencies
- `pdfium-render` — PDF read, render, extract text, page objects
- `lopdf` — low-level PDF object editing
- `document_tree` or `docx-rs` — read/write Word documents
- `calamine` — read Excel workbooks
- `rust_xlsxwriter` — create/update Excel files
- `zip` — PPTX archive reading (DOCX/XLSX/PPTX are ZIP containers)
- `quick-xml` — parse Office Open XML (PPTX, HTML)
- `image` — decode/manipulate images (thumbnails, dimensions, crop)
- `tesseract-rs` — OCR scanned documents (future Phase 7)

### Testing Approach
- Fixture files in `tests/fixtures/` per format: valid minimal, complex, corrupted
- Generated files for text/markdown
- Real-world documents (5+ per format) for integration tests
- Fuzz testing via random byte mutation loops
- Memory measurement via `alloc` counter or `peak_alloc` crate

### Implementation Tips
- Start with `TxtDocument` (trivial) to validate `Document` trait + registry architecture before tackling complex formats
- For DOCX/PPTX/XLSX, cache the parsed document structure in memory (paragraphs, tables, sheets) so `read()` is O(1) lookup not O(n) rescan
- PDF is highest risk — implement last when the trait interfaces are stable
- Make `Document` + `Send + Sync` so documents can be shared across rayon threads
- Use `OnceCell` or `once_cell` for lazy parsing in backends (open is cheap, parse on first read)
- Selector → Backend mapping table (from design.md) is useful as a runtime validation check

### Future Phases Reference
Phase 2 covers only the `Document` trait + Read API (R1–R10). The following phases will build on this foundation:
- **Phase 3** — Search API (`search_text`, `search_regex`, `search_heading`, `search_table`, etc.)
- **Phase 4** — Navigation (`outline`, `children`, `parent`, `next`, `previous`, `breadcrumbs`, `goto`, `references`)
- **Phase 5** — Tables (`tables()`, `rows()`, `columns()`, `cell()`, `header()`, `footer()`, `merge()`, `split()`)
- **Phase 6** — Spreadsheet (`sheet()`, `range()`, `formula()`, `value()`, `style()`, `filter()`, `sort()`)
- **Phase 7** — Images (`images()`, `thumbnail()`, `dimensions()`, `crop()`, `ocr()`, `alt_text()`)
- **Phase 8** — Editing (`replace()`, `insert()`, `delete()`, `move()`, `rename()`, `merge()`, `split()`, `duplicate()`)
- **Phase 9** — Structural Editing (`insert_heading`, `insert_paragraph`, `insert_table`, etc.)
- **Phase 10** — Formatting (`style()`, `font()`, `color()`, `alignment()`, `spacing()`)
- **Phase 11** — Comparison (`compare()`, `diff()`, `changes()`, `similarity()`, `duplicates()`)
- **Phase 12** — Validation (`validate()`, `broken_links()`, `missing_images()`, `invalid_formula()`)
- **Phase 13** — Export (`save()`, `save_as()`, `export_pdf()`, `export_docx()`, `export_markdown()`, `export_html()`)
- **Phase 14** — Rendering (`render_page()`, `render_slide()`, `render_sheet()`, `thumbnail()`, `preview()`)
- **Phase 15** — Batch Operations (`batch_read()`, `batch_search()`, `batch_replace()`, `batch_validate()`, `batch_export()`)

### What NOT to build yet
- AI integrations, embeddings/vector search, MCP server, HTTP API, GUI
- Database storage, document indexing service, workflow engine
- Collaborative editing, version control, agent-specific abstractions

### Risk Mitigation
- [ ] PDF text extraction quality verified against real-world PDFs (contracts, reports, scanned docs)
- [ ] Memory bounds verified for largest expected file (500MB PDF)
- [ ] Each backend independently testable via `Document` trait interface
- [ ] Registry pattern allows disabling problematic backends at runtime
- [ ] Selector validation prevents format-specific errors from reaching callers
