use crate::ocean_chunk::{Chunk, ChunkType};
use crate::ocean_vector::store::*;

fn make_test_chunk(id: &str, content: &str) -> Chunk {
    Chunk {
        id: id.into(),
        file_id: "file-1".into(),
        content: content.into(),
        heading: Some("Test".into()),
        page: Some(1),
        slide: None,
        sheet: None,
        block_type: ChunkType::Text,
        start_offset: None,
        end_offset: None,
    }
}

#[test]
fn test_chunk_record_from_chunk() {
    let chunk = make_test_chunk("c1", "hello world");
    let embedding = vec![0.1f32, 0.2, 0.3];
    let record = ChunkRecord::from_chunk(&chunk, embedding.clone(), "test-model");

    assert_eq!(record.chunk_id, "c1");
    assert_eq!(record.file_id, "file-1");
    assert_eq!(record.content, "hello world");
    assert_eq!(record.heading, Some("Test".into()));
    assert_eq!(record.page, Some(1));
    assert_eq!(record.slide, None);
    assert_eq!(record.block_type, "Text");
    assert_eq!(record.embedding, embedding);
    assert_eq!(record.model, "test-model");
    assert_eq!(record.dimension, 3);
    assert!(!record.content_hash.is_empty());
}

#[test]
fn test_store_insert_and_get() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(3).unwrap();

    let chunk = make_test_chunk("c-test-1", "hello store");
    let record = ChunkRecord::from_chunk(&chunk, vec![0.1, 0.2, 0.3], "test-model");
    store.insert_chunk(record.clone()).unwrap();

    let found = store.get_chunk("c-test-1").unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.chunk_id, "c-test-1");
    assert_eq!(found.content, "hello store");
}

#[test]
fn test_store_get_nonexistent() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(3).unwrap();

    let found = store.get_chunk("nonexistent").unwrap();
    assert!(found.is_none());
}

#[test]
fn test_store_insert_batch() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(3).unwrap();

    let records: Vec<ChunkRecord> = (0..5)
        .map(|i| {
            let chunk = make_test_chunk(&format!("c-batch-{}", i), &format!("content {}", i));
            ChunkRecord::from_chunk(&chunk, vec![0.1, 0.2, 0.3], "test-model")
        })
        .collect();

    store.insert_chunks_batch(records).unwrap();
    assert_eq!(store.count().unwrap(), 5);
}

#[test]
fn test_store_delete_by_file() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(3).unwrap();

    let chunk = make_test_chunk("c-del-1", "to delete");
    let record = ChunkRecord::from_chunk(&chunk, vec![0.1, 0.2, 0.3], "test-model");
    store.insert_chunk(record).unwrap();
    assert_eq!(store.count().unwrap(), 1);

    let deleted = store.delete_chunks_by_file("file-1").unwrap();
    assert_eq!(deleted, 1);
    assert_eq!(store.count().unwrap(), 0);
}

#[test]
fn test_store_count_empty() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(3).unwrap();
    assert_eq!(store.count().unwrap(), 0);
}

#[test]
fn test_store_chunk_exists() {
    let store = VectorStore::new_memory().unwrap();
    store.initialize_schema(3).unwrap();

    let chunk = make_test_chunk("c-exists-1", "check existence");
    let record = ChunkRecord::from_chunk(&chunk, vec![0.1, 0.2, 0.3], "test-model");
    let hash = record.content_hash.clone();
    store.insert_chunk(record).unwrap();

    assert!(store.chunk_exists(&hash, "test-model").unwrap());
    assert!(!store.chunk_exists(&hash, "other-model").unwrap());
    assert!(!store.chunk_exists("nonexistent", "test-model").unwrap());
}
