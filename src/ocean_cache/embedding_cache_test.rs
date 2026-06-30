use crate::ocean_cache::embedding_cache::EmbeddingCache;

#[test]
fn test_embedding_cache_l1_hit() {
    let cache = EmbeddingCache::new(10, None);
    assert_eq!(cache.get("hash1", "model1"), None);
    cache.set("hash1", "model1", vec![0.1, 0.2, 0.3]);
    let result = cache.get("hash1", "model1");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), vec![0.1, 0.2, 0.3]);
}

#[test]
fn test_embedding_cache_miss() {
    let cache = EmbeddingCache::new(10, None);
    assert_eq!(cache.get("nonexistent", "model1"), None);
}

#[test]
fn test_embedding_cache_different_model() {
    let cache = EmbeddingCache::new(10, None);
    cache.set("hash1", "model1", vec![0.1, 0.2]);
    assert_eq!(cache.get("hash1", "model2"), None);
}

#[test]
fn test_embedding_cache_overwrite() {
    let cache = EmbeddingCache::new(10, None);
    cache.set("hash1", "model1", vec![0.1, 0.2]);
    cache.set("hash1", "model1", vec![0.3, 0.4]);
    let result = cache.get("hash1", "model1").unwrap();
    assert_eq!(result, vec![0.3, 0.4]);
}

#[test]
fn test_embedding_cache_lru_eviction() {
    let cache = EmbeddingCache::new(2, None);
    cache.set("hash1", "m", vec![1.0]);
    cache.set("hash2", "m", vec![2.0]);
    assert!(cache.get("hash1", "m").is_some());
    assert!(cache.get("hash2", "m").is_some());
    cache.set("hash3", "m", vec![3.0]);
    assert!(cache.get("hash3", "m").is_some());
}

#[test]
fn test_embedding_cache_clear_l1() {
    let cache = EmbeddingCache::new(10, None);
    cache.set("hash1", "m", vec![1.0]);
    assert!(cache.get("hash1", "m").is_some());
    cache.clear_l1();
    assert_eq!(cache.get("hash1", "m"), None);
}

#[test]
fn test_embedding_cache_get_batch() {
    let cache = EmbeddingCache::new(10, None);
    cache.set("hash1", "m", vec![1.0]);
    cache.set("hash2", "m", vec![2.0]);
    let results = cache.get_batch(&[("hash1", "m"), ("hash2", "m"), ("hash3", "m")]);
    assert_eq!(results.len(), 2);
    assert!(results.contains_key(&("hash1".into(), "m".into())));
    assert!(results.contains_key(&("hash2".into(), "m".into())));
}
