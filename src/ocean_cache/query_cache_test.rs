use std::thread;
use std::time::Duration;

use crate::ocean_cache::query_cache::{QueryCache, QueryCacheKey};
use crate::ocean_query::types::QueryMode;

fn make_key(text: &str) -> QueryCacheKey {
    QueryCacheKey {
        query_text: text.to_string(),
        mode: QueryMode::Hybrid,
        top_k: 10,
        filter_hash: 0,
    }
}

fn make_results(n: usize) -> Vec<crate::ocean_query::types::RankedChunk> {
    (0..n)
        .map(|i| crate::ocean_query::types::RankedChunk {
            chunk_id: format!("chunk_{}", i),
            file_id: "file1".into(),
            content: format!("content {}", i),
            heading: None,
            score: 1.0 - (i as f32 * 0.1),
            vector_score: None,
            fts_score: None,
            graph_score: None,
            block_type: None,
        })
        .collect()
}

#[test]
fn test_query_cache_hit_within_ttl() {
    let cache = QueryCache::new(10, 60);
    let key = make_key("test query");
    let results = make_results(3);
    cache.set(key.clone(), results.clone());
    let cached = cache.get(&key);
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().len(), 3);
}

#[test]
fn test_query_cache_miss() {
    let cache = QueryCache::new(10, 60);
    let key = make_key("nonexistent");
    assert!(cache.get(&key).is_none());
}

#[test]
fn test_query_cache_invalidate() {
    let cache = QueryCache::new(10, 60);
    let key = make_key("test");
    cache.set(key.clone(), make_results(2));
    assert!(cache.get(&key).is_some());
    cache.invalidate();
    assert!(cache.get(&key).is_none());
}

#[test]
fn test_query_cache_ttl_expiry() {
    let cache = QueryCache::new(10, 0);
    let key = make_key("test");
    cache.set(key.clone(), make_results(1));
    thread::sleep(Duration::from_millis(10));
    assert!(cache.get(&key).is_none());
}

#[test]
fn test_query_cache_different_keys() {
    let cache = QueryCache::new(10, 60);
    let key1 = make_key("query one");
    let key2 = make_key("query two");
    cache.set(key1.clone(), make_results(1));
    assert!(cache.get(&key1).is_some());
    assert!(cache.get(&key2).is_none());
}

#[test]
fn test_query_cache_was_invalidated_since() {
    let cache = QueryCache::new(10, 60);
    let before = std::time::Instant::now();
    assert!(!cache.was_invalidated_since(before));
    cache.invalidate();
    assert!(cache.was_invalidated_since(before));
}
