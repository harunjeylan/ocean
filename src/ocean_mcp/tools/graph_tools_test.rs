use crate::ocean_mcp::tools::graph_tools::{GraphInfoParams, GraphExpandParams, GraphStatsParams};

#[test]
fn test_graph_info_params() {
    let json = serde_json::json!({
        "file_path": "doc.pdf",
        "db_path": "/tmp/ocean"
    });
    let params: GraphInfoParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.file_path, "doc.pdf");
    assert_eq!(params.db_path.as_deref(), Some("/tmp/ocean"));
}

#[test]
fn test_graph_info_params_required_only() {
    let json = serde_json::json!({
        "file_path": "doc.pdf"
    });
    let params: GraphInfoParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.file_path, "doc.pdf");
    assert!(params.db_path.is_none());
}

#[test]
fn test_graph_expand_params() {
    let json = serde_json::json!({
        "node_id": "node-1",
        "depth": 3,
        "direction": "forward",
        "db_path": "/tmp/ocean"
    });
    let params: GraphExpandParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.node_id, "node-1");
    assert_eq!(params.depth, Some(3));
    assert_eq!(params.direction.as_deref(), Some("forward"));
}

#[test]
fn test_graph_expand_defaults() {
    let json = serde_json::json!({
        "node_id": "node-1"
    });
    let params: GraphExpandParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.node_id, "node-1");
    assert!(params.depth.is_none());
    assert!(params.direction.is_none());
}

#[test]
fn test_graph_stats_params() {
    let json = serde_json::json!({
        "db_path": "/tmp/ocean"
    });
    let params: GraphStatsParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.db_path.as_deref(), Some("/tmp/ocean"));
}

#[test]
fn test_graph_stats_params_empty() {
    let json = serde_json::json!({});
    let params: GraphStatsParams = serde_json::from_value(json).unwrap();
    assert!(params.db_path.is_none());
}

#[test]
fn test_graph_info_missing_file_path() {
    let json = serde_json::json!({});
    let result: Result<GraphInfoParams, _> = serde_json::from_value(json);
    assert!(result.is_err());
}

#[test]
fn test_graph_expand_missing_node_id() {
    let json = serde_json::json!({});
    let result: Result<GraphExpandParams, _> = serde_json::from_value(json);
    assert!(result.is_err());
}
