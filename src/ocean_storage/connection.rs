use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;

pub(crate) async fn connect_surrealkv(path: &str) -> Result<Surreal<Db>, surrealdb::Error> {
    match Surreal::new::<SurrealKv>(path).await {
        Ok(db) => Ok(db),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("LOCK") || msg.contains("os error 33") || msg.contains("locked") {
                let _ = std::fs::remove_file(std::path::Path::new(path).join("LOCK"));
                Surreal::new::<SurrealKv>(path).await
            } else {
                Err(e)
            }
        }
    }
}
