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
