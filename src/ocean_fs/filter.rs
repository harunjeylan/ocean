use std::path::Path;

const DEFAULT_IGNORE_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".cache",
    "__pycache__",
    ".svn",
    ".hg",
    ".idea",
    ".vscode",
    "target",
    "build",
    "dist",
    ".next",
];

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "pdf", "docx", "pptx", "xlsx", "txt", "md", "html", "htm", "png", "jpg", "jpeg",
];

#[derive(Debug, Clone)]
pub struct FileFilter {
    ignore_dirs: Vec<String>,
    ignore_patterns: Vec<String>,
    supported_extensions: Vec<String>,
}

impl Default for FileFilter {
    fn default() -> Self {
        Self {
            ignore_dirs: DEFAULT_IGNORE_DIRS.iter().map(|s| s.to_string()).collect(),
            ignore_patterns: Vec::new(),
            supported_extensions: SUPPORTED_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl FileFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ignore_dirs(mut self, dirs: Vec<String>) -> Self {
        self.ignore_dirs = dirs;
        self
    }

    pub fn with_extra_ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns.extend(patterns);
        self
    }

    pub fn with_supported_extensions(mut self, extensions: Vec<String>) -> Self {
        self.supported_extensions = extensions;
        self
    }

    pub fn is_hidden(entry: &walkdir::DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map_or(false, |s| s.starts_with('.'))
    }

    pub fn should_ignore_dir(&self, dir_name: &str) -> bool {
        let lower = dir_name.to_lowercase();
        self.ignore_dirs.iter().any(|d| d == &lower)
            || self.ignore_patterns.iter().any(|p| {
                glob::Pattern::new(p)
                    .ok()
                    .map_or(false, |pat| pat.matches(dir_name))
            })
    }

    pub fn is_supported_extension(&self, ext: &str) -> bool {
        let lower = ext.to_lowercase();
        self.supported_extensions.contains(&lower)
    }

    pub fn should_include(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if ext.is_empty() {
            return false;
        }

        self.is_supported_extension(&ext)
    }

    pub fn supported_extensions(&self) -> &[String] {
        &self.supported_extensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let dir = tempfile::tempdir().unwrap();
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
}
