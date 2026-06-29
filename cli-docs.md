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

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (file not found, unsupported format, invalid selector, parse failure) |
