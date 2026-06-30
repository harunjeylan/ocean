pub mod config;
pub mod error;
pub mod file_store;
pub mod chunk_store;
pub mod vector_store;
pub mod graph_store;
pub mod state_store;
pub mod transaction;

mod file_store_impl;
mod chunk_store_impl;
mod vector_store_impl;
mod graph_store_impl;
mod state_store_impl;
mod storage_impl;

pub use config::StorageConfig;
pub use error::StorageError;
pub use file_store::{FileMeta, FileStore};
pub use chunk_store::{ChunkData, ChunkRecord, ChunkStore};
pub use vector_store::VectorStore;
pub use graph_store::{
    Edge, EdgeDirection, GraphStore, Node, NodeType, RelationType,
};
pub use state_store::{IndexStatus, StateRecord, StateStore};
pub use transaction::{StagedWrite, TransactionStaging};

pub use file_store_impl::SurrealFileStore;
pub use chunk_store_impl::SurrealChunkStore;
pub use vector_store_impl::SurrealVectorStore;
pub use graph_store_impl::SurrealGraphStore;
pub use state_store_impl::SurrealStateStore;
pub use storage_impl::SurrealStorage;

#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub file_count: u64,
    pub chunk_count: u64,
    pub node_count: u64,
    pub edge_count: u64,
}

pub trait Storage: Send + Sync {
    fn files(&self) -> &dyn FileStore;
    fn chunks(&self) -> &dyn ChunkStore;
    fn vectors(&self) -> &dyn VectorStore;
    fn graph(&self) -> &dyn GraphStore;
    fn state(&self) -> &dyn StateStore;

    fn begin_transaction(&mut self) -> Result<(), StorageError>;
    fn commit(&mut self) -> Result<(), StorageError>;
    fn rollback(&mut self) -> Result<(), StorageError>;
    fn in_transaction(&self) -> bool;

    fn storage_path(&self) -> &str;
    fn count_all(&self) -> Result<StorageStats, StorageError>;
}
