use serde::{Deserialize, Serialize};

use crate::ocean_chunk::Chunk;
use crate::ocean_storage::error::StorageError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRecord {
    pub chunk_id: String,
    pub file_id: String,
    pub content: String,
    pub heading: Option<String>,
    pub page: Option<i64>,
    pub slide: Option<i64>,
    pub sheet: Option<String>,
    pub block_type: String,
    pub content_hash: String,
    pub created_at: i64,
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: i64,
}

impl ChunkRecord {
    pub fn from_chunk(chunk: &Chunk, embedding: Vec<f32>, model: &str) -> Self {
        let dimension = embedding.len() as i64;
        let content_hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(chunk.content.as_bytes());
            format!("{:x}", hasher.finalize())
        };
        Self {
            chunk_id: chunk.id.clone(),
            file_id: chunk.file_id.clone(),
            content: chunk.content.clone(),
            heading: chunk.heading.clone(),
            page: chunk.page.map(|p| p as i64),
            slide: chunk.slide.map(|s| s as i64),
            sheet: chunk.sheet.clone(),
            block_type: format!("{:?}", chunk.block_type),
            content_hash,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            embedding,
            model: model.to_string(),
            dimension,
        }
    }
}

pub trait ChunkStore: Send + Sync {
    fn insert_chunk(&self, chunk: &ChunkRecord) -> Result<(), StorageError>;
    fn upsert_chunk(&self, chunk: &ChunkRecord) -> Result<(), StorageError>;
    fn get_chunk(&self, chunk_id: &str) -> Result<Option<ChunkRecord>, StorageError>;
    fn delete_chunks_by_file(&self, file_id: &str) -> Result<u64, StorageError>;
    fn count(&self) -> Result<u64, StorageError>;
    fn chunk_exists(&self, content_hash: &str, model: &str) -> Result<bool, StorageError>;
    fn get_by_file_and_heading(
        &self,
        file_id: &str,
        heading: Option<&str>,
    ) -> Result<Vec<ChunkRecord>, StorageError>;
}
