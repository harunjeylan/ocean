use std::sync::atomic::Ordering;

use crate::ocean_cli::metrics::{MetricsCollector, global_metrics};

#[test]
fn test_metrics_new_all_zero() {
    let m = MetricsCollector::new();
    let snap = m.snapshot();
    assert_eq!(snap.queries_total, 0);
    assert_eq!(snap.files_indexed, 0);
    assert_eq!(snap.embedding_calls, 0);
}

#[test]
fn test_metrics_increment_counters() {
    let m = MetricsCollector::new();
    m.increment(&m.queries_total);
    m.increment(&m.queries_total);
    m.increment(&m.files_indexed);
    assert_eq!(m.queries_total.load(Ordering::Relaxed), 2);
    assert_eq!(m.files_indexed.load(Ordering::Relaxed), 1);
}

#[test]
fn test_metrics_snapshot_consistent() {
    let m = MetricsCollector::new();
    m.queries_total.store(10, Ordering::Relaxed);
    m.cache_hits.store(8, Ordering::Relaxed);
    m.cache_misses.store(2, Ordering::Relaxed);
    let snap = m.snapshot();
    assert_eq!(snap.queries_total, 10);
    assert_eq!(snap.cache_hit_rate, 0.8);
}

#[test]
fn test_metrics_snapshot_zero_division() {
    let m = MetricsCollector::new();
    let snap = m.snapshot();
    assert_eq!(snap.cache_hit_rate, 0.0);
}

#[test]
fn test_metrics_reset() {
    let m = MetricsCollector::new();
    m.queries_total.store(100, Ordering::Relaxed);
    m.reset();
    assert_eq!(m.queries_total.load(Ordering::Relaxed), 0);
}

#[test]
fn test_global_metrics_singleton() {
    let g1 = global_metrics();
    let g2 = global_metrics();
    assert!(std::ptr::eq(g1, g2));
}
