use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum SecurityError {
    PathOutsideWorkspace { path: String, workspace: String },
    UnsupportedExtension { path: String, extension: String },
    SymlinkDenied { path: String },
    CanonicalizationFailed { path: String, error: String },
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::PathOutsideWorkspace { path, workspace } => {
                write!(f, "path '{}' is outside workspace '{}'", path, workspace)
            }
            SecurityError::UnsupportedExtension { path, extension } => {
                write!(f, "unsupported extension '{}' for path '{}'", extension, path)
            }
            SecurityError::SymlinkDenied { path } => {
                write!(f, "symlink denied: '{}' points outside workspace", path)
            }
            SecurityError::CanonicalizationFailed { path, error } => {
                write!(f, "canonicalization failed for '{}': {}", path, error)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

pub struct Sandbox {
    workspace_root: PathBuf,
    allowed_extensions: Vec<String>,
}

impl Sandbox {
    pub fn new(workspace_root: &Path) -> Result<Self, SecurityError> {
        let canonical = workspace_root
            .canonicalize()
            .map_err(|e| SecurityError::CanonicalizationFailed {
                path: workspace_root.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;
        Ok(Self {
            workspace_root: canonical,
            allowed_extensions: vec![
                "pdf".into(), "docx".into(), "pptx".into(), "xlsx".into(),
                "txt".into(), "md".into(), "html".into(), "htm".into(),
                "png".into(), "jpg".into(), "jpeg".into(),
            ],
        })
    }

    pub fn validate(&self, path: &Path) -> Result<(), SecurityError> {
        let canonical = path.canonicalize().map_err(|e| {
            SecurityError::CanonicalizationFailed {
                path: path.to_string_lossy().to_string(),
                error: e.to_string(),
            }
        })?;

        if canonical.is_symlink() {
            let target = std::fs::read_link(&canonical).unwrap_or_default();
            if !target.starts_with(&self.workspace_root) {
                return Err(SecurityError::SymlinkDenied {
                    path: path.to_string_lossy().to_string(),
                });
            }
        }

        if !canonical.starts_with(&self.workspace_root) {
            return Err(SecurityError::PathOutsideWorkspace {
                path: path.to_string_lossy().to_string(),
                workspace: self.workspace_root.to_string_lossy().to_string(),
            });
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if !self.allowed_extensions.contains(&ext_lower) {
                return Err(SecurityError::UnsupportedExtension {
                    path: path.to_string_lossy().to_string(),
                    extension: ext_lower,
                });
            }
        }

        Ok(())
    }

    pub fn allow_extension(&mut self, ext: &str) {
        let ext_lower = ext.to_lowercase();
        if !self.allowed_extensions.contains(&ext_lower) {
            self.allowed_extensions.push(ext_lower);
        }
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }
}
