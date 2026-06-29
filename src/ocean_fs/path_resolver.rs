use crate::ocean_fs::path_entities;
use crate::ocean_fs::types::PathMove;
use sea_orm::entity::*;
use sea_orm::query::*;
use sea_orm::sea_query::SqliteQueryBuilder;
use sea_orm::{Database, DatabaseConnection, DatabaseBackend, Schema, Statement};
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

impl From<sea_orm::DbErr> for ResolverError {
    fn from(e: sea_orm::DbErr) -> Self {
        ResolverError::DatabaseError(e.to_string())
    }
}

pub struct PathResolver {
    db: DatabaseConnection,
    rt: Runtime,
}

impl PathResolver {
    pub fn new(db_path: &str) -> Result<Self, ResolverError> {
        let url = format!("sqlite://{}?mode=rwc", db_path);
        let rt = Runtime::new().map_err(|e| ResolverError::IoError(e.to_string()))?;
        let db = rt
            .block_on(Database::connect(&url))
            .map_err(|e| ResolverError::DatabaseError(e.to_string()))?;
        let resolver = Self { db, rt };
        resolver.initialize()?;
        Ok(resolver)
    }

    pub fn in_memory() -> Result<Self, ResolverError> {
        let rt = Runtime::new().map_err(|e| ResolverError::IoError(e.to_string()))?;
        let db = rt
            .block_on(Database::connect("sqlite::memory:"))
            .map_err(|e| ResolverError::DatabaseError(e.to_string()))?;
        let resolver = Self { db, rt };
        resolver.initialize()?;
        Ok(resolver)
    }

    fn initialize(&self) -> Result<(), ResolverError> {
        let schema = Schema::new(DatabaseBackend::Sqlite);
        let mut create_stmt = schema.create_table_from_entity(path_entities::Entity);
        create_stmt.if_not_exists();
        let sql = create_stmt.to_string(SqliteQueryBuilder);
        self.rt
            .block_on(self.db.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                sql,
            )))?;
        Ok(())
    }

    pub fn record_move(
        &self,
        file_id: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), ResolverError> {
        let model = path_entities::ActiveModel {
            file_id: Set(file_id.to_string()),
            old_path: Set(old_path.to_string()),
            new_path: Set(new_path.to_string()),
            timestamp: Set(crate::ocean_fs::types::timestamp_ms() as i64),
            ..Default::default()
        };
        self.rt.block_on(model.insert(&self.db))?;
        Ok(())
    }

    pub fn resolve_path(&self, file_id: &str) -> Option<String> {
        let result: Option<path_entities::Model> = self
            .rt
            .block_on(
                path_entities::Entity::find()
                    .filter(path_entities::Column::FileId.eq(file_id))
                    .order_by(path_entities::Column::Timestamp, Order::Desc)
                    .order_by(path_entities::Column::Id, Order::Desc)
                    .one(&self.db),
            )
            .ok()?;
        result.map(|m| m.new_path)
    }

    pub fn get_move_history(&self, file_id: &str) -> Vec<PathMove> {
        let results: Vec<path_entities::Model> = self
            .rt
            .block_on(
                path_entities::Entity::find()
                    .filter(path_entities::Column::FileId.eq(file_id))
                    .order_by(path_entities::Column::Timestamp, Order::Asc)
                    .all(&self.db),
            )
            .unwrap_or_default();

        results
            .into_iter()
            .map(|m| PathMove {
                file_id: m.file_id,
                old_path: m.old_path,
                new_path: m.new_path,
                timestamp: m.timestamp as u64,
            })
            .collect()
    }
}
