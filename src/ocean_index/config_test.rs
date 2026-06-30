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
        max_retries: 3,
        no_graph: false,
    };
    assert_eq!(config.mode, IndexMode::Incremental);
    assert_eq!(config.dir, "/tmp");
    assert_eq!(config.batch_size, 10);
    assert_eq!(config.max_retries, 3);
    assert!(!config.no_graph);
}
