use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum QueryMode {
    Auto,
    Vector,
    Hybrid,
    Expand,
}

impl Default for QueryMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone)]
pub struct Query {
    pub text: String,
    pub mode: QueryMode,
    pub top_k: usize,
    pub expand_depth: usize,
    pub filter: Option<crate::ocean_vector::search::SearchFilter>,
    pub include_context: bool,
    pub context_chunks: usize,
    pub rerank_by_heading: bool,
    pub rerank_by_file: bool,
}

impl Default for Query {
    fn default() -> Self {
        Self {
            text: String::new(),
            mode: QueryMode::Auto,
            top_k: 10,
            expand_depth: 0,
            filter: None,
            include_context: false,
            context_chunks: 3,
            rerank_by_heading: false,
            rerank_by_file: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub results: Vec<RankedChunk>,
    pub context_windows: Vec<ContextWindow>,
    pub execution: ExecutionMeta,
}

#[derive(Debug, Clone)]
pub struct RankedChunk {
    pub chunk_id: String,
    pub file_id: String,
    pub content: String,
    pub heading: Option<String>,
    pub score: f32,
    pub vector_score: Option<f32>,
    pub fts_score: Option<f32>,
    pub graph_score: Option<f32>,
    pub block_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContextWindow {
    pub anchor_chunk_id: String,
    pub chunks: Vec<ContextChunk>,
    pub total_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct ContextChunk {
    pub chunk_id: String,
    pub content: String,
    pub heading: Option<String>,
    pub score: f32,
    pub distance_from_anchor: i32,
}

#[derive(Debug, Clone)]
pub struct ExecutionMeta {
    pub query_mode: QueryMode,
    pub total_results: usize,
    pub vector_search_time_ms: u64,
    pub graph_expand_time_ms: Option<u64>,
    pub fusion_time_ms: u64,
    pub total_time_ms: u64,
}
