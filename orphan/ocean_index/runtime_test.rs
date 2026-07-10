use crate::ocean_index::runtime::RetryPolicy;

#[test]
fn retry_policy_defaults() {
    let p = RetryPolicy::default();
    assert_eq!(p.max_retries, 3);
    assert_eq!(p.initial_backoff_ms, 100);
    assert_eq!(p.max_backoff_ms, 30_000);
}

#[test]
fn retry_policy_next_delay_geometric() {
    let p = RetryPolicy::new(3, 100, 30_000);
    assert_eq!(p.next_delay(0).as_millis(), 100);
    assert_eq!(p.next_delay(1).as_millis(), 200);
    assert_eq!(p.next_delay(2).as_millis(), 400);
    assert_eq!(p.next_delay(3).as_millis(), 800);
}

#[test]
fn retry_policy_next_delay_clamps_to_max() {
    let p = RetryPolicy::new(3, 100, 500);
    assert_eq!(p.next_delay(0).as_millis(), 100);
    assert_eq!(p.next_delay(1).as_millis(), 200);
    assert_eq!(p.next_delay(2).as_millis(), 400);
    assert_eq!(p.next_delay(3).as_millis(), 500);
}

#[test]
fn retry_policy_is_transient_timeout() {
    let p = RetryPolicy::default();
    let err = std::io::Error::new(std::io::ErrorKind::TimedOut, "connection timed out");
    assert!(p.is_transient(&err));
}

#[test]
fn retry_policy_is_transient_rate_limit() {
    let p = RetryPolicy::default();
    let err = std::io::Error::new(std::io::ErrorKind::Other, "429 too many requests");
    assert!(p.is_transient(&err));
}

#[test]
fn retry_policy_is_transient_not_transient() {
    let p = RetryPolicy::default();
    let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "invalid api key");
    assert!(!p.is_transient(&err));
}

#[test]
fn retry_policy_is_transient_corrupt_file() {
    let p = RetryPolicy::default();
    let err = std::io::Error::new(std::io::ErrorKind::InvalidData, "corrupt file: invalid header");
    assert!(!p.is_transient(&err));
}

#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TestError {}

#[test]
fn retry_policy_is_transient_service_unavailable() {
    let p = RetryPolicy::default();
    let err = TestError("service temporarily unavailable".into());
    assert!(p.is_transient(&err));
}

#[test]
fn retry_policy_is_transient_conflict() {
    let p = RetryPolicy::default();
    let err = TestError("storage conflict detected".into());
    assert!(p.is_transient(&err));
}
