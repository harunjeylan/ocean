# Design Document: ocean-parser — Document Abstraction Layer

## Overview

This design introduces the **document abstraction layer (ocean-parser)** — a unified API over all document formats. Every format implements the `Document` trait, and callers interact exclusively through free functions and `Selector` values. The design eliminates format-specific branching in application code and makes document operations feel like a standard library.

### Design Philosophy

The entire API is built on three pillars:

1. **Trait polymorphism** — `Box<dyn Document>` is the only document type. No generics, no type parameters.
2. **Selector addressing** — Every addressable element is identified by a `Selector` enum value. No format-specific coordinate systems leak into the API.
3. **Free functions** — `open()`, `read()`, `search_text()` are top-level functions. No builders, no framework.

### Key Design Decisions

- **Document trait over enum dispatch**: A trait with per-format impls is more extensible than a `DocumentFormat` enum with a giant match. New formats don't require modifying existing code.
- **Selectors over string paths**: An enum variant per element type (`Selector::Page(u32)`, `Selector::Heading(String)`) is type-safe, self-documenting, and composable. No parsing of selector strings.
- **ReadResult as opaque enum**: `ReadResult` wraps varying content (text, table, image bytes, metadata map) in a single type so `read()` always returns the same result type regardless of selector.
- **Separate search from read**: `search()` returns match locations (selector + score + context), not content. Callers `read()` the matches they care about. This keeps each operation focused and cacheable.
- **Functions as the public API**: Free functions (`open`, `read`, `search_text`) are easier to discover, import, and test than methods on a builder chain.

---

## Architecture

```mermaid
graph TD
    User[Application Code] --> Open[open(path)]
    Open --> Registry[Backend Registry]
    Registry --> PDF[PdfDocument]
    Registry --> DOCX[DocxDocument]
    Registry --> XLSX[XlsxDocument]
    Registry --> PPTX[PptxDocument]
    Registry --> TXT[TxtDocument]
    Registry --> MD[MarkdownDocument]
    Registry --> HTML[HtmlDocument]
    Registry --> Error[DocumentError]

    User --> Read[read / read_page / read_heading / ...]
    Read --> Document[&dyn Document]
    Document --> Selector[&Selector]
    Document --> ReadResult[Result&lt;ReadResult, DocumentError&gt;]

    User --> Search[search_text / search_regex]
    Search --> Document
    Search --> Vec[Vec&lt;Match&gt;]

    User --> Metadata[doc.metadata]
    Document --> MetadataResult[DocumentMetadata]

    User --> Outline[doc.outline]
    Document --> OutlineResult[Outline]
```

**Data flow**: All operations start from a `Box<dyn Document>` obtained via `open()`. Every operation on the document goes through the `Document` trait — `read()`, `search()`, `metadata()`, `outline()`. Each backend implements these methods using format-specific libraries internally, but callers see only uniform interfaces.

---

## Components and Interfaces

### 1. Document Trait

The core abstraction. Every format backend implements this.

```rust
pub trait Document {
    /// Document-level metadata
    fn metadata(&self) -> DocumentMetadata;

    /// Hierarchical table of contents
    fn outline(&self) -> Outline;

    /// Page count (None for formats without pages)
    fn page_count(&self) -> Option<u32>;

    /// Search for text across the document
    fn search(&self, query: &str) -> Vec<Match>;

    /// Read content at the given selector
    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError>;
}

/// Free function to open any supported document
pub fn open(path: &str) -> Result<Box<dyn Document>, DocumentError>;
```

### 2. Selector Enum

Universal addressing — every element in any document is addressed by one of these variants.

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Selector {
    Page(u32),
    Pages(Vec<u32>),
    Heading(String),
    Paragraph(u32),
    Table(u32),
    Row(u32),
    Column(u32),
    Cell(String),
    Sheet(String),
    Slide(u32),
    Image(u32),
    Note(u32),
    Range { start: usize, end: usize },
}
```

### 3. Read API (Free Functions)

Convenience wrappers around `Document::read()`.

```rust
pub fn read(document: &dyn Document, selector: &Selector) -> Result<ReadResult, DocumentError>;

pub fn read_page(document: &dyn Document, page: u32) -> Result<ReadResult, DocumentError>;
pub fn read_pages(document: &dyn Document, pages: Vec<u32>) -> Result<ReadResult, DocumentError>;
pub fn read_heading(document: &dyn Document, heading: &str) -> Result<ReadResult, DocumentError>;
pub fn read_paragraph(document: &dyn Document, paragraph: u32) -> Result<ReadResult, DocumentError>;
pub fn read_table(document: &dyn Document, table: u32) -> Result<ReadResult, DocumentError>;
pub fn read_sheet(document: &dyn Document, sheet: &str) -> Result<ReadResult, DocumentError>;
pub fn read_slide(document: &dyn Document, slide: u32) -> Result<ReadResult, DocumentError>;
pub fn read_cell(document: &dyn Document, cell: &str) -> Result<ReadResult, DocumentError>;
pub fn read_image(document: &dyn Document, image: u32) -> Result<ReadResult, DocumentError>;
pub fn read_notes(document: &dyn Document) -> Result<ReadResult, DocumentError>;
pub fn read_range(document: &dyn Document, start: usize, end: usize) -> Result<ReadResult, DocumentError>;
```

### 4. ReadResult

The polymorphic return type of `read()`.

```rust
pub enum ReadResult {
    Text(String),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Image {
        bytes: Vec<u8>,
        format: ImageFormat,
        caption: Option<String>,
    },
    Metadata(DocumentMetadata),
    Outline(Outline),
    Page {
        number: u32,
        text: String,
    },
    Slide {
        number: u32,
        title: Option<String>,
        content: String,
    },
    Sheet {
        name: String,
        rows: Vec<Vec<String>>,
    },
    CellValue(String),
    MatchResult(Vec<Match>),
}
```

### 5. DocumentMetadata

```rust
pub struct DocumentMetadata {
    pub path: PathBuf,
    pub format: DocumentFormat,
    pub title: Option<String>,
    pub author: Option<String>,
    pub created: Option<u64>,
    pub modified: Option<u64>,
    pub page_count: Option<u32>,
    pub size: u64,
}

pub enum DocumentFormat {
    Pdf, Docx, Xlsx, Pptx, Text, Markdown, Html,
}
```

### 6. Outline

```rust
#[derive(Clone, Debug)]
pub struct Outline {
    pub entries: Vec<OutlineEntry>,
}

#[derive(Clone, Debug)]
pub struct OutlineEntry {
    pub label: String,
    pub level: u8,
    pub selector: Selector,
    pub children: Vec<OutlineEntry>,
}
```

### 7. Match (search result)

```rust
pub struct Match {
    pub selector: Selector,
    pub text: String,
    pub context: String,
    pub score: f64,
}
```

### 8. DocumentError

```rust
pub enum DocumentError {
    UnsupportedFormat(String),
    InvalidSelector(String),
    CorruptedFile(String),
    PermissionDenied(String),
    ReadOnly(String),
    InvalidEncoding(String),
    OCRFailed(String),
    ParseFailed(String),
    SaveFailed(String),
}
```

### 9. Backend Registry

```rust
pub struct BackendRegistry {
    backends: HashMap<String, Box<dyn DocumentFactory>>,
}

trait DocumentFactory: Send + Sync {
    fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError>;
}
```

---

## Backend Implementations

### PdfDocument

**Backend**: `pdfium-render` (primary) + `lopdf` (fallback)

- `metadata()`: Extract from PDF info dict (title, author, created date)
- `outline()`: Build from PDF bookmarks/outlines tree; fall back to heading detection via font size heuristics
- `page_count()`: Direct from PDF catalog
- `search()`: Linear text scan per page; context includes page number
- `read(Selector::Page(n))`: Extract all text from page n, group into paragraphs
- `read(Selector::Heading(s))`: Find first heading matching `s`, return content until next heading at same or higher level

### DocxDocument

**Backend**: `document_tree` or `docx-rs`

- `metadata()`: From `docProps/core.xml` and `docProps/app.xml`
- `outline()`: Build from heading paragraphs (`<w:pStyle w:val="Heading1"/>`) in document order
- `page_count()`: None (DOCX is flow layout)
- `search()`: Linear scan of all paragraph text; context includes preceding heading
- `read(Selector::Table(n))`: Extract nth `<w:tbl>` as table with rows/cells
- `read(Selector::Paragraph(n))`: Extract nth paragraph text

### XlsxDocument

**Backend**: `calamine` (read) + `rust_xlsxwriter` (write/create)

- `metadata()`: Workbook properties where available
- `outline()`: Flat list of sheet names
- `page_count()`: None
- `search()`: Scan all cells in all sheets; returns sheet name + cell reference in context
- `read(Selector::Sheet(s))`: Return all rows for sheet `s`
- `read(Selector::Cell("B12"))`: Return single cell value

### PptxDocument

**Backend**: `zip` + `quick-xml`

- `metadata()`: From `docProps/core.xml`
- `outline()`: Flat list of slide titles
- `page_count()`: Number of slides
- `search()`: Scan all slide text; context includes slide title
- `read(Selector::Slide(n))`: Extract all text/shapes from slide n
- `read(Selector::Image(n))`: Extract nth image from the slide deck

### TxtDocument / MarkdownDocument / HtmlDocument

**Backend**: `std::fs::read_to_string` + simple markup detection

- `metadata()`: Only path, format, size available
- `outline()`: Markdown — build from `#` lines; TXT — empty; HTML — from `<h1>`–`<h6>`
- `page_count()`: None
- `search()`: Regex or substring scan
- `read(Selector::Heading(s))`: Markdown/HTML — heading content until next heading; TXT — `InvalidSelector`

---

## Data Models

### DocumentFormat

| Variant | Extensions | Backend Crate |
|---------|------------|---------------|
| `Pdf` | `pdf` | `pdfium-render` + `lopdf` |
| `Docx` | `docx` | `document_tree` or `docx-rs` |
| `Xlsx` | `xlsx` | `calamine` + `rust_xlsxwriter` |
| `Pptx` | `pptx` | `zip` + `quick-xml` |
| `Text` | `txt` | `std::fs` |
| `Markdown` | `md` | `std::fs` + line parsing |
| `Html` | `html`, `htm` | `quick-xml` or `ego-tree` |

### Selector → Backend Mapping

| Selector | Pdf | Docx | Xlsx | Pptx | Txt | Md | Html |
|----------|:---:|:----:|:----:|:----:|:---:|:--:|:----:|
| `Page(n)` | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| `Pages(v)` | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| `Heading(s)` | ✓ | ✓ | ✗ | ✓ | ✗ | ✓ | ✓ |
| `Paragraph(n)` | ✓ | ✓ | ✗ | ✓ | ✓ | ✓ | ✓ |
| `Table(n)` | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✓ |
| `Row(n)` | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✓ |
| `Column(n)` | ✗ | ✓ | ✓ | ✗ | ✗ | ✗ | ✓ |
| `Cell(r)` | ✗ | ✓ | ✓ | ✗ | ✗ | ✗ | ✓ |
| `Sheet(s)` | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ |
| `Slide(n)` | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ |
| `Image(n)` | ✗ | ✓ | ✗ | ✓ | ✗ | ✗ | ✓ |
| `Note(n)` | ✗ | ✓ | ✗ | ✓ | ✗ | ✗ | ✓ |
| `Range(s,e)` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

---

## Correctness Properties

### Property 1: Polymorphic Access

*For any* `Box<dyn Document>` obtained via `open(path)`, calling `read(&doc, selector)` SHALL return the same `ReadResult` type regardless of which backend implements the document.

**Validates:** R1, R3

### Property 2: Selector Validity

*For any* document and selector combination marked ✗ in the Selector→Backend Mapping, `read(&doc, &selector)` SHALL return `Err(DocumentError::InvalidSelector)`.

**Validates:** R2, R3.3

### Property 3: Deterministic Open

*For any* file path, calling `open(path)` twice SHALL return documents with identical `metadata()`, `outline()`, and `read()` output for any valid selector.

**Validates:** R8.9, R10

### Property 4: No Panics

*For any* file path (valid, corrupted, empty, permission-denied, unsupported format), `open(path)` SHALL return `Result` — never panic. Likewise for all `Document` trait methods.

**Validates:** R6

### Property 5: Extensibility

*For any* new format backend implementing `Document` and registered via `BackendRegistry`, `open()` SHALL dispatch to it without modifying existing code.

**Validates:** R7.5

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| Unsupported file extension | `open()` returns `Err(DocumentError::UnsupportedFormat)` |
| Corrupted PDF/DOCX/XLSX/PPTX | Backend returns `Err(DocumentError::CorruptedFile)` |
| Missing file / permission denied | `open()` returns `Err(DocumentError::PermissionDenied)` |
| Non-UTF-8 text file | Backend returns `Err(DocumentError::InvalidEncoding)` |
| Selector out of range (e.g., page 999) | `read()` returns `Err(DocumentError::InvalidSelector)` |
| Selector not valid for format (e.g., Slide on TXT) | `read()` returns `Err(DocumentError::InvalidSelector)` |
| File >500MB | Backend returns `Err(DocumentError::ParseFailed)` |
| PDF with no extractable text | `read(Selector::Page(n))` returns `Ok(ReadResult::Text(""))` — not an error |
| Image extraction failure | Skip image, continue, do not error |

---

## Testing Strategy

### Unit Tests

- Each backend tested with:
  - Valid minimal document (1 page, 1 paragraph, 1 table)
  - Complex document (mixed content, multiple pages/slides/sheets)
  - Corrupted/malformed file
  - Empty file
  - Large file (near boundary)
- Selector validity tests for every (selector × format) combination
- DocumentError display and error trait impls
- ReadResult construction and debug

### Property-Based Tests

- Property 1 (Polymorphic Access): Open same content in different formats, verify read output is structurally comparable
- Property 3 (Deterministic Open): Open same file 10 times, assert metadata + outline + read identical
- Property 4 (No Panics): Fuzz each backend with randomly corrupted byte slices (100 iterations)

### Integration Tests

- `open()` → `metadata()` → `outline()` → `read()` pipeline for every format
- `search_text()` across different document types
- Real-world documents: 5+ real PDF/DOCX/XLSX/PPTX files
- Cross-format: open a DOCX, save it conceptually and verify outline matches PDF outline for equivalent content
