use std::fmt;

#[derive(Debug, Clone)]
pub enum DocumentError {
    UnsupportedFormat(String),
    InvalidSelector(String),
    CorruptedFile(String),
    PermissionDenied(String),
    InvalidEncoding(String),
    ParseFailed(String),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentError::UnsupportedFormat(msg) => write!(f, "unsupported format: {}", msg),
            DocumentError::InvalidSelector(msg) => write!(f, "invalid selector: {}", msg),
            DocumentError::CorruptedFile(msg) => write!(f, "corrupted file: {}", msg),
            DocumentError::PermissionDenied(msg) => write!(f, "permission denied: {}", msg),
            DocumentError::InvalidEncoding(msg) => write!(f, "invalid encoding: {}", msg),
            DocumentError::ParseFailed(msg) => write!(f, "parse failed: {}", msg),
        }
    }
}

