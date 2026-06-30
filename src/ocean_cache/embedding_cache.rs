use std::collections::HashMap;
use std::sync::Mutex;

use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;
use tokio::runtime::Runtime;

use crate::ocean_cache::lru::LruCache;

struct L2Store {
    db: Surreal<Db>,
    rt: Runtime,
}

impl L2Store {
    fn new(path: &str) -> Option<Self> {
        let rt = Runtime::new().ok()?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<SurrealKv>(path).await.ok()?;
                db.use_ns("ocean").use_db("ocean_cache").await.ok()?;
                db.query(
                    "CREATE TABLE IF NOT EXISTS embedding_cache (
                        content_hash STRING,
                        model STRING,
                        embedding ARRAY,
                        created_at INT
                    )",
                )
                .await
                .ok()?;
                db.query(
                    "CREATE UNIQUE INDEX IF NOT EXISTS idx_embedding_key ON embedding_cache (content_hash, model)",
                )
                .await
                .ok()?;
                Some(db)
            })?;
        Some(Self { db, rt })
    }

    fn get(&self, content_hash: &str, model: &str) -> Option<Vec<f32>> {
        self.rt
            .block_on(async {
                let mut result = self
                    .db
                    .query(
                        "SELECT embedding FROM embedding_cache WHERE content_hash = $hash AND model = $model",
                    )
                    .bind(("hash", content_hash.to_string()))
                    .bind(("model", model.to_string()))
                    .await
                    .ok()?;
                let rows: Vec<serde_json::Value> = result.take(0).ok()?;
                let row = rows.into_iter().next()?;
                let emb: Vec<f32> = serde_json::from_value(row.get("embedding")?.clone()).ok()?;
                Some(emb)
            })
    }

    fn set(&self, content_hash: &str, model: &str, embedding: &[f32]) {
        let emb_json = serde_json::json!(embedding);
        let now = chrono::Utc::now().timestamp() as i64;
        self.rt.block_on(async {
            let _ = self
                .db
                .query(
                    "UPSERT embedding_cache CONTENT {
                        content_hash: $hash,
                        model: $model,
                        embedding: $embedding,
                        created_at: $created_at
                    }",
                )
                .bind(("hash", content_hash.to_string()))
                .bind(("model", model.to_string()))
                .bind(("embedding", emb_json))
                .bind(("created_at", now))
                .await;
        });
    }
}

pub struct EmbeddingCache {
    l1: Mutex<LruCache<(String, String), Vec<f32>>>,
    l2: Option<L2Store>,
}

impl EmbeddingCache {
    pub fn new(l1_capacity: usize, l2_path: Option<&str>) -> Self {
        let l2 = l2_path.and_then(|p| {
            let full_path = format!("{}/embeddings.db", p);
            L2Store::new(&full_path)
        });
        Self {
            l1: Mutex::new(LruCache::new(l1_capacity)),
            l2,
        }
    }

    pub fn get(&self, content_hash: &str, model: &str) -> Option<Vec<f32>> {
        let key = (content_hash.to_string(), model.to_string());

        {
            let mut l1 = self.l1.lock().ok()?;
            if let Some(emb) = l1.get(&key) {
                return Some(emb.clone());
            }
        }

        if let Some(ref l2) = self.l2 {
            if let Some(emb) = l2.get(content_hash, model) {
                let mut l1 = self.l1.lock().ok()?;
                l1.put(key, emb.clone());
                return Some(emb);
            }
        }

        None
    }

    pub fn set(&self, content_hash: &str, model: &str, embedding: Vec<f32>) {
        let key = (content_hash.to_string(), model.to_string());

        {
            let mut l1 = self.l1.lock().ok();
            if let Some(ref mut l1) = l1 {
                l1.put(key, embedding.clone());
            }
        }

        if let Some(ref l2) = self.l2 {
            l2.set(content_hash, model, &embedding);
        }
    }

    pub fn get_batch(&self, keys: &[(&str, &str)]) -> HashMap<(String, String), Vec<f32>> {
        let mut results = HashMap::new();

        for &(content_hash, model) in keys {
            if let Some(emb) = self.get(content_hash, model) {
                results.insert((content_hash.to_string(), model.to_string()), emb);
            }
        }

        results
    }

    pub fn clear_l1(&self) {
        if let Ok(mut l1) = self.l1.lock() {
            l1.clear();
        }
    }
}
