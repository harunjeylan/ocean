use crate::ocean_fs::types::PathMove;
use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::Surreal;
use tokio::runtime::Runtime;

#[derive(Debug, Clone)]
pub enum ResolverError {
    DatabaseError(String),
    IoError(String),
}

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolverError::DatabaseError(msg) => write!(f, "database error: {}", msg),
            ResolverError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ResolverError {}

impl From<surrealdb::Error> for ResolverError {
    fn from(e: surrealdb::Error) -> Self {
        ResolverError::DatabaseError(e.to_string())
    }
}

pub struct PathResolver {
    db: Surreal<Db>,
    rt: Runtime,
}

impl PathResolver {
    pub fn new(db_path: &str) -> Result<Self, ResolverError> {
        let rt = Runtime::new().map_err(|e| ResolverError::IoError(e.to_string()))?;
        let db = rt.block_on(async {
            let db = Surreal::new::<SurrealKv>(db_path).await?;
            db.use_ns("ocean").use_db("ocean").await?;
            Ok::<_, surrealdb::Error>(db)
        })?;
        let resolver = Self { db, rt };
        resolver.initialize()?;
        Ok(resolver)
    }

    pub fn in_memory() -> Result<Self, ResolverError> {
        let rt = Runtime::new().map_err(|e| ResolverError::IoError(e.to_string()))?;
        let db = rt.block_on(async {
            let db = Surreal::new::<Mem>(()).await?;
            db.use_ns("ocean").use_db("ocean").await?;
            Ok::<_, surrealdb::Error>(db)
        })?;
        let resolver = Self { db, rt };
        resolver.initialize()?;
        Ok(resolver)
    }

    fn initialize(&self) -> Result<(), ResolverError> {
        self.rt.block_on(async {
            self.db
                .query(
                    "DEFINE TABLE IF NOT EXISTS file_path SCHEMAFULL;
                     DEFINE FIELD IF NOT EXISTS file_id ON TABLE file_path TYPE string;
                     DEFINE FIELD IF NOT EXISTS old_path ON TABLE file_path TYPE string;
                     DEFINE FIELD IF NOT EXISTS new_path ON TABLE file_path TYPE string;
                     DEFINE FIELD IF NOT EXISTS timestamp ON TABLE file_path TYPE int;
                     DEFINE INDEX IF NOT EXISTS idx_file_id ON TABLE file_path FIELDS file_id;",
                )
                .await?;
            Ok::<_, surrealdb::Error>(())
        })?;
        Ok(())
    }

    pub fn record_move(
        &self,
        file_id: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), ResolverError> {
        let fid = file_id.to_string();
        let old = old_path.to_string();
        let new = new_path.to_string();
        let ts = crate::ocean_fs::types::timestamp_ms() as i64;
        self.rt.block_on(async {
            self.db
                .query("CREATE file_path CONTENT { file_id: $fid, old_path: $old, new_path: $new, timestamp: $ts }")
                .bind(("fid", fid))
                .bind(("old", old))
                .bind(("new", new))
                .bind(("ts", ts))
                .await?;
            Ok::<_, surrealdb::Error>(())
        })?;
        Ok(())
    }

    pub fn resolve_path(&self, file_id: &str) -> Option<String> {
        let fid = file_id.to_string();
        self.rt
            .block_on(async {
                let mut results = self
                    .db
                    .query("SELECT * FROM file_path WHERE file_id = $fid ORDER BY timestamp DESC LIMIT 1")
                    .bind(("fid", fid))
                    .await
                    .ok()?;
                results.take::<Vec<PathMove>>(0).ok()?.into_iter().next().map(|r| r.new_path)
            })
    }

    pub fn get_move_history(&self, file_id: &str) -> Vec<PathMove> {
        let fid = file_id.to_string();
        self.rt
            .block_on(async {
                let mut results = self
                    .db
                    .query("SELECT * FROM file_path WHERE file_id = $fid ORDER BY timestamp ASC")
                    .bind(("fid", fid))
                    .await
                    .ok()?;
                results.take::<Vec<PathMove>>(0).ok()
            })
            .unwrap_or_default()
    }
}
