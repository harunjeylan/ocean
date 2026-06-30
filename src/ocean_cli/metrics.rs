use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use serde::Serialize;

#[derive(Debug)]
pub struct MetricsCollector {
    pub queries_total: AtomicU64,
    pub queries_cached: AtomicU64,
    pub files_indexed: AtomicU64,
    pub files_skipped: AtomicU64,
    pub files_failed: AtomicU64,
    pub embedding_calls: AtomicU64,
    pub embedding_cached: AtomicU64,
    pub graph_expansions: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            queries_total: AtomicU64::new(0),
            queries_cached: AtomicU64::new(0),
            files_indexed: AtomicU64::new(0),
            files_skipped: AtomicU64::new(0),
            files_failed: AtomicU64::new(0),
            embedding_calls: AtomicU64::new(0),
            embedding_cached: AtomicU64::new(0),
            graph_expansions: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };
        MetricsSnapshot {
            queries_total: self.queries_total.load(Ordering::Relaxed),
            queries_cached: self.queries_cached.load(Ordering::Relaxed),
            files_indexed: self.files_indexed.load(Ordering::Relaxed),
            files_skipped: self.files_skipped.load(Ordering::Relaxed),
            files_failed: self.files_failed.load(Ordering::Relaxed),
            embedding_calls: self.embedding_calls.load(Ordering::Relaxed),
            embedding_cached: self.embedding_cached.load(Ordering::Relaxed),
            graph_expansions: self.graph_expansions.load(Ordering::Relaxed),
            cache_hits: hits,
            cache_misses: misses,
            cache_hit_rate: hit_rate,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    pub fn increment(&self, counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add(&self, counter: &AtomicU64, delta: u64) {
        counter.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        self.queries_total.store(0, Ordering::Relaxed);
        self.queries_cached.store(0, Ordering::Relaxed);
        self.files_indexed.store(0, Ordering::Relaxed);
        self.files_skipped.store(0, Ordering::Relaxed);
        self.files_failed.store(0, Ordering::Relaxed);
        self.embedding_calls.store(0, Ordering::Relaxed);
        self.embedding_cached.store(0, Ordering::Relaxed);
        self.graph_expansions.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub queries_total: u64,
    pub queries_cached: u64,
    pub files_indexed: u64,
    pub files_skipped: u64,
    pub files_failed: u64,
    pub embedding_calls: u64,
    pub embedding_cached: u64,
    pub graph_expansions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
    pub uptime_seconds: u64,
}

static GLOBAL_METRICS: OnceLock<MetricsCollector> = OnceLock::new();

pub fn global_metrics() -> &'static MetricsCollector {
    GLOBAL_METRICS.get_or_init(MetricsCollector::new)
}

pub fn print_metrics(snapshot: &MetricsSnapshot) {
    println!("Metrics:");
    println!("  Uptime: {}s", snapshot.uptime_seconds);
    println!("  Queries total: {}", snapshot.queries_total);
    println!("  Queries cached: {}", snapshot.queries_cached);
    println!("  Files indexed: {}", snapshot.files_indexed);
    println!("  Files skipped: {}", snapshot.files_skipped);
    println!("  Files failed: {}", snapshot.files_failed);
    println!("  Embedding calls: {}", snapshot.embedding_calls);
    println!("  Embedding cached: {}", snapshot.embedding_cached);
    println!("  Graph expansions: {}", snapshot.graph_expansions);
    println!("  Cache hits: {}", snapshot.cache_hits);
    println!("  Cache misses: {}", snapshot.cache_misses);
    println!("  Cache hit rate: {:.2}%", snapshot.cache_hit_rate * 100.0);
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
