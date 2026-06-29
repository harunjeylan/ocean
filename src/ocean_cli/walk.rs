use std::path::{Path, PathBuf};

const SUPPORTED_EXTS: &[&str] = &["pdf", "docx", "xlsx", "pptx", "txt", "md", "html", "htm"];

pub fn walk_supported_files(dir: &Path) -> Vec<PathBuf> {
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
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if SUPPORTED_EXTS.contains(&ext.to_lowercase().as_str()) {
                            files.push(path);
                        }
                    }
                }
            }
        }
    }
    files.sort();
    files
}
