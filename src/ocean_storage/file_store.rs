use serde::{Deserialize, Serialize};

use crate::ocean_storage::error::StorageError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMeta {
    #[serde(rename = "file_id")]
    pub id: String,
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub modified: i64,
    pub extension: String,
    pub last_indexed: i64,
}

pub trait FileStore: Send + Sync {
    fn upsert_file(&self, file: &FileMeta) -> Result<(), StorageError>;
    fn get_file(&self, id: &str) -> Result<Option<FileMeta>, StorageError>;
    fn get_file_by_path(&self, path: &str) -> Result<Option<FileMeta>, StorageError>;
    fn delete_file(&self, id: &str) -> Result<bool, StorageError>;
    fn list_files(&self) -> Result<Vec<FileMeta>, StorageError>;
    fn needs_update(&self, file: &FileMeta) -> Result<bool, StorageError>;
}
