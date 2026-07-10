use crate::ocean_mcp::tools::query_tools::{QueryParams, VectorStatusParams};

#[test]
fn test_query_params_deserialize() {
    let json = serde_json::json!({
        "query": "test query",
        "mode": "hybrid",
        "top_k": 5,
        "expand_depth": 1,
        "include_context": true
    });
    let params: QueryParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.query, "test query");
    assert_eq!(params.mode.as_deref(), Some("hybrid"));
    assert_eq!(params.top_k, Some(5));
    assert_eq!(params.expand_depth, Some(1));
    assert_eq!(params.include_context, Some(true));
}

#[test]
fn test_query_params_defaults() {
    let json = serde_json::json!({
        "query": "hello"
    });
    let params: QueryParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.query, "hello");
    assert!(params.mode.is_none());
    assert!(params.top_k.is_none());
}

#[test]
fn test_query_params_with_filters() {
    let json = serde_json::json!({
        "query": "search",
        "filter_file_id": "file-123",
        "filter_heading": "Introduction",
        "filter_block_type": "Text"
    });
    let params: QueryParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.filter_file_id.as_deref(), Some("file-123"));
    assert_eq!(params.filter_heading.as_deref(), Some("Introduction"));
    assert_eq!(params.filter_block_type.as_deref(), Some("Text"));
}

#[test]
fn test_vector_status_params_deserialize() {
    let json = serde_json::json!({
        "db_path": "/tmp/ocean",
        "provider": "ollama"
    });
    let params: VectorStatusParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.db_path.as_deref(), Some("/tmp/ocean"));
    assert_eq!(params.provider.as_deref(), Some("ollama"));
}

#[test]
fn test_vector_status_params_empty() {
    let json = serde_json::json!({});
    let params: VectorStatusParams = serde_json::from_value(json).unwrap();
    assert!(params.db_path.is_none());
    assert!(params.provider.is_none());
    assert!(params.model.is_none());
}

#[test]
fn test_query_params_invalid_mode() {
    let json = serde_json::json!({
        "query": "test"
    });
    let params: QueryParams = serde_json::from_value(json).unwrap();
    assert!(params.mode.is_none());
}
