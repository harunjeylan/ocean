use surrealdb::engine::local::{Db, Mem};
use surrealdb::types::SurrealValue;
use surrealdb::Surreal;
use tokio::runtime::Runtime;

use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::error::StorageError;
use crate::ocean_storage::state_store::{IndexStatus, StateRecord, StateStore};

pub struct SurrealStateStore {
    db: Surreal<Db>,
    rt: Runtime,
}

#[derive(serde::Serialize, serde::Deserialize, SurrealValue)]
struct StateRow {
    file_id: String,
    hash: String,
    last_indexed: i64,
    status: String,
}

fn status_to_string(s: &IndexStatus) -> &str {
    match s {
        IndexStatus::Pending => "Pending",
        IndexStatus::Indexed => "Indexed",
        IndexStatus::Failed => "Failed",
    }
}

fn string_to_status(s: &str) -> IndexStatus {
    match s {
        "Indexed" => IndexStatus::Indexed,
        "Failed" => IndexStatus::Failed,
        _ => IndexStatus::Pending,
    }
}

impl SurrealStateStore {
    pub fn new_persistent(config: &StorageConfig) -> Result<Self, StorageError> {
        let path = config.state_path();
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("StateStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = crate::ocean_storage::connect_surrealkv(&path).await
                    .map_err(|e| StorageError::ConnectionFailed("StateStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("StateStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;

        let store = Self { db, rt };
        store.initialize_schema()?;
        Ok(store)
    }

    pub fn new_memory() -> Result<Self, StorageError> {
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("StateStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<Mem>(()).await
                    .map_err(|e| StorageError::ConnectionFailed("StateStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("StateStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;

        let store = Self { db, rt };
        store.initialize_schema()?;
        Ok(store)
    }

    fn initialize_schema(&self) -> Result<(), StorageError> {
        self.rt.block_on(async {
            let sql = "DEFINE TABLE IF NOT EXISTS index_state SCHEMAFULL;
             DEFINE FIELD IF NOT EXISTS file_id ON TABLE index_state TYPE string;
             DEFINE FIELD IF NOT EXISTS hash ON TABLE index_state TYPE string;
             DEFINE FIELD IF NOT EXISTS last_indexed ON TABLE index_state TYPE int;
             DEFINE FIELD IF NOT EXISTS status ON TABLE index_state TYPE string;
             DEFINE INDEX IF NOT EXISTS idx_state_file_id ON TABLE index_state COLUMNS file_id UNIQUE;
             DEFINE INDEX IF NOT EXISTS idx_state_status ON TABLE index_state COLUMNS status;";
            self.db.query(sql).await
                .map_err(|e| StorageError::SchemaError("StateStore".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }
}

impl StateStore for SurrealStateStore {
    fn update_state(&self, file_id: &str, hash: &str, status: IndexStatus) -> Result<(), StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let data = serde_json::json!({
            "file_id": file_id,
            "hash": hash,
            "last_indexed": now,
            "status": status_to_string(&status),
        });
        self.rt.block_on(async {
            self.db
                .query(format!("UPSERT index_state:`{}` CONTENT $data", file_id))
                .bind(("data", data))
                .await
                .map_err(|e| StorageError::QueryFailed("StateStore::update_state".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }

    fn get_state(&self, file_id: &str) -> Result<Option<StateRecord>, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT * FROM index_state WHERE file_id = $fid")
                .bind(("fid", file_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("StateStore::get_state".into(), e.to_string()))?;
            let rows: Vec<StateRow> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("StateStore::get_state".into(), e.to_string()))?;
            Ok(rows.into_iter().next().map(|r| StateRecord {
                file_id: r.file_id,
                hash: r.hash,
                last_indexed: r.last_indexed,
                status: string_to_status(&r.status),
            }))
        })
    }

    fn delete_state(&self, file_id: &str) -> Result<bool, StorageError> {
        let exists = self.get_state(file_id)?.is_some();
        if !exists {
            return Ok(false);
        }
        self.rt.block_on(async {
            self.db
                .query("DELETE index_state WHERE file_id = $fid")
                .bind(("fid", file_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("StateStore::delete_state".into(), e.to_string()))?;
            Ok::<(), StorageError>(())
        })?;
        Ok(true)
    }

    fn list_pending(&self) -> Result<Vec<StateRecord>, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT * FROM index_state WHERE status != 'Indexed' ORDER BY file_id")
                .await
                .map_err(|e| StorageError::QueryFailed("StateStore::list_pending".into(), e.to_string()))?;
            let rows: Vec<StateRow> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("StateStore::list_pending".into(), e.to_string()))?;
            Ok(rows
                .into_iter()
                .map(|r| StateRecord {
                    file_id: r.file_id,
                    hash: r.hash,
                    last_indexed: r.last_indexed,
                    status: string_to_status(&r.status),
                })
                .collect())
        })
    }

    fn list_all(&self) -> Result<Vec<StateRecord>, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT * FROM index_state ORDER BY file_id")
                .await
                .map_err(|e| StorageError::QueryFailed("StateStore::list_all".into(), e.to_string()))?;
            let rows: Vec<StateRow> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("StateStore::list_all".into(), e.to_string()))?;
            Ok(rows
                .into_iter()
                .map(|r| StateRecord {
                    file_id: r.file_id,
                    hash: r.hash,
                    last_indexed: r.last_indexed,
                    status: string_to_status(&r.status),
                })
                .collect())
        })
    }
}
