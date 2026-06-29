use crate::ocean_fs::filter::FileFilter;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_default_filter_ignores_hidden_dirs() {
    let filter = FileFilter::default();
    assert!(filter.should_ignore_dir(".git"));
    assert!(filter.should_ignore_dir("node_modules"));
    assert!(filter.should_ignore_dir(".cache"));
}

#[test]
fn test_default_filter_supports_extensions() {
    let filter = FileFilter::default();
    assert!(filter.is_supported_extension("pdf"));
    assert!(filter.is_supported_extension("txt"));
    assert!(filter.is_supported_extension("md"));
    assert!(filter.is_supported_extension("png"));
    assert!(filter.is_supported_extension("jpg"));
    assert!(filter.is_supported_extension("html"));
    assert!(filter.is_supported_extension("docx"));
    assert!(filter.is_supported_extension("pptx"));
    assert!(filter.is_supported_extension("xlsx"));
}

#[test]
fn test_default_filter_rejects_unknown_extensions() {
    let filter = FileFilter::default();
    assert!(!filter.is_supported_extension("exe"));
    assert!(!filter.is_supported_extension("dll"));
    assert!(!filter.is_supported_extension("zip"));
}

#[test]
fn test_filter_case_insensitive() {
    let filter = FileFilter::default();
    assert!(filter.is_supported_extension("PDF"));
    assert!(filter.is_supported_extension("TXT"));
    assert!(filter.is_supported_extension("Md"));
}

#[test]
fn test_custom_ignore_dirs() {
    let filter = FileFilter::new().with_ignore_dirs(vec!["custom_cache".to_string()]);
    assert!(filter.should_ignore_dir("custom_cache"));
    assert!(!filter.should_ignore_dir(".git"));
}

#[test]
fn test_custom_supported_extensions() {
    let filter = FileFilter::new().with_supported_extensions(vec!["rs".to_string()]);
    assert!(filter.is_supported_extension("rs"));
    assert!(!filter.is_supported_extension("pdf"));
}

#[test]
fn test_empty_extension_not_supported() {
    let filter = FileFilter::default();
    assert!(!filter.should_include(Path::new("Makefile")));
}

#[test]
fn test_hidden_file_detection() {
    let dir = tempdir().unwrap();
    let hidden_path = dir.path().join(".hidden.txt");
    std::fs::write(&hidden_path, b"test").unwrap();
    for entry in walkdir::WalkDir::new(dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().file_name().unwrap_or_default() == ".hidden.txt" {
            assert!(FileFilter::is_hidden(&entry));
            return;
        }
    }
    panic!("hidden file not found");
}
