use rmcp::model::CallToolResult;
use serde::Deserialize;

use crate::ocean_api::{self, ReadRequest};
use crate::ocean_chunk::ChunkConfig;
use crate::ocean_parser::Selector;

use super::{to_text, to_error, file_not_found, dir_not_found};

#[derive(Debug, Deserialize)]
pub struct ReadParams {
    pub file_path: String,
    pub selector_type: Option<String>,
    pub selector_value: Option<String>,
    pub skip: Option<u32>,
    pub take: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub file_path: String,
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct GrepParams {
    pub directory: String,
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct InfoParams {
    pub file_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ScanParams {
    pub directory: String,
    pub include_hash: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkParams {
    pub file_path: String,
    pub min_size: Option<usize>,
    pub max_size: Option<usize>,
    pub overlap: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyParams {
    pub file_path: String,
    pub expected_hash: String,
}

fn build_selector(params: &ReadParams) -> Result<Selector, CallToolResult> {
    match (params.selector_type.as_deref(), params.selector_value.as_deref(), params.skip, params.take) {
        (Some("page"), Some(val), _, _) => {
            let n = val.parse::<u32>().map_err(|_| to_error("Invalid page number"))?;
            Ok(Selector::Page(n))
        }
        (Some("heading"), Some(val), _, _) => Ok(Selector::Heading(val.to_string())),
        (Some("paragraph"), Some(val), _, _) => {
            let n = val.parse::<u32>().map_err(|_| to_error("Invalid paragraph number"))?;
            Ok(Selector::Paragraph(n))
        }
        (Some("table"), Some(val), _, _) => {
            let n = val.parse::<u32>().map_err(|_| to_error("Invalid table number"))?;
            Ok(Selector::Table(n))
        }
        (Some("slide"), Some(val), _, _) => {
            let n = val.parse::<u32>().map_err(|_| to_error("Invalid slide number"))?;
            Ok(Selector::Slide(n))
        }
        (Some("sheet"), Some(val), _, _) => Ok(Selector::Sheet(val.to_string())),
        (Some("cell"), Some(val), _, _) => Ok(Selector::Cell(val.to_string())),
        (Some("range"), Some(val), _, _) => {
            let parts: Vec<&str> = val.split(',').collect();
            if parts.len() != 2 {
                return Err(to_error("Range requires start,end format"));
            }
            let start = parts[0].trim().parse::<usize>().map_err(|_| to_error("Invalid range start"))?;
            let end = parts[1].trim().parse::<usize>().map_err(|_| to_error("Invalid range end"))?;
            Ok(Selector::Range { start, end })
        }
        (_, _, Some(skip), Some(take)) => Ok(Selector::Slice { skip, take }),
        (_, _, Some(skip), None) => Ok(Selector::Slice { skip, take: 1 }),
        (_, _, None, Some(take)) => Ok(Selector::Slice { skip: 0, take }),
        (_, _, _, _) => Ok(Selector::Slice { skip: 0, take: 20 }),
    }
}

pub async fn handle_read(params: ReadParams) -> CallToolResult {
    let path = params.file_path.clone();
    if !std::path::PathBuf::from(&path).exists() {
        return file_not_found(&path);
    }
    let selector = match build_selector(&params) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let request = ReadRequest { file: path, selector };
    match tokio::task::spawn_blocking(move || ocean_api::docs::read_doc(&request)).await {
        Ok(Ok(result)) => to_text(format!("{:?}", result)),
        Ok(Err(e)) => to_error(&e.to_string()),
        Err(e) => to_error(&format!("Task failed: {}", e)),
    }
}

pub async fn handle_search(params: SearchParams) -> CallToolResult {
    let path = params.file_path.clone();
    if !std::path::PathBuf::from(&path).exists() {
        return file_not_found(&path);
    }
    let q = params.query.clone();
    match tokio::task::spawn_blocking(move || ocean_api::docs::search_doc(&path, &q)).await {
        Ok(Ok(matches)) => {
            if matches.is_empty() {
                return to_text("No matches found.".to_string());
            }
            let mut lines = Vec::new();
            for (i, m) in matches.iter().enumerate() {
                lines.push(format!("Match #{} (score: {:.2}):", i + 1, m.score));
                lines.push(format!("  Context: {}", m.context));
                lines.push(format!("  Text: {}", m.text));
                lines.push(String::new());
            }
            to_text(lines.join("\n"))
        }
        Ok(Err(e)) => to_error(&e.to_string()),
        Err(e) => to_error(&format!("Task failed: {}", e)),
    }
}

pub async fn handle_grep(params: GrepParams) -> CallToolResult {
    let dir = params.directory.clone();
    if !std::path::PathBuf::from(&dir).is_dir() {
        return dir_not_found(&dir);
    }
    let q = params.query.clone();
    match tokio::task::spawn_blocking(move || ocean_api::docs::grep_docs(&dir, &q)).await {
        Ok(Ok(grep_result)) => {
            if grep_result.total_matches == 0 {
                return to_text("No matches found.".to_string());
            }
            let mut lines = vec![
                format!("Total matches: {} across {} files", grep_result.total_matches, grep_result.total_files),
                String::new(),
            ];
            for fm in &grep_result.file_matches {
                lines.push(format!("File: {} ({} match(es))", fm.file, fm.matches.len()));
                for m in &fm.matches {
                    lines.push(format!("  [score {:.2}] {}", m.score, m.context));
                }
                lines.push(String::new());
            }
            to_text(lines.join("\n"))
        }
        Ok(Err(e)) => to_error(&e.to_string()),
        Err(e) => to_error(&format!("Task failed: {}", e)),
    }
}

pub async fn handle_info(params: InfoParams) -> CallToolResult {
    let path = params.file_path.clone();
    if !std::path::PathBuf::from(&path).exists() {
        return file_not_found(&path);
    }
    match tokio::task::spawn_blocking(move || ocean_api::docs::open_doc(&path)).await {
        Ok(Ok(doc_result)) => {
            let meta = &doc_result.metadata;
            let mut lines = vec![
                format!("File: {}", meta.path.display()),
                format!("Format: {:?}", meta.format),
                format!("Size: {} bytes", meta.size),
            ];
            if let Some(ref t) = meta.title {
                lines.push(format!("Title: {}", t));
            }
            if let Some(ref a) = meta.author {
                lines.push(format!("Author: {}", a));
            }
            if let Some(pc) = meta.page_count {
                lines.push(format!("Page count: {}", pc));
            }
            lines.push(String::new());
            lines.push("Outline:".to_string());
            lines.push(format_outline_entries(&doc_result.outline.entries, 0));
            to_text(lines.join("\n"))
        }
        Ok(Err(e)) => to_error(&e.to_string()),
        Err(e) => to_error(&format!("Task failed: {}", e)),
    }
}

fn format_outline_entries(entries: &[crate::ocean_parser::OutlineEntry], depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let mut parts = Vec::new();
    for entry in entries {
        parts.push(format!("{}- {} (level {})", indent, entry.label, entry.level));
        if !entry.children.is_empty() {
            parts.push(format_outline_entries(&entry.children, depth + 1));
        }
    }
    parts.join("\n")
}

pub async fn handle_scan(params: ScanParams) -> CallToolResult {
    let dir = params.directory.clone();
    if !std::path::PathBuf::from(&dir).is_dir() {
        return dir_not_found(&dir);
    }
    let include_hash = params.include_hash.unwrap_or(false);
    let dir_for_closure = dir.clone();
    match tokio::task::spawn_blocking(move || ocean_api::fs::scan_files(&dir_for_closure)).await {
        Ok(Ok(files)) => {
            if files.is_empty() {
                return to_text("No supported documents found.".to_string());
            }
            let mut lines = vec![format!("Found {} file(s) in '{}':", files.len(), dir)];
            for meta in &files {
                let hash_part = if include_hash {
                    if let Ok(h) = crate::ocean_api::fs::hash_file(&meta.path) {
                        format!(" | hash: {}", &h[..16])
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                let ext = std::path::Path::new(&meta.path).extension().and_then(|e| e.to_str()).unwrap_or("?");
                lines.push(format!("  {} ({} bytes, .{}){}", meta.path, meta.size, ext, hash_part));
            }
            to_text(lines.join("\n"))
        }
        Ok(Err(e)) => to_error(&e.to_string()),
        Err(e) => to_error(&format!("Task failed: {}", e)),
    }
}

pub async fn handle_chunk(params: ChunkParams) -> CallToolResult {
    let path = params.file_path.clone();
    if !std::path::PathBuf::from(&path).exists() {
        return file_not_found(&path);
    }
    let config = ChunkConfig {
        min_tokens: params.min_size.unwrap_or(100),
        max_tokens: params.max_size.unwrap_or(800),
        overlap_sentences: params.overlap.unwrap_or(1),
        include_images: false,
        rows_per_sheet_chunk: 50,
        token_estimator: crate::ocean_chunk::default_token_estimator,
    };
    match tokio::task::spawn_blocking(move || ocean_api::docs::chunk_doc(&path, Some(config))).await {
        Ok(Ok(chunks)) => {
            if chunks.is_empty() {
                return to_text("No chunks generated.".to_string());
            }
            let mut lines = vec![format!("Generated {} chunk(s):", chunks.len()), String::new()];
            for (i, c) in chunks.iter().enumerate() {
                lines.push(format!("Chunk #{} [id={}] [type={:?}]", i + 1, c.id, c.block_type));
                if let Some(ref h) = c.heading {
                    lines.push(format!("  Heading: {}", h));
                }
                let preview: String = c.content.chars().take(200).collect();
                lines.push(format!("  Content: {}...", preview));
                let token_estimate = (c.content.len() / 4).max(1);
                lines.push(format!("  Tokens (est): {}", token_estimate));
                lines.push(String::new());
            }
            to_text(lines.join("\n"))
        }
        Ok(Err(e)) => to_error(&e.to_string()),
        Err(e) => to_error(&format!("Task failed: {}", e)),
    }
}

pub async fn handle_verify(params: VerifyParams) -> CallToolResult {
    let path = params.file_path.clone();
    if !std::path::PathBuf::from(&path).exists() {
        return file_not_found(&path);
    }
    let h = params.expected_hash.clone();
    match tokio::task::spawn_blocking(move || ocean_api::fs::verify_file(&path, &h)).await {
        Ok(Ok(matched)) => to_text(if matched { "true".to_string() } else { "false".to_string() }),
        Ok(Err(e)) => to_error(&e.to_string()),
        Err(e) => to_error(&format!("Task failed: {}", e)),
    }
}
