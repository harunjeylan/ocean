use surrealdb::engine::local::{Db, Mem};
use surrealdb::types::SurrealValue;
use surrealdb::Surreal;
use tokio::runtime::Runtime;

use crate::ocean_storage::chunk_store::{ChunkRecord, ChunkStore};
use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::error::StorageError;

pub struct SurrealChunkStore {
    db: Surreal<Db>,
    rt: Runtime,
}

impl SurrealChunkStore {
    pub fn new_persistent_at(path: &str) -> Result<Self, StorageError> {
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("ChunkStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = crate::ocean_storage::connect_surrealkv(path).await
                    .map_err(|e| StorageError::ConnectionFailed("ChunkStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("ChunkStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;

        let store = Self { db, rt };
        store.initialize_schema(768)?;
        Ok(store)
    }

    pub fn new_persistent(config: &StorageConfig) -> Result<Self, StorageError> {
        let path = config.chunks_path();
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("ChunkStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = crate::ocean_storage::connect_surrealkv(&path).await
                    .map_err(|e| StorageError::ConnectionFailed("ChunkStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("ChunkStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;

        let store = Self { db, rt };
        store.initialize_schema(768)?;
        Ok(store)
    }

    pub fn new_memory() -> Result<Self, StorageError> {
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("ChunkStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<Mem>(()).await
                    .map_err(|e| StorageError::ConnectionFailed("ChunkStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("ChunkStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;

        let store = Self { db, rt };
        store.initialize_schema(768)?;
        Ok(store)
    }

    fn initialize_schema(&self, _dimension: usize) -> Result<(), StorageError> {
        self.rt.block_on(async {
            let surql = format!(
                "DEFINE TABLE IF NOT EXISTS chunk SCHEMAFULL;
             DEFINE FIELD IF NOT EXISTS chunk_id ON TABLE chunk TYPE string;
             DEFINE FIELD IF NOT EXISTS file_id ON TABLE chunk TYPE string;
             DEFINE FIELD IF NOT EXISTS content ON TABLE chunk TYPE string;
             DEFINE FIELD IF NOT EXISTS heading ON TABLE chunk TYPE option<string>;
             DEFINE FIELD IF NOT EXISTS page ON TABLE chunk TYPE option<int>;
             DEFINE FIELD IF NOT EXISTS slide ON TABLE chunk TYPE option<int>;
             DEFINE FIELD IF NOT EXISTS sheet ON TABLE chunk TYPE option<string>;
             DEFINE FIELD IF NOT EXISTS block_type ON TABLE chunk TYPE string;
             DEFINE FIELD IF NOT EXISTS embedding ON TABLE chunk TYPE array<float>;
             DEFINE FIELD IF NOT EXISTS model ON TABLE chunk TYPE string;
             DEFINE FIELD IF NOT EXISTS dimension ON TABLE chunk TYPE int;
             DEFINE FIELD IF NOT EXISTS content_hash ON TABLE chunk TYPE string;
             DEFINE FIELD IF NOT EXISTS created_at ON TABLE chunk TYPE int;
             DEFINE INDEX IF NOT EXISTS idx_chunk_id ON TABLE chunk COLUMNS chunk_id UNIQUE;
             DEFINE INDEX IF NOT EXISTS idx_chunk_file ON TABLE chunk COLUMNS file_id;
             DEFINE INDEX IF NOT EXISTS idx_chunk_hash ON TABLE chunk COLUMNS content_hash;"
            );
            self.db.query(surql).await
                .map_err(|e| StorageError::SchemaError("ChunkStore".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }
}

impl ChunkStore for SurrealChunkStore {
    fn insert_chunk(&self, chunk: &ChunkRecord) -> Result<(), StorageError> {
        let cid = chunk.chunk_id.clone();
        self.rt.block_on(async {
            self.db
                .query(format!("CREATE chunk:`{}` CONTENT $data", cid))
                .bind(("data", chunk.clone()))
                .await
                .map_err(|e| StorageError::QueryFailed("ChunkStore::insert_chunk".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }

    fn upsert_chunk(&self, chunk: &ChunkRecord) -> Result<(), StorageError> {
        let cid = chunk.chunk_id.clone();
        self.rt.block_on(async {
            self.db
                .query(format!("UPSERT chunk:`{}` CONTENT $data", cid))
                .bind(("data", chunk.clone()))
                .await
                .map_err(|e| StorageError::QueryFailed("ChunkStore::upsert_chunk".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }

    fn get_chunk(&self, chunk_id: &str) -> Result<Option<ChunkRecord>, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT * FROM chunk WHERE chunk_id = $id")
                .bind(("id", chunk_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("ChunkStore::get_chunk".into(), e.to_string()))?;
            let records: Vec<ChunkRecord> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("ChunkStore::get_chunk".into(), e.to_string()))?;
            Ok(records.into_iter().next())
        })
    }

    fn delete_chunks_by_file(&self, file_id: &str) -> Result<u64, StorageError> {
        let before = self.count()?;
        self.rt.block_on(async {
            self.db
                .query("DELETE chunk WHERE file_id = $fid")
                .bind(("fid", file_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("ChunkStore::delete_chunks_by_file".into(), e.to_string()))?;
            Ok::<(), StorageError>(())
        })?;
        let after = self.count()?;
        Ok(before.saturating_sub(after))
    }

    fn count(&self) -> Result<u64, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT count() AS total FROM chunk GROUP BY count")
                .await
                .map_err(|e| StorageError::QueryFailed("ChunkStore::count".into(), e.to_string()))?;
            #[derive(serde::Deserialize, SurrealValue)]
            struct CountResult {
                total: u64,
            }
            let rows: Vec<CountResult> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("ChunkStore::count".into(), e.to_string()))?;
            Ok(rows.first().map(|r| r.total).unwrap_or(0))
        })
    }

    fn chunk_exists(&self, content_hash: &str, _model: &str) -> Result<bool, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT count() AS cnt FROM chunk WHERE content_hash = $hash GROUP BY count")
                .bind(("hash", content_hash.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("ChunkStore::chunk_exists".into(), e.to_string()))?;
            #[derive(serde::Deserialize, SurrealValue)]
            struct ExistsResult {
                cnt: u64,
            }
            let rows: Vec<ExistsResult> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("ChunkStore::chunk_exists".into(), e.to_string()))?;
            Ok(rows.first().map(|r| r.cnt > 0).unwrap_or(false))
        })
    }

    fn get_by_file_and_heading(
        &self,
        file_id: &str,
        heading: Option<&str>,
    ) -> Result<Vec<ChunkRecord>, StorageError> {
        self.rt.block_on(async {
            let result = if let Some(h) = heading {
                self.db
                    .query("SELECT * FROM chunk WHERE file_id = $fid AND heading = $h ORDER BY chunk_id ASC")
                    .bind(("fid", file_id.to_string()))
                    .bind(("h", h.to_string()))
            } else {
                self.db
                    .query("SELECT * FROM chunk WHERE file_id = $fid AND heading IS NONE ORDER BY chunk_id ASC")
                    .bind(("fid", file_id.to_string()))
            };
            let mut result = result
                .await
                .map_err(|e| StorageError::QueryFailed("ChunkStore::get_by_file_and_heading".into(), e.to_string()))?;
            let records: Vec<ChunkRecord> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("ChunkStore::get_by_file_and_heading".into(), e.to_string()))?;
            Ok(records)
        })
    }
}
