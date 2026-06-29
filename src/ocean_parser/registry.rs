use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::ocean_parser::{Document, DocumentError, DocumentFormat};

pub trait DocumentFactory: Send + Sync {
    fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError>;
}

pub type SharedFactory = Arc<dyn DocumentFactory>;

pub struct BackendRegistry {
    backends: HashMap<String, SharedFactory>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self { backends: HashMap::new() }
    }

    pub fn register(&mut self, format: DocumentFormat, factory: SharedFactory) {
        for ext in format.extensions() {
            self.backends.insert(ext.to_string(), Arc::clone(&factory));
        }
    }

    pub fn get(&self, ext: &str) -> Option<SharedFactory> {
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

        let factory = self.get(&ext).ok_or_else(|| {
            DocumentError::UnsupportedFormat(format!(".{} files are not supported", ext))
        })?;

        factory.open(path)
    }
}

static GLOBAL_REGISTRY: OnceLock<BackendRegistry> = OnceLock::new();

pub fn open(path: &str) -> Result<Box<dyn Document>, DocumentError> {
    let registry = GLOBAL_REGISTRY.get_or_init(default_registry);
    registry.open(path)
}

pub fn init_registry(registry: BackendRegistry) {
    GLOBAL_REGISTRY.set(registry).ok();
}

pub fn default_registry() -> BackendRegistry {
    let mut registry = BackendRegistry::new();
    registry.register(
        DocumentFormat::Text,
        Arc::new(super::txt::TxtFactory),
    );
    registry.register(
        DocumentFormat::Markdown,
        Arc::new(super::markdown::MarkdownFactory),
    );
    registry.register(
        DocumentFormat::Html,
        Arc::new(super::html::HtmlFactory),
    );
    registry.register(
        DocumentFormat::Docx,
        Arc::new(super::docx::DocxFactory),
    );
    registry.register(
        DocumentFormat::Pptx,
        Arc::new(super::pptx::PptxFactory),
    );
    registry.register(
        DocumentFormat::Xlsx,
        Arc::new(super::xlsx::XlsxFactory),
    );
    registry.register(
        DocumentFormat::Pdf,
        Arc::new(super::pdf::PdfFactory),
    );
    registry
}
