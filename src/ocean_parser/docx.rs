use std::io::Read;
use std::path::PathBuf;

use quick_xml::events::Event;
use quick_xml::Reader;
use zip::ZipArchive;

use crate::ocean_parser::{
    Document, DocumentError, DocumentFactory, DocumentFormat, DocumentMetadata, Match, Outline,
    OutlineEntry, ReadResult, Selector,
};

pub struct DocxDocument {
    path: PathBuf,
    size: u64,
    paragraphs: Vec<(String, Option<String>)>,
    page_texts: Vec<String>,
    tables: Vec<Vec<Vec<String>>>,
    images: Vec<(String, Vec<u8>)>,
}

impl DocxDocument {
    pub fn open(path: &str) -> Result<Self, DocumentError> {
        let p = PathBuf::from(path);
        let meta = std::fs::metadata(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        if meta.len() > 500 * 1024 * 1024 {
            return Err(DocumentError::ParseFailed(format!(
                "file too large ({} bytes): {}",
                meta.len(),
                path
            )));
        }

        let file = std::fs::File::open(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        let mut archive = ZipArchive::new(file)
            .map_err(|e| DocumentError::CorruptedFile(format!("invalid docx: {}", e)))?;

        let mut paragraphs = vec![];
        let mut page_break_at = vec![];
        let mut tables = vec![];

        if let Ok(mut doc_xml) = archive.by_name("word/document.xml") {
            let mut xml_content = String::new();
            if doc_xml.read_to_string(&mut xml_content).is_ok() {
                Self::parse_document_xml(&xml_content, &mut paragraphs, &mut page_break_at, &mut tables);
            }
        }

        let mut page_texts = Vec::new();
        let mut page_start = 0usize;
        for (i, &is_break) in page_break_at.iter().enumerate().skip(1) {
            if is_break {
                let text: Vec<&str> = paragraphs[page_start..i]
                    .iter()
                    .map(|(t, _)| t.as_str())
                    .collect();
                page_texts.push(text.join("\n"));
                page_start = i;
            }
        }
        if page_start < paragraphs.len() {
            let text: Vec<&str> = paragraphs[page_start..]
                .iter()
                .map(|(t, _)| t.as_str())
                .collect();
            page_texts.push(text.join("\n"));
        }
        if page_texts.is_empty() && !paragraphs.is_empty() {
            let text: Vec<&str> = paragraphs.iter().map(|(t, _)| t.as_str()).collect();
            page_texts.push(text.join("\n"));
        }

        let mut images = vec![];
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).ok();
            if let Some(ref mut entry) = entry {
                let name = entry.name().to_string();
                if name.starts_with("word/media/") {
                    let mut buf = vec![];
                    if entry.read_to_end(&mut buf).is_ok() {
                        let file_name = std::path::Path::new(&name)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        images.push((file_name, buf));
                    }
                }
            }
        }

        Ok(Self {
            path: p,
            size: meta.len(),
            paragraphs,
            page_texts,
            tables,
            images,
        })
    }

    fn parse_document_xml(
        xml: &str,
        paragraphs: &mut Vec<(String, Option<String>)>,
        page_break_at: &mut Vec<bool>,
        tables: &mut Vec<Vec<Vec<String>>>,
    ) {
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut in_paragraph = false;
        let mut paragraph_text = String::new();
        let mut paragraph_style: Option<String> = None;
        let mut has_page_break = false;
        let mut in_run = false;
        let mut in_style = false;
        let mut in_table = false;
        let mut in_table_row = false;
        let mut current_row = vec![];
        let mut current_cell = String::new();
        let mut in_table_cell = false;
        let mut table_rows: Vec<Vec<String>> = vec![];

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                    match tag.as_str() {
                        "w:p" => {
                            in_paragraph = true;
                            paragraph_text.clear();
                            paragraph_style = None;
                            has_page_break = false;
                        }
                        "w:r" => {
                            in_run = true;
                        }
                        "w:br" => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                let val = String::from_utf8_lossy(attr.value.as_ref()).to_lowercase();
                                if (key == "w:type" || key == "type") && val == "page" {
                                    has_page_break = true;
                                }
                            }
                        }
                        "w:pstyle" | "w:pPr" => {
                            in_style = true;
                        }
                        "w:tbl" => {
                            in_table = true;
                            table_rows = vec![];
                        }
                        "w:tr" if in_table => {
                            in_table_row = true;
                            current_row = vec![];
                        }
                        "w:tc" if in_table_row => {
                            in_table_cell = true;
                            current_cell.clear();
                        }
                        _ => {}
                    }
                    if in_style && tag.ends_with(":pstyle") {
                        if let Some(attr) = e.attributes().flatten().next() {
                            paragraph_style = Some(String::from_utf8_lossy(attr.value.as_ref()).to_string());
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                    match tag.as_str() {
                        "w:p" => {
                            in_paragraph = false;
                            if !paragraph_text.is_empty() {
                                paragraphs.push((paragraph_text.clone(), paragraph_style.clone()));
                                page_break_at.push(has_page_break);
                            }
                            has_page_break = false;
                            in_style = false;
                        }
                        "w:r" => {
                            in_run = false;
                        }
                        "w:pstyle" | "w:pPr" => {
                            in_style = false;
                        }
                        "w:tbl" => {
                            in_table = false;
                            tables.push(table_rows.clone());
                        }
                        "w:tr" if in_table => {
                            in_table_row = false;
                            table_rows.push(current_row.clone());
                        }
                        "w:tc" if in_table_row => {
                            in_table_cell = false;
                            current_row.push(current_cell.clone());
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if in_table_cell {
                        current_cell.push_str(&text);
                    } else if in_run && in_paragraph {
                        paragraph_text.push_str(&text);
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    fn is_heading_style(style: &Option<String>) -> Option<u8> {
        match style.as_deref() {
            Some(s) if s == "Heading1" || s == "heading1" => Some(1),
            Some(s) if s == "Heading2" || s == "heading2" => Some(2),
            Some(s) if s == "Heading3" || s == "heading3" => Some(3),
            Some(s) if s == "Heading4" || s == "heading4" => Some(4),
            Some(s) if s == "Heading5" || s == "heading5" => Some(5),
            Some(s) if s == "Heading6" || s == "heading6" => Some(6),
            _ => None,
        }
    }
}

impl Document for DocxDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Docx,
            title: None,
            author: None,
            created: None,
            modified: None,
            page_count: None,
            size: self.size,
        }
    }

    fn outline(&self) -> Outline {
        let mut entries: Vec<OutlineEntry> = vec![];
        let mut stack: Vec<usize> = vec![];

        for (text, style) in &self.paragraphs {
            if let Some(level) = Self::is_heading_style(style) {
                let entry = OutlineEntry {
                    label: text.clone(),
                    level,
                    selector: Selector::Heading(text.clone()),
                    children: vec![],
                };

                while let Some(&top) = stack.last() {
                    if entries[top].level < level {
                        break;
                    }
                    stack.pop();
                }

                if let Some(&parent) = stack.last() {
                    entries[parent].children.push(entry);
                } else {
                    entries.push(entry);
                    stack.push(entries.len() - 1);
                }
            }
        }

        Outline { entries }
    }

    fn page_count(&self) -> Option<u32> {
        if self.page_texts.len() > 1 {
            Some(self.page_texts.len() as u32)
        } else {
            None
        }
    }

    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        let mut results = vec![];

        for (i, (text, _)) in self.paragraphs.iter().enumerate() {
            if text.to_lowercase().contains(&q) {
                results.push(Match {
                    selector: Selector::Paragraph(i as u32),
                    text: text.clone(),
                    context: String::new(),
                    score: 1.0,
                });
            }
        }

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Paragraph(n) => {
                let idx = *n as usize;
                if idx < self.paragraphs.len() {
                    Ok(ReadResult::Text(self.paragraphs[idx].0.clone()))
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "paragraph {} not found",
                        n
                    )))
                }
            }
            Selector::Heading(heading) => {
                let q = heading.to_lowercase();
                let mut start: Option<usize> = None;

                for (i, (text, style)) in self.paragraphs.iter().enumerate() {
                    if Self::is_heading_style(style).is_some()
                        && text.to_lowercase() == q
                    {
                        start = Some(i);
                        break;
                    }
                }

                if let Some(s) = start {
                    let heading_level = Self::is_heading_style(&self.paragraphs[s].1).unwrap_or(1);
                    let mut content = String::new();
                    for i in s + 1..self.paragraphs.len() {
                        let (text, style) = &self.paragraphs[i];
                        if let Some(level) = Self::is_heading_style(style) {
                            if level <= heading_level {
                                break;
                            }
                        }
                        if !content.is_empty() {
                            content.push('\n');
                        }
                        content.push_str(text);
                    }
                    Ok(ReadResult::Text(content))
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "heading '{}' not found",
                        heading
                    )))
                }
            }
            Selector::Table(n) => {
                let idx = *n as usize;
                if idx < self.tables.len() {
                    let t = &self.tables[idx];
                    let headers = if !t.is_empty() { t[0].clone() } else { vec![] };
                    let rows = if t.len() > 1 { t[1..].to_vec() } else { vec![] };
                    Ok(ReadResult::Table { headers, rows })
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "table {} not found",
                        n
                    )))
                }
            }
            Selector::Image(n) => {
                let idx = *n as usize;
                if idx < self.images.len() {
                    let (name, bytes) = &self.images[idx];
                    let fmt = if name.ends_with(".png") {
                        crate::ocean_parser::ImageFormat::Png
                    } else if name.ends_with(".jpg") || name.ends_with(".jpeg") {
                        crate::ocean_parser::ImageFormat::Jpeg
                    } else {
                        crate::ocean_parser::ImageFormat::Unknown
                    };
                    Ok(ReadResult::Image {
                        bytes: bytes.clone(),
                        format: fmt,
                        caption: Some(name.clone()),
                    })
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "image {} not found",
                        n
                    )))
                }
            }
            Selector::Slice { skip, take } => {
                let s = *skip as usize;
                let t = *take as usize;
                if s >= self.page_texts.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond page count {}",
                        skip,
                        self.page_texts.len()
                    )));
                }
                let end = std::cmp::min(s + t, self.page_texts.len());
                let pages = &self.page_texts[s..end];
                Ok(ReadResult::Text(pages.join("\n\n---\n\n")))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for docx documents",
                selector
            ))),
        }
    }
}

pub struct DocxFactory;

impl DocumentFactory for DocxFactory {
    fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError> {
        DocxDocument::open(path).map(|d| Box::new(d) as Box<dyn Document>)
    }
}
