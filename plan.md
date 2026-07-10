╭─── • Read local://backend-markdown-plan.md ─────────────────────────╮
│   1 ## Context                                                      │
│   2                                                                 │
│   3 User's DOCX `--page 1` returned entire document because parser  │
│ only creates pages on explicit `<w:br w:type="page"/>`. Fix: virtual│
│ page splitting when no explicit breaks exist. User then expanded:   │
│ apply same Markdown-native approach to PDF, improve XLSX selectors  │
│ (Sheet/Sheets/Row/Rows/Column/Columns/Cell/Slice row-level), improve│
│ PPTX selectors (Slide/Slides/Slice slide-level). All three backends │
│ need consistent page/slide/sheet-based selectors with Slice support.│
│   4                                                                 │
│   5 ## Approach                                                     │
│   6                                                                 │
│   7 ### Part A: DOCX virtual page splitting (prerequisite)          │
│   8                                                                 │
│   9 **File:** `src/ocean_parser/docx.rs`, end of                    │
│ `parse_document_xml()`                                              │
│  10                                                                 │
│  11 After the "If no pages but has paragraphs" fallback, add virtual│
│ page splitting. When `pages.len() == 1` and elements > 20, split    │
│ elements into chunks of 20. Each chunk becomes a virtual page.      │
│ Update `para_to_page` to distribute paragraphs evenly across virtual│
│ pages.                                                              │
│  12                                                                 │
│  13 **Done in plan Step 3g above at                                 │
│ `local://docx-markdown-conversion-plan.md`.**                       │
│  14                                                                 │
│  15 ---                                                             │
│  16                                                                 │
│  17 ### Part B: PDF Markdown-native conversion                      │
│  18                                                                 │
│  19 **Goal:** Same approach as DOCX — structured page model         │
│ (PageLayout with elements), Markdown rendering in `read()`,         │
│ simplified selectors (Page, Pages, Slice only).                     │
│  20                                                                 │
│  21 **Files:** `src/ocean_parser/pdf.rs` (rewrite),                 │
│ `src/ocean_parser/read_api.rs` (update PDF arm)                     │
│  22                                                                 │
│  23 **Step B1 — Simplify PdfDocument struct and ParsedPage**        │
│  24                                                                 │
│  25 Replace `PdfDocument` fields:                                   │
│  26 - Keep: `path`, `size`, `pages: Vec<PageLayout>`, `metadata:    │
│ PdfMetadataExtended`, `page_map`                                    │
│  27 - Remove: `headings: Vec<HeadingInfo>` — outline will map to    │
│ Page selectors                                                      │
│  28 - Remove: `ParsedPage` struct — use `PageLayout` from types.rs  │
│ instead                                                             │
│  29                                                                 │
│  30 New struct:                                                     │
│  31 ```rust                                                         │
│  32 pub struct PdfDocument {                                        │
│  33     path: PathBuf,                                              │
│  34     size: u64,                                                  │
│  35     pages: Vec<PageLayout>,                                     │
│  36     metadata: PdfMetadataExtended,                              │
│  37     page_map: HashMap<u32, usize>,                              │
│  38 }                                                               │
│  39 ```                                                             │
│  40                                                                 │
│  41 **Step B2 — Rewrite parse_pages to build PageLayout**           │
│  42                                                                 │
│  43 Current `parse_pages` returns `Vec<ParsedPage>` — text +        │
│ image_count + link_count. Replace to return `Vec<PageLayout>`. For  │
│ each PDF page:                                                      │
│  44                                                                 │
│  45 ```rust                                                         │
│  46 fn parse_pages(doc: &PdfDoc) -> Result<Vec<PageLayout>,         │
│ DocumentError> {                                                    │
│  47     let page_count = doc.get_pages().len() as u32;              │
│  48     let mut pages = Vec::with_capacity(page_count as usize);    │
│  49                                                                 │
│  50     // Get default page size (Letter: 612x792 points = 8.5x11   │
│ inches)                                                             │
│  51     let default_w = 612.0;                                      │
│  52     let default_h = 792.0;                                      │
│  53                                                                 │
│  54     for page_num in 1..=page_count {                            │
│  55         let text = extract_page_text(doc, page_num)?;           │
│  56         // Split text into paragraph-like chunks (by double     │
│ newline)                                                            │
│  57         let elements = if text.trim().is_empty() {              │
│  58             vec![]                                              │
│  59         } else {                                                │
│  60             text.split("\n\n")                                  │
│  61                 .filter(|p| !p.trim().is_empty())               │
│  62                 .map(|p| {                                      │
│  63                     DocumentElement::Paragraph(ParagraphElement │
│ {                                                                   │
│  64                         runs: vec![Run {                        │
│  65                             text: p.trim().to_string(),         │
│  66                             bold: false, italic: false,         │
│ underline: false,                                                   │
│  67                             strike: false, subscript: false,    │
│ superscript: false,                                                 │
│  68                             code: false, hyperlink_url: None,   │
│  69                             font_size: None, font_family: None, │
│  70                             font_color: None, highlight_color:  │
│ None,                                                               │
│  71                         }],                                     │
│  72                         alignment: None, indent_left: None,     │
│ indent_right: None,                                                 │
│  73                         indent_first_line: None, spacing_before:│
│ None,                                                               │
│  74                         spacing_after: None, line_spacing: None,│
│  75                         border_top: false, border_bottom: false,│
│  76                         background_color: None,                 │
│  77                     })                                          │
│  78                 })                                              │
│  79                 .collect()                                      │
│  80         };                                                      │
│  81                                                                 │
│  82         pages.push(PageLayout {                                 │
│  83             number: page_num,                                   │
│  84             elements,                                           │
│  85             columns: 1,                                         │
│  86             column_widths: vec![],                              │
│  87             page_width: Some(default_w / 72.0),  // points →    │
│ inches                                                              │
│  88             page_height: Some(default_h / 72.0),                │
│  89             orientation: PageOrientation::Portrait,             │
│  90             margins: Margins { top: 1.0, bottom: 1.0, left: 1.0,│
│ right: 1.0 },                                                       │
│  91             header: None,                                       │
│  92             footer: None,                                       │
│  93         });                                                     │
│  94     }                                                           │
│  95     Ok(pages)                                                   │
│  96 }                                                               │
│  97 ```                                                             │
│  98                                                                 │
│  99 **Text extraction:** Current code uses lopdf's                  │
│ `doc.extract_text(&[page_num])`. Keep this. Returns raw text with   │
│ newlines. Split on `\n\n` to create paragraph-like blocks.          │
│ 100                                                                 │
│ 101 **Image/heading detection:** NOT included in v1. Images are     │
│ embedded as streams in PDF with no position info via lopdf's API.   │
│ Headings are not reliably detectable from PDF text without layout   │
│ analysis. Both can be added in follow-ups.                          │
│ 102                                                                 │
│ 103 **Step B3 — Update read() to Page/Pages/Slice only**            │
│ 104                                                                 │
│ 105 Remove: Heading, Range, Paragraph selectors. Keep only:         │
│ 106 - `Selector::Page(n)` → render page to Markdown, return         │
│ `ReadResult::Text(md)`                                              │
│ 107 - `Selector::Pages(pages)` → render multiple pages, join with   │
│ `\n\n---\n\n`                                                       │
│ 108 - `Selector::Slice { skip, take }` → page-index-based, same as  │
│ DOCX                                                                │
│ 109 - All others → `DocumentError::InvalidSelector`                 │
│ 110                                                                 │
│ 111 **Markdown rendering:** Reuse DOCX helper functions if possible,│
│ or declare inline:                                                  │
│ 112 - `render_page_to_markdown(page: &PageLayout) -> String` —      │
│ renders a PageLayout to Markdown                                    │
│ 113 - For PDF v1: no formatting, just plain paragraphs separated by │
│ `\n\n`. Each page gets a `<!-- Page N -->` comment and layout       │
│ annotation.                                                         │
│ 114                                                                 │
│ 115 **Step B4 — Update outline()**                                  │
│ 116                                                                 │
│ 117 Change from heading-based to page-based:                        │
│ 118 ```rust                                                         │
│ 119 fn outline(&self) -> Outline {                                  │
│ 120     let entries: Vec<OutlineEntry> = self.pages.iter().map(|p| {│
│ 121         OutlineEntry {                                          │
│ 122             label: format!("Page {}", p.number),                │
│ 123             children: vec![],                                   │
│ 124             selector: Selector::Page(p.number),                 │
│ 125         }                                                       │
│ 126     }).collect();                                               │
│ 127     Outline { entries }                                         │
│ 128 }                                                               │
│ 129 ```                                                             │
│ 130                                                                 │
│ 131 **Step B5 — Update read_all_blocks()**                          │
│ 132                                                                 │
│ 133 Current PDF arm (`read_api.rs` lines ~55-80) iterates           │
│ `Selector::Page(i)`. With the new model, each page renders as       │
│ Markdown. The iteration logic stays the same — it calls             │
│ `doc.read(&Selector::Page(i))` for each page.                       │
│ 134                                                                 │
│ 135 **Step B6 — Remove dead code**                                  │
│ 136                                                                 │
│ 137 - Remove `HeadingInfo` struct                                   │
│ 138 - Remove `detect_headings_advanced()` function                  │
│ 139 - Remove `parse_pages()` — replace with new version returning   │
│ `Vec<PageLayout>`                                                   │
│ 140 - Remove `extract_metadata()` — keep as-is                      │
│ 141 - Remove `parse_document_xml_or_parse()` if exists (not seen —  │
│ check)                                                              │
│ 142 - Remove `build_outline_tree_from_selectors` import — no longer │
│ needed                                                              │
│ 143 - Clean up unused imports                                       │
│ 144                                                                 │
│ 145 **Edge cases:**                                                 │
│ 146 - Empty document (0 pages) → `read(Page(1))` returns error      │
│ 147 - Page with only whitespace → empty elements vec                │
│ 148 - Hyperlinks in text → keep as raw text (no Markdown link syntax│
│ in v1)                                                              │
│ 149                                                                 │
│ 150 ---                                                             │
│ 151                                                                 │
│ 152 ### Part C: XLSX selector improvements                          │
│ 153                                                                 │
│ 154 **Goal:** Support Sheet, Sheets, Row, Rows, Column, Columns,    │
│ Cell selectors. Change Slice to row-level (skip/take rows per       │
│ sheet). Apply Row/Rows/Column/Columns/Cell across all sheets by     │
│ default (using sheet prefix in Cell syntax).                        │
│ 155                                                                 │
│ 156 **Files:** `src/ocean_parser/types.rs` (add enum variants),     │
│ `src/ocean_parser/xlsx.rs` (update read()),                         │
│ `src/ocean_parser/read_api.rs` (check if update needed)             │
│ 157                                                                 │
│ 158 **Step C1 — Add new Selector variants**                         │
│ 159                                                                 │
│ 160 In `types.rs`, add to `pub enum Selector`:                      │
│ 161 ```rust                                                         │
│ 162 Sheets(Vec<String>),                                            │
│ 163 Slides(Vec<u32>),                                               │
│ 164 Rows(Vec<u32>),                                                 │
│ 165 Columns(Vec<u32>),                                              │
│ 166 ```                                                             │
│ 167                                                                 │
│ 168 These are shared across all backends. Backends that don't       │
│ support them fall through to their `_` catch-all error.             │
│ 169                                                                 │
│ 170 **Step C2 — Update XLSX read() with Sheets arm**                │
│ 171                                                                 │
│ 172 ```rust                                                         │
│ 173 Selector::Sheets(names) => {                                    │
│ 174     let mut all_sheets = Vec::new();                            │
│ 175     for name in names {                                         │
│ 176         if let Some(idx) = self.find_sheet(name) {              │
│ 177             let rows = self.sheets[idx].clone();                │
│ 178             all_sheets.push(format!("=== Sheet: {} ===", name));│
│ 179             for row in &rows {                                  │
│ 180                 all_sheets.push(row.join(" | "));               │
│ 181             }                                                   │
│ 182         } else {                                                │
│ 183             return Err(DocumentError::InvalidSelector(...));    │
│ 184         }                                                       │
│ 185     }                                                           │
│ 186     Ok(ReadResult::Text(all_sheets.join("\n")))                 │
│ 187 }                                                               │
│ 188 ```                                                             │
│ 189                                                                 │
│ 190 **Step C3 — Update XLSX read() with Rows arm**                  │
│ 191                                                                 │
│ 192 Rows takes a sheet context from a preceding `Sheet` selector... │
│ but the backend is stateless (no "current sheet"). Options:         │
│ 193 1. `Rows(name, indices)` → requires signature change            │
│ 194 2. `Rows { sheet: String, rows: Vec<u32> }` → complex           │
│ 195 3. Use Cell syntax: `"SheetName!1,2,3"` → ugly                  │
│ 196                                                                 │
│ 197 **Decision:** For v1, `Rows(Vec<u32>)` and `Columns(Vec<u32>)`  │
│ operate on ALL sheets. Each sheet's rows/columns are returned       │
│ concatenated with sheet headers. Row indices are 0-based.           │
│ 198                                                                 │
│ 199 ```rust                                                         │
│ 200 Selector::Rows(rows) => {                                       │
│ 201     let mut parts = Vec::new();                                 │
│ 202     for (idx, name) in self.sheet_names.iter().enumerate() {    │
│ 203         parts.push(format!("--- {} ---", name));                │
│ 204         for &r in rows {                                        │
│ 205             let ri = r as usize;                                │
│ 206             if ri < self.sheets[idx].len() {                    │
│ 207                 parts.push(self.sheets[idx][ri].join(" | "));   │
│ 208             }                                                   │
│ 209         }                                                       │
│ 210     }                                                           │
│ 211     Ok(ReadResult::Text(parts.join("\n")))                      │
│ 212 }                                                               │
│ 213 ```                                                             │
│ 214                                                                 │
│ 215 **Step C4 — Change XLSX Slice to row-level**                    │
│ 216                                                                 │
│ 217 Current Slice: `skip` skips sheets, `take` limits sheets. Change│
│ to row-level within sheets:                                         │
│ 218                                                                 │
│ 219 ```rust                                                         │
│ 220 Selector::Slice { skip, take } => {                             │
│ 221     let s = *skip as usize;                                     │
│ 222     let t = *take as usize;                                     │
│ 223     let mut parts = Vec::new();                                 │
│ 224     for (idx, name) in self.sheet_names.iter().enumerate() {    │
│ 225         parts.push(format!("--- {} ---", name));                │
│ 226         let end = std::cmp::min(s + t, self.sheets[idx].len()); │
│ 227         if s < self.sheets[idx].len() {                         │
│ 228             for row in self.sheets[idx][s..end].iter() {        │
│ 229                 parts.push(row.join(" | "));                    │
│ 230             }                                                   │
│ 231         }                                                       │
│ 232     }                                                           │
│ 233     Ok(ReadResult::Text(parts.join("\n")))                      │
│ 234 }                                                               │
│ 235 ```                                                             │
│ 236                                                                 │
│ 237 **Step C5 — Update outline() to include selectors for           │
│ Rows/Columns**                                                      │
│ 238                                                                 │
│ 239 Current outline returns sheet-level entries. Optionally add     │
│ Row/Column selectors as sub-entries. For v1, keep sheet-level only. │
│ 240                                                                 │
│ 241 **Step C6 — Update CLI args**                                   │
│ 242                                                                 │
│ 243 Add to `ReadArgs`:                                              │
│ 244 - `--sheets <NAMES>` or reuse `--sheet` with multiple values?   │
│ `--sheet "Sheet1" --sheet "Sheet2"`                                 │
│ 245 - `--rows <INDICES>` → `--rows 1 2 3`                           │
│ 246 - `--columns <INDICES>` → `--columns A B C`                     │
│ 247                                                                 │
│ 248 **Decision:** For v1, skip CLI flag additions. Users can use    │
│ `--sheet "name"` for single sheets. Multi-sheet/row/column selectors│
│ can be added when CLI is redesigned for structured output.          │
│ 249                                                                 │
│ 250 ---                                                             │
│ 251                                                                 │
│ 252 ### Part D: PPTX selector improvements                          │
│ 253                                                                 │
│ 254 **Goal:** Add Slides(Vec<u32>) selector. Keep Slide/Slice. Add  │
│ Markdown rendering of slide content (bold/italic from a:t runs?     │
│ currently flat text).                                               │
│ 255                                                                 │
│ 256 **Files:** `src/ocean_parser/pptx.rs` (update read()),          │
│ `types.rs` (Slides already added in Step C1)                        │
│ 257                                                                 │
│ 258 **Step D1 — Add Slides arm to PPTX read()**                     │
│ 259                                                                 │
│ 260 ```rust                                                         │
│ 261 Selector::Slides(slides) => {                                   │
│ 262     let mut parts = Vec::new();                                 │
│ 263     for n in slides {                                           │
│ 264         if let Some(slide) = self.slides.iter().find(|s|        │
│ s.number == *n) {                                                   │
│ 265             parts.push(format!("## Slide {}\n\n{}",             │
│ slide.number, slide.content));                                      │
│ 266         }                                                       │
│ 267     }                                                           │
│ 268     if parts.is_empty() {                                       │
│ 269         Err(DocumentError::InvalidSelector("none of the         │
│ requested slides exist".into()))                                    │
│ 270     } else {                                                    │
│ 271         Ok(ReadResult::Text(parts.join("\n\n---\n\n")))         │
│ 272     }                                                           │
│ 273 }                                                               │
│ 274 ```                                                             │
│ 275                                                                 │
│ 276 **Step D2 — Keep Slice at slide level** (already slide-level, no│
│ change needed).                                                     │
│ 277                                                                 │
│ 278 **Step D3 — Markdown formatting for slide content** (optional v1│
│ improvement):                                                       │
│ 279 Current slide content is flat text from XML `a:t` elements.     │
│ Bold/italic are ignored. For v1, keep flat text. Markdown rendering │
│ can be added later when run-level formatting is captured.           │
│ 280                                                                 │
│ 281 ---                                                             │
│ 282                                                                 │
│ 283 ### Part E: Update Selector enum catch-all arms                 │
│ 284                                                                 │
│ 285 Every backend's `read()` has a `_ =>                            │
│ Err(DocumentError::InvalidSelector(...))` catch-all. New variants   │
│ (Sheets, Slides, Rows, Columns) automatically fall through to these │
│ for backends that don't support them. No changes needed.            │
│ 286                                                                 │
│ 287 **Verify each backend's catch-all:**                            │
│ 288 - PDF: `_ => Err(...supported for pdf...)` — ✓ catches all      │
│ unsupported                                                         │
│ 289 - DOCX: `_ => Err(...supported for docx...)` — ✓ catches all    │
│ unsupported                                                         │
│ 290 - XLSX: `_ => Err(...supported for xlsx...)` — ✓ catches all    │
│ unsupported (needs new arms for Sheets/Rows/Columns before          │
│ catch-all)                                                          │
│ 291 - PPTX: `_ => Err(...supported for pptx...)` — ✓ catches all    │
│ unsupported (needs new Slides arm before catch-all)                 │
│ 292 - TXT: `_ => Err(...)` — verify                                 │
│ 293 - MD: `_ => Err(...)` — verify                                  │
│ 294 - HTML: `_ => Err(...)` — verify                                │
│ 295                                                                 │
│ 296 ---                                                             │
│ 297                                                                 │
│ 298 ## Critical files & anchors                                     │
│ 299                                                                 │
│ 300 | File | Symbol/region | Why |                                  │
│     …                                                               │
│ 307 | `src/ocean_parser/pptx.rs` | `impl Document for PptxDocument`,│
│ `read()` (line 545) | Add Slides arm |                              │
│ 308
