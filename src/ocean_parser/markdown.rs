use std::collections::HashMap;
use std::path::PathBuf;

use crate::ocean_parser::helpers::{build_outline_tree, validate_file_size};
use crate::ocean_parser::{
    Document, DocumentError, DocumentFormat, DocumentMetadata, Match, Outline,
    ReadResult, Selector,
};

/// Parsed heading with level and content
#[derive(Debug, Clone)]
struct Heading {
    level: u8,
    text: String,
    line_num: usize,
}

/// Markdown metadata/frontmatter
#[derive(Debug, Clone)]
pub(crate) struct Metadata {
    parsed: HashMap<String, String>,
}

/// Document statistics
#[derive(Debug, Clone)]
struct MarkdownStats {
    line_count: usize,
}

pub struct MarkdownDocument {
    path: PathBuf,
    content: String,
    size: u64,

    // Parsed structure
    lines: Vec<String>,
    headings: Vec<Heading>,
    metadata: Option<Metadata>,

    // Statistics
    stats: MarkdownStats,
}

impl MarkdownDocument {
    pub fn open(path: &str) -> Result<Self, DocumentError> {
        let p = PathBuf::from(path);
        let meta = std::fs::metadata(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        if !meta.is_file() {
            return Err(DocumentError::ParseFailed(format!(
                "path is not a file: {}",
                path
            )));
        }

        validate_file_size(path, meta.len())?;

        let content = std::fs::read_to_string(&p).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                DocumentError::PermissionDenied(format!("{}: {}", path, e))
            }
            std::io::ErrorKind::InvalidData => {
                DocumentError::InvalidEncoding(format!("{} is not valid UTF-8", path))
            }
            _ => DocumentError::ParseFailed(format!("{}: {}", path, e)),
        })?;

        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        // Parse metadata/frontmatter
        let (metadata, _content_start) = Self::parse_metadata(&lines);

        // Parse all markdown elements
        let headings = Self::parse_headings(&lines);

        // Calculate statistics
        let stats = MarkdownStats {
            line_count: lines.len(),
        };

        Ok(Self {
            path: p,
            content,
            size: meta.len(),
            lines,
            headings,
            metadata,
            stats,
        })
    }

    /// Parse YAML/TOML frontmatter
    pub(crate) fn parse_metadata(lines: &[String]) -> (Option<Metadata>, usize) {
        if lines.is_empty() {
            return (None, 0);
        }

        let first_line = &lines[0];
        let is_yaml = first_line.trim() == "---";
        let is_toml = first_line.trim() == "+++";

        if !is_yaml && !is_toml {
            return (None, 0);
        }
        let delimiter = if is_yaml { "---" } else { "+++" };

        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == delimiter {
                let metadata_content = lines[1..i].join("\n");
                let parsed = Self::parse_metadata_fields(&metadata_content);

                return (
                    Some(Metadata {
                        parsed,
                    }),
                    i + 1,
                );
            }
        }

        (None, 0)
    }

    /// Parse simple metadata key-value pairs
    fn parse_metadata_fields(content: &str) -> HashMap<String, String> {
        let mut fields = HashMap::new();

        for line in content.lines() {
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                if !key.is_empty() {
                    fields.insert(key, value);
                }
            }
        }

        fields
    }

    /// Parse all headings (ATX and Setext style)
    fn parse_headings(lines: &[String]) -> Vec<Heading> {
        let mut headings = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // ATX-style headings
            if let Some(level) = Self::atx_heading_level(trimmed) {
                let text = trimmed
                    .trim_start_matches('#')
                    .trim_start()
                    .trim_end_matches('#')
                    .trim_end()
                    .to_string();

                headings.push(Heading {
                    level,
                    text,
                    line_num: i,
                });
                continue;
            }

            // Setext-style headings (underline with = or -)
            if i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();

                if !trimmed.is_empty() && !next_line.is_empty() {
                    if next_line.chars().all(|c| c == '=') && next_line.len() >= 1 {
                        headings.push(Heading {
                            level: 1,
                            text: trimmed.to_string(),
                            line_num: i,
                        });
                    } else if next_line.chars().all(|c| c == '-') && next_line.len() >= 1 {
                        headings.push(Heading {
                            level: 2,
                            text: trimmed.to_string(),
                            line_num: i,
                        });
                    }
                }
            }
        }

        headings
    }

    /// Detect ATX-style heading level
    pub(crate) fn atx_heading_level(line: &str) -> Option<u8> {
        if !line.starts_with('#') {
            return None;
        }

        let level = line.chars().take_while(|c| *c == '#').count();

        if level > 6 || level == 0 {
            return None;
        }

        // Must have space after hashes
        if line.len() > level && line.chars().nth(level) != Some(' ') {
            return None;
        }

        Some(level as u8)
    }

    fn calculate_relevance_score(&self, query: &str, line: &str, line_num: usize) -> f32 {
        let query_lower = query.to_lowercase();
        let line_lower = line.to_lowercase();

        // Boost for headings
        let is_heading = self.headings.iter().any(|h| h.line_num == line_num);
        let heading_boost: f32 = if is_heading { 0.3 } else { 0.0 };

        // Exact match
        if line_lower == query_lower {
            return (1.0 + heading_boost).min(1.0);
        }

        // Case-insensitive starts with
        if line_lower.starts_with(&query_lower) {
            return (0.9 + heading_boost).min(1.0);
        }

        // Contains
        let match_count = line_lower.matches(&query_lower).count();
        let first_match_pos = line_lower.find(&query_lower).unwrap_or(usize::MAX) as f32;
        let position_score = 1.0 - (first_match_pos / line.len().max(1) as f32 * 0.2);
        let frequency_bonus = (match_count as f32 * 0.05).min(0.15);

        (0.5 + position_score * 0.3 + frequency_bonus + heading_boost).min(0.99)
    }


    /// Get context around a line with markdown awareness
    fn get_line_context(&self, line_num: usize, context_lines: usize) -> String {
        let start = line_num.saturating_sub(context_lines);
        let end = (line_num + context_lines + 1).min(self.lines.len());

        self.lines[start..end].join("\n")
    }
}

impl Document for MarkdownDocument {
    fn metadata(&self) -> DocumentMetadata {
        let mut title = None;
        let mut author = None;

        // Extract title from metadata or first heading
        if let Some(meta) = &self.metadata {
            title = meta.parsed.get("title").cloned();
            author = meta.parsed.get("author").cloned();
        }

        if title.is_none() && !self.headings.is_empty() {
            if self.headings[0].level == 1 {
                title = Some(self.headings[0].text.clone());
            }
        }

        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Markdown,
            title,
            author,
            created: None,
            modified: None,
            page_count: Some(((self.stats.line_count as f32) / 50.0).ceil() as u32),
            size: self.size,
        }
    }

    fn outline(&self) -> Outline {
        let headings: Vec<(u8, String)> = self
            .headings
            .iter()
            .map(|h| (h.level, h.text.clone()))
            .collect();
        Outline { entries: build_outline_tree(&headings) }
    }

    fn page_count(&self) -> Option<u32> {
        Some(((self.stats.line_count as f32) / 50.0).ceil() as u32)
    }

    fn search(&self, query: &str) -> Vec<Match> {
        if query.is_empty() {
            return vec![];
        }

        let q = query.to_lowercase();
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (line_num, line) in self.lines.iter().enumerate() {
            if !line.to_lowercase().contains(&q) {
                continue;
            }

            if seen.contains(&line_num) {
                continue;
            }
            seen.insert(line_num);

            let score = self.calculate_relevance_score(&q, line, line_num);
            let context = self.get_line_context(line_num, 2);

            results.push(Match {
                selector: Selector::Page(line_num as u32),
                text: line.clone(),
                context,
                score: score as f64,
            });
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(500);

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Range { start, end } => {
                if *start > *end {
                    return Err(DocumentError::InvalidSelector(format!(
                        "invalid range: start ({}) > end ({})",
                        start, end
                    )));
                }

                if *start >= self.content.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "start position {} beyond content length {}",
                        start,
                        self.content.len()
                    )));
                }

                let actual_end = (*end).min(self.content.len());
                Ok(ReadResult::Text(
                    self.content[*start..actual_end].to_string(),
                ))
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

                let end = (s + t).min(self.lines.len());
                let mut result = self.lines[s..end].join("\n");

                if end < self.lines.len() {
                    result.push_str(&format!("\n... ({} more lines)", self.lines.len() - end));
                }

                Ok(ReadResult::Text(result))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for markdown documents",
                selector
            ))),
        }
    }
}




