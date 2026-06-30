use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use crate::ocean_cache::EmbeddingCache;
use crate::ocean_chunk::Chunk;
use crate::ocean_storage::chunk_store::ChunkRecord;
use crate::ocean_storage::{ChunkStore, VectorStore};
use crate::ocean_vector::embedder::{Embedder, EmbedderError};

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
    Store(String),
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

pub struct IndexPipeline {
    store: Arc<dyn VectorStore>,
    chunk_store: Arc<dyn ChunkStore>,
    embed_cache: Option<EmbeddingCache>,
}

impl IndexPipeline {
    pub fn new(
        store: Arc<dyn VectorStore>,
        chunk_store: Arc<dyn ChunkStore>,
        embed_cache: Option<EmbeddingCache>,
    ) -> Self {
        Self { store, chunk_store, embed_cache }
    }

    pub fn set_embed_cache(&mut self, cache: EmbeddingCache) {
        self.embed_cache = Some(cache);
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

        fn content_hash(text: &str) -> String {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            format!("{:x}", hasher.finalize())
        }

        for batch in chunks.chunks(config.batch_size) {
            let mut to_embed: Vec<(&Chunk, usize, String)> = Vec::new();
            let mut cached: Vec<(ChunkRecord, Vec<f32>)> = Vec::new();
            let mut skip_indices: Vec<bool> = vec![false; batch.len()];

            for (i, chunk) in batch.iter().enumerate() {
                let hash = content_hash(&chunk.content);

                let mut already_cached = false;
                if let Some(ref cache) = self.embed_cache {
                    if let Some(emb) = cache.get(&hash, &config.model) {
                        let record = ChunkRecord::from_chunk(chunk, emb.clone(), &config.model);
                        cached.push((record, emb));
                        already_cached = true;
                    }
                }

                if already_cached {
                    continue;
                }

                if !config.reindex {
                    match self.chunk_store.chunk_exists(&hash, &config.model) {
                        Ok(true) => {
                            skipped += 1;
                            skip_indices[i] = true;
                        }
                        Ok(false) => {
                            to_embed.push((chunk, i, hash));
                        }
                        Err(e) => {
                            failed += 1;
                            errors.push(IndexError::Store(e.to_string()));
                            skip_indices[i] = true;
                        }
                    }
                } else {
                    to_embed.push((chunk, i, hash));
                }
            }

            for (record, _) in &cached {
                match self.store.insert(record) {
                    Ok(_) => embedded += 1,
                    Err(e) => {
                        failed += 1;
                        errors.push(IndexError::Store(e.to_string()));
                    }
                }
            }

            if to_embed.is_empty() {
                continue;
            }

            let texts: Vec<&str> = to_embed.iter().map(|(c, _, _)| c.content.as_str()).collect();

            match embedder.embed_batch(&texts) {
                Ok(embeddings) => {
                    for ((chunk, _, hash), embedding) in to_embed.into_iter().zip(embeddings.into_iter()) {
                        let record = ChunkRecord::from_chunk(chunk, embedding.clone(), &config.model);
                        match self.store.insert(&record) {
                            Ok(_) => embedded += 1,
                            Err(e) => {
                                failed += 1;
                                errors.push(IndexError::Store(e.to_string()));
                            }
                        }
                        if let Some(ref cache) = self.embed_cache {
                            cache.set(&hash, &config.model, embedding);
                        }
                    }
                }
                Err(e) => {
                    failed += texts.len();
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
