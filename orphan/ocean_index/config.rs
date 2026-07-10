use crate::ocean_chunk::ChunkConfig;
use crate::ocean_graph::GraphConfig;
use crate::ocean_index::runtime::RetryPolicy;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexMode {
    Full,
    Incremental,
    Watch,
}

impl IndexMode {
    pub fn from_reindex(reindex: bool) -> Self {
        if reindex { IndexMode::Full } else { IndexMode::Incremental }
    }
}

impl Default for IndexMode {
    fn default() -> Self {
        IndexMode::Incremental
    }
}

#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub mode: IndexMode,
    pub dir: String,
    pub chunk_config: ChunkConfig,
    pub graph_config: GraphConfig,
    pub batch_size: usize,
    pub retry_policy: RetryPolicy,
    pub rate_limiter: RateLimiterConfig,
    pub backpressure: BackpressureConfig,
    pub io_threads: Option<usize>,
    pub cpu_threads: Option<usize>,
    pub no_graph: bool,
}

#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    pub max_concurrent: usize,
    pub requests_per_minute: Option<u64>,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 2,
            requests_per_minute: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    pub max_queue_size: usize,
    pub max_in_flight: usize,
    pub max_ai_concurrent: usize,
    pub pause_check_ms: u64,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10_000,
            max_in_flight: 10,
            max_ai_concurrent: 2,
            pause_check_ms: 1_000,
        }
    }
}
