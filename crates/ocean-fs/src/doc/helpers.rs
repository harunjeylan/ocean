use crate::doc::{DocumentError, OutlineEntry, Selector};

pub const MAX_FILE_SIZE: u64 = 500 * 1024 * 1024;

pub fn validate_file_size(path: &str, len: u64) -> Result<(), DocumentError> {
    if len > MAX_FILE_SIZE {
        return Err(DocumentError::CorruptedFile(format!(
            "{} exceeds maximum file size of {} bytes",
            path, MAX_FILE_SIZE
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
        return Some(d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() as u64);
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
        return Some(d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() as u64);
    }
    None
}

/// Build an outline tree from a flat list of (level, heading_text) pairs.
pub fn build_outline_tree(headings: &[(u8, String)]) -> Vec<OutlineEntry> {
    let mut stack: Vec<(usize, OutlineEntry)> = Vec::new();
    for (level, text) in headings {
        let entry = OutlineEntry {
            level: *level,
            text: text.clone(),
            selector: None,
            children: Vec::new(),
        };
        while let Some((l, _)) = stack.last() {
            if *l >= *level as usize {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some((_, parent)) = stack.last_mut() {
            parent.children.push(entry.clone());
        }
        stack.push((*level as usize, entry));
    }
    let mut tree = Vec::new();
    for (level, entry) in &stack {
        if *level == 0 || stack.iter().all(|(l, _)| *l != 0) {
            tree.push(entry.clone());
        }
    }
    tree
}

/// Build an outline tree from a flat list of (level, heading_text, selector) tuples.
pub fn build_outline_tree_from_selectors(headings: &[(u8, String, Selector)]) -> Vec<OutlineEntry> {
    let mut stack: Vec<(usize, OutlineEntry)> = Vec::new();
    for (level, text, selector) in headings {
        let entry = OutlineEntry {
            level: *level,
            text: text.clone(),
            selector: Some(selector.clone()),
            children: Vec::new(),
        };
        while let Some((l, _)) = stack.last() {
            if *l >= *level as usize {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some((_, parent)) = stack.last_mut() {
            parent.children.push(entry.clone());
        }
        stack.push((*level as usize, entry));
    }
    let mut tree = Vec::new();
    for (level, entry) in &stack {
        if *level == 0 || stack.iter().all(|(l, _)| *l != 0) {
            tree.push(entry.clone());
        }
    }
    tree
}
