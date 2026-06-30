use std::fmt;

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::Surreal;
use tokio::runtime::Runtime;

use crate::ocean_chunk::Chunk;

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
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: i64,
    pub content_hash: String,
    pub created_at: i64,
}

impl ChunkRecord {
    pub fn from_chunk(chunk: &Chunk, embedding: Vec<f32>, model: &str) -> Self {
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
            dimension: embedding.len() as i64,
            embedding,
            model: model.to_string(),
            content_hash,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StoreError {
    ConnectionFailed(String),
    QueryFailed(String),
    RecordNotFound(String),
    SchemaError(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::ConnectionFailed(msg) => write!(f, "store connection failed: {}", msg),
            StoreError::QueryFailed(msg) => write!(f, "query failed: {}", msg),
            StoreError::RecordNotFound(msg) => write!(f, "record not found: {}", msg),
            StoreError::SchemaError(msg) => write!(f, "schema error: {}", msg),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<surrealdb::Error> for StoreError {
    fn from(e: surrealdb::Error) -> Self {
        StoreError::QueryFailed(e.to_string())
    }
}

pub struct VectorStore {
    db: Surreal<Db>,
    rt: Runtime,
}

impl VectorStore {
    pub fn new_memory() -> Result<Self, StoreError> {
        let rt = Runtime::new().map_err(|e| StoreError::ConnectionFailed(e.to_string()))?;
        let db = rt.block_on(async {
            let db = Surreal::new::<Mem>(()).await?;
            db.use_ns("ocean").use_db("ocean").await?;
            Ok::<_, surrealdb::Error>(db)
        })?;
        Ok(Self { db, rt })
    }

    pub fn new_persistent(path: &str) -> Result<Self, StoreError> {
        let rt = Runtime::new().map_err(|e| StoreError::ConnectionFailed(e.to_string()))?;
        let db = rt.block_on(async {
            let db = Surreal::new::<SurrealKv>(path).await?;
            db.use_ns("ocean").use_db("ocean").await?;
            Ok::<_, surrealdb::Error>(db)
        })?;
        Ok(Self { db, rt })
    }

    pub fn initialize_schema(&self, dimension: usize) -> Result<(), StoreError> {
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
            self.db.query(surql).await?;
            Ok::<_, surrealdb::Error>(())
        })?;
        Ok(())
    }

    pub fn insert_chunk(&self, chunk: ChunkRecord) -> Result<(), StoreError> {
        self.rt.block_on(async {
            let _: Option<ChunkRecord> = self
                .db
                .create(("chunk", &chunk.chunk_id))
                .content(chunk)
                .await?;
            Ok::<_, surrealdb::Error>(())
        })?;
        Ok(())
    }

    pub fn insert_chunks_batch(&self, records: Vec<ChunkRecord>) -> Result<(), StoreError> {
        self.rt.block_on(async {
            for record in records {
                let _: Option<ChunkRecord> = self
                    .db
                    .create(("chunk", &record.chunk_id))
                    .content(record)
                    .await?;
            }
            Ok::<_, surrealdb::Error>(())
        })?;
        Ok(())
    }

    pub fn upsert_chunk(&self, chunk: ChunkRecord) -> Result<(), StoreError> {
        let chunk_id = chunk.chunk_id.clone();
        self.rt.block_on(async {
            self.db
                .query(format!("UPSERT chunk:`{}` CONTENT $data", chunk_id))
                .bind(("data", chunk))
                .await?;
            Ok::<_, surrealdb::Error>(())
        })?;
        Ok(())
    }

    pub fn get_chunk(&self, chunk_id: &str) -> Result<Option<ChunkRecord>, StoreError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT * FROM chunk WHERE chunk_id = $id")
                .bind(("id", chunk_id.to_string()))
                .await?;
            let record: Option<ChunkRecord> = results.take(0)?;
            Ok(record)
        })
    }

    pub fn delete_chunks_by_file(&self, file_id: &str) -> Result<u64, StoreError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("DELETE chunk WHERE file_id = $fid RETURN BEFORE")
                .bind(("fid", file_id.to_string()))
                .await?;
            let deleted: Vec<ChunkRecord> = results.take(0)?;
            Ok(deleted.len() as u64)
        })
    }

    pub fn count(&self) -> Result<u64, StoreError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT count() AS total FROM chunk GROUP BY count")
                .await?;
            #[derive(serde::Deserialize)]
            struct CountResult {
                total: u64,
            }
            let rows: Vec<CountResult> = results.take(0)?;
            Ok(rows.first().map(|r| r.total).unwrap_or(0))
        })
    }

    pub fn chunk_exists(&self, content_hash: &str, model: &str) -> Result<bool, StoreError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT count() AS cnt FROM chunk WHERE content_hash = $hash AND model = $model GROUP BY count")
                .bind(("hash", content_hash.to_string()))
                .bind(("model", model.to_string()))
                .await?;
            #[derive(serde::Deserialize)]
            struct ExistsResult {
                cnt: u64,
            }
            let rows: Vec<ExistsResult> = results.take(0)?;
            Ok(rows.first().map(|r| r.cnt > 0).unwrap_or(false))
        })
    }

    pub fn vector_search(
        &self,
        query_vec: &[f32],
        top_k: usize,
        extra_where: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, StoreError> {
        let query_vec_json = serde_json::json!(query_vec);
        let top_k = top_k.min(100).max(1);

        self.rt.block_on(async {
            let mut q = if let Some(cond) = extra_where {
                self.db
                    .query(format!(
                        "SELECT chunk_id, file_id, content, heading, block_type, \
                         vector::distance::cosine(embedding, $query_vec) AS score \
                         FROM chunk WHERE {} AND embedding <|{}|> $query_vec \
                         ORDER BY score ASC LIMIT {}",
                        cond, top_k, top_k
                    ))
            } else {
                self.db
                    .query(format!(
                        "SELECT chunk_id, file_id, content, heading, block_type, \
                         vector::distance::cosine(embedding, $query_vec) AS score \
                         FROM chunk WHERE embedding <|{}|> $query_vec \
                         ORDER BY score ASC LIMIT {}",
                        top_k, top_k
                    ))
            };
            q = q.bind(("query_vec", query_vec_json));
            let mut results = q.await?;
            let rows: Vec<serde_json::Value> = results.take(0)?;
            Ok(rows)
        })
    }

    pub fn fts_search(
        &self,
        query: &str,
        top_k: usize,
        extra_where: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, StoreError> {
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
            let mut results = q.await?;
            let rows: Vec<serde_json::Value> = results.take(0)?;
            Ok(rows)
        })
    }
}
