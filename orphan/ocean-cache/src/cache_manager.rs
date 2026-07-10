use crate::ocean_cache::config::CacheConfig;
use crate::ocean_cache::embedding_cache::EmbeddingCache;
use crate::ocean_cache::graph_cache::GraphCache;
use crate::ocean_cache::query_cache::QueryCache;

pub struct CacheManager {
    pub embeddings: EmbeddingCache,
    pub queries: QueryCache,
    pub graph: GraphCache,
    pub config: CacheConfig,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        let l2_path = config.embedding_cache_path.as_deref();

        Self {
            embeddings: EmbeddingCache::new(config.embedding_cache_size, l2_path),
            queries: QueryCache::new(config.query_cache_size, config.query_ttl_secs),
            graph: GraphCache::new(config.graph_cache_size),
            config,
        }
    }

    pub fn disabled() -> Self {
        Self {
            embeddings: EmbeddingCache::new(0, None),
            queries: QueryCache::new(0, 0),
            graph: GraphCache::new(0),
            config: CacheConfig {
                enabled: false,
                ..Default::default()
            },
        }
    }

    pub fn invalidate_all(&self) {
        self.embeddings.clear_l1();
        self.queries.invalidate();
        self.graph.invalidate_all();
    }

    pub fn invalidate_file_graph(&self, file_id: &str) {
        self.graph.invalidate_node(file_id);
    }
}
