use crate::ocean_query::types::*;

#[test]
fn query_default_impl() {
    let q = Query::default();
    assert!(q.text.is_empty());
    assert_eq!(q.mode, QueryMode::Auto);
    assert_eq!(q.top_k, 10);
    assert_eq!(q.expand_depth, 0);
    assert!(q.filter.is_none());
    assert!(!q.include_context);
    assert_eq!(q.context_chunks, 3);
    assert!(!q.rerank_by_heading);
    assert!(!q.rerank_by_file);
}

#[test]
fn query_mode_auto_default() {
    assert_eq!(QueryMode::default(), QueryMode::Auto);
}

#[test]
fn execution_meta_population() {
    let meta = ExecutionMeta {
        query_mode: QueryMode::Hybrid,
        total_results: 5,
        vector_search_time_ms: 100,
        graph_expand_time_ms: Some(50),
        fusion_time_ms: 10,
        total_time_ms: 160,
    };
    assert_eq!(meta.query_mode, QueryMode::Hybrid);
    assert_eq!(meta.total_results, 5);
    assert_eq!(meta.vector_search_time_ms, 100);
    assert_eq!(meta.graph_expand_time_ms, Some(50));
    assert_eq!(meta.fusion_time_ms, 10);
    assert_eq!(meta.total_time_ms, 160);
}

#[test]
fn ranked_chunk_creation() {
    let chunk = RankedChunk {
        chunk_id: "chunk-1".into(),
        file_id: "file-1".into(),
        content: "test content".into(),
        heading: Some("Section 1".into()),
        score: 0.95,
        vector_score: Some(0.95),
        fts_score: None,
        graph_score: None,
        block_type: Some("Text".into()),
    };
    assert_eq!(chunk.chunk_id, "chunk-1");
    assert_eq!(chunk.score, 0.95);
}

#[test]
fn context_window_creation() {
    let cw = ContextWindow {
        anchor_chunk_id: "anchor-1".into(),
        chunks: vec![ContextChunk {
            chunk_id: "ctx-1".into(),
            content: "context".into(),
            heading: None,
            score: 0.5,
            distance_from_anchor: -1,
        }],
        total_tokens: 5,
    };
    assert_eq!(cw.anchor_chunk_id, "anchor-1");
    assert_eq!(cw.chunks.len(), 1);
    assert_eq!(cw.total_tokens, 5);
}

#[test]
fn query_mode_serialize_roundtrip() {
    let modes = vec![
        QueryMode::Auto,
        QueryMode::Vector,
        QueryMode::Hybrid,
        QueryMode::Expand,
    ];
    for mode in &modes {
        let json = serde_json::to_string(mode).unwrap();
        let deserialized: QueryMode = serde_json::from_str(&json).unwrap();
        assert_eq!(*mode, deserialized);
    }
}

#[test]
fn execution_meta_graph_none() {
    let meta = ExecutionMeta {
        query_mode: QueryMode::Vector,
        total_results: 0,
        vector_search_time_ms: 0,
        graph_expand_time_ms: None,
        fusion_time_ms: 0,
        total_time_ms: 0,
    };
    assert!(meta.graph_expand_time_ms.is_none());
}
