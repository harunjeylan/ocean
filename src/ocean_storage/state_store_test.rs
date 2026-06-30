use crate::ocean_storage::state_store::{IndexStatus, StateStore};
use crate::ocean_storage::SurrealStateStore;

#[test]
fn test_state_store_update_get() {
    let store = SurrealStateStore::new_memory().unwrap();
    store.update_state("f1", "hash1", IndexStatus::Indexed).unwrap();

    let state = store.get_state("f1").unwrap().unwrap();
    assert_eq!(state.file_id, "f1");
    assert_eq!(state.hash, "hash1");
    assert_eq!(state.status, IndexStatus::Indexed);
}

#[test]
fn test_state_store_delete() {
    let store = SurrealStateStore::new_memory().unwrap();
    store.update_state("f1", "hash1", IndexStatus::Indexed).unwrap();

    let deleted = store.delete_state("f1").unwrap();
    assert!(deleted);

    let deleted2 = store.delete_state("f1").unwrap();
    assert!(!deleted2);
}

#[test]
fn test_state_store_list_pending() {
    let store = SurrealStateStore::new_memory().unwrap();

    store.update_state("f1", "hash1", IndexStatus::Indexed).unwrap();
    store.update_state("f2", "hash2", IndexStatus::Pending).unwrap();
    store.update_state("f3", "hash3", IndexStatus::Failed).unwrap();

    let pending = store.list_pending().unwrap();
    assert_eq!(pending.len(), 2);
    let statuses: Vec<IndexStatus> = pending.into_iter().map(|r| r.status).collect();
    assert!(statuses.contains(&IndexStatus::Pending));
    assert!(statuses.contains(&IndexStatus::Failed));
}

#[test]
fn test_state_store_list_all() {
    let store = SurrealStateStore::new_memory().unwrap();
    store.update_state("f1", "hash1", IndexStatus::Indexed).unwrap();
    store.update_state("f2", "hash2", IndexStatus::Pending).unwrap();

    let all = store.list_all().unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_state_store_get_nonexistent() {
    let store = SurrealStateStore::new_memory().unwrap();
    let got = store.get_state("nonexistent").unwrap();
    assert!(got.is_none());
}
