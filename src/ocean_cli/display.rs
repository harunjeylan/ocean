use crate::ocean_graph::types::{Node, NodeType, Edge, Subgraph};
use crate::ocean_query::types::QueryResult;

use crate::ocean_parser::*;

pub fn print_graph_subgraph(subgraph: &Subgraph) {
    println!("Subgraph (seed: {}, depth: {}):", subgraph.seed_id, subgraph.depth);
    println!("  Nodes: {}", subgraph.nodes.len());
    println!("  Edges: {}", subgraph.edges.len());
    for node in &subgraph.nodes {
        let label = node.label.as_deref().unwrap_or("-");
        println!("  [{:?}] {}  label=\"{}\"", node.node_type, node.id, label);
    }
    for edge in &subgraph.edges {
        println!("  {} --({:?}, w={})--> {}", edge.from, edge.relation, edge.weight, edge.to);
    }
}

pub fn print_graph_node(node: &Node) {
    let label = node.label.as_deref().unwrap_or("-");
    println!("  [{:?}] {}  label=\"{}\"", node.node_type, node.id, label);
}

pub fn print_graph_info(node_count: u64, edge_count: u64, type_breakdown: Vec<(NodeType, usize)>) {
    println!("Graph Info:");
    println!("  Total nodes: {}", node_count);
    println!("  Total edges: {}", edge_count);
    println!("  Breakdown by type:");
    for (nt, count) in &type_breakdown {
        println!("    {:?}: {}", nt, count);
    }
}

pub fn print_graph_path(path: &[Edge]) {
    if path.is_empty() {
        println!("Direct edge (same node)");
        return;
    }
    println!("Path ({} hops):", path.len());
    for (i, edge) in path.iter().enumerate() {
        println!(
            "  {}. {} --({:?}, w={})--> {}",
            i + 1,
            edge.from,
            edge.relation,
            edge.weight,
            edge.to
        );
    }
}

pub fn print_graph_stats(node_count: u64, edge_count: u64, type_counts: Vec<(String, u64)>) {
    println!("Graph Stats:");
    println!("  Total nodes: {}", node_count);
    println!("  Total edges: {}", edge_count);
    println!("  By type:");
    for (type_name, count) in &type_counts {
        println!("    {}: {}", type_name, count);
    }
}

pub fn print_graph_status(
    db_path: &str,
    accessible: bool,
    schema_initialized: bool,
    node_count: u64,
    edge_count: u64,
    type_breakdown: Vec<(String, u64)>,
) {
    println!("Graph Status");
    println!("  Database: {}", db_path);
    println!("  Accessible: {}", if accessible { "Yes" } else { "No" });
    if accessible {
        println!("  Schema: {}", if schema_initialized { "Initialized" } else { "Not initialized" });
        println!("  Nodes: {}", node_count);
        println!("  Edges: {}", edge_count);
        if !type_breakdown.is_empty() {
            println!("  By type:");
            for (type_name, count) in &type_breakdown {
                println!("    {}: {}", type_name, count);
            }
        }
        if node_count == 0 && edge_count == 0 {
            println!("  └─ Run `ocean index .` to build the graph");
        }
    } else {
        println!("  └─ Database not found or inaccessible. Run `ocean index .` first");
    }
}

pub fn print_vector_status(
    db_path: &str,
    accessible: bool,
    schema_initialized: bool,
    chunk_count: u64,
    provider: &str,
    model: &str,
    dimension: usize,
    api_key_set: bool,
    api_key_required: bool,
    connection_result: Option<String>,
    connection_ms: Option<u64>,
) {
    println!("Vector Status");
    println!("  Database: {}", db_path);
    println!("  Accessible: {}", if accessible { "Yes" } else { "No" });
    if accessible {
        println!("  Schema: {}", if schema_initialized { "Initialized" } else { "Not initialized" });
        println!("  Indexed chunks: {}", chunk_count);
        println!(
            "  Embedder: {} / {} (dim={})",
            provider, model, dimension
        );
        if api_key_required && !api_key_set {
            println!("  API key: Not set (required for {})", provider);
        }
        if let Some(result) = connection_result {
            if let Some(ms) = connection_ms {
                println!("  Connection: {} ({}ms)", result, ms);
            } else {
                println!("  Connection: {}", result);
            }
        }
        if !schema_initialized || chunk_count == 0 {
            println!("  └─ Run `ocean index .` to index documents");
        }
    } else {
        println!("  └─ Database not found or inaccessible. Run `ocean index .` first");
    }
}

pub fn print_graph_expanded(subgraph: &Subgraph) {
    println!("Expanded from '{}' (depth: {}):", subgraph.seed_id, subgraph.depth);
    println!();
    for node in &subgraph.nodes {
        let label = node.label.as_deref().unwrap_or("-");
        println!("  [{:?}] {}  \"{}\"", node.node_type, node.id, label);
    }
    println!();
    println!("Edges:");
    for edge in &subgraph.edges {
        println!(
            "  {}  --{:?}-->  {}  (w: {})",
            edge.from, edge.relation, edge.to, edge.weight
        );
    }
}

pub fn print_meta(meta: &DocumentMetadata) {
    println!("Metadata:");
    println!("  Path:    {}", meta.path.display());
    println!("  Format:  {:?}", meta.format);
    println!("  Size:    {} bytes", meta.size);
    if let Some(ref t) = meta.title { println!("  Title:   {}", t); }
    if let Some(ref a) = meta.author { println!("  Author:  {}", a); }
    if let Some(ref c) = meta.created { println!("  Created: {}", c); }
    if let Some(ref m) = meta.modified { println!("  Modified: {}", m); }
    if let Some(ref p) = meta.page_count { println!("  Pages:   {}", p); }
}

pub fn print_outline(outline: &Outline, indent: usize) {
    for entry in &outline.entries {
        println!("{:indent$}- [L{}] {}  ({:?})", "", entry.level, entry.label, entry.selector, indent = indent);
        if !entry.children.is_empty() {
            let child_outline = Outline { entries: entry.children.clone() };
            print_outline(&child_outline, indent + 2);
        }
    }
}

pub fn print_query_result(result: &QueryResult, verbose: bool) {
    if result.results.is_empty() {
        println!("No results found.");
        return;
    }

    println!("Top {} results:", result.results.len());
    for (i, r) in result.results.iter().enumerate() {
        let content_short = if r.content.len() > 120 {
            format!("{}...", &r.content[..120])
        } else {
            r.content.clone()
        };
        let heading = r.heading.as_deref().unwrap_or("(none)");
        let graph_info = match r.graph_score {
            Some(gs) => format!(" graph={:.4}", gs),
            None => String::new(),
        };
        let score_info = if let (Some(vs), Some(fs)) = (r.vector_score, r.fts_score) {
            format!("score={:.4} (vec={:.4}, fts={:.4}){}", r.score, vs, fs, graph_info)
        } else {
            format!("score={:.4}{}", r.score, graph_info)
        };
        println!(
            "  {}. {}  file={}  heading=\"{}\"",
            i + 1,
            score_info,
            &r.file_id[..r.file_id.len().min(8)],
            heading,
        );
        println!("     \"{}\"", content_short);
    }

    if !result.context_windows.is_empty() {
        println!();
        println!("--- Context Windows ---");
        for (i, cw) in result.context_windows.iter().enumerate() {
            println!("Window {} (anchor: {}, tokens: {}):", i + 1, cw.anchor_chunk_id, cw.total_tokens);
            for chunk in &cw.chunks {
                let prefix = if chunk.distance_from_anchor == 0 {
                    "[*]"
                } else if chunk.distance_from_anchor < 0 {
                    "[↑]"
                } else {
                    "[↓]"
                };
                let short = if chunk.content.len() > 80 {
                    format!("{}...", &chunk.content[..80])
                } else {
                    chunk.content.clone()
                };
                println!("  {} {} (dist={})", prefix, short, chunk.distance_from_anchor);
            }
        }
    }

    if verbose {
        let meta = &result.execution;
        println!();
        println!("--- Execution ---");
        println!("Mode: {:?}", meta.query_mode);
        println!("Total: {} results in {}ms", meta.total_results, meta.total_time_ms);
        println!("Vector search: {}ms", meta.vector_search_time_ms);
        println!("Fusion: {}ms", meta.fusion_time_ms);
        if let Some(gt) = meta.graph_expand_time_ms {
            println!("Graph expand: {}ms", gt);
        }
    }
}

pub fn print_read_result(result: ReadResult) {
    match result {
        ReadResult::Text(t) => println!("{}", t),
        ReadResult::Table { headers, rows } => {
            if !headers.is_empty() {
                println!("{}", headers.join(" | "));
                println!("{}", vec!["---"; headers.len()].join(" | "));
            }
            for row in &rows {
                println!("{}", row.join(" | "));
            }
        }
        ReadResult::Image { bytes, format, caption } => {
            println!("Image: {} bytes, format: {:?}", bytes.len(), format);
            if let Some(ref cap) = caption { println!("Caption: {}", cap); }
        }
        ReadResult::Metadata(meta) => print_meta(&meta),
        ReadResult::Outline(ref outline) => print_outline(outline, 0),
        ReadResult::Page { number, text } => {
            println!("--- Page {} ---", number);
            println!("{}", text);
        }
        ReadResult::Slide { number, title, content } => {
            println!("--- Slide {} ---", number);
            if let Some(ref t) = title { println!("Title: {}", t); }
            println!("{}", content);
        }
        ReadResult::Sheet { name, rows } => {
            println!("--- Sheet: {} ---", name);
            for row in &rows {
                println!("{}", row.join(" | "));
            }
        }
        ReadResult::CellValue(v) => println!("{}", v),
        ReadResult::MatchResult(matches) => {
            for m in matches {
                println!("  {:?}: \"{}\" (score: {})", m.selector, m.text, m.score);
                if !m.context.is_empty() {
                    println!("    context: {}", m.context);
                }
            }
        }
    }
}
