use std::path::PathBuf;

use lopdf::Document as PdfDoc;

use crate::ocean_parser::{
    Document, DocumentError, DocumentFactory, DocumentFormat, DocumentMetadata, Match, Outline,
    OutlineEntry, ReadResult, Selector,
};

pub struct PdfDocument {
    path: PathBuf,
    size: u64,
    doc: PdfDoc,
    page_texts: Vec<String>,
}

impl PdfDocument {
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

        let doc = PdfDoc::load(&p)
            .map_err(|e| DocumentError::CorruptedFile(format!("invalid pdf: {}", e)))?;

        let page_count = doc.get_pages().len();
        let mut page_texts = Vec::with_capacity(page_count);

        for (page_num, _) in &doc.get_pages() {
            let text = doc.extract_text(&[*page_num]).unwrap_or_default();
            page_texts.push(text);
        }

        Ok(Self {
            path: p,
            size: meta.len(),
            doc,
            page_texts,
        })
    }

    fn detect_headings(&self) -> Vec<(u8, String, u32)> {
        let mut headings = vec![];
        for (i, text) in self.page_texts.iter().enumerate() {
            let page_num = (i + 1) as u32;
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.len() > 3
                    && trimmed.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_punctuation())
                {
                    let word_count = trimmed.split_whitespace().count();
                    let level = if word_count <= 3 { 1 } else if word_count <= 6 { 2 } else { 3 };
                    headings.push((level, trimmed.to_string(), page_num));
                }
            }
        }
        headings
    }
}

impl Document for PdfDocument {
    fn metadata(&self) -> DocumentMetadata {
        let (title, author) = self.doc.trailer.get(b"Info").ok()
            .and_then(|obj| obj.as_dict().ok())
            .map(|d| {
                let t = d.get(b"Title").ok()
                    .and_then(|o| o.as_str().ok())
                    .and_then(|b| std::str::from_utf8(b).ok().map(|s| s.to_string()));
                let a = d.get(b"Author").ok()
                    .and_then(|o| o.as_str().ok())
                    .and_then(|b| std::str::from_utf8(b).ok().map(|s| s.to_string()));
                (t, a)
            })
            .unwrap_or((None, None));

        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Pdf,
            title,
            author,
            created: None,
            modified: None,
            page_count: Some(self.page_texts.len() as u32),
            size: self.size,
        }
    }

    fn outline(&self) -> Outline {
        let headings = self.detect_headings();
        let mut entries: Vec<OutlineEntry> = vec![];

        for (level, text, _page_num) in &headings {
            let entry = OutlineEntry {
                label: text.clone(),
                level: *level,
                selector: Selector::Heading(text.clone()),
                children: vec![],
            };
            entries.push(entry);
        }

        Outline { entries }
    }

    fn page_count(&self) -> Option<u32> {
        Some(self.page_texts.len() as u32)
    }

    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        let mut results = vec![];

        for (i, text) in self.page_texts.iter().enumerate() {
            let page_num = (i + 1) as u32;
            if text.to_lowercase().contains(&q) {
                results.push(Match {
                    selector: Selector::Page(page_num),
                    text: text.clone(),
                    context: format!("Page {}", page_num),
                    score: 1.0,
                });
            }
        }

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Page(n) => {
                let idx = *n as usize;
                if idx >= 1 && idx <= self.page_texts.len() {
                    let text = self.page_texts[idx - 1].clone();
                    Ok(ReadResult::Page {
                        number: *n,
                        text,
                    })
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "page {} not found (document has {} pages)",
                        n,
                        self.page_texts.len()
                    )))
                }
            }
            Selector::Pages(pages) => {
                let mut combined = String::new();
                for p in pages {
                    if *p >= 1 && *p as usize <= self.page_texts.len() {
                        if !combined.is_empty() {
                            combined.push_str("\n\n");
                        }
                        combined.push_str(&self.page_texts[*p as usize - 1]);
                    }
                }
                if combined.is_empty() {
                    Err(DocumentError::InvalidSelector("no valid pages found".to_string()))
                } else {
                    Ok(ReadResult::Text(combined))
                }
            }
            Selector::Heading(heading) => {
                let q = heading.to_lowercase();
                let headings = self.detect_headings();
                let mut start_page: Option<u32> = None;
                let mut start_level: u8 = 0;

                for (level, text, page_num) in &headings {
                    if text.to_lowercase() == q {
                        start_page = Some(*page_num);
                        start_level = *level;
                        break;
                    }
                }

                if let Some(sp) = start_page {
                    let start_idx = sp as usize - 1;
                    let mut end_idx = self.page_texts.len();
                    for (level, _, page_num) in &headings {
                        if *page_num > sp && *level <= start_level {
                            end_idx = *page_num as usize - 1;
                            break;
                        }
                    }
                    let content = self.page_texts[start_idx..end_idx].join("\n\n");
                    Ok(ReadResult::Text(content))
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "heading '{}' not found",
                        heading
                    )))
                }
            }
            Selector::Range { start, end } => {
                // Concatenate all page text and slice
                let full_text = self.page_texts.join("\n\n");
                if *start >= full_text.len() || *end > full_text.len() || start >= end {
                    return Err(DocumentError::InvalidSelector(format!(
                        "invalid range {}..{} (content length: {})",
                        start,
                        end,
                        full_text.len()
                    )));
                }
                Ok(ReadResult::Text(full_text[*start..*end].to_string()))
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
                "selector {:?} not supported for pdf documents",
                selector
            ))),
        }
    }
}

pub struct PdfFactory;

impl DocumentFactory for PdfFactory {
    fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError> {
        PdfDocument::open(path).map(|d| Box::new(d) as Box<dyn Document>)
    }
}
