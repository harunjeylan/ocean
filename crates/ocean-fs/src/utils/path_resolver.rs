use crate::shared::types::PathMove;
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem, SurrealKv};
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

        let db = rt
            .block_on(async {
                let db = Surreal::new::<SurrealKv>(db_path).await?;
                db.use_ns("ocean").use_db("ocean").await?;
                Ok::<_, surrealdb::Error>(db)
            })
            .map_err(|e| ResolverError::DatabaseError(e.to_string()))?;

        let resolver = PathResolver { db, rt };
        resolver.initialize()?;
        Ok(resolver)
    }

    pub fn in_memory() -> Result<Self, ResolverError> {
        let rt = Runtime::new().map_err(|e| ResolverError::IoError(e.to_string()))?;

        let db = rt
            .block_on(async {
                let db = Surreal::new::<Mem>(()).await?;
                db.use_ns("ocean").use_db("ocean").await?;
                Ok::<_, surrealdb::Error>(db)
            })
            .map_err(|e| ResolverError::DatabaseError(e.to_string()))?;

        let resolver = PathResolver { db, rt };
        resolver.initialize()?;
        Ok(resolver)
    }

    fn initialize(&self) -> Result<(), ResolverError> {
        self.rt.block_on(async {
            let sql = "
                DEFINE TABLE IF NOT EXISTS path_moves SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS file_id ON TABLE path_moves TYPE string;
                DEFINE FIELD IF NOT EXISTS old_path ON TABLE path_moves TYPE string;
                DEFINE FIELD IF NOT EXISTS new_path ON TABLE path_moves TYPE string;
                DEFINE FIELD IF NOT EXISTS timestamp ON TABLE path_moves TYPE number;
            ";
            self.db
                .query(sql)
                .await
                .map_err(|e| ResolverError::DatabaseError(e.to_string()))?;
            Ok(())
        })
    }

    pub fn record_move(
        &self,
        file_id: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), ResolverError> {
        self.rt.block_on(async {
            let sql = "CREATE path_moves SET file_id = $file_id, old_path = $old_path, new_path = $new_path, timestamp = time::now()";
            self.db
                .query(sql)
                .bind(("file_id", file_id))
                .bind(("old_path", old_path))
                .bind(("new_path", new_path))
                .await?;
            Ok(())
        })
    }

    pub fn resolve_path(&self, file_id: &str) -> Option<String> {
        self.rt.block_on(async {
            let sql = "SELECT new_path FROM path_moves WHERE file_id = $file_id ORDER BY timestamp DESC LIMIT 1";
            let mut result = self.db.query(sql).bind(("file_id", file_id)).await.ok()?;
            result.take::<Option<String>>(0).ok().flatten()
        })
    }

    pub fn get_move_history(&self, file_id: &str) -> Vec<PathMove> {
        self.rt.block_on(async {
            let sql = "SELECT * FROM path_moves WHERE file_id = $file_id ORDER BY timestamp ASC";
            let mut result = self.db.query(sql).bind(("file_id", file_id)).await.unwrap();
            result
                .take::<Option<Vec<PathMove>>>(0)
                .ok()
                .flatten()
                .unwrap_or_default()
        })
    }
}
