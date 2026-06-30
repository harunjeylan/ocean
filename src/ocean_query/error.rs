use std::fmt;

use crate::ocean_graph::error::GraphError;
use crate::ocean_vector::embedder::EmbedderError;
use crate::ocean_vector::store::StoreError;

#[derive(Debug, Clone)]
pub enum QueryError {
    NoResults,
    EmbeddingFailed(EmbedderError),
    VectorSearchFailed(StoreError),
    GraphExpandFailed(GraphError),
    ContextBuildFailed(String),
    InvalidQuery(String),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::NoResults => write!(f, "no results found"),
            QueryError::EmbeddingFailed(e) => write!(f, "embedding failed: {}", e),
            QueryError::VectorSearchFailed(e) => write!(f, "vector search failed: {}", e),
            QueryError::GraphExpandFailed(e) => write!(f, "graph expansion failed: {}", e),
            QueryError::ContextBuildFailed(msg) => write!(f, "context build failed: {}", msg),
            QueryError::InvalidQuery(msg) => write!(f, "invalid query: {}", msg),
        }
    }
}

impl std::error::Error for QueryError {}
