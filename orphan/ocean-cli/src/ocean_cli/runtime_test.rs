use crate::ocean_cli::runtime::RuntimeMode;

#[test]
fn test_runtime_mode_desktop_defaults() {
    let mode = RuntimeMode::Desktop;
    let d = mode.defaults();
    assert_eq!(d.max_ai_concurrent, Some(2));
    assert_eq!(d.embedding_cache_size, Some(1000));
    assert_eq!(d.query_cache_size, Some(100));
    assert_eq!(d.max_in_flight, Some(10));
    assert_eq!(d.max_queue_size, Some(10_000));
    assert_eq!(d.embedding_batch_size, Some(10));
}

#[test]
fn test_runtime_mode_server_defaults() {
    let mode = RuntimeMode::Server;
    let d = mode.defaults();
    assert_eq!(d.max_ai_concurrent, Some(4));
    assert_eq!(d.embedding_cache_size, Some(5000));
    assert_eq!(d.query_cache_size, Some(500));
    assert_eq!(d.max_in_flight, Some(50));
    assert_eq!(d.max_queue_size, Some(100_000));
    assert_eq!(d.embedding_batch_size, Some(32));
}

#[test]
fn test_runtime_mode_embedded_defaults() {
    let mode = RuntimeMode::Embedded;
    let d = mode.defaults();
    assert_eq!(d.io_threads, Some(2));
    assert_eq!(d.cpu_threads, Some(1));
    assert_eq!(d.max_ai_concurrent, Some(1));
    assert_eq!(d.embedding_cache_size, Some(100));
    assert_eq!(d.query_cache_size, Some(20));
    assert_eq!(d.max_in_flight, Some(3));
    assert_eq!(d.max_queue_size, Some(1_000));
    assert_eq!(d.embedding_batch_size, Some(4));
}

#[test]
fn test_runtime_mode_from_mode_str() {
    assert_eq!(RuntimeMode::from_mode_str("desktop"), Some(RuntimeMode::Desktop));
    assert_eq!(RuntimeMode::from_mode_str("server"), Some(RuntimeMode::Server));
    assert_eq!(RuntimeMode::from_mode_str("embedded"), Some(RuntimeMode::Embedded));
    assert_eq!(RuntimeMode::from_mode_str("unknown"), None);
    assert_eq!(RuntimeMode::from_mode_str("DESKTOP"), Some(RuntimeMode::Desktop));
}

#[test]
fn test_runtime_mode_resolve() {
    assert_eq!(RuntimeMode::resolve(Some("desktop"), None), RuntimeMode::Desktop);
    assert_eq!(RuntimeMode::resolve(Some("server"), None), RuntimeMode::Server);
    assert_eq!(RuntimeMode::resolve(None, Some("embedded")), RuntimeMode::Embedded);
}

#[test]
fn test_runtime_mode_auto_detect() {
    let mode = RuntimeMode::auto_detect();
    assert!(mode == RuntimeMode::Desktop || mode == RuntimeMode::Server || mode == RuntimeMode::Embedded);
}
