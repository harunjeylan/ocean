use std::fmt;

use crate::ocean_chunk::ChunkConfig;

#[derive(Debug)]
pub enum ApiError {
    DocError(String),
    IndexError(String),
    QueryError(String),
    FsError(String),
    EmbedderError(String),
    ConfigError(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::DocError(msg) => write!(f, "{}", msg),
            ApiError::IndexError(msg) => write!(f, "{}", msg),
            ApiError::QueryError(msg) => write!(f, "{}", msg),
            ApiError::FsError(msg) => write!(f, "{}", msg),
            ApiError::EmbedderError(msg) => write!(f, "{}", msg),
            ApiError::ConfigError(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<crate::ocean_parser::DocumentError> for ApiError {
    fn from(e: crate::ocean_parser::DocumentError) -> Self {
        ApiError::DocError(e.to_string())
    }
}

impl From<crate::ocean_chunk::ChunkError> for ApiError {
    fn from(e: crate::ocean_chunk::ChunkError) -> Self {
        ApiError::DocError(e.to_string())
    }
}

impl From<crate::ocean_index::IndexError> for ApiError {
    fn from(e: crate::ocean_index::IndexError) -> Self {
        ApiError::IndexError(e.to_string())
    }
}

impl From<crate::ocean_query::QueryError> for ApiError {
    fn from(e: crate::ocean_query::QueryError) -> Self {
        ApiError::QueryError(e.to_string())
    }
}

impl From<crate::ocean_vector::EmbedderError> for ApiError {
    fn from(e: crate::ocean_vector::EmbedderError) -> Self {
        ApiError::EmbedderError(e.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct DocResult {
    pub metadata: crate::ocean_parser::DocumentMetadata,
    pub outline: crate::ocean_parser::Outline,
}

#[derive(Debug, Clone)]
pub struct GrepResult {
    pub total_matches: u32,
    pub total_files: usize,
    pub file_matches: Vec<FileMatches>,
}

#[derive(Debug, Clone)]
pub struct FileMatches {
    pub file: String,
    pub matches: Vec<crate::ocean_parser::Match>,
}

#[derive(Debug, Clone)]
pub struct ReadRequest {
    pub file: String,
    pub selector: crate::ocean_parser::Selector,
}

#[derive(Debug, Clone)]
pub struct IndexRequest {
    pub dir: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub dimension: Option<usize>,
    pub db_path: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub batch_size: usize,
    pub reindex: bool,
    pub no_graph: bool,
    pub no_references: bool,
    pub no_entities: bool,
    pub watch: bool,
    pub chunk_config: Option<ChunkConfig>,
    pub io_threads: Option<usize>,
    pub cpu_threads: Option<usize>,
    pub max_ai_concurrent: Option<usize>,
    pub max_retries: Option<u32>,
    pub retry_backoff_ms: Option<u64>,
    pub max_queue_size: Option<usize>,
    pub max_in_flight: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct QueryRequest {
    pub text: String,
    pub mode: Option<String>,
    pub top_k: usize,
    pub expand_depth: usize,
    pub include_context: bool,
    pub context_chunks: Option<usize>,
    pub filter_file_id: Option<String>,
    pub filter_heading: Option<String>,
    pub filter_block_type: Option<String>,
    pub rerank_by_heading: bool,
    pub rerank_by_file: bool,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub dimension: Option<usize>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub db_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VectorSearchRequest {
    pub query: String,
    pub top_k: usize,
    pub hybrid: bool,
    pub expand_depth: usize,
    pub filter_file_id: Option<String>,
    pub filter_heading: Option<String>,
    pub filter_block_type: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub dimension: Option<usize>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub db_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GraphRequest {
    pub db_path: Option<String>,
}

pub type IndexResult = crate::ocean_index::IndexReport;
