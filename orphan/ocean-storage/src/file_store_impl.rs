use surrealdb::engine::local::{Db, Mem};
use surrealdb::Surreal;
use tokio::runtime::Runtime;

use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::error::StorageError;
use crate::ocean_storage::file_store::{FileMeta, FileStore};

pub struct SurrealFileStore {
    db: Surreal<Db>,
    rt: Runtime,
}

impl SurrealFileStore {
    pub fn new_persistent(config: &StorageConfig) -> Result<Self, StorageError> {
        let path = config.files_path();
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("FileStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = crate::ocean_storage::connect_surrealkv(&path).await
                    .map_err(|e| StorageError::ConnectionFailed("FileStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("FileStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;

        let store = Self { db, rt };
        store.initialize_schema()?;
        Ok(store)
    }

    pub fn new_memory() -> Result<Self, StorageError> {
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("FileStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<Mem>(()).await
                    .map_err(|e| StorageError::ConnectionFailed("FileStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("FileStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;

        let store = Self { db, rt };
        store.initialize_schema()?;
        Ok(store)
    }

    fn initialize_schema(&self) -> Result<(), StorageError> {
        self.rt
            .block_on(async {
                let sql = "DEFINE TABLE IF NOT EXISTS file SCHEMAFULL;
                 DEFINE FIELD IF NOT EXISTS file_id ON TABLE file TYPE string;
                 DEFINE FIELD IF NOT EXISTS path ON TABLE file TYPE string;
                 DEFINE FIELD IF NOT EXISTS hash ON TABLE file TYPE string;
                 DEFINE FIELD IF NOT EXISTS size ON TABLE file TYPE int;
                 DEFINE FIELD IF NOT EXISTS modified ON TABLE file TYPE int;
                 DEFINE FIELD IF NOT EXISTS extension ON TABLE file TYPE string;
                 DEFINE FIELD IF NOT EXISTS last_indexed ON TABLE file TYPE int;
                 DEFINE INDEX IF NOT EXISTS idx_file_id ON TABLE file COLUMNS file_id UNIQUE;
                 DEFINE INDEX IF NOT EXISTS idx_file_path ON TABLE file COLUMNS path UNIQUE;";
                self.db.query(sql).await
                    .map_err(|e| StorageError::SchemaError("FileStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(())
            })
    }
}

impl FileStore for SurrealFileStore {
    fn upsert_file(&self, file: &FileMeta) -> Result<(), StorageError> {
        let fid = file.file_id.clone();
        self.rt.block_on(async {
            self.db
                .query(format!("UPSERT file:`{}` CONTENT $data", fid))
                .bind(("data", file.clone()))
                .await
                .map_err(|e| StorageError::QueryFailed("FileStore::upsert_file".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }

    fn get_file(&self, id: &str) -> Result<Option<FileMeta>, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT * FROM file WHERE file_id = $fid")
                .bind(("fid", id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("FileStore::get_file".into(), e.to_string()))?;
            let records: Vec<FileMeta> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("FileStore::get_file".into(), e.to_string()))?;
            Ok(records.into_iter().next())
        })
    }

    fn get_file_by_path(&self, path: &str) -> Result<Option<FileMeta>, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT * FROM file WHERE path = $path")
                .bind(("path", path.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("FileStore::get_file_by_path".into(), e.to_string()))?;
            let records: Vec<FileMeta> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("FileStore::get_file_by_path".into(), e.to_string()))?;
            Ok(records.into_iter().next())
        })
    }

    fn delete_file(&self, id: &str) -> Result<bool, StorageError> {
        let exists = self.get_file(id)?.is_some();
        if !exists {
            return Ok(false);
        }
        self.rt.block_on(async {
            self.db
                .query("DELETE file WHERE file_id = $fid")
                .bind(("fid", id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("FileStore::delete_file".into(), e.to_string()))?;
            Ok::<(), StorageError>(())
        })?;
        Ok(true)
    }

    fn list_files(&self) -> Result<Vec<FileMeta>, StorageError> {
        self.rt.block_on(async {
            let mut result = self
                .db
                .query("SELECT * FROM file ORDER BY path")
                .await
                .map_err(|e| StorageError::QueryFailed("FileStore::list_files".into(), e.to_string()))?;
            let records: Vec<FileMeta> = result
                .take(0)
                .map_err(|e| StorageError::QueryFailed("FileStore::list_files".into(), e.to_string()))?;
            Ok(records)
        })
    }

    fn needs_update(&self, file: &FileMeta) -> Result<bool, StorageError> {
        let stored = self.get_file_by_path(&file.path)?;
        match stored {
            None => Ok(true),
            Some(s) => Ok(s.hash != file.hash || s.modified != file.modified),
        }
    }
}
