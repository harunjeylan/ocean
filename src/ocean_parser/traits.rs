use crate::ocean_parser::{DocumentError, DocumentMetadata, Match, Outline, ReadResult, Selector};

pub trait Document: Send + Sync {
    fn metadata(&self) -> DocumentMetadata;
    fn outline(&self) -> Outline;
    fn page_count(&self) -> Option<u32>;
    fn search(&self, query: &str) -> Vec<Match>;
    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError>;
}
