use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use super::*;
use crate::doc::{Document, DocumentError, DocumentFormat};

pub type OpenFn = Arc<dyn Fn(&str) -> Result<Box<dyn Document>, DocumentError> + Send + Sync>;

pub struct BackendRegistry {
    backends: HashMap<String, OpenFn>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    pub fn register(&mut self, format: DocumentFormat, open_fn: OpenFn) {
        for ext in format.extensions() {
            self.backends.insert(ext.to_string(), open_fn.clone());
        }
    }

    pub fn get(&self, ext: &str) -> Option<OpenFn> {
        self.backends.get(ext).cloned()
    }

    pub fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError> {
        let file_path = Path::new(path);
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| DocumentError::UnsupportedFormat(format!("no extension: {}", path)))?;

        let ext_lower = ext.to_lowercase();
        let open_fn = self
            .get(&ext_lower)
            .ok_or_else(|| DocumentError::UnsupportedFormat(format!("{}: {}", path, ext_lower)))?;

        open_fn(path)
    }
}

static GLOBAL_REGISTRY: OnceLock<BackendRegistry> = OnceLock::new();

pub fn open(path: &str) -> Result<Box<dyn Document>, DocumentError> {
    let registry = GLOBAL_REGISTRY.get_or_init(default_registry);
    registry.open(path)
}

pub fn init_registry(registry: BackendRegistry) {
    if GLOBAL_REGISTRY.set(registry).is_err() {
        eprintln!("warning: ocean_fs::doc registry already initialized, ignoring duplicate");
    }
}

pub fn default_registry() -> BackendRegistry {
    let mut registry = BackendRegistry::new();
    registry.register(
        DocumentFormat::Text,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(TxtDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Markdown,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(MarkdownDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Html,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(HtmlDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Docx,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(DocxDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Pptx,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(PptxDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Xlsx,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(XlsxDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Pdf,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(PdfDocument::open(path)?))
        }),
    );
    registry
}
