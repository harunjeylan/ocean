use crate::ocean_fs::parser::{DocumentError, DocumentMetadata, Match, ReadResult, Selector};

pub trait Document: Send + Sync {
    fn metadata(&self) -> DocumentMetadata;
    fn search(&self, query: &str) -> Vec<Match>;
    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError>;
}
