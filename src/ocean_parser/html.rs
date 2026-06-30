use std::path::PathBuf;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::ocean_parser::{
    Document, DocumentError, DocumentFactory, DocumentFormat, DocumentMetadata, ImageFormat, Match,
    Outline, OutlineEntry, ReadResult, Selector,
};

pub struct HtmlDocument {
    path: PathBuf,
    content: String,
    size: u64,
    headings: Vec<(u8, String, usize)>,
    paragraphs: Vec<String>,
    tables: Vec<Vec<Vec<String>>>,
    images: Vec<(String, String)>,
}

impl HtmlDocument {
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

        let content =
            std::fs::read_to_string(&p).map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    DocumentError::PermissionDenied(format!("{}: {}", path, e))
                }
                std::io::ErrorKind::InvalidData => {
                    DocumentError::InvalidEncoding(format!("{} is not valid UTF-8", path))
                }
                _ => DocumentError::ParseFailed(format!("{}: {}", path, e)),
            })?;

        let (headings, paragraphs, tables, images) = Self::parse_html(&content);

        Ok(Self {
            path: p,
            content,
            size: meta.len(),
            headings,
            paragraphs,
            tables,
            images,
        })
    }

    fn parse_html(
        html: &str,
    ) -> (
        Vec<(u8, String, usize)>,
        Vec<String>,
        Vec<Vec<Vec<String>>>,
        Vec<(String, String)>,
    ) {
        let mut reader = Reader::from_str(html);
        reader.config_mut().trim_text(true);

        let mut headings = vec![];
        let mut paragraphs = vec![];
        let mut tables = vec![];
        let mut images = vec![];

        let mut buf = Vec::new();
        let mut in_table = false;
        let mut in_tr = false;
        let mut current_row = vec![];
        let mut current_cell = String::new();
        let mut table_rows = vec![];
        let mut in_para = false;
        let mut para_text = String::new();
        let mut in_heading = false;
        let mut heading_level = 0u8;
        let mut heading_text = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();

                    match tag.as_str() {
                        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                            in_heading = true;
                            heading_text.clear();
                            heading_level = tag[1..].parse().unwrap_or(1);
                        }
                        "p" => {
                            in_para = true;
                            para_text.clear();
                        }
                        "table" => {
                            in_table = true;
                            table_rows = vec![];
                        }
                        "tr" if in_table => {
                            in_tr = true;
                            current_row = vec![];
                        }
                        "td" | "th" if in_tr => {
                            current_cell.clear();
                        }
                        "img" => {
                            let mut src = String::new();
                            let mut alt = String::new();
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                let val = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                match key.as_str() {
                                    "src" => src = val,
                                    "alt" => alt = val,
                                    _ => {}
                                }
                            }
                            images.push((src, alt));
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();

                    match tag.as_str() {
                        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                            in_heading = false;
                            if !heading_text.is_empty() {
                                headings.push((heading_level, heading_text.clone(), paragraphs.len()));
                            }
                        }
                        "p" => {
                            in_para = false;
                            if !para_text.is_empty() {
                                paragraphs.push(para_text.clone());
                            }
                        }
                        "td" | "th" if in_tr => {
                            current_row.push(current_cell.clone());
                            current_cell.clear();
                        }
                        "tr" if in_table => {
                            in_tr = false;
                            table_rows.push(current_row.clone());
                        }
                        "table" => {
                            in_table = false;
                            tables.push(table_rows.clone());
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let text = quick_xml::escape::unescape(&e.decode().unwrap_or_default()).unwrap_or_default().to_string();
                    if in_heading {
                        heading_text.push_str(&text);
                    } else if in_para {
                        para_text.push_str(&text);
                    } else if in_tr {
                        current_cell.push_str(&text);
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        (headings, paragraphs, tables, images)
    }

}

impl Document for HtmlDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Html,
            title: None,
            author: None,
            created: None,
            modified: None,
            page_count: None,
            size: self.size,
        }
    }

    fn outline(&self) -> Outline {
        let headings: Vec<(u8, String)> = self.headings.iter().map(|(l, t, _)| (*l, t.clone())).collect();

        fn build_tree(
            headings: &[(u8, String)],
            start: &mut usize,
            parent_level: u8,
        ) -> Vec<OutlineEntry> {
            let mut entries = vec![];
            while *start < headings.len() {
                let (level, text) = &headings[*start];
                if *level <= parent_level {
                    break;
                }
                *start += 1;
                let mut entry = OutlineEntry {
                    label: text.clone(),
                    level: *level,
                    selector: Selector::Heading(text.clone()),
                    children: vec![],
                };
                entry.children = build_tree(headings, start, *level);
                entries.push(entry);
            }
            entries
        }

        let mut idx = 0;
        let entries = build_tree(&headings, &mut idx, 0);
        Outline { entries }
    }

    fn page_count(&self) -> Option<u32> {
        None
    }

    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        let mut results = vec![];

        for (i, para) in self.paragraphs.iter().enumerate() {
            if para.to_lowercase().contains(&q) {
                results.push(Match {
                    selector: Selector::Paragraph(i as u32),
                    text: para.clone(),
                    context: String::new(),
                    score: 1.0,
                });
            }
        }

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Heading(heading) => {
                for (_, text, para_start) in &self.headings {
                    if text.to_lowercase() == heading.to_lowercase() {
                        let mut content = String::new();
                        let mut found_end = false;
                        for (h_level, h_text, _) in &self.headings {
                            if *h_text == *text {
                                if found_end {
                                    break;
                                }
                                found_end = true;
                                continue;
                            }
                            if found_end {
                                if *h_level <= 1 {
                                    // same level or higher, stop
                                    break;
                                }
                            }
                        }
                        // Simpler: just collect from para_start to next heading's para_start
                        let start = *para_start;
                        let mut end = self.paragraphs.len();
                        for (_, _, next_start) in &self.headings {
                            if *next_start > start {
                                end = *next_start;
                                break;
                            }
                        }
                        for p in &self.paragraphs[start..end] {
                            if !content.is_empty() {
                                content.push('\n');
                            }
                            content.push_str(p);
                        }
                        return Ok(ReadResult::Text(content));
                    }
                }
                Err(DocumentError::InvalidSelector(format!(
                    "heading '{}' not found",
                    heading
                )))
            }
            Selector::Paragraph(n) => {
                let idx = *n as usize;
                if idx < self.paragraphs.len() {
                    Ok(ReadResult::Text(self.paragraphs[idx].clone()))
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "paragraph {} not found",
                        n
                    )))
                }
            }
            Selector::Table(n) => {
                let idx = *n as usize;
                if idx < self.tables.len() {
                    let t = &self.tables[idx];
                    let headers = if !t.is_empty() { t[0].clone() } else { vec![] };
                    let rows = if t.len() > 1 {
                        t[1..].to_vec()
                    } else {
                        vec![]
                    };
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
                    let (_src, alt) = &self.images[idx];
                    Ok(ReadResult::Image {
                        bytes: vec![],
                        format: ImageFormat::Unknown,
                        caption: Some(alt.clone()),
                    })
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "image {} not found",
                        n
                    )))
                }
            }
            Selector::Range { start, end } => {
                if *start >= self.content.len()
                    || *end > self.content.len()
                    || start >= end
                {
                    return Err(DocumentError::InvalidSelector(format!(
                        "invalid range {}..{} (content length: {})",
                        start,
                        end,
                        self.content.len()
                    )));
                }
                Ok(ReadResult::Text(self.content[*start..*end].to_string()))
            }
            Selector::Slice { skip, take } => {
                let s = *skip as usize;
                let t = *take as usize;
                if s >= self.paragraphs.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond paragraph count {}",
                        skip,
                        self.paragraphs.len()
                    )));
                }
                let end = std::cmp::min(s + t, self.paragraphs.len());
                Ok(ReadResult::Text(self.paragraphs[s..end].join("\n")))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for html documents",
                selector
            ))),
        }
    }
}

pub struct HtmlFactory;

impl DocumentFactory for HtmlFactory {
    fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError> {
        HtmlDocument::open(path).map(|d| Box::new(d) as Box<dyn Document>)
    }
}
