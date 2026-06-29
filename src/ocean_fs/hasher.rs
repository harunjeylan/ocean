use sha2::{Digest, Sha256};
use std::io::{self, BufReader, Read};
use std::path::Path;

const BUFFER_SIZE: usize = 64 * 1024;
const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub enum HashError {
    IoError(String),
    FileTooLarge(u64),
}

impl std::fmt::Display for HashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashError::IoError(msg) => write!(f, "I/O error: {}", msg),
            HashError::FileTooLarge(size) => write!(f, "file too large: {} bytes", size),
        }
    }
}

impl std::error::Error for HashError {}

impl From<io::Error> for HashError {
    fn from(e: io::Error) -> Self {
        HashError::IoError(e.to_string())
    }
}

pub fn hash_file(path: &str) -> Result<String, HashError> {
    let file_path = Path::new(path);
    let metadata = std::fs::metadata(file_path)?;
    let file_size = metadata.len();

    if file_size > MAX_FILE_SIZE {
        return Err(HashError::FileTooLarge(file_size));
    }

    let file = std::fs::File::open(file_path)?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub fn verify_hash(path: &str, expected: &str) -> bool {
    hash_file(path).ok().map_or(false, |h| h == expected)
}
