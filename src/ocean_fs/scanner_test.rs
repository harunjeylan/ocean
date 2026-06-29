use crate::ocean_fs::scanner::{scan_dir, scan_dir_filtered};
use crate::ocean_fs::types::ScanError;
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
