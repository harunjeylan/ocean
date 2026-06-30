use crate::ocean_mcp::tools::doc_tools::{
    ReadParams, SearchParams, VerifyParams,
};
use crate::ocean_mcp::tools::{to_text, to_error, file_not_found, dir_not_found};

#[test]
fn test_file_not_found_response() {
    let result = file_not_found("nonexistent.pdf");
    let err_str = format!("{:?}", result);
    assert!(err_str.contains("not found") || err_str.contains("File"));
}

#[test]
fn test_dir_not_found_response() {
    let result = dir_not_found("nonexistent_dir");
    let err_str = format!("{:?}", result);
    assert!(err_str.contains("not found") || err_str.contains("Directory"));
}

#[test]
fn test_to_text_response() {
    let result = to_text("hello world".to_string());
    let err_str = format!("{:?}", result);
    assert!(err_str.contains("hello world"));
}

#[test]
fn test_to_error_response() {
    let result = to_error("something went wrong");
    let err_str = format!("{:?}", result);
    assert!(err_str.contains("something went wrong"));
}

#[test]
fn test_read_params_deserialize() {
    let json = serde_json::json!({
        "file_path": "test.pdf",
        "selector_type": "page",
        "selector_value": "1"
    });
    let params: ReadParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.file_path, "test.pdf");
    assert_eq!(params.selector_type.as_deref(), Some("page"));
    assert_eq!(params.selector_value.as_deref(), Some("1"));
}

#[test]
fn test_search_params_deserialize() {
    let json = serde_json::json!({
        "file_path": "test.pdf",
        "query": "keyword"
    });
    let params: SearchParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.file_path, "test.pdf");
    assert_eq!(params.query, "keyword");
}

#[test]
fn test_verify_params_deserialize() {
    let json = serde_json::json!({
        "file_path": "test.pdf",
        "expected_hash": "abc123"
    });
    let params: VerifyParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.file_path, "test.pdf");
    assert_eq!(params.expected_hash, "abc123");
}

#[test]
fn test_read_params_missing_file_path() {
    let json = serde_json::json!({
        "selector_type": "page"
    });
    let result: Result<ReadParams, _> = serde_json::from_value(json);
    assert!(result.is_err());
}
