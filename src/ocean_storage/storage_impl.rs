use std::sync::Mutex;

use crate::ocean_storage::chunk_store::ChunkStore;
use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::error::StorageError;
use crate::ocean_storage::file_store::FileStore;
use crate::ocean_storage::graph_store::GraphStore;
use crate::ocean_storage::state_store::StateStore;
use crate::ocean_storage::transaction::TransactionStaging;
use crate::ocean_storage::vector_store::VectorStore;

use super::file_store_impl::SurrealFileStore;
use super::chunk_store_impl::SurrealChunkStore;
use super::vector_store_impl::SurrealVectorStore;
use super::graph_store_impl::SurrealGraphStore;
use super::state_store_impl::SurrealStateStore;
use super::{Storage, StorageStats};

pub struct SurrealStorage {
    config: StorageConfig,
    files: SurrealFileStore,
    chunks: SurrealChunkStore,
    vectors: SurrealVectorStore,
    graph: SurrealGraphStore,
    state: SurrealStateStore,
    transaction_depth: Mutex<u32>,
    staging: Mutex<TransactionStaging>,
}

impl SurrealStorage {
    pub fn new(base_path: &str) -> Result<Self, StorageError> {
        let config = StorageConfig::new(base_path);
        config.ensure_dirs().map_err(|e| {
            StorageError::ConnectionFailed("Storage".into(), format!("Failed to create directories: {}", e))
        })?;

        let files = SurrealFileStore::new_persistent(&config)?;
        let chunks = SurrealChunkStore::new_persistent(&config)?;
        let vectors = SurrealVectorStore::new_persistent(&config)?;
        let graph = SurrealGraphStore::new_persistent(&config)?;
        let state = SurrealStateStore::new_persistent(&config)?;

        Ok(Self {
            config,
            files,
            chunks,
            vectors,
            graph,
            state,
            transaction_depth: Mutex::new(0),
            staging: Mutex::new(TransactionStaging::new()),
        })
    }

    pub fn new_memory() -> Result<Self, StorageError> {
        let config = StorageConfig::new(":memory:");
        let files = SurrealFileStore::new_memory()?;
        let chunks = SurrealChunkStore::new_memory()?;
        let vectors = SurrealVectorStore::new_memory(&config)?;
        let graph = SurrealGraphStore::new_memory(&config)?;
        let state = SurrealStateStore::new_memory()?;

        Ok(Self {
            config,
            files,
            chunks,
            vectors,
            graph,
            state,
            transaction_depth: Mutex::new(0),
            staging: Mutex::new(TransactionStaging::new()),
        })
    }

    pub fn initialize_vector_schema(&self, dimension: usize) -> Result<(), StorageError> {
        self.vectors.initialize_schema(dimension)
    }

    pub fn initialize_graph_schema(&self) -> Result<(), StorageError> {
        self.graph.initialize_schema()
    }

    pub fn config(&self) -> &StorageConfig {
        &self.config
    }
}

impl Storage for SurrealStorage {
    fn files(&self) -> &dyn FileStore {
        &self.files
    }

    fn chunks(&self) -> &dyn ChunkStore {
        &self.chunks
    }

    fn vectors(&self) -> &dyn VectorStore {
        &self.vectors
    }

    fn graph(&self) -> &dyn GraphStore {
        &self.graph
    }

    fn state(&self) -> &dyn StateStore {
        &self.state
    }

    fn begin_transaction(&mut self) -> Result<(), StorageError> {
        let mut depth = self.transaction_depth.lock().unwrap();
        if *depth == 0 {
            self.staging.lock().unwrap().clear();
        }
        *depth += 1;
        Ok(())
    }

    fn commit(&mut self) -> Result<(), StorageError> {
        let mut depth = self.transaction_depth.lock().unwrap();
        if *depth == 0 {
            return Err(StorageError::TransactionFailed {
                succeeded: vec![],
                failed: vec![("Transaction".into(), "No transaction in progress".into())],
            });
        }
        if *depth > 1 {
            *depth -= 1;
            return Ok(());
        }
        *depth = 0;
        drop(depth);

        let writes = self.staging.lock().unwrap().drain();
        if writes.is_empty() {
            return Ok(());
        }

        let mut succeeded: Vec<String> = Vec::new();
        let mut failed: Vec<(String, String)> = Vec::new();

        for write in &writes {
            let store_name = write.store_name.clone();
            let result: Result<(), StorageError> = match store_name.as_str() {
                "files" => {
                    let val = &write.data;
                    let file_id = val.get("file_id").and_then(|v| v.as_str()).unwrap_or("");
                    let file = serde_json::from_value(write.data.clone())
                        .map_err(|e| StorageError::QueryFailed("FileStore".into(), e.to_string()))?;
                    self.files.upsert_file(&file).or_else(|_| {
                        self.state
                            .update_state(file_id, "", crate::ocean_storage::state_store::IndexStatus::Failed)
                            .ok();
                        Err(StorageError::QueryFailed(
                            "FileStore".into(),
                            format!("commit failed for {}", file_id),
                        ))
                    })
                }
                "chunks" => {
                    let chunk: crate::ocean_storage::chunk_store::ChunkRecord = serde_json::from_value(write.data.clone())
                        .map_err(|e| StorageError::QueryFailed("ChunkStore".into(), e.to_string()))?;
                    self.chunks.upsert_chunk(&chunk)
                }
                "vectors" => {
                    let chunk: crate::ocean_storage::chunk_store::ChunkRecord = serde_json::from_value(write.data.clone())
                        .map_err(|e| StorageError::QueryFailed("VectorStore".into(), e.to_string()))?;
                    self.vectors.insert(&chunk)
                }
                "graph" => {
                    let file_id = write.record_id.clone();
                    let node: crate::ocean_storage::graph_store::Node = serde_json::from_value(write.data.clone())
                        .map_err(|e| StorageError::QueryFailed("GraphStore".into(), e.to_string()))?;
                    self.graph.insert_node(&node, &file_id)
                }
                other => Err(StorageError::QueryFailed(
                    other.into(),
                    "Unknown store in transaction".into(),
                )),
            };

            match result {
                Ok(()) => succeeded.push(store_name),
                Err(e) => failed.push((store_name, e.to_string())),
            }
        }

        if failed.is_empty() {
            Ok(())
        } else {
            Err(StorageError::TransactionFailed { succeeded, failed })
        }
    }

    fn rollback(&mut self) -> Result<(), StorageError> {
        let mut depth = self.transaction_depth.lock().unwrap();
        if *depth == 0 {
            return Err(StorageError::TransactionFailed {
                succeeded: vec![],
                failed: vec![("Transaction".into(), "No transaction in progress".into())],
            });
        }
        *depth = 0;
        drop(depth);
        self.staging.lock().unwrap().clear();
        Ok(())
    }

    fn in_transaction(&self) -> bool {
        *self.transaction_depth.lock().unwrap() > 0
    }

    fn storage_path(&self) -> &str {
        &self.config.base_path
    }

    fn count_all(&self) -> Result<StorageStats, StorageError> {
        let file_count = self.files.list_files().map(|v| v.len() as u64).unwrap_or(0);
        let chunk_count = self.chunks.count().unwrap_or(0);
        let node_count = self.graph.count_nodes().unwrap_or(0);
        let edge_count = self.graph.count_edges().unwrap_or(0);
        Ok(StorageStats {
            file_count,
            chunk_count,
            node_count,
            edge_count,
        })
    }
}
