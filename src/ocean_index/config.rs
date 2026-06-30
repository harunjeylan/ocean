use crate::ocean_chunk::ChunkConfig;
use crate::ocean_graph::GraphConfig;

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
    pub max_retries: u32,
    pub no_graph: bool,
}
