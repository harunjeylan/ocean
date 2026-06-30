use crate::ocean_query::error::QueryError;
use crate::ocean_query::types::{ContextChunk, ContextWindow, RankedChunk};
use crate::ocean_vector::store::VectorStore;

pub struct ContextWindowBuilder {
    store: VectorStore,
}

impl ContextWindowBuilder {
    pub fn new(store: VectorStore) -> Self {
        Self { store }
    }

    pub fn build(
        &self,
        anchor: &RankedChunk,
        context_chunks: usize,
    ) -> Result<ContextWindow, QueryError> {
        let n = context_chunks.max(1).min(10);
        let half = n / 2;

        let anchor_record = self
            .store
            .get_chunk(&anchor.chunk_id)
            .map_err(|e| QueryError::ContextBuildFailed(e.to_string()))?;

        let anchor_content = anchor_record
            .as_ref()
            .map(|r| r.content.clone())
            .unwrap_or_else(|| anchor.content.clone());

        let anchor_heading = anchor_record
            .as_ref()
            .and_then(|r| r.heading.clone())
            .or_else(|| anchor.heading.clone());

        let file_id = anchor.file_id.clone();

        let all_chunks = self
            .store
            .get_chunks_by_file_and_heading(&file_id, anchor_heading.as_deref())
            .map_err(|e| QueryError::ContextBuildFailed(e.to_string()))?;

        let anchor_pos = all_chunks
            .iter()
            .position(|c| c.chunk_id == anchor.chunk_id);

        let anchor_pos = match anchor_pos {
            Some(p) => p,
            None => {
                let tokens = anchor_content.len() / 4;
                let single = ContextChunk {
                    chunk_id: anchor.chunk_id.clone(),
                    content: anchor_content,
                    heading: anchor_heading.clone(),
                    score: anchor.score,
                    distance_from_anchor: 0,
                };
                return Ok(ContextWindow {
                    anchor_chunk_id: anchor.chunk_id.clone(),
                    chunks: vec![single],
                    total_tokens: tokens,
                });
            }
        };

        let before_count = half.min(anchor_pos);
        let after_count = half.min(all_chunks.len().saturating_sub(anchor_pos + 1));

        let start = anchor_pos - before_count;
        let end = anchor_pos + after_count + 1;

        let mut chunks = Vec::new();
        let mut total_tokens = 0usize;

        for (i, record) in all_chunks[start..end].iter().enumerate() {
            let distance = (i as i32) - (before_count as i32);
            let tokens_est = record.content.len() / 4;
            total_tokens += tokens_est;

            chunks.push(ContextChunk {
                chunk_id: record.chunk_id.clone(),
                content: record.content.clone(),
                heading: record.heading.clone(),
                score: 0.0,
                distance_from_anchor: distance,
            });
        }

        Ok(ContextWindow {
            anchor_chunk_id: anchor.chunk_id.clone(),
            chunks,
            total_tokens,
        })
    }
}
