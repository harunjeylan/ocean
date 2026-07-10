use std::path::PathBuf;

use crate::doc::helpers::validate_file_size;
use crate::doc::{
    Document, DocumentError, DocumentFormat, DocumentMetadata, Match, ReadResult, Selector,
};

/// Line boundary information for efficient access
#[derive(Debug, Clone)]
pub(crate) struct LineBoundary {
    pub(crate) line_num: usize,
    pub(crate) content: String,
}


/// Text statistics cache
#[derive(Debug, Clone)]
pub(crate) struct TextStats {
    pub(crate) line_count: usize,
}

pub struct TxtDocument {
    path: PathBuf,
    content: String,
    size: u64,

    // Performance caches
    line_boundaries: Vec<LineBoundary>,
    stats: TextStats,
}

impl TxtDocument {
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

        if content.is_empty() {
            return Ok(Self {
                path: p,
                content,
                size: meta.len(),
                line_boundaries: vec![],
                stats: TextStats { line_count: 0 },
            });
        }

        // Build line boundaries
        let mut line_boundaries = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            line_boundaries.push(LineBoundary {
                line_num,
                content: line.to_string(),
            });
        }

        // Calculate text statistics
        let stats = Self::calculate_stats(&content, &line_boundaries);

        Ok(Self {
            path: p,
            content,
            size: meta.len(),
            line_boundaries,
            stats,
        })
    }

    /// Calculate comprehensive text statistics
    pub(crate) fn calculate_stats(_content: &str, boundaries: &[LineBoundary]) -> TextStats {
        let line_count = boundaries.len();
        TextStats { line_count }
    }

    /// Get surrounding context around a line
    fn get_line_context(&self, line_num: usize, context_lines: usize) -> String {
        let start = line_num.saturating_sub(context_lines);
        let end = (line_num + context_lines + 1).min(self.line_boundaries.len());

        self.line_boundaries[start..end]
            .iter()
            .map(|b| b.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Calculate search match relevance score
    fn calculate_relevance_score(&self, query: &str, line: &str, _line_num: usize) -> f32 {
        let query_lower = query.to_lowercase();
        let line_lower = line.to_lowercase();

        // Exact match
        if line_lower == query_lower {
            return 1.0;
        }

        // Case-insensitive starts with
        if line_lower.starts_with(&query_lower) {
            return 0.95;
        }

        // Contains
        let match_count = line_lower.matches(&query_lower).count();
        let first_match_pos = line_lower.find(&query_lower).unwrap_or(usize::MAX) as f32;

        // Position score (earlier matches score higher)
        let position_score = 1.0 - (first_match_pos / line.len().max(1) as f32 * 0.2);

        // Multiple matches boost
        let frequency_bonus = (match_count as f32 * 0.1).min(0.2);

        // Short lines match better (more signal-to-noise)
        let length_bonus = if line.len() < 100 { 0.05 } else { 0.0 };

        (0.5 + position_score * 0.3 + frequency_bonus + length_bonus).min(0.99)
    }
}

impl Document for TxtDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.to_string_lossy().to_string(),
            format: DocumentFormat::Text,
            title: None,
            author: None,
            created: None,
            modified: None,
            page_count: Some(((self.stats.line_count as f32) / 50.0).ceil() as u32),
            size: self.size,
        }
    }

    fn search(&self, query: &str) -> Vec<Match> {
        if query.is_empty() {
            return vec![];
        }

        let q = query.to_lowercase();
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for boundary in &self.line_boundaries {
            if !boundary.content.to_lowercase().contains(&q) {
                continue;
            }

            // Deduplicate
            if seen.contains(&boundary.line_num) {
                continue;
            }
            seen.insert(boundary.line_num);

            let score = self.calculate_relevance_score(&q, &boundary.content, boundary.line_num);
            let context = self.get_line_context(boundary.line_num, 2);

            results.push(Match {
                selector: Selector::Page(boundary.line_num as u32),
                text: boundary.content.clone(),
                context,
                score: score as f64,
            });
        }

        // Sort by relevance score
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results to prevent overwhelming output
        results.truncate(500);

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Range { start, end, .. } => {
                if *start > *end {
                    return Err(DocumentError::InvalidSelector(format!(
                        "invalid range: start ({}) > end ({})",
                        start, end
                    )));
                }

                if *start >= self.content.len() as u32 {
                    return Err(DocumentError::InvalidSelector(format!(
                        "start position {} beyond content length {}",
                        start,
                        self.content.len()
                    )));
                }

                let actual_end = (*end).min(self.content.len() as u32);
                let range_start = *start as usize;
                let range_end = actual_end as usize;
                Ok(ReadResult::Text(
                    self.content[range_start..range_end].to_string(),
                ))
            }
            Selector::Slice { skip, take } => {
                let s = *skip as usize;
                let t = *take as usize;

                if s >= self.stats.line_count {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond line count {}",
                        skip, self.stats.line_count
                    )));
                }

                let end = (s + t).min(self.line_boundaries.len());
                let slice: Vec<String> = self.line_boundaries[s..end]
                    .iter()
                    .map(|b| b.content.clone())
                    .collect();

                let mut result = slice.join("\n");
                if end < self.line_boundaries.len() {
                    result.push_str(&format!(
                        "\n... ({} more lines)",
                        self.line_boundaries.len() - end
                    ));
                }

                Ok(ReadResult::Text(result))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for text documents",
                selector
            ))),
        }
    }
}
