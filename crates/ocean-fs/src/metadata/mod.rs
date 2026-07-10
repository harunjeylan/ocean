use crate::doc::{DocumentError, DocumentMetadata, open};

/// Open a document at `path` and return its metadata.
pub fn metadata(path: &str) -> Result<DocumentMetadata, DocumentError> {
    Ok(open(path)?.metadata())
}
