use crate::utils::filter::FileFilter;
use std::path::{Path, PathBuf};

/// Walk `dir` recursively and return paths of files with supported extensions.
/// No hashing, no metadata collection — lighter than `scan_dir`.
pub fn find_files(dir: &Path) -> Vec<PathBuf> {
    let filter = FileFilter::default();
    let mut files = vec![];
    if !dir.is_dir() {
        return files;
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if !path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map_or(false, |n| n.starts_with('.'))
                    {
                        stack.push(path);
                    }
                } else if path.is_file() && filter.should_include(&path) {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    files
}
