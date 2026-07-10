use std::sync::Arc;

use crate::ocean_graph::ExpansionEngine;
use crate::ocean_graph::types::{Edge, Subgraph};
use crate::ocean_graph::NodeType;
use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::graph_store::{EdgeDirection, GraphStore};
use crate::ocean_storage::SurrealGraphStore;

use super::types::ApiError;

fn open_store(db_path: &str) -> Result<SurrealGraphStore, ApiError> {
    let config = StorageConfig::new(db_path);
    SurrealGraphStore::new_persistent_at(db_path, &config)
        .map_err(|e| ApiError::FsError(format!("Failed to open graph store: {}", e)))
}

#[derive(Debug, Clone)]
pub struct GraphInfoResult {
    pub node_count: u64,
    pub edge_count: u64,
    pub type_breakdown: Vec<(NodeType, usize)>,
}

#[derive(Debug, Clone)]
pub struct GraphStatsResult {
    pub node_count: u64,
    pub edge_count: u64,
}

pub fn graph_info(file: &str, db_path: &str) -> Result<GraphInfoResult, ApiError> {
    let store = open_store(db_path)?;

    let metas = crate::ocean_fs::scan_dir(file)
        .map_err(|e| ApiError::FsError(format!("Scan failed: {}", e)))?;
    if metas.is_empty() {
        return Err(ApiError::DocError(format!("No supported files found matching: {}", file)));
    }
    let file_id = &metas[0].id;

    let subgraph = ExpansionEngine::new(Arc::new(store))
        .get_file_graph(file_id)
        .map_err(|e| ApiError::FsError(format!("Failed to get file graph: {}", e)))?;

    let mut type_counts: std::collections::HashMap<NodeType, usize> = std::collections::HashMap::new();
    for node in &subgraph.nodes {
        *type_counts.entry(node.node_type.clone()).or_insert(0) += 1;
    }

    Ok(GraphInfoResult {
        node_count: subgraph.nodes.len() as u64,
        edge_count: subgraph.edges.len() as u64,
        type_breakdown: type_counts.into_iter().collect(),
    })
}

pub fn graph_expand(node_id: &str, depth: usize, direction: &str, db_path: &str) -> Result<Subgraph, ApiError> {
    let store = open_store(db_path)?;

    let dir = match direction.to_lowercase().as_str() {
        "forward" => EdgeDirection::Forward,
        "backward" => EdgeDirection::Backward,
        "both" => EdgeDirection::Both,
        other => return Err(ApiError::ConfigError(format!("invalid direction '{}'. Use: forward, backward, both", other))),
    };

    let engine = ExpansionEngine::new(Arc::new(store));
    let subgraph = engine
        .expand(node_id, depth, dir)
        .map_err(|e| ApiError::FsError(format!("Expansion failed: {}", e)))?;

    Ok(subgraph)
}

pub fn graph_path(from: &str, to: &str, max_depth: usize, db_path: &str) -> Result<Option<Vec<Edge>>, ApiError> {
    let store = open_store(db_path)?;

    let engine = ExpansionEngine::new(Arc::new(store));
    let path = engine
        .find_path(from, to, max_depth)
        .map_err(|e| ApiError::FsError(format!("Path find failed: {}", e)))?;

    Ok(path)
}

pub fn graph_stats(db_path: &str) -> Result<GraphStatsResult, ApiError> {
    let store = open_store(db_path)?;

    let node_count = store.count_nodes().map_err(|e| ApiError::FsError(format!("Count failed: {}", e)))?;
    let edge_count = store.count_edges().map_err(|e| ApiError::FsError(format!("Count failed: {}", e)))?;

    Ok(GraphStatsResult {
        node_count,
        edge_count,
    })
}
