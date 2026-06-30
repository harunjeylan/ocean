use std::fmt;
use std::time::Instant;

use crate::ocean_chunk::Chunk;
use crate::ocean_vector::embedder::{Embedder, EmbedderError};
use crate::ocean_vector::store::{ChunkRecord, StoreError, VectorStore};

#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub batch_size: usize,
    pub reindex: bool,
    pub model: String,
    pub dimension: usize,
    pub ollama_url: Option<String>,
    pub openai_api_key: Option<String>,
    pub db_path: String,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            reindex: false,
            model: "nomic-embed-text".into(),
            dimension: 768,
            ollama_url: Some("http://localhost:11434".into()),
            openai_api_key: None,
            db_path: "ocean.db".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexReport {
    pub total: usize,
    pub embedded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub duration_ms: u64,
    pub errors: Vec<IndexError>,
    pub graph_nodes: usize,
    pub graph_edges: usize,
}

#[derive(Debug, Clone)]
pub enum IndexError {
    Embedder(EmbedderError),
    Store(StoreError),
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexError::Embedder(e) => write!(f, "embedder error: {}", e),
            IndexError::Store(e) => write!(f, "store error: {}", e),
        }
    }
}

impl std::error::Error for IndexError {}

impl From<EmbedderError> for IndexError {
    fn from(e: EmbedderError) -> Self {
        IndexError::Embedder(e)
    }
}

impl From<StoreError> for IndexError {
    fn from(e: StoreError) -> Self {
        IndexError::Store(e)
    }
}

pub struct IndexPipeline {
    store: VectorStore,
}

impl IndexPipeline {
    pub fn new(store: VectorStore) -> Self {
        Self { store }
    }

    pub fn index_chunks(
        &self,
        chunks: Vec<Chunk>,
        embedder: &dyn Embedder,
        config: &IndexConfig,
    ) -> Result<IndexReport, IndexError> {
        let start = Instant::now();
        let mut embedded = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;
        let mut errors = Vec::new();
        let total = chunks.len();

        for batch in chunks.chunks(config.batch_size) {
            let mut to_embed: Vec<(&Chunk, usize)> = Vec::new();
            let mut skip_indices: Vec<bool> = vec![false; batch.len()];

            for (i, chunk) in batch.iter().enumerate() {
                let content_hash = {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(chunk.content.as_bytes());
                    format!("{:x}", hasher.finalize())
                };

                if !config.reindex {
                    match self.store.chunk_exists(&content_hash, &config.model) {
                        Ok(true) => {
                            skipped += 1;
                            skip_indices[i] = true;
                        }
                        Ok(false) => {
                            to_embed.push((chunk, i));
                        }
                        Err(e) => {
                            failed += 1;
                            errors.push(IndexError::Store(e));
                            skip_indices[i] = true;
                        }
                    }
                } else {
                    to_embed.push((chunk, i));
                }
            }

            if to_embed.is_empty() {
                continue;
            }

            let batch_len = to_embed.len();
            let texts: Vec<&str> = to_embed.iter().map(|(c, _)| c.content.as_str()).collect();

            match embedder.embed_batch(&texts) {
                Ok(embeddings) => {
                    let records: Vec<ChunkRecord> = to_embed
                        .into_iter()
                        .zip(embeddings.into_iter())
                        .map(|((chunk, _), embedding)| {
                            ChunkRecord::from_chunk(chunk, embedding, &config.model)
                        })
                        .collect();

                    let ok = if config.reindex {
                        let mut all_ok = true;
                        for r in &records {
                            if let Err(e) = self.store.upsert_chunk(r.clone()) {
                                errors.push(IndexError::Store(e));
                                all_ok = false;
                            }
                        }
                        all_ok
                    } else {
                        self.store.insert_chunks_batch(records).is_ok()
                    };

                    if ok {
                        embedded += batch_len;
                    } else {
                        failed += batch_len;
                    }
                }
                Err(e) => {
                    failed += batch_len;
                    errors.push(IndexError::Embedder(e));
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(IndexReport {
            total,
            embedded,
            skipped,
            failed,
            duration_ms,
            errors,
            graph_nodes: 0,
            graph_edges: 0,
        })
    }
}
