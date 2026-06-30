use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::types::SurrealValue;
use surrealdb::Surreal;
use tokio::runtime::Runtime;

use crate::ocean_storage::chunk_store::{ChunkRecord, ChunkStore};
use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::error::StorageError;
use crate::ocean_storage::vector_store::VectorStore;

pub struct SurrealVectorStore {
    db: Surreal<Db>,
    rt: Runtime,
}

impl SurrealVectorStore {
    pub fn new_persistent(config: &StorageConfig) -> Result<Self, StorageError> {
        let path = config.vectors_path();
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("VectorStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<SurrealKv>(&path).await
                    .map_err(|e| StorageError::ConnectionFailed("VectorStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("VectorStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;
        Ok(Self { db, rt })
    }

    pub fn new_persistent_at(path: &str, _config: &StorageConfig) -> Result<Self, StorageError> {
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("VectorStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<SurrealKv>(path).await
                    .map_err(|e| StorageError::ConnectionFailed("VectorStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("VectorStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;
        Ok(Self { db, rt })
    }

    pub fn new_memory(_config: &StorageConfig) -> Result<Self, StorageError> {
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("VectorStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<Mem>(()).await
                    .map_err(|e| StorageError::ConnectionFailed("VectorStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("VectorStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;
        Ok(Self { db, rt })
    }
}

impl VectorStore for SurrealVectorStore {
    fn insert(&self, record: &ChunkRecord) -> Result<(), StorageError> {
        let cid = record.chunk_id.clone();
        self.rt.block_on(async {
            self.db
                .query(format!("UPSERT chunk:`{}` CONTENT $data", cid))
                .bind(("data", record.clone()))
                .await
                .map_err(|e| StorageError::QueryFailed("VectorStore::insert".into(), e.to_string()))?;
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
                .map_err(|e| StorageError::QueryFailed("VectorStore::get_chunk".into(), e.to_string()))?;
            let records: Vec<ChunkRecord> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("VectorStore::get_chunk".into(), e.to_string()))?;
            Ok(records.into_iter().next())
        })
    }

    fn vector_search(
        &self,
        query_vec: &[f32],
        top_k: usize,
        extra_where: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, StorageError> {
        let query_vec_json = serde_json::json!(query_vec);
        let top_k = top_k.min(100).max(1);

        self.rt.block_on(async {
            let mut q = if let Some(cond) = extra_where {
                self.db
                    .query(format!(
                        "SELECT chunk_id, file_id, content, heading, block_type, \
                         vector::similarity::cosine(embedding, $query_vec) AS score \
                         FROM chunk WHERE {} \
                         ORDER BY score DESC LIMIT {}",
                        cond, top_k
                    ))
            } else {
                self.db
                    .query(format!(
                        "SELECT chunk_id, file_id, content, heading, block_type, \
                         vector::similarity::cosine(embedding, $query_vec) AS score \
                         FROM chunk \
                         ORDER BY score DESC LIMIT {}",
                        top_k
                    ))
            };
            q = q.bind(("query_vec", query_vec_json));
            let mut results = q.await
                .map_err(|e| StorageError::QueryFailed("VectorStore::vector_search".into(), e.to_string()))?;
            let rows: Vec<serde_json::Value> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("VectorStore::vector_search".into(), e.to_string()))?;
            Ok(rows)
        })
    }

    fn fts_search(
        &self,
        query: &str,
        top_k: usize,
        extra_where: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, StorageError> {
        let top_k = top_k.min(100).max(1);

        self.rt.block_on(async {
            let mut q = if let Some(cond) = extra_where {
                self.db
                    .query(format!(
                        "SELECT chunk_id, file_id, content, heading, block_type, \
                         search::score(0) AS fts_score \
                         FROM chunk WHERE {} AND content @@ $query \
                         ORDER BY fts_score DESC LIMIT {}",
                        cond, top_k
                    ))
            } else {
                self.db
                    .query(format!(
                        "SELECT chunk_id, file_id, content, heading, block_type, \
                         search::score(0) AS fts_score \
                         FROM chunk WHERE content @@ $query \
                         ORDER BY fts_score DESC LIMIT {}",
                        top_k
                    ))
            };
            q = q.bind(("query", query.to_string()));
            let mut results = q.await
                .map_err(|e| StorageError::QueryFailed("VectorStore::fts_search".into(), e.to_string()))?;
            let rows: Vec<serde_json::Value> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("VectorStore::fts_search".into(), e.to_string()))?;
            Ok(rows)
        })
    }

    fn delete_by_file(&self, file_id: &str) -> Result<u64, StorageError> {
        let before = VectorStore::count(self)?;
        self.rt.block_on(async {
            self.db
                .query("DELETE chunk WHERE file_id = $fid")
                .bind(("fid", file_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("VectorStore::delete_by_file".into(), e.to_string()))?;
            Ok::<(), StorageError>(())
        })?;
        let after = VectorStore::count(self)?;
        Ok(before.saturating_sub(after))
    }

    fn count(&self) -> Result<u64, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT count() AS total FROM chunk GROUP BY count")
                .await
                .map_err(|e| StorageError::QueryFailed("VectorStore::count".into(), e.to_string()))?;
            #[derive(serde::Deserialize, SurrealValue)]
            struct CountResult { total: u64 }
            let rows: Vec<CountResult> = result.take(0)
                .map_err(|e| StorageError::QueryFailed("VectorStore::count".into(), e.to_string()))?;
            Ok(rows.first().map(|r| r.total).unwrap_or(0))
        })
    }

    fn initialize_schema(&self, dimension: usize) -> Result<(), StorageError> {
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
                 DEFINE INDEX IF NOT EXISTS idx_embedding ON TABLE chunk FIELDS embedding HNSW DIMENSION {} DIST COSINE;
                 DEFINE INDEX IF NOT EXISTS idx_file_id ON TABLE chunk FIELDS file_id;
                 DEFINE INDEX IF NOT EXISTS idx_chunk_id ON TABLE chunk FIELDS chunk_id;",
                dimension
            );
            self.db.query(surql).await
                .map_err(|e| StorageError::SchemaError("VectorStore".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }
}

impl ChunkStore for SurrealVectorStore {
    fn insert_chunk(&self, chunk: &ChunkRecord) -> Result<(), StorageError> {
        self.insert(chunk)
    }

    fn upsert_chunk(&self, chunk: &ChunkRecord) -> Result<(), StorageError> {
        self.insert(chunk)
    }

    fn get_chunk(&self, chunk_id: &str) -> Result<Option<ChunkRecord>, StorageError> {
        <Self as VectorStore>::get_chunk(self, chunk_id)
    }

    fn delete_chunks_by_file(&self, file_id: &str) -> Result<u64, StorageError> {
        self.delete_by_file(file_id)
    }

    fn count(&self) -> Result<u64, StorageError> {
        <Self as VectorStore>::count(self)
    }

    fn chunk_exists(&self, content_hash: &str, model: &str) -> Result<bool, StorageError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT count() AS cnt FROM chunk WHERE content_hash = $hash AND model = $model GROUP BY count")
                .bind(("hash", content_hash.to_string()))
                .bind(("model", model.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("VectorStore::chunk_exists".into(), e.to_string()))?;
            #[derive(serde::Deserialize, SurrealValue)]
            struct ExistsResult { cnt: u64 }
            let rows: Vec<ExistsResult> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("VectorStore::chunk_exists".into(), e.to_string()))?;
            Ok(rows.first().map(|r| r.cnt > 0).unwrap_or(false))
        })
    }

    fn get_by_file_and_heading(
        &self,
        file_id: &str,
        heading: Option<&str>,
    ) -> Result<Vec<ChunkRecord>, StorageError> {
        self.rt.block_on(async {
            let query = if let Some(h) = heading {
                self.db
                    .query("SELECT * FROM chunk WHERE file_id = $fid AND heading = $h ORDER BY chunk_id ASC")
                    .bind(("fid", file_id.to_string()))
                    .bind(("h", h.to_string()))
            } else {
                self.db
                    .query("SELECT * FROM chunk WHERE file_id = $fid AND heading IS NONE ORDER BY chunk_id ASC")
                    .bind(("fid", file_id.to_string()))
            };
            let mut results = query.await
                .map_err(|e| StorageError::QueryFailed("VectorStore::get_by_file_and_heading".into(), e.to_string()))?;
            let records: Vec<ChunkRecord> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("VectorStore::get_by_file_and_heading".into(), e.to_string()))?;
            Ok(records)
        })
    }
}
