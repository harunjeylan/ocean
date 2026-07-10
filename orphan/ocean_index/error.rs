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
    Runtime(RuntimeError),
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
            IndexError::Runtime(e) => write!(f, "runtime error: {}", e),
        }
    }
}

impl std::error::Error for IndexError {}

impl From<StorageError> for IndexError {
    fn from(e: StorageError) -> Self {
        IndexError::StorageError(e)
    }
}

impl From<RuntimeError> for IndexError {
    fn from(e: RuntimeError) -> Self {
        IndexError::Runtime(e)
    }
}

#[derive(Debug, Clone)]
pub enum RuntimeError {
    PoolPanic(String),
    QueueFull(usize),
    RetryExhausted {
        path: String,
        retries: u32,
        last_error: String,
    },
    RateLimitExceeded,
    BackpressureTimeout,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::PoolPanic(msg) => write!(f, "worker pool panic: {}", msg),
            RuntimeError::QueueFull(size) => write!(f, "job queue full ({} items)", size),
            RuntimeError::RetryExhausted { path, retries, last_error } => {
                write!(f, "retry exhausted for '{}' after {} attempts: {}", path, retries, last_error)
            }
            RuntimeError::RateLimitExceeded => write!(f, "rate limit exceeded"),
            RuntimeError::BackpressureTimeout => write!(f, "backpressure timeout"),
        }
    }
}

impl std::error::Error for RuntimeError {}
