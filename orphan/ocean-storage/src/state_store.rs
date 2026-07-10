use crate::ocean_storage::error::StorageError;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum IndexStatus {
    Pending,
    Indexed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StateRecord {
    pub file_id: String,
    pub hash: String,
    pub last_indexed: i64,
    pub status: IndexStatus,
}

pub trait StateStore: Send + Sync {
    fn update_state(&self, file_id: &str, hash: &str, status: IndexStatus) -> Result<(), StorageError>;
    fn get_state(&self, file_id: &str) -> Result<Option<StateRecord>, StorageError>;
    fn delete_state(&self, file_id: &str) -> Result<bool, StorageError>;
    fn list_pending(&self) -> Result<Vec<StateRecord>, StorageError>;
    fn list_all(&self) -> Result<Vec<StateRecord>, StorageError>;
}
