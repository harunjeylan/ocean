use std::fmt;

#[derive(Debug, Clone)]
pub enum GraphError {
    StoreError(String),
    NodeNotFound(String),
    EdgeNotFound(String),
    InvalidDepth(String),
    CycleDetected,
    SerializationError(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::StoreError(msg) => write!(f, "graph store error: {}", msg),
            GraphError::NodeNotFound(id) => write!(f, "node not found: {}", id),
            GraphError::EdgeNotFound(id) => write!(f, "edge not found: {}", id),
            GraphError::InvalidDepth(msg) => write!(f, "invalid expansion depth: {}", msg),
            GraphError::CycleDetected => write!(f, "cycle detected during graph traversal"),
            GraphError::SerializationError(msg) => write!(f, "graph serialization error: {}", msg),
        }
    }
}

impl std::error::Error for GraphError {}

impl From<surrealdb::Error> for GraphError {
    fn from(e: surrealdb::Error) -> Self {
        GraphError::StoreError(e.to_string())
    }
}

impl From<crate::ocean_storage::error::StorageError> for GraphError {
    fn from(e: crate::ocean_storage::error::StorageError) -> Self {
        GraphError::StoreError(e.to_string())
    }
}
