use std::path::PathBuf;

use crate::ocean_parser::{
    Document, DocumentError, DocumentFactory, DocumentFormat, DocumentMetadata, Match, Outline,
    OutlineEntry, ReadResult, Selector,
};

pub struct MarkdownDocument {
    path: PathBuf,
    content: String,
    lines: Vec<(usize, String)>,
    size: u64,
}

impl MarkdownDocument {
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

        let lines: Vec<(usize, String)> = content
            .lines()
            .enumerate()
            .map(|(i, l)| (i, l.to_string()))
            .collect();

        Ok(Self {
            path: p,
            content,
            lines,
            size: meta.len(),
        })
    }

    fn heading_level(line: &str) -> Option<u8> {
        let trimmed = line.trim();
        if trimmed.starts_with("###### ") {
            Some(6)
        } else if trimmed.starts_with("##### ") {
            Some(5)
        } else if trimmed.starts_with("#### ") {
            Some(4)
        } else if trimmed.starts_with("### ") {
            Some(3)
        } else if trimmed.starts_with("## ") {
            Some(2)
        } else if trimmed.starts_with("# ") {
            Some(1)
        } else {
            None
        }
    }

    fn is_heading(line: &str) -> bool {
        Self::heading_level(line).is_some()
    }

    fn find_heading_range(&self, heading_text: &str) -> Option<(usize, usize)> {
        let q = heading_text.to_lowercase();
        let mut start: Option<usize> = None;
        let mut start_level: u8 = 0;

        for (i, line) in &self.lines {
            if let Some(level) = Self::heading_level(line) {
                let text = line.trim().splitn(2, ' ').nth(1).unwrap_or("").trim();
                if text.to_lowercase() == q {
                    start = Some(*i);
                    start_level = level;
                    break;
                }
            }
        }

        let start = start?;

        let mut end = self.lines.len();
        for (i, line) in &self.lines {
            if *i > start {
                if let Some(level) = Self::heading_level(line) {
                    if level <= start_level {
                        end = *i;
                        break;
                    }
                }
            }
        }

        Some((start, end))
    }
}

impl Document for MarkdownDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Markdown,
            title: None,
            author: None,
            created: None,
            modified: None,
            page_count: None,
            size: self.size,
        }
    }

    fn outline(&self) -> Outline {
        let headings: Vec<(u8, String)> = self
            .lines
            .iter()
            .filter_map(|(_, line)| {
                Self::heading_level(line).map(|level| {
                    let text = line.trim().splitn(2, ' ').nth(1).unwrap_or("").to_string();
                    (level, text)
                })
            })
            .collect();

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

        for (i, line) in &self.lines {
            if line.to_lowercase().contains(&q) {
                let start = i.saturating_sub(2);
                let end = std::cmp::min(i + 3, self.lines.len());
                let context_lines: Vec<&str> = self.lines[start..end]
                    .iter()
                    .map(|(_, l)| l.as_str())
                    .collect();
                results.push(Match {
                    selector: Selector::Paragraph(*i as u32),
                    text: line.clone(),
                    context: context_lines.join("\n"),
                    score: 1.0,
                });
            }
        }

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Heading(heading) => {
                if let Some((start, end)) = self.find_heading_range(heading) {
                    let text: Vec<&str> =
                        self.lines[start..end].iter().map(|(_, l)| l.as_str()).collect();
                    Ok(ReadResult::Text(text.join("\n")))
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "heading '{}' not found",
                        heading
                    )))
                }
            }
            Selector::Paragraph(n) => {
                let mut para_idx = 0u32;
                for (_, line) in &self.lines {
                    if !Self::is_heading(line) && !line.trim().is_empty() {
                        if para_idx == *n {
                            return Ok(ReadResult::Text(line.clone()));
                        }
                        para_idx += 1;
                    }
                }
                Err(DocumentError::InvalidSelector(format!(
                    "paragraph {} not found",
                    n
                )))
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
                if s >= self.lines.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond line count {}",
                        skip,
                        self.lines.len()
                    )));
                }
                let end = std::cmp::min(s + t, self.lines.len());
                let texts: Vec<&str> = self.lines[s..end].iter().map(|(_, l)| l.as_str()).collect();
                Ok(ReadResult::Text(texts.join("\n")))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for markdown documents",
                selector
            ))),
        }
    }
}

pub struct MarkdownFactory;

impl DocumentFactory for MarkdownFactory {
    fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError> {
        MarkdownDocument::open(path).map(|d| Box::new(d) as Box<dyn Document>)
    }
}
