use std::time::{SystemTime, UNIX_EPOCH};
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
    Created(FileMeta),
    Modified(FileMeta),
    Deleted(FileId),
    Renamed {
        file_id: FileId,
        old_path: String,
        new_path: String,
    },
    Moved {
        file_id: FileId,
        old_path: String,
        new_path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    Document,
    Spreadsheet,
    Presentation,
    Image,
    Text,
    Unknown,
}

impl FileCategory {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "pdf" | "docx" => FileCategory::Document,
            "xlsx" => FileCategory::Spreadsheet,
            "pptx" => FileCategory::Presentation,
            "png" | "jpg" | "jpeg" => FileCategory::Image,
            "txt" | "md" | "html" | "htm" => FileCategory::Text,
            _ => FileCategory::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFile {
    pub id: FileId,
    pub meta: FileMeta,
    pub mime_type: String,
    pub category: FileCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PathMove {
    pub file_id: FileId,
    pub old_path: String,
    pub new_path: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub enum ScanError {
    InvalidPath(String),
    IoError(String),
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanError::InvalidPath(p) => write!(f, "invalid path: {}", p),
            ScanError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for ScanError {}
