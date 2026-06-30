use crate::ocean_cache::lru::LruCache;

#[test]
fn test_new_cache_is_empty() {
    let cache: LruCache<i32, i32> = LruCache::new(5);
    assert_eq!(cache.len(), 0);
    assert!(!cache.is_full());
}

#[test]
fn test_put_and_get() {
    let mut cache = LruCache::new(3);
    cache.put(1, "one");
    cache.put(2, "two");
    assert_eq!(cache.get(&1), Some(&"one"));
    assert_eq!(cache.get(&2), Some(&"two"));
    assert_eq!(cache.get(&3), None);
}

#[test]
fn test_eviction_at_capacity() {
    let mut cache = LruCache::new(3);
    cache.put(1, "a");
    cache.put(2, "b");
    cache.put(3, "c");
    cache.put(4, "d");
    assert_eq!(cache.get(&1), None);
    assert_eq!(cache.get(&4), Some(&"d"));
    assert_eq!(cache.get(&2), Some(&"b"));
    assert_eq!(cache.get(&3), Some(&"c"));
}

#[test]
fn test_lru_preserves_recently_used() {
    let mut cache = LruCache::new(3);
    cache.put(1, "a");
    cache.put(2, "b");
    cache.put(3, "c");
    cache.get(&1);
    cache.put(4, "d");
    assert_eq!(cache.get(&1), Some(&"a"));
    assert_eq!(cache.get(&2), None);
    assert_eq!(cache.get(&3), Some(&"c"));
    assert_eq!(cache.get(&4), Some(&"d"));
}

#[test]
fn test_remove() {
    let mut cache = LruCache::new(3);
    cache.put(1, "a");
    cache.put(2, "b");
    assert_eq!(cache.remove(&1), Some("a"));
    assert_eq!(cache.get(&1), None);
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_clear() {
    let mut cache = LruCache::new(3);
    cache.put(1, "a");
    cache.put(2, "b");
    cache.clear();
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.get(&1), None);
}

#[test]
fn test_capacity_zero() {
    let mut cache: LruCache<i32, i32> = LruCache::new(0);
    cache.put(1, 10);
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.get(&1), None);
}

#[test]
fn test_update_existing_key() {
    let mut cache = LruCache::new(3);
    cache.put(1, "a");
    cache.put(1, "b");
    assert_eq!(cache.get(&1), Some(&"b"));
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_is_full() {
    let mut cache = LruCache::new(2);
    assert!(!cache.is_full());
    cache.put(1, "a");
    assert!(!cache.is_full());
    cache.put(2, "b");
    assert!(cache.is_full());
}

#[test]
fn test_contains() {
    let mut cache = LruCache::new(3);
    cache.put(1, "a");
    assert!(cache.contains(&1));
    assert!(!cache.contains(&2));
}
