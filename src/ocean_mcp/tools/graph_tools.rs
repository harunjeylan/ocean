use rmcp::model::{CallToolResult, ContentBlock};
use serde::Deserialize;

use crate::ocean_api;

use super::to_text;

#[derive(Debug, Deserialize)]
pub struct GraphInfoParams {
    pub file_path: String,
    pub db_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GraphExpandParams {
    pub node_id: String,
    pub depth: Option<usize>,
    pub direction: Option<String>,
    pub db_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GraphStatsParams {
    pub db_path: Option<String>,
}

pub async fn handle_graph_info(params: GraphInfoParams) -> CallToolResult {
    let db_path = crate::ocean_cli::config::resolve_db_path(params.db_path.as_deref(), None);
    let file = params.file_path.clone();
    let file_for_closure = file.clone();

    match tokio::task::spawn_blocking(move || ocean_api::graph::graph_info(&file_for_closure, &db_path)).await {
        Ok(Ok(info)) => {
            let mut lines = vec![
                format!("Graph info for '{}'", file),
                format!("  Nodes: {}", info.node_count),
                format!("  Edges: {}", info.edge_count),
                format!("  Type breakdown:"),
            ];
            for (nt, count) in &info.type_breakdown {
                lines.push(format!("    {:?}: {}", nt, count));
            }
            to_text(lines.join("\n"))
        }
        Ok(Err(e)) => {
            let msg = e.to_string();
            if msg.contains("Failed to open graph store") || msg.contains("not found") {
                CallToolResult::error(vec![ContentBlock::text(format!("{}. Ensure 'ocean index' has been run with graph enabled.", msg))])
            } else {
                CallToolResult::error(vec![ContentBlock::text(msg)])
            }
        }
        Err(e) => CallToolResult::error(vec![ContentBlock::text(format!("Task failed: {}", e))]),
    }
}

pub async fn handle_graph_expand(params: GraphExpandParams) -> CallToolResult {
    let db_path = crate::ocean_cli::config::resolve_db_path(params.db_path.as_deref(), None);
    let node_id = params.node_id.clone();
    let depth = params.depth.unwrap_or(2);
    let direction = params.direction.clone().unwrap_or_else(|| "both".to_string());
    let node_id_for_closure = node_id.clone();
    let direction_for_closure = direction.clone();

    match tokio::task::spawn_blocking(move || ocean_api::graph::graph_expand(&node_id_for_closure, depth, &direction_for_closure, &db_path)).await {
        Ok(Ok(subgraph)) => {
            let mut lines = vec![
                format!("Graph expansion from '{}' (depth={}, direction={})", node_id, subgraph.depth, direction),
                format!("  Nodes: {}", subgraph.nodes.len()),
                format!("  Edges: {}", subgraph.edges.len()),
                String::new(),
                "Nodes:".to_string(),
            ];
            for node in &subgraph.nodes {
                let label = node.label.as_deref().unwrap_or("<no label>");
                lines.push(format!("  [{}] {:?} — {} (ref: {})", node.id, node.node_type, label, node.ref_id));
            }
            lines.push(String::new());
            lines.push("Edges:".to_string());
            for edge in &subgraph.edges {
                lines.push(format!("  {} -> {} ({:?}, weight={})", edge.from, edge.to, edge.relation, edge.weight));
                if let Some(ref meta) = edge.metadata {
                    lines.push(format!("    metadata: {}", meta));
                }
            }
            to_text(lines.join("\n"))
        }
        Ok(Err(e)) => {
            let msg = e.to_string();
            if msg.contains("Failed to open graph store") {
                CallToolResult::error(vec![ContentBlock::text(format!("{}. Ensure 'ocean index' has been run with graph enabled.", msg))])
            } else {
                CallToolResult::error(vec![ContentBlock::text(msg)])
            }
        }
        Err(e) => CallToolResult::error(vec![ContentBlock::text(format!("Task failed: {}", e))]),
    }
}

pub async fn handle_graph_stats(params: GraphStatsParams) -> CallToolResult {
    let db_path = crate::ocean_cli::config::resolve_db_path(params.db_path.as_deref(), None);

    match tokio::task::spawn_blocking(move || ocean_api::graph::graph_stats(&db_path)).await {
        Ok(Ok(stats)) => {
            let lines = vec![
                format!("Graph Stats"),
                format!("  Total nodes: {}", stats.node_count),
                format!("  Total edges: {}", stats.edge_count),
            ];
            to_text(lines.join("\n"))
        }
        Ok(Err(e)) => {
            let msg = e.to_string();
            if msg.contains("Failed to open graph store") || msg.contains("not found") {
                CallToolResult::error(vec![ContentBlock::text(format!("{}. Ensure 'ocean index' has been run with graph enabled.", msg))])
            } else {
                CallToolResult::error(vec![ContentBlock::text(msg)])
            }
        }
        Err(e) => CallToolResult::error(vec![ContentBlock::text(format!("Task failed: {}", e))]),
    }
}
