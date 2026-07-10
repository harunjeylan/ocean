use mime_guess;
use std::time::{SystemTime, UNIX_EPOCH};
use surrealdb::types::SurrealValue;
use uuid::Uuid;

pub type FileId = String;

pub fn generate_file_id() -> FileId {
    Uuid::now_v7().to_string()
}

pub fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMeta {
    pub id: FileId,
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub modified: u64,
    pub extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    Created(String),
    Modified(String),
    Deleted(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    Document,
    Spreadsheet,
    Presentation,
    Image,
    Text,
    Other,
}

impl FileCategory {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "pdf" | "docx" => FileCategory::Document,
            "xlsx" => FileCategory::Spreadsheet,
            "pptx" => FileCategory::Presentation,
            "png" | "jpg" | "jpeg" => FileCategory::Image,
            "txt" | "md" | "html" | "htm" => FileCategory::Text,
            _ => FileCategory::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFile {
    pub id: FileId,
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub modified: u64,
    pub extension: String,
    pub category: FileCategory,
    pub mime_type: String,
}

const MIME_MAPPINGS: &[(&str, &str, &str)] = &[
    ("pdf", "application/pdf", "document"),
    (
        "docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "document",
    ),
    (
        "pptx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "presentation",
    ),
    (
        "xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "spreadsheet",
    ),
    ("txt", "text/plain", "text"),
    ("md", "text/markdown", "text"),
    ("html", "text/html", "text"),
    ("htm", "text/html", "text"),
    ("png", "image/png", "image"),
    ("jpg", "image/jpeg", "image"),
    ("jpeg", "image/jpeg", "image"),
];

pub fn normalize(meta: FileMeta) -> NormalizedFile {
    let category = FileCategory::from_extension(&meta.extension);
    let mime_type = MIME_MAPPINGS
        .iter()
        .find(|(ext, _, _)| *ext == meta.extension)
        .map(|(_, mime, _)| (*mime).to_string())
        .or_else(|| {
            mime_guess::from_ext(&meta.extension)
                .first()
                .map(|m| m.to_string())
        })
        .unwrap_or_else(|| "application/octet-stream".to_string());

    NormalizedFile {
        id: meta.id,
        path: meta.path,
        hash: meta.hash,
        size: meta.size,
        modified: meta.modified,
        extension: meta.extension,
        category,
        mime_type,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub struct PathMove {
    pub file_id: FileId,
    pub old_path: String,
    pub new_path: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum ScanError {
    InvalidPath(String),
    WalkError(String),
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanError::InvalidPath(msg) => write!(f, "invalid path: {}", msg),
            ScanError::WalkError(msg) => write!(f, "walk error: {}", msg),
        }
    }
}

impl std::error::Error for ScanError {}
