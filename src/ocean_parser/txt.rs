use std::path::PathBuf;

use crate::ocean_parser::{
    Document, DocumentError, DocumentFactory, DocumentFormat, DocumentMetadata, Match, Outline,
    ReadResult, Selector,
};

pub struct TxtDocument {
    path: PathBuf,
    content: String,
    size: u64,
}

impl TxtDocument {
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

        Ok(Self {
            path: p,
            content,
            size: meta.len(),
        })
    }
}

impl Document for TxtDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Text,
            title: None,
            author: None,
            created: None,
            modified: None,
            page_count: None,
            size: self.size,
        }
    }

    fn outline(&self) -> Outline {
        Outline { entries: vec![] }
    }

    fn page_count(&self) -> Option<u32> {
        None
    }

    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        self.content
            .lines()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&q))
            .map(|(i, line)| {
                let total = self.content.lines().count();
                let start = i.saturating_sub(2);
                let end = std::cmp::min(i + 3, total);
                let context_lines: Vec<&str> =
                    self.content.lines().skip(start).take(end - start).collect();
                Match {
                    selector: Selector::Paragraph(i as u32),
                    text: line.to_string(),
                    context: context_lines.join("\n"),
                    score: 1.0,
                }
            })
            .collect()
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Paragraph(n) => {
                let line = self
                    .content
                    .lines()
                    .nth(*n as usize)
                    .ok_or_else(|| {
                        DocumentError::InvalidSelector(format!("paragraph {} not found", n))
                    })?;
                Ok(ReadResult::Text(line.to_string()))
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
                let lines: Vec<&str> = self.content.lines().collect();
                if s >= lines.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond line count {}",
                        skip,
                        lines.len()
                    )));
                }
                let end = std::cmp::min(s + t, lines.len());
                Ok(ReadResult::Text(lines[s..end].join("\n")))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for text documents",
                selector
            ))),
        }
    }
}

pub struct TxtFactory;

impl DocumentFactory for TxtFactory {
    fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError> {
        TxtDocument::open(path).map(|d| Box::new(d) as Box<dyn Document>)
    }
}
