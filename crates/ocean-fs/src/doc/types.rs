use std::path::PathBuf;

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
    Range { page: u32, start: u32, end: u32 },
    Slice { skip: u32, take: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Webp,
    Bmp,
    Svg,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReadResult {
    Text(String),
    Table(Vec<Vec<String>>),
    Image(Vec<u8>, ImageFormat),
    Metadata(Vec<(String, String)>),
    Outline(Vec<OutlineEntry>),
    Page(u32, String),
    Slide(u32, String),
    Sheet(String, Vec<Vec<String>>),
    CellValue(String),
    MatchResult(Vec<Match>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub format: DocumentFormat,
    pub page_count: Option<u32>,
    pub created: Option<u64>,
    pub modified: Option<u64>,
    pub path: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DocumentFormat {
    Text,
    Markdown,
    Html,
    Docx,
    Pptx,
    Xlsx,
    Pdf,
}

impl DocumentFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "txt" => Some(DocumentFormat::Text),
            "md" => Some(DocumentFormat::Markdown),
            "html" | "htm" => Some(DocumentFormat::Html),
            "docx" => Some(DocumentFormat::Docx),
            "pptx" => Some(DocumentFormat::Pptx),
            "xlsx" => Some(DocumentFormat::Xlsx),
            "pdf" => Some(DocumentFormat::Pdf),
            _ => None,
        }
    }

    pub fn extensions(&self) -> &[&str] {
        match self {
            DocumentFormat::Text => &["txt"],
            DocumentFormat::Markdown => &["md"],
            DocumentFormat::Html => &["html", "htm"],
            DocumentFormat::Docx => &["docx"],
            DocumentFormat::Pptx => &["pptx"],
            DocumentFormat::Xlsx => &["xlsx"],
            DocumentFormat::Pdf => &["pdf"],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    pub entries: Vec<OutlineEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutlineEntry {
    pub level: u8,
    pub text: String,
    pub selector: Option<Selector>,
    pub children: Vec<OutlineEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub selector: Selector,
    pub context: String,
    pub text: String,
    pub score: f64,
}

// === Document Model (shared primitives) ===

/// Top-level document element that can appear on a page or slide.
#[derive(Clone, Debug, PartialEq)]
pub enum DocumentElement {
    Paragraph(ParagraphElement),
    Table(TableElement),
    Image(ImageElement),
    Heading(HeadingElement),
    List(ListElement),
    TextBox(TextBoxElement),
    HorizontalRule,
    PageBreak,
    SectionBreak { columns: u32 },
}

/// A paragraph with run-level formatting.
#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphElement {
    pub runs: Vec<Run>,
    pub alignment: Option<TextAlignment>,
    pub indent_left: Option<f64>,
    pub indent_right: Option<f64>,
    pub indent_first_line: Option<f64>,
    pub spacing_before: Option<f64>,
    pub spacing_after: Option<f64>,
    pub line_spacing: Option<f64>,
    pub border_top: bool,
    pub border_bottom: bool,
    pub background_color: Option<String>,
}

/// A single formatted text run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub font_size: Option<f64>,
    pub font_name: Option<String>,
    pub color: Option<String>,
    pub hyperlink: Option<String>,
    pub code: bool,
    pub strike: bool,
    pub superscript: bool,
    pub subscript: bool,
    pub hyperlink_url: Option<String>,
    pub font_family: Option<String>,
    pub font_color: Option<String>,
    pub highlight_color: Option<String>,
}

/// Text alignment.
#[derive(Clone, Debug, PartialEq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
    Justified,
}

/// A table with cell-level metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct TableElement {
    pub rows: Vec<Vec<TableCell>>,
    pub caption: Option<String>,
    pub headers: Vec<TableCell>,
    pub column_widths: Vec<f64>,
    pub borders: bool,
    pub width: Option<f64>,
    pub alignment: Option<TextAlignment>,
}

/// A single cell in a table.
#[derive(Clone, Debug, PartialEq)]
pub struct TableCell {
    pub text: String,
    pub colspan: Option<u32>,
    pub rowspan: Option<u32>,
    pub bold: bool,
    pub italic: bool,
    pub horizontal_alignment: Option<TextAlignment>,
    pub background_color: Option<String>,
    pub width: Option<f64>,
}

/// An image with position and size.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageElement {
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub caption: Option<String>,
    pub position: ImagePosition,
    pub path: String,
    pub bytes: Vec<u8>,
    pub alt_text: Option<String>,
}

/// Where an image sits relative to text.
#[derive(Clone, Debug, PartialEq)]
pub enum ImagePosition {
    Inline,
    FloatLeft,
    FloatRight,
    Absolute { x: f64, y: f64 },
    Center,
    BehindText,
    InFrontOfText,
}

/// A heading with level.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadingElement {
    pub level: u8,
    pub text: String,
    pub runs: Vec<Run>,
}

/// A list (bullet or numbered).
#[derive(Clone, Debug, PartialEq)]
pub struct ListElement {
    pub items: Vec<ListItem>,
    pub ordered: bool,
    pub start_number: Option<u32>,
}

/// One list item, possibly nested.
#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    pub text: String,
    pub children: Vec<ListItem>,
    pub checked: Option<bool>,
    pub level: u32,
    pub runs: Vec<Run>,
}

/// A text box / callout shape.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBoxElement {
    pub runs: Vec<Run>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub position: ImagePosition,
}

/// Page-level layout properties.
#[derive(Clone, Debug, PartialEq)]
pub struct PageLayout {
    pub width: f64,
    pub height: f64,
    pub orientation: PageOrientation,
    pub margins: Margins,
    pub number: u32,
    pub elements: Vec<DocumentElement>,
    pub columns: u32,
    pub column_widths: Vec<f64>,
    pub page_width: Option<f64>,
    pub page_height: Option<f64>,
    pub header: Option<String>,
    pub footer: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Margins {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

/// Create a path for a named region/page within a document.
pub fn path_for(doc_path: &str, name: &str) -> PathBuf {
    let mut p = PathBuf::from(doc_path);
    p.set_file_name(format!(
        "{}.{}",
        doc_path.rsplit('/').next().unwrap_or(doc_path),
        name
    ));
    p
}
