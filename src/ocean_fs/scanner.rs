use crate::ocean_fs::filter::FileFilter;
use crate::ocean_fs::hasher;
use crate::ocean_fs::types::{self, FileMeta, ScanError};
use rayon::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

pub fn scan_dir(path: &str) -> Result<Vec<FileMeta>, ScanError> {
    scan_dir_filtered(path, |_| true)
}

pub fn scan_dir_filtered(
    path: &str,
    extra_filter: impl Fn(&FileMeta) -> bool + Send + Sync,
) -> Result<Vec<FileMeta>, ScanError> {
    let root = Path::new(path);
    if !root.exists() {
        return Err(ScanError::InvalidPath(path.to_string()));
    }
    if !root.is_dir() {
        return Err(ScanError::InvalidPath(format!("{} is not a directory", path)));
    }

    let filter = FileFilter::default();

    let entries: Vec<_> = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let file_name = e.file_name().to_str().unwrap_or("");
            if e.file_type().is_dir() {
                !FileFilter::is_hidden(e) && !filter.should_ignore_dir(file_name)
            } else if e.file_type().is_symlink() {
                false
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !FileFilter::is_hidden(e))
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map_or(false, |ext| filter.is_supported_extension(ext))
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    let metas: Vec<FileMeta> = entries
        .par_iter()
        .filter_map(|entry_path| {
            let path_str = entry_path.to_str()?;
            let metadata = std::fs::metadata(entry_path).ok()?;

            let hash = hasher::hash_file(path_str).ok()?;

            let ext = entry_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Some(FileMeta {
                id: types::generate_file_id(),
                path: path_str.to_string(),
                hash,
                size: metadata.len(),
                modified,
                extension: ext,
            })
        })
        .filter(|meta| extra_filter(meta))
        .collect();

    Ok(metas)
}
