use crate::ocean_graph::types::{Node, NodeType, Edge, Subgraph};

use crate::ocean_parser::*;

pub fn print_graph_subgraph(subgraph: &Subgraph) {
    println!("Subgraph (seed: {}, depth: {}):", subgraph.seed_id, subgraph.depth);
    println!("  Nodes: {}", subgraph.nodes.len());
    println!("  Edges: {}", subgraph.edges.len());
    for node in &subgraph.nodes {
        let label = node.label.as_deref().unwrap_or("-");
        println!("  [{}] {}  label=\"{}\"", format!("{:?}", node.node_type), node.id, label);
    }
    for edge in &subgraph.edges {
        println!("  {} --({:?}, w={})--> {}", edge.from, edge.relation, edge.weight, edge.to);
    }
}

pub fn print_graph_node(node: &Node) {
    let label = node.label.as_deref().unwrap_or("-");
    println!("  [{}] {}  label=\"{}\"", format!("{:?}", node.node_type), node.id, label);
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

pub fn print_graph_expanded(subgraph: &Subgraph) {
    println!("Expanded from '{}' (depth: {}):", subgraph.seed_id, subgraph.depth);
    println!();
    for node in &subgraph.nodes {
        let label = node.label.as_deref().unwrap_or("-");
        println!("  [{}] {}  \"{}\"", format!("{:?}", node.node_type), node.id, label);
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
