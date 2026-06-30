use std::fmt;

use crate::ocean_storage::StorageError;

#[derive(Debug, Clone)]
pub enum IndexError {
    FileProcessError {
        file_id: String,
        stage: String,
        error: String,
    },
    StorageError(StorageError),
    ScanError(String),
    Aborted,
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexError::FileProcessError { file_id, stage, error } => {
                write!(f, "file {} failed at '{}': {}", &file_id[..file_id.len().min(8)], stage, error)
            }
            IndexError::StorageError(e) => write!(f, "storage error: {}", e),
            IndexError::ScanError(msg) => write!(f, "scan error: {}", msg),
            IndexError::Aborted => write!(f, "indexing aborted"),
        }
    }
}

impl std::error::Error for IndexError {}

impl From<StorageError> for IndexError {
    fn from(e: StorageError) -> Self {
        IndexError::StorageError(e)
    }
}
