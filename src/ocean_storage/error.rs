use std::fmt;

#[derive(Debug, Clone)]
pub enum StorageError {
    ConnectionFailed(String, String),
    QueryFailed(String, String),
    RecordNotFound(String, String),
    SchemaError(String, String),
    TransactionFailed {
        succeeded: Vec<String>,
        failed: Vec<(String, String)>,
    },
    BatchFailed(String, u64, u64, String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::ConnectionFailed(store, details) => {
                write!(f, "{}::connection: {}", store, details)
            }
            StorageError::QueryFailed(store, details) => {
                write!(f, "{}::query: {}", store, details)
            }
            StorageError::RecordNotFound(store, id) => {
                write!(f, "{}::not_found: {}", store, id)
            }
            StorageError::SchemaError(store, details) => {
                write!(f, "{}::schema: {}", store, details)
            }
            StorageError::TransactionFailed { succeeded, failed } => {
                write!(
                    f,
                    "TransactionFailed: {} stores succeeded, {} stores failed: {:?}",
                    succeeded.len(),
                    failed.len(),
                    failed
                )
            }
            StorageError::BatchFailed(store, ok, nok, err) => {
                write!(f, "{}::batch: {} ok, {} failed: {}", store, ok, nok, err)
            }
        }
    }
}

impl std::error::Error for StorageError {}

impl From<surrealdb::Error> for StorageError {
    fn from(e: surrealdb::Error) -> Self {
        StorageError::QueryFailed("unknown".into(), e.to_string())
    }
}

impl From<StorageError> for String {
    fn from(e: StorageError) -> Self {
        e.to_string()
    }
}
