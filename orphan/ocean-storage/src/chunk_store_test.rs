use crate::ocean_storage::chunk_store::{ChunkRecord, ChunkStore};
use crate::ocean_storage::SurrealChunkStore;

fn make_chunk(id: &str, file_id: &str) -> ChunkRecord {
    ChunkRecord {
        chunk_id: id.to_string(),
        file_id: file_id.to_string(),
        content: format!("Content of chunk {}", id),
        heading: Some("Test Heading".into()),
        page: Some(1),
        slide: None,
        sheet: None,
        block_type: "Text".into(),
        content_hash: format!("hash_{}", id),
        created_at: 1000,
        embedding: Vec::new(),
        model: "model".into(),
        dimension: 0,
    }
}

#[test]
fn test_chunk_store_insert_get() {
    let store = SurrealChunkStore::new_memory().unwrap();
    let c = make_chunk("c1", "f1");

    store.insert_chunk(&c).unwrap();

    let got = store.get_chunk("c1").unwrap().unwrap();
    assert_eq!(got.chunk_id, "c1");
    assert_eq!(got.content, "Content of chunk c1");
}

#[test]
fn test_chunk_store_upsert() {
    let store = SurrealChunkStore::new_memory().unwrap();
    let c = make_chunk("c1", "f1");

    store.upsert_chunk(&c).unwrap();

    let mut c2 = c.clone();
    c2.content = "Updated content".into();
    store.upsert_chunk(&c2).unwrap();

    let got = store.get_chunk("c1").unwrap().unwrap();
    assert_eq!(got.content, "Updated content");
}

#[test]
fn test_chunk_store_delete_by_file() {
    let store = SurrealChunkStore::new_memory().unwrap();
    store.insert_chunk(&make_chunk("c1", "f1")).unwrap();
    store.insert_chunk(&make_chunk("c2", "f1")).unwrap();
    store.insert_chunk(&make_chunk("c3", "f2")).unwrap();

    assert_eq!(store.count().unwrap(), 3);

    let deleted = store.delete_chunks_by_file("f1").unwrap();
    assert_eq!(deleted, 2);

    assert_eq!(store.count().unwrap(), 1);
}

#[test]
fn test_chunk_store_count() {
    let store = SurrealChunkStore::new_memory().unwrap();
    assert_eq!(store.count().unwrap(), 0);
    store.insert_chunk(&make_chunk("c1", "f1")).unwrap();
    assert_eq!(store.count().unwrap(), 1);
}

#[test]
fn test_chunk_store_exists() {
    let store = SurrealChunkStore::new_memory().unwrap();
    let c = make_chunk("c1", "f1");
    store.insert_chunk(&c).unwrap();
    assert!(store.chunk_exists("hash_c1", "model").unwrap());
    assert!(!store.chunk_exists("nonexistent", "model").unwrap());
}

#[test]
fn test_chunk_store_get_by_file_and_heading() {
    let store = SurrealChunkStore::new_memory().unwrap();
    store.insert_chunk(&make_chunk("c1", "f1")).unwrap();

    let mut c2 = make_chunk("c2", "f1");
    c2.heading = None;
    store.insert_chunk(&c2).unwrap();

    let results = store.get_by_file_and_heading("f1", Some("Test Heading")).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].chunk_id, "c1");
}
