use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub enum Selector {
    Page(u32),
    Pages(Vec<u32>),
    Row(u32),
    Column(u32),
    Cell(String),
    Sheet(String),
    Slide(u32),
    Sheets(Vec<String>),
    Slides(Vec<u32>),
    Rows(Vec<u32>),
    Columns(Vec<u32>),
    Range { start: usize, end: usize },
    Slice { skip: u32, take: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Bmp,
    Svg,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReadResult {
    Text(String),
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    Image { bytes: Vec<u8>, format: ImageFormat, caption: Option<String> },
    Metadata(DocumentMetadata),
    Outline(Outline),
    Page { number: u32, text: String },
    Slide { number: u32, title: Option<String>, content: String },
    Sheet { name: String, rows: Vec<Vec<String>> },
    CellValue(String),
    MatchResult(Vec<Match>),
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub enum DocumentFormat {
    Pdf,
    Docx,
    Xlsx,
    Pptx,
    Text,
    Markdown,
    Html,
}

impl DocumentFormat {
    pub fn extensions(&self) -> Vec<&'static str> {
        match self {
            DocumentFormat::Pdf => vec!["pdf"],
            DocumentFormat::Docx => vec!["docx"],
            DocumentFormat::Xlsx => vec!["xlsx"],
            DocumentFormat::Pptx => vec!["pptx"],
            DocumentFormat::Text => vec!["txt"],
            DocumentFormat::Markdown => vec!["md"],
            DocumentFormat::Html => vec!["html", "htm"],
        }
    }

    pub fn from_extension(ext: &str) -> Option<DocumentFormat> {
        match ext.to_lowercase().as_str() {
            "pdf" => Some(DocumentFormat::Pdf),
            "docx" => Some(DocumentFormat::Docx),
            "xlsx" => Some(DocumentFormat::Xlsx),
            "pptx" => Some(DocumentFormat::Pptx),
            "txt" => Some(DocumentFormat::Text),
            "md" => Some(DocumentFormat::Markdown),
            "html" | "htm" => Some(DocumentFormat::Html),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    pub entries: Vec<OutlineEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutlineEntry {
    pub label: String,
    pub level: u8,
    pub selector: Selector,
    pub children: Vec<OutlineEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub selector: Selector,
    pub text: String,
    pub context: String,
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
    HorizontalRule,
    PageBreak,
    SectionBreak { columns: Option<u32> },
    TextBox(TextBoxElement),
}

/// A paragraph with run-level formatting.
#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphElement {
    pub runs: Vec<Run>,
    pub alignment: Option<TextAlignment>,
    pub indent_left: Option<f32>,
    pub indent_right: Option<f32>,
    pub indent_first_line: Option<f32>,
    pub spacing_before: Option<f32>,
    pub spacing_after: Option<f32>,
    pub line_spacing: Option<f32>,
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
    pub strike: bool,
    pub subscript: bool,
    pub superscript: bool,
    pub code: bool,
    pub hyperlink_url: Option<String>,
    pub font_size: Option<f32>,
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
    pub headers: Vec<TableCell>,
    pub rows: Vec<Vec<TableCell>>,
    pub column_widths: Vec<Option<f32>>,
    pub caption: Option<String>,
    pub borders: bool,
    pub width: Option<f32>,
    pub alignment: Option<TextAlignment>,
}

/// A single cell in a table.
#[derive(Clone, Debug, PartialEq)]
pub struct TableCell {
    pub text: String,
    pub rowspan: u32,
    pub colspan: u32,
    pub horizontal_alignment: Option<TextAlignment>,
    pub background_color: Option<String>,
    pub width: Option<f32>,
}

/// An image with position and size.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageElement {
    pub path: String,
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
    pub caption: Option<String>,
    pub alt_text: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub position: ImagePosition,
}

/// Where an image sits relative to text.
#[derive(Clone, Debug, PartialEq)]
pub enum ImagePosition {
    Inline,
    FloatLeft,
    FloatRight,
    Center,
    BehindText,
    InFrontOfText,
}

/// A heading with level.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadingElement {
    pub level: u8,
    pub runs: Vec<Run>,
}

/// A list (bullet or numbered).
#[derive(Clone, Debug, PartialEq)]
pub struct ListElement {
    pub items: Vec<ListItem>,
    pub ordered: bool,
    pub start_number: Option<i32>,
}

/// One list item, possibly nested.
#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    pub level: i32,
    pub runs: Vec<Run>,
    pub children: Vec<ListItem>,
}

/// A text box / callout shape.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBoxElement {
    pub runs: Vec<Run>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub position: ImagePosition,
    pub background_color: Option<String>,
    pub border: bool,
}

/// Page-level layout properties.
#[derive(Clone, Debug, PartialEq)]
pub struct PageLayout {
    pub number: u32,
    pub elements: Vec<DocumentElement>,
    pub columns: u32,
    pub column_widths: Vec<f32>,
    pub page_width: Option<f32>,
    pub page_height: Option<f32>,
    pub orientation: PageOrientation,
    pub margins: Margins,
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
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}
