use crate::ocean_parser::{DocumentError, OutlineEntry, Selector};

pub const MAX_FILE_SIZE: u64 = 500 * 1024 * 1024;

pub fn validate_file_size(path: &str, len: u64) -> Result<(), DocumentError> {
    if len > MAX_FILE_SIZE {
        return Err(DocumentError::ParseFailed(format!(
            "file too large ({} bytes): {}",
            len, path
        )));
    }
    Ok(())
}

/// Parse ISO 8601 date string from OOXML/Dublin Core to Unix epoch seconds.
/// Handles formats like "2024-01-15T10:30:00Z" and "2024-01-15T10:30:00".
pub fn parse_iso8601_date(s: &str) -> Option<u64> {
    let s = s.trim().trim_end_matches('Z');
    // Try full ISO 8601 with time: 2024-01-15T10:30:00
    if let Ok(ts) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(ts.and_utc().timestamp() as u64);
    }
    // Try with fractional seconds: 2024-01-15T10:30:00.123
    if let Ok(ts) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(ts.and_utc().timestamp() as u64);
    }
    // Try date only: 2024-01-15
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(
            d.and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp() as u64,
        );
    }
    None
}

/// Parse PDF date format "D:YYYYMMDDHHmmSSOHH'mm'" to Unix epoch seconds.
pub fn parse_pdf_date(s: &str) -> Option<u64> {
    let s = s.trim();
    let s = s.strip_prefix("D:").unwrap_or(s);
    // Try full: 20240115103000+0530
    // Basic format: YYYYMMDDHHmmSS
    let datetime_str = s.chars().take(14).collect::<String>();
    if let Ok(ts) = chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y%m%d%H%M%S") {
        return Some(ts.and_utc().timestamp() as u64);
    }
    // Date only: YYYYMMDD
    let date_str = s.chars().take(8).collect::<String>();
    if let Ok(d) = chrono::NaiveDate::parse_from_str(&date_str, "%Y%m%d") {
        return Some(
            d.and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp() as u64,
        );
    }
    None
}

/// Build an outline tree from a flat list of (level, heading_text) pairs.
/// This is the standard algorithm used by PDF, DOCX, HTML, and Markdown backends.
pub fn build_outline_tree(headings: &[(u8, String)]) -> Vec<OutlineEntry> {
    fn build(
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
                selector: Selector::Slice { skip: 0, take: u32::MAX },
                children: vec![],
            };
            entry.children = build(headings, start, *level);
            entries.push(entry);
        }
        entries
    }

    let mut idx = 0;
    build(headings, &mut idx, 0)
}

/// Build an outline tree from a flat list of (level, heading_text, selector) tuples.
/// More flexible version used when the selector differs from the heading text.
pub fn build_outline_tree_from_selectors(
    headings: &[(u8, String, Selector)],
) -> Vec<OutlineEntry> {
    fn build(
        headings: &[(u8, String, Selector)],
        start: &mut usize,
        parent_level: u8,
    ) -> Vec<OutlineEntry> {
        let mut entries = vec![];
        while *start < headings.len() {
            let (level, text, selector) = &headings[*start];
            if *level <= parent_level {
                break;
            }
            *start += 1;
            let mut entry = OutlineEntry {
                label: text.clone(),
                level: *level,
                selector: selector.clone(),
                children: vec![],
            };
            entry.children = build(headings, start, *level);
            entries.push(entry);
        }
        entries
    }

    let mut idx = 0;
    build(headings, &mut idx, 0)
}
