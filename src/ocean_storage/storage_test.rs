use crate::ocean_storage::state_store::IndexStatus;
use crate::ocean_storage::*;

#[test]
fn test_storage_new_memory() {
    let storage = SurrealStorage::new_memory().unwrap();
    assert!(storage.in_transaction() == false);
    assert!(storage.storage_path() == ":memory:");
}

#[test]
fn test_storage_all_stores_accessible() {
    let storage = SurrealStorage::new_memory().unwrap();
    let _files = storage.files();
    let _chunks = storage.chunks();
    let _vectors = storage.vectors();
    let _graph = storage.graph();
    let _state = storage.state();
}

#[test]
fn test_storage_transaction_commit_rollback() {
    let mut storage = SurrealStorage::new_memory().unwrap();

    storage.begin_transaction().unwrap();
    assert!(storage.in_transaction());

    storage.rollback().unwrap();
    assert!(!storage.in_transaction());
}

#[test]
fn test_storage_transaction_nested() {
    let mut storage = SurrealStorage::new_memory().unwrap();

    storage.begin_transaction().unwrap();
    storage.begin_transaction().unwrap();
    assert!(storage.in_transaction());

    storage.commit().unwrap();
    // Still in transaction due to nesting
    assert!(storage.in_transaction());

    storage.commit().unwrap();
    assert!(!storage.in_transaction());
}

#[test]
fn test_storage_transaction_rollback_nested() {
    let mut storage = SurrealStorage::new_memory().unwrap();

    storage.begin_transaction().unwrap();
    storage.begin_transaction().unwrap();
    storage.rollback().unwrap();
    // Rollback clears all depth
    assert!(!storage.in_transaction());
}

#[test]
fn test_storage_count_all() {
    let storage = SurrealStorage::new_memory().unwrap();
    let stats = storage.count_all().unwrap();
    assert_eq!(stats.file_count, 0);
    assert_eq!(stats.chunk_count, 0);
    assert_eq!(stats.node_count, 0);
    assert_eq!(stats.edge_count, 0);
}

#[test]
fn test_storage_file_and_state_integration() {
    let storage = SurrealStorage::new_memory().unwrap();

    let file = file_store::FileMeta {
        file_id: "f1".into(),
        path: "/test/doc.txt".into(),
        hash: "abc".into(),
        size: 100,
        modified: 1000,
        extension: "txt".into(),
        last_indexed: 0,
    };

    storage.files().upsert_file(&file).unwrap();
    storage.state().update_state("f1", "abc", IndexStatus::Indexed).unwrap();

    let got = storage.files().get_file("f1").unwrap().unwrap();
    assert_eq!(got.path, "/test/doc.txt");

    let state = storage.state().get_state("f1").unwrap().unwrap();
    assert_eq!(state.status, IndexStatus::Indexed);

    let stats = storage.count_all().unwrap();
    assert_eq!(stats.file_count, 1);
}
