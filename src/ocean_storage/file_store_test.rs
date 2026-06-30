use crate::ocean_storage::file_store::{FileMeta, FileStore};
use crate::ocean_storage::SurrealFileStore;

fn make_file(id: &str) -> FileMeta {
    FileMeta {
        id: id.to_string(),
        path: format!("/test/{}", id),
        hash: "abc123".into(),
        size: 1024,
        modified: 1000,
        extension: "txt".into(),
        last_indexed: 1000,
    }
}

#[test]
fn test_file_store_crud() {
    let store = SurrealFileStore::new_memory().unwrap();
    let f = make_file("f1");

    store.upsert_file(&f).unwrap();
    let got = store.get_file("f1").unwrap().unwrap();
    assert_eq!(got.id, "f1");
    assert_eq!(got.path, "/test/f1");

    let got_path = store.get_file_by_path("/test/f1").unwrap().unwrap();
    assert_eq!(got_path.id, "f1");

    let all = store.list_files().unwrap();
    assert_eq!(all.len(), 1);

    let deleted = store.delete_file("f1").unwrap();
    assert!(deleted);

    let gone = store.get_file("f1").unwrap();
    assert!(gone.is_none());
}

#[test]
fn test_file_store_needs_update() {
    let store = SurrealFileStore::new_memory().unwrap();
    let f = make_file("f1");

    // Not stored yet -> needs update
    assert!(store.needs_update(&f).unwrap());

    store.upsert_file(&f).unwrap();

    // Same hash -> no update needed
    assert!(!store.needs_update(&f).unwrap());

    // Different hash -> needs update
    let mut f2 = f.clone();
    f2.hash = "def456".into();
    assert!(store.needs_update(&f2).unwrap());
}

#[test]
fn test_file_store_delete_nonexistent() {
    let store = SurrealFileStore::new_memory().unwrap();
    let deleted = store.delete_file("nonexistent").unwrap();
    assert!(!deleted);
}

#[test]
fn test_file_store_get_nonexistent() {
    let store = SurrealFileStore::new_memory().unwrap();
    let got = store.get_file("nonexistent").unwrap();
    assert!(got.is_none());
}
