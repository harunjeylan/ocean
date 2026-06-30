use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::ocean_storage::VectorStore as OceanVectorStore;
use crate::ocean_vector::embedder::{Embedder, EmbedderError};

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk_id: String,
    pub file_id: String,
    pub content: String,
    pub heading: Option<String>,
    pub score: f32,
    pub block_type: String,
    pub vector_score: Option<f32>,
    pub fts_score: Option<f32>,
    pub graph_score: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    pub file_id: Option<String>,
    pub heading_prefix: Option<String>,
    pub block_type: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
}

impl SearchFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file_id(mut self, id: &str) -> Self {
        self.file_id = Some(id.to_string());
        self
    }

    pub fn with_heading(mut self, prefix: &str) -> Self {
        self.heading_prefix = Some(prefix.to_string());
        self
    }

    pub fn with_block_type(mut self, type_name: &str) -> Self {
        self.block_type = Some(type_name.to_string());
        self
    }

    pub fn with_created_range(
        mut self,
        after: chrono::DateTime<chrono::Utc>,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.created_after = Some(after);
        self.created_before = Some(before);
        self
    }

    pub fn build_where_clause(&self) -> Option<String> {
        let mut conditions: Vec<String> = Vec::new();
        if let Some(ref fid) = self.file_id {
            conditions.push(format!("file_id = '{}'", fid.replace('\'', "''")));
        }
        if let Some(ref prefix) = self.heading_prefix {
            conditions.push(format!(
                "heading STARTSWITH '{}'",
                prefix.replace('\'', "''")
            ));
        }
        if let Some(ref bt) = self.block_type {
            conditions.push(format!("block_type = '{}'", bt.replace('\'', "''")));
        }
        if let Some(ref after) = self.created_after {
            conditions.push(format!("created_at > '{}'", after.to_rfc3339()));
        }
        if let Some(ref before) = self.created_before {
            conditions.push(format!("created_at < '{}'", before.to_rfc3339()));
        }
        if conditions.is_empty() {
            None
        } else {
            Some(conditions.join(" AND "))
        }
    }
}

#[derive(Debug, Clone)]
pub enum SearchError {
    Embedder(EmbedderError),
    Store(String),
    NoResults(String),
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchError::Embedder(e) => write!(f, "embedder error: {}", e),
            SearchError::Store(msg) => write!(f, "store error: {}", msg),
            SearchError::NoResults(msg) => write!(f, "no results: {}", msg),
        }
    }
}

impl std::error::Error for SearchError {}

impl From<EmbedderError> for SearchError {
    fn from(e: EmbedderError) -> Self {
        SearchError::Embedder(e)
    }
}

impl From<crate::ocean_storage::error::StorageError> for SearchError {
    fn from(e: crate::ocean_storage::error::StorageError) -> Self {
        SearchError::Store(e.to_string())
    }
}

pub struct SearchEngine {
    store: Arc<dyn OceanVectorStore>,
}

impl SearchEngine {
    pub fn new(store: Arc<dyn OceanVectorStore>) -> Self {
        Self { store }
    }

    pub fn store_ref(&self) -> &dyn OceanVectorStore {
        &*self.store
    }

    pub fn search(
        &self,
        query: &str,
        embedder: &dyn Embedder,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let query_vec = embedder.embed(query)?;
        let rows = self.store.vector_search(&query_vec, top_k, None)?;
        parse_search_results(rows, true, false)
    }

    pub fn hybrid_search(
        &self,
        query: &str,
        embedder: &dyn Embedder,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let query_vec = embedder.embed(query)?;

        let vector_results = self.store.vector_search(&query_vec, top_k, None)?;
        let fts_results = self.store.fts_search(query, top_k, None)?;

        let vector_parsed = parse_search_results_raw(&vector_results, true, false)?;
        let fts_parsed = parse_search_results_raw(&fts_results, false, true)?;

        Ok(fuse_rrf(vector_parsed, fts_parsed, 60.0, top_k))
    }

    pub fn filtered_search(
        &self,
        query: &str,
        embedder: &dyn Embedder,
        filter: &SearchFilter,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let query_vec = embedder.embed(query)?;
        let where_clause = filter.build_where_clause();
        let rows = self.store.vector_search(&query_vec, top_k, where_clause.as_deref())?;
        parse_search_results(rows, true, false)
    }

    pub fn hybrid_filtered_search(
        &self,
        query: &str,
        embedder: &dyn Embedder,
        filter: &SearchFilter,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let query_vec = embedder.embed(query)?;
        let where_clause = filter.build_where_clause();

        let vector_results =
            self.store.vector_search(&query_vec, top_k, where_clause.as_deref())?;
        let fts_results = self.store.fts_search(query, top_k, where_clause.as_deref())?;

        let vector_parsed = parse_search_results_raw(&vector_results, true, false)?;
        let fts_parsed = parse_search_results_raw(&fts_results, false, true)?;

        Ok(fuse_rrf(vector_parsed, fts_parsed, 60.0, top_k))
    }

}

fn parse_search_results(
    rows: Vec<serde_json::Value>,
    has_vector: bool,
    has_fts: bool,
) -> Result<Vec<SearchResult>, SearchError> {
    parse_search_results_raw(&rows, has_vector, has_fts)
}

pub(crate) fn parse_search_results_raw(
    rows: &[serde_json::Value],
    has_vector: bool,
    has_fts: bool,
) -> Result<Vec<SearchResult>, SearchError> {
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        let chunk_id = row
            .get("chunk_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let file_id = row
            .get("file_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let content = row
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let heading = row
            .get("heading")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let block_type = row
            .get("block_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let vector_score = if has_vector {
            row.get("score").and_then(|v| v.as_f64()).map(|s| s as f32)
        } else {
            None
        };

        let fts_score = if has_fts {
            row.get("fts_score")
                .and_then(|v| v.as_f64())
                .map(|s| s as f32)
        } else {
            None
        };

        let score = vector_score.or(fts_score).unwrap_or(0.0);

        results.push(SearchResult {
            chunk_id,
            file_id,
            content,
            heading,
            score,
            block_type,
            vector_score,
            fts_score,
            graph_score: None,
        });
    }
    Ok(results)
}

pub(crate) fn fuse_rrf(
    vector_results: Vec<SearchResult>,
    fts_results: Vec<SearchResult>,
    k: f32,
    top_k: usize,
) -> Vec<SearchResult> {
    let mut rrf_scores: HashMap<String, (SearchResult, f32)> = HashMap::new();

    for (rank, result) in vector_results.iter().enumerate() {
        let score = 1.0 / (k + rank as f32 + 1.0);
        let key = result.chunk_id.clone();
        rrf_scores
            .entry(key)
            .and_modify(|(existing, acc)| {
                *acc += score;
                existing.vector_score = result.vector_score;
            })
            .or_insert_with(|| {
                let mut r = result.clone();
                r.score = score;
                (r, score)
            });
    }

    for (rank, result) in fts_results.iter().enumerate() {
        let score = 1.0 / (k + rank as f32 + 1.0);
        let key = result.chunk_id.clone();
        rrf_scores
            .entry(key)
            .and_modify(|(existing, acc)| {
                *acc += score;
                existing.fts_score = result.fts_score;
            })
            .or_insert_with(|| {
                let mut r = result.clone();
                r.score = score;
                r.vector_score = None;
                (r, score)
            });
    }

    let mut fused: Vec<SearchResult> = rrf_scores
        .into_values()
        .map(|(mut result, acc)| {
            result.score = acc;
            result
        })
        .collect();

    fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    fused.truncate(top_k);
    fused
}
