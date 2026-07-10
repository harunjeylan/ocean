#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub embedding_cache_size: usize,
    pub query_cache_size: usize,
    pub graph_cache_size: usize,
    pub query_ttl_secs: u64,
    pub embedding_cache_path: Option<String>,
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            embedding_cache_size: 1000,
            query_cache_size: 100,
            graph_cache_size: 5000,
            query_ttl_secs: 60,
            embedding_cache_path: None,
            enabled: true,
        }
    }
}
