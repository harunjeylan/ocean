use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;

use crate::ocean_fs::{scan_dir, FileEvent, FileMeta, FileWatcher};

use super::types::ApiError;

pub fn scan_files(dir: &str) -> Result<Vec<FileMeta>, ApiError> {
    let dir_path = PathBuf::from(dir);
    if !dir_path.is_dir() {
        return Err(ApiError::FsError(format!("directory not found: {}", dir)));
    }
    scan_dir(dir).map_err(|e| ApiError::FsError(format!("Scan failed: {}", e)))
}

pub fn hash_file(path: &str) -> Result<String, ApiError> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(ApiError::FsError(format!("file not found: {}", path)));
    }
    crate::ocean_fs::hash_file(path)
        .map_err(|e| ApiError::FsError(format!("Hash failed: {}", e)))
}

pub fn verify_file(path: &str, hash: &str) -> Result<bool, ApiError> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(ApiError::FsError(format!("file not found: {}", path)));
    }
    Ok(crate::ocean_fs::verify_hash(path, hash))
}

pub fn watch_directory<F>(dir: &str, callback: F) -> Result<(), ApiError>
where
    F: Fn(FileEvent) + Send + 'static,
{
    let dir_path = PathBuf::from(dir);
    if !dir_path.is_dir() {
        return Err(ApiError::FsError(format!("directory not found: {}", dir)));
    }

    let watcher = FileWatcher::new();
    let (tx, rx) = mpsc::channel::<FileEvent>();

    let cb = Arc::new(move |event: FileEvent| {
        let _ = tx.send(event);
    });

    let handle = watcher
        .watch(dir, cb)
        .map_err(|e| ApiError::FsError(format!("Watch failed: {}", e)))?;

    for event in rx {
        callback(event);
    }

    watcher.unwatch(handle).map_err(|e| ApiError::FsError(format!("Unwatch failed: {}", e)))
}
