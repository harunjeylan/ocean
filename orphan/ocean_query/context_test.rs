use crate::ocean_query::context::ContextWindowBuilder;
use crate::ocean_query::types::RankedChunk;
use crate::ocean_storage::chunk_store::ChunkStore;
use crate::ocean_storage::SurrealChunkStore;
use std::sync::Arc;

fn init_store() -> Arc<dyn ChunkStore> {
    Arc::new(SurrealChunkStore::new_memory().expect("failed to create memory store"))
}

#[test]
fn context_window_empty_neighbors() {
    let store = init_store();
    let builder = ContextWindowBuilder::new(store);
    let anchor = RankedChunk {
        chunk_id: "nonexistent".into(),
        file_id: "file-1".into(),
        content: "anchor content".into(),
        heading: Some("Section 1".into()),
        score: 0.9,
        vector_score: Some(0.9),
        fts_score: None,
        graph_score: None,
        block_type: Some("Text".into()),
    };
    let result = builder.build(&anchor, 3);
    assert!(result.is_ok());
    let cw = result.unwrap();
    assert_eq!(cw.anchor_chunk_id, "nonexistent");
    // No chunks in store, so window has only anchor content
    assert!(!cw.chunks.is_empty());
    assert_eq!(cw.chunks[0].chunk_id, "nonexistent");
    assert_eq!(cw.chunks[0].distance_from_anchor, 0);
}

#[test]
fn context_window_single_chunk() {
    let store = init_store();
    let builder = ContextWindowBuilder::new(store);
    let anchor = RankedChunk {
        chunk_id: "chunk-1".into(),
        file_id: "file-1".into(),
        content: "single chunk".into(),
        heading: None,
        score: 0.8,
        vector_score: Some(0.8),
        fts_score: None,
        graph_score: None,
        block_type: Some("Text".into()),
    };
    let result = builder.build(&anchor, 1);
    assert!(result.is_ok());
    let cw = result.unwrap();
    assert_eq!(cw.anchor_chunk_id, "chunk-1");
}

#[test]
fn context_window_clamp() {
    let store = init_store();
    let builder = ContextWindowBuilder::new(store);
    let anchor = RankedChunk {
        chunk_id: "chunk-c".into(),
        file_id: "file-1".into(),
        content: "anchor content".into(),
        heading: None,
        score: 0.7,
        vector_score: Some(0.7),
        fts_score: None,
        graph_score: None,
        block_type: Some("Text".into()),
    };
    // clamp context_chunks to [1, 10]
    let result = builder.build(&anchor, 100);
    assert!(result.is_ok());
    let cw = result.unwrap();
    // even with request for 100, it returns at least the anchor
    assert!(!cw.chunks.is_empty());
}

#[test]
fn context_window_heading_boundary() {
    // verify that build() doesn't crash with heading=None chunks
    let store = init_store();
    let builder = ContextWindowBuilder::new(store);
    let anchor = RankedChunk {
        chunk_id: "chunk-a".into(),
        file_id: "file-1".into(),
        content: "some content".into(),
        heading: None,
        score: 0.6,
        vector_score: Some(0.6),
        fts_score: None,
        graph_score: None,
        block_type: Some("Text".into()),
    };
    let result = builder.build(&anchor, 5);
    assert!(result.is_ok());
}
