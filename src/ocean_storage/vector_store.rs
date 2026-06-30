use crate::ocean_storage::error::StorageError;

pub trait VectorStore: Send + Sync {
    fn insert(&self, record: &crate::ocean_storage::chunk_store::ChunkRecord) -> Result<(), StorageError>;
    fn get_chunk(&self, chunk_id: &str) -> Result<Option<crate::ocean_storage::chunk_store::ChunkRecord>, StorageError>;
    fn vector_search(
        &self,
        query_vec: &[f32],
        top_k: usize,
        extra_where: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, StorageError>;
    fn fts_search(
        &self,
        query: &str,
        top_k: usize,
        extra_where: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, StorageError>;
    fn delete_by_file(&self, file_id: &str) -> Result<u64, StorageError>;
    fn count(&self) -> Result<u64, StorageError>;
    fn initialize_schema(&self, dimension: usize) -> Result<(), StorageError>;
}
