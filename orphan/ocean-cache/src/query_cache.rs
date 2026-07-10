use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::ocean_cache::lru::LruCache;
use crate::ocean_query::types::{QueryMode, RankedChunk};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct QueryCacheKey {
    pub query_text: String,
    pub mode: QueryMode,
    pub top_k: usize,
    pub filter_hash: u64,
}

#[derive(Clone)]
struct CacheEntry {
    results: Vec<RankedChunk>,
    cached_at: Instant,
}

pub struct QueryCache {
    l1: Mutex<LruCache<QueryCacheKey, CacheEntry>>,
    ttl: Duration,
    last_invalidated: Mutex<Instant>,
}

impl QueryCache {
    pub fn new(capacity: usize, ttl_secs: u64) -> Self {
        Self {
            l1: Mutex::new(LruCache::new(capacity)),
            ttl: Duration::from_secs(ttl_secs),
            last_invalidated: Mutex::new(Instant::now()),
        }
    }

    pub fn get(&self, key: &QueryCacheKey) -> Option<Vec<RankedChunk>> {
        let mut l1 = self.l1.lock().ok()?;
        let entry = l1.get(key)?;

        if entry.cached_at.elapsed() > self.ttl {
            l1.remove(key);
            return None;
        }

        Some(entry.results.clone())
    }

    pub fn set(&self, key: QueryCacheKey, results: Vec<RankedChunk>) {
        if let Ok(mut l1) = self.l1.lock() {
            l1.put(
                key,
                CacheEntry {
                    results,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    pub fn invalidate(&self) {
        if let Ok(mut l1) = self.l1.lock() {
            l1.clear();
        }
        if let Ok(mut ts) = self.last_invalidated.lock() {
            *ts = Instant::now();
        }
    }

    pub fn was_invalidated_since(&self, since: Instant) -> bool {
        if let Ok(ts) = self.last_invalidated.lock() {
            *ts > since
        } else {
            false
        }
    }
}
