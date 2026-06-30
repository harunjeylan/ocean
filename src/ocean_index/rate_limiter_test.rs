use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crate::ocean_index::config::RateLimiterConfig;
use crate::ocean_index::rate_limiter::RateLimiter;

#[test]
fn rate_limiter_acquire_release() {
    let config = RateLimiterConfig { max_concurrent: 2, requests_per_minute: None };
    let limiter = RateLimiter::new(&config);
    let guard = limiter.acquire().unwrap();
    assert!(limiter.available_permits() <= 1);
    drop(guard);
    assert_eq!(limiter.available_permits(), 2);
}

#[test]
fn rate_limiter_try_acquire() {
    let config = RateLimiterConfig { max_concurrent: 1, requests_per_minute: None };
    let limiter = RateLimiter::new(&config);
    let g1 = limiter.try_acquire().unwrap();
    assert!(g1.is_some());
    let g2 = limiter.try_acquire().unwrap();
    assert!(g2.is_none());
    drop(g1);
    let g3 = limiter.try_acquire().unwrap();
    assert!(g3.is_some());
}

#[test]
fn rate_limiter_blocks_at_capacity() {
    let config = RateLimiterConfig { max_concurrent: 1, requests_per_minute: None };
    let limiter = Arc::new(RateLimiter::new(&config));
    let l2 = limiter.clone();
    let acquired = Arc::new(AtomicUsize::new(0));
    let a2 = acquired.clone();

    let handle = thread::spawn(move || {
        let _g = l2.acquire().unwrap();
        a2.store(1, Ordering::SeqCst);
        thread::sleep(std::time::Duration::from_millis(100));
    });

    thread::sleep(std::time::Duration::from_millis(20));
    // at this point the other thread should have acquired the permit
    let g2 = limiter.try_acquire().unwrap();
    assert!(g2.is_none());

    handle.join().unwrap();
    // after the other thread releases, we should be able to acquire
    let g3 = limiter.try_acquire().unwrap();
    assert!(g3.is_some());
}

#[test]
fn rate_limiter_available_permits() {
    let config = RateLimiterConfig { max_concurrent: 3, requests_per_minute: None };
    let limiter = RateLimiter::new(&config);
    assert_eq!(limiter.available_permits(), 3);
    let g1 = limiter.acquire().unwrap();
    assert_eq!(limiter.available_permits(), 2);
    let g2 = limiter.acquire().unwrap();
    assert_eq!(limiter.available_permits(), 1);
    drop(g1);
    assert_eq!(limiter.available_permits(), 2);
    drop(g2);
    assert_eq!(limiter.available_permits(), 3);
}
