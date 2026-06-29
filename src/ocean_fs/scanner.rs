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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_scan_empty_directory() {
        let dir = tempdir().unwrap();
        let results = scan_dir(dir.path().to_str().unwrap()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_nested_directories() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir").join("test.txt"), b"hello").unwrap();
        fs::write(dir.path().join("root.txt"), b"world").unwrap();

        let results = scan_dir(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_scan_ignores_hidden_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".hidden.txt"), b"secret").unwrap();
        fs::write(dir.path().join("visible.txt"), b"hello").unwrap();

        let results = scan_dir(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].path.contains(".hidden"));
    }

    #[test]
    fn test_scan_ignores_node_modules() {
        let dir = tempdir().unwrap();
        let nm = dir.path().join("node_modules");
        fs::create_dir_all(&nm).unwrap();
        fs::write(nm.join("module.js"), b"code").unwrap();

        let results = scan_dir(dir.path().to_str().unwrap()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_ignores_git_directory() {
        let dir = tempdir().unwrap();
        let git = dir.path().join(".git");
        fs::create_dir_all(&git).unwrap();
        fs::write(git.join("config"), b"config").unwrap();

        let results = scan_dir(dir.path().to_str().unwrap()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_only_supported_extensions() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file.txt"), b"text").unwrap();
        fs::write(dir.path().join("file.exe"), b"binary").unwrap();
        fs::write(dir.path().join("file.pdf"), b"pdf").unwrap();

        let results = scan_dir(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_scan_invalid_path() {
        let result = scan_dir("C:\\nonexistent_path_xyz_123");
        assert!(result.is_err());
        match result {
            Err(ScanError::InvalidPath(_)) => {}
            _ => panic!("expected InvalidPath error"),
        }
    }

    #[test]
    fn test_scan_file_metadata_fields() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        fs::write(&file_path, b"# Hello").unwrap();

        let results = scan_dir(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 1);
        let meta = &results[0];
        assert!(!meta.id.is_empty());
        assert!(meta.path.contains("test.md"));
        assert_eq!(meta.hash.len(), 64);
        assert_eq!(meta.size, 7);
        assert!(meta.modified > 0);
        assert_eq!(meta.extension, "md");
    }

    #[test]
    fn test_scan_large_number_of_files() {
        let dir = tempdir().unwrap();
        for i in 0..100 {
            let mut f = fs::File::create(dir.path().join(format!("file_{}.txt", i))).unwrap();
            f.write_all(format!("content {}", i).as_bytes()).unwrap();
        }

        let results = scan_dir(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 100);
    }

    #[test]
    fn test_scan_filtered_callback() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("keep.txt"), b"keep").unwrap();
        fs::write(dir.path().join("skip.txt"), b"skip").unwrap();

        let results = scan_dir_filtered(
            dir.path().to_str().unwrap(),
            |meta| meta.path.contains("keep"),
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("keep"));
    }
}
