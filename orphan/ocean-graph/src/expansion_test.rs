use crate::ocean_graph::expansion::ExpansionEngine;
use crate::ocean_graph::types::{Edge, EdgeDirection, Node, NodeType, RelationType};
use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::graph_store::GraphStore;
use crate::ocean_storage::SurrealGraphStore;
use std::sync::Arc;

fn init_store() -> Arc<SurrealGraphStore> {
    let config = StorageConfig::new(":memory:");
    Arc::new(SurrealGraphStore::new_memory(&config).unwrap())
}

fn build_small_graph(store: &SurrealGraphStore) {
    store.initialize_schema().unwrap();

    store.insert_nodes_batch(vec![
        (Node { id: "file:f1".into(), node_type: NodeType::File, ref_id: "f1".into(), label: None }, "f1".into()),
        (Node { id: "chunk:c1".into(), node_type: NodeType::Chunk, ref_id: "c1".into(), label: None }, "f1".into()),
        (Node { id: "chunk:c2".into(), node_type: NodeType::Chunk, ref_id: "c2".into(), label: None }, "f1".into()),
        (Node { id: "chunk:c3".into(), node_type: NodeType::Chunk, ref_id: "c3".into(), label: None }, "f1".into()),
        (Node { id: "heading:h1".into(), node_type: NodeType::Heading, ref_id: "h1".into(), label: Some("Intro".into()) }, "f1".into()),
    ]).unwrap();

    store.insert_edges_batch(vec![
        (Edge { from: "file:f1".into(), to: "chunk:c1".into(), relation: RelationType::Contains, weight: 1.0, metadata: None }, "f1".into()),
        (Edge { from: "file:f1".into(), to: "chunk:c2".into(), relation: RelationType::Contains, weight: 1.0, metadata: None }, "f1".into()),
        (Edge { from: "file:f1".into(), to: "chunk:c3".into(), relation: RelationType::Contains, weight: 1.0, metadata: None }, "f1".into()),
        (Edge { from: "chunk:c1".into(), to: "heading:h1".into(), relation: RelationType::BelongsTo, weight: 1.0, metadata: None }, "f1".into()),
        (Edge { from: "chunk:c2".into(), to: "heading:h1".into(), relation: RelationType::BelongsTo, weight: 1.0, metadata: None }, "f1".into()),
        (Edge { from: "chunk:c1".into(), to: "chunk:c2".into(), relation: RelationType::References, weight: 0.7, metadata: None }, "f1".into()),
    ]).unwrap();
}

#[test]
fn test_expand_depth_1() {
    let store = init_store();
    build_small_graph(&store);
    let engine = ExpansionEngine::new(store);

    let subgraph = engine.expand("file:f1", 1, EdgeDirection::Forward).unwrap();
    assert_eq!(subgraph.nodes.len(), 4);
    assert_eq!(subgraph.edges.len(), 3);
}

#[test]
fn test_expand_depth_2() {
    let store = init_store();
    build_small_graph(&store);
    let engine = ExpansionEngine::new(store);

    let subgraph = engine.expand("file:f1", 2, EdgeDirection::Both).unwrap();
    assert!(subgraph.nodes.len() >= 4);
    assert!(subgraph.edges.len() >= 3);
}

#[test]
fn test_expand_deduplication() {
    let store = init_store();
    build_small_graph(&store);
    let engine = ExpansionEngine::new(store);

    let subgraph = engine.expand_from_chunks(&["c1".to_string(), "c2".to_string()], 1).unwrap();

    let mut node_ids: Vec<String> = subgraph.nodes.iter().map(|n| n.id.clone()).collect();
    node_ids.sort();
    node_ids.dedup();
    assert_eq!(subgraph.nodes.len(), node_ids.len());
}

#[test]
fn test_expand_invalid_depth() {
    let store = init_store();
    store.initialize_schema().unwrap();
    let engine = ExpansionEngine::new(store);

    let result = engine.expand("file:f1", 0, EdgeDirection::Both);
    assert!(result.is_err());

    let result = engine.expand("file:f1", 6, EdgeDirection::Both);
    assert!(result.is_err());
}

#[test]
fn test_expand_nonexistent_node() {
    let store = init_store();
    store.initialize_schema().unwrap();
    let engine = ExpansionEngine::new(store);

    let result = engine.expand("nonexistent", 1, EdgeDirection::Both);
    assert!(result.is_err());
}

#[test]
fn test_find_path() {
    let store = init_store();
    build_small_graph(&store);
    let engine = ExpansionEngine::new(store);

    let path = engine.find_path("file:f1", "heading:h1", 5).unwrap();
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(!path.is_empty());
}

#[test]
fn test_find_path_disconnected() {
    let store = init_store();
    store.initialize_schema().unwrap();

    store.insert_nodes_batch(vec![
        (Node { id: "n1".into(), node_type: NodeType::Chunk, ref_id: "n1".into(), label: None }, "f1".into()),
        (Node { id: "n2".into(), node_type: NodeType::Chunk, ref_id: "n2".into(), label: None }, "f1".into()),
    ]).unwrap();

    let engine = ExpansionEngine::new(store);
    let path = engine.find_path("n1", "n2", 5).unwrap();
    assert!(path.is_none());
}

#[test]
fn test_find_path_same_node() {
    let store = init_store();
    store.initialize_schema().unwrap();

    store.insert_node(
        &Node { id: "n1".into(), node_type: NodeType::Chunk, ref_id: "n1".into(), label: None },
        "f1",
    ).unwrap();

    let engine = ExpansionEngine::new(store);
    let path = engine.find_path("n1", "n1", 5).unwrap();
    assert!(path.is_some());
    assert_eq!(path.unwrap().len(), 0);
}

#[test]
fn test_get_file_graph() {
    let store = init_store();
    build_small_graph(&store);
    let engine = ExpansionEngine::new(store);

    let subgraph = engine.get_file_graph("f1").unwrap();
    assert_eq!(subgraph.nodes.len(), 5);
    assert_eq!(subgraph.edges.len(), 6);
    assert_eq!(subgraph.depth, 0);
}

#[test]
fn test_cycle_safety() {
    let store = init_store();
    store.initialize_schema().unwrap();

    store.insert_nodes_batch(vec![
        (Node { id: "n1".into(), node_type: NodeType::Chunk, ref_id: "n1".into(), label: None }, "f1".into()),
        (Node { id: "n2".into(), node_type: NodeType::Chunk, ref_id: "n2".into(), label: None }, "f1".into()),
        (Node { id: "n3".into(), node_type: NodeType::Chunk, ref_id: "n3".into(), label: None }, "f1".into()),
    ]).unwrap();

    store.insert_edges_batch(vec![
        (Edge { from: "n1".into(), to: "n2".into(), relation: RelationType::References, weight: 1.0, metadata: None }, "f1".into()),
        (Edge { from: "n2".into(), to: "n3".into(), relation: RelationType::References, weight: 1.0, metadata: None }, "f1".into()),
        (Edge { from: "n3".into(), to: "n1".into(), relation: RelationType::References, weight: 1.0, metadata: None }, "f1".into()),
    ]).unwrap();

    let engine = ExpansionEngine::new(store);
    let subgraph = engine.expand("n1", 3, EdgeDirection::Both).unwrap();
    assert_eq!(subgraph.nodes.len(), 3);
}
