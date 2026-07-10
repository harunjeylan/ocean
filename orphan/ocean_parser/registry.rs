use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::ocean_fs::parser::{Document, DocumentError, DocumentFormat};

pub type OpenFn = Arc<dyn Fn(&str) -> Result<Box<dyn Document>, DocumentError> + Send + Sync>;

pub struct BackendRegistry {
    backends: HashMap<String, OpenFn>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self { backends: HashMap::new() }
    }

    pub fn register(&mut self, format: DocumentFormat, open_fn: OpenFn) {
        for ext in format.extensions() {
            self.backends.insert(ext.to_string(), Arc::clone(&open_fn));
        }
    }

    pub fn get(&self, ext: &str) -> Option<OpenFn> {
        self.backends.get(ext).cloned()
    }

    pub fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError> {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .ok_or_else(|| {
                DocumentError::UnsupportedFormat(format!("no file extension: {}", path))
            })?;

        let open_fn = self.get(&ext).ok_or_else(|| {
            DocumentError::UnsupportedFormat(format!(".{} files are not supported", ext))
        })?;

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
        eprintln!("warning: ocean_parser registry already initialized, ignoring duplicate");
    }
}

pub fn default_registry() -> BackendRegistry {
    let mut registry = BackendRegistry::new();
    registry.register(
        DocumentFormat::Text,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(super::txt::TxtDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Markdown,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(super::markdown::MarkdownDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Html,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(super::html::HtmlDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Docx,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(super::docx::DocxDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Pptx,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(super::pptx::PptxDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Xlsx,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(super::xlsx::XlsxDocument::open(path)?))
        }),
    );
    registry.register(
        DocumentFormat::Pdf,
        Arc::new(|path| -> Result<Box<dyn Document>, DocumentError> {
            Ok(Box::new(super::pdf::PdfDocument::open(path)?))
        }),
    );
    registry
}
