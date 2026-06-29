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
