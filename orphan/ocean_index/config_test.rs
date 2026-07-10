use crate::ocean_index::config::*;

#[test]
fn index_mode_from_reindex() {
    assert_eq!(IndexMode::from_reindex(true), IndexMode::Full);
    assert_eq!(IndexMode::from_reindex(false), IndexMode::Incremental);
}

#[test]
fn index_mode_default() {
    assert_eq!(IndexMode::default(), IndexMode::Incremental);
}

#[test]
fn index_mode_partial_eq() {
    assert_eq!(IndexMode::Full, IndexMode::Full);
    assert_ne!(IndexMode::Full, IndexMode::Incremental);
    assert_ne!(IndexMode::Watch, IndexMode::Incremental);
}

#[test]
fn index_config_default() {
    let config = crate::ocean_index::config::IndexConfig {
        mode: IndexMode::Incremental,
        dir: "/tmp".into(),
        chunk_config: crate::ocean_chunk::ChunkConfig::default(),
        graph_config: crate::ocean_graph::GraphConfig::default(),
        batch_size: 10,
        retry_policy: crate::ocean_index::runtime::RetryPolicy::default(),
        rate_limiter: crate::ocean_index::config::RateLimiterConfig::default(),
        backpressure: crate::ocean_index::config::BackpressureConfig::default(),
        io_threads: None,
        cpu_threads: None,
        no_graph: false,
    };
    assert_eq!(config.mode, IndexMode::Incremental);
    assert_eq!(config.dir, "/tmp");
    assert_eq!(config.batch_size, 10);
    assert_eq!(config.retry_policy.max_retries, 3);
    assert!(!config.no_graph);
}

#[test]
fn rate_limiter_config_default() {
    let rl = RateLimiterConfig::default();
    assert_eq!(rl.max_concurrent, 2);
    assert!(rl.requests_per_minute.is_none());
}

#[test]
fn backpressure_config_default() {
    let bp = BackpressureConfig::default();
    assert_eq!(bp.max_queue_size, 10_000);
    assert_eq!(bp.max_in_flight, 10);
    assert_eq!(bp.max_ai_concurrent, 2);
    assert_eq!(bp.pause_check_ms, 1_000);
}
