use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::SurrealVectorStore;
use crate::ocean_storage::vector_store::VectorStore;

#[test]
fn test_vector_store_initialize_schema() {
    let config = StorageConfig::new(":memory:");
    let store = SurrealVectorStore::new_memory(&config).unwrap();
    store.initialize_schema(4).unwrap();
    // Schema is idempotent — calling twice should not error
    store.initialize_schema(4).unwrap();
}

#[test]
fn test_vector_store_count_empty() {
    let config = StorageConfig::new(":memory:");
    let store = SurrealVectorStore::new_memory(&config).unwrap();
    store.initialize_schema(4).unwrap();
    assert_eq!(store.count().unwrap(), 0);
}

#[test]
fn test_vector_store_new_memory() {
    let config = StorageConfig::new(":memory:");
    let store = SurrealVectorStore::new_memory(&config).unwrap();
    store.initialize_schema(4).unwrap();
    assert_eq!(store.count().unwrap(), 0);
}
