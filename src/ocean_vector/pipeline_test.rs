use crate::ocean_chunk::{Chunk, ChunkType};
use super::embedder_spec::MockEmbedder;
use crate::ocean_vector::pipeline::*;
use crate::ocean_vector::store::VectorStore;

fn make_chunk(id: &str, content: &str) -> Chunk {
    Chunk {
        id: id.into(),
        file_id: "file-pipe".into(),
        content: content.into(),
        heading: Some("Section".into()),
        page: None,
        slide: None,
        sheet: None,
        block_type: ChunkType::Text,
        start_offset: None,
        end_offset: None,
    }
}

#[test]
fn test_pipeline_index_chunks() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(4).unwrap();

    let embedder = MockEmbedder::new(4, "mock-model");
    let pipeline = IndexPipeline::new(store);

    let chunks = vec![
        make_chunk("p1", "first chunk content"),
        make_chunk("p2", "second chunk content"),
        make_chunk("p3", "third chunk content"),
    ];

    let config = IndexConfig {
        batch_size: 2,
        reindex: false,
        model: "mock-model".into(),
        dimension: 4,
        ..Default::default()
    };

    let report = pipeline.index_chunks(chunks, &embedder, &config).unwrap();
    assert_eq!(report.total, 3);
    assert_eq!(report.embedded, 3);
    assert_eq!(report.skipped, 0);
    assert_eq!(report.failed, 0);
}

#[test]
fn test_pipeline_idempotent_skip() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(4).unwrap();

    let embedder = MockEmbedder::new(4, "mock-model");
    let pipeline = IndexPipeline::new(store);

    let chunks = vec![make_chunk("p-idem", "idempotent test")];

    let config = IndexConfig {
        batch_size: 10,
        reindex: false,
        model: "mock-model".into(),
        dimension: 4,
        ..Default::default()
    };

    let report1 = pipeline.index_chunks(chunks.clone(), &embedder, &config).unwrap();
    assert_eq!(report1.embedded, 1);
    assert_eq!(report1.skipped, 0);

    let report2 = pipeline.index_chunks(chunks, &embedder, &config).unwrap();
    assert_eq!(report2.embedded, 0);
    assert_eq!(report2.skipped, 1);
}

#[test]
fn test_pipeline_reindex_flag() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(4).unwrap();

    let embedder = MockEmbedder::new(4, "mock-model");
    let pipeline = IndexPipeline::new(store);

    let chunks = vec![make_chunk("p-rei", "reindex test")];

    let config = IndexConfig {
        batch_size: 10,
        reindex: true,
        model: "mock-model".into(),
        dimension: 4,
        ..Default::default()
    };

    let report1 = pipeline.index_chunks(chunks.clone(), &embedder, &config).unwrap();
    assert_eq!(report1.embedded, 1);

    let report2 = pipeline.index_chunks(chunks, &embedder, &config).unwrap();
    assert_eq!(report2.embedded, 1);
    assert_eq!(report2.skipped, 0);
}

#[test]
fn test_pipeline_empty_chunks() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(4).unwrap();

    let embedder = MockEmbedder::new(4, "mock-model");
    let pipeline = IndexPipeline::new(store);

    let config = IndexConfig::default();
    let report = pipeline.index_chunks(vec![], &embedder, &config).unwrap();
    assert_eq!(report.total, 0);
    assert_eq!(report.embedded, 0);
}

#[test]
fn test_index_report_display_fields() {
    let report = IndexReport {
        total: 10,
        embedded: 7,
        skipped: 2,
        failed: 1,
        duration_ms: 1500,
        errors: vec![],
        graph_nodes: 0,
        graph_edges: 0,
    };
    assert_eq!(report.total, 10);
    assert_eq!(report.embedded, 7);
    assert_eq!(report.skipped, 2);
    assert_eq!(report.failed, 1);
    assert_eq!(report.duration_ms, 1500);
}
