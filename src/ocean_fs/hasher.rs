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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_hash_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        fs::write(&path, b"").unwrap();
        let hash = hash_file(path.to_str().unwrap()).unwrap();
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_hash_text_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        fs::write(&path, b"Hello, World!").unwrap();
        let hash = hash_file(path.to_str().unwrap()).unwrap();
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_hash_binary_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("binary.bin");
        let data: Vec<u8> = (0..255).collect();
        fs::write(&path, &data).unwrap();
        let hash = hash_file(path.to_str().unwrap()).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_verify_hash_valid() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, b"content").unwrap();
        let hash = hash_file(path.to_str().unwrap()).unwrap();
        assert!(verify_hash(path.to_str().unwrap(), &hash));
    }

    #[test]
    fn test_verify_hash_invalid() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, b"content").unwrap();
        assert!(!verify_hash(
            path.to_str().unwrap(),
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn test_hash_large_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("large.txt");
        let mut f = fs::File::create(&path).unwrap();
        let data = vec![b'a'; 1024 * 1024];
        for _ in 0..10 {
            f.write_all(&data).unwrap();
        }
        drop(f);
        let hash = hash_file(path.to_str().unwrap()).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hash_nonexistent_file() {
        let result = hash_file("C:\\nonexistent_file_12345.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_consistency() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("consistent.txt");
        fs::write(&path, b"same content").unwrap();
        let hash1 = hash_file(path.to_str().unwrap()).unwrap();
        let hash2 = hash_file(path.to_str().unwrap()).unwrap();
        assert_eq!(hash1, hash2);
    }
}
