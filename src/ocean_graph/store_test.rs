use crate::ocean_graph::store::GraphStore;
use crate::ocean_graph::types::{Edge, EdgeDirection, Node, NodeType, RelationType};

fn make_test_node(id: &str, node_type: NodeType, file_id: &str) -> (Node, String) {
    (
        Node {
            id: id.to_string(),
            node_type,
            ref_id: file_id.to_string(),
            label: None,
        },
        file_id.to_string(),
    )
}

fn make_test_edge(from: &str, to: &str, relation: RelationType, file_id: &str) -> (Edge, String) {
    (
        Edge {
            from: from.to_string(),
            to: to.to_string(),
            relation,
            weight: 1.0,
            metadata: None,
        },
        file_id.to_string(),
    )
}

#[test]
fn test_insert_and_get_node() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    let (node, fid) = make_test_node("file:abc-123", NodeType::File, "abc-123");
    store.insert_node(node.clone(), &fid).unwrap();

    let fetched = store.get_node("file:abc-123").unwrap().unwrap();
    assert_eq!(fetched.id, "file:abc-123");
    assert_eq!(fetched.node_type, NodeType::File);
}

#[test]
fn test_insert_and_get_node_by_ref() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    let (node, fid) = make_test_node("file:abc-123", NodeType::File, "abc-123");
    store.insert_node(node, &fid).unwrap();

    let fetched = store.get_node_by_ref("abc-123").unwrap().unwrap();
    assert_eq!(fetched.id, "file:abc-123");
}

#[test]
fn test_batch_insert_nodes_and_edges() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    let nodes = vec![
        make_test_node("file:f1", NodeType::File, "f1"),
        make_test_node("chunk:c1", NodeType::Chunk, "f1"),
        make_test_node("heading:h1", NodeType::Heading, "f1"),
    ];
    store.insert_nodes_batch(nodes).unwrap();

    let edges = vec![
        make_test_edge("file:f1", "chunk:c1", RelationType::Contains, "f1"),
        make_test_edge("chunk:c1", "heading:h1", RelationType::BelongsTo, "f1"),
    ];
    store.insert_edges_batch(edges).unwrap();

    assert_eq!(store.count_nodes().unwrap(), 3);
    assert_eq!(store.count_edges().unwrap(), 2);
}

#[test]
fn test_get_nodes_by_type() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    store.insert_nodes_batch(vec![
        make_test_node("file:f1", NodeType::File, "f1"),
        make_test_node("chunk:c1", NodeType::Chunk, "f1"),
        make_test_node("chunk:c2", NodeType::Chunk, "f1"),
    ]).unwrap();

    let chunks = store.get_nodes_by_type(NodeType::Chunk).unwrap();
    assert_eq!(chunks.len(), 2);

    let files = store.get_nodes_by_type(NodeType::File).unwrap();
    assert_eq!(files.len(), 1);
}

#[test]
fn test_get_neighbors() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    store.insert_nodes_batch(vec![
        make_test_node("file:f1", NodeType::File, "f1"),
        make_test_node("chunk:c1", NodeType::Chunk, "f1"),
        make_test_node("chunk:c2", NodeType::Chunk, "f1"),
    ]).unwrap();

    store.insert_edges_batch(vec![
        make_test_edge("file:f1", "chunk:c1", RelationType::Contains, "f1"),
        make_test_edge("file:f1", "chunk:c2", RelationType::Contains, "f1"),
        make_test_edge("chunk:c1", "chunk:c2", RelationType::References, "f1"),
    ]).unwrap();

    let neighbors = store.get_neighbors("file:f1").unwrap();
    assert_eq!(neighbors.len(), 2);

    let all_edges = store.get_edges("chunk:c1", EdgeDirection::Both).unwrap();
    assert_eq!(all_edges.len(), 2);
}

#[test]
fn test_get_edges_by_relation() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    store.insert_nodes_batch(vec![
        make_test_node("file:f1", NodeType::File, "f1"),
        make_test_node("chunk:c1", NodeType::Chunk, "f1"),
    ]).unwrap();

    store.insert_edges_batch(vec![
        make_test_edge("file:f1", "chunk:c1", RelationType::Contains, "f1"),
    ]).unwrap();

    let edges = store.get_edges_by_relation(RelationType::Contains).unwrap();
    assert_eq!(edges.len(), 1);

    let refs = store.get_edges_by_relation(RelationType::References).unwrap();
    assert_eq!(refs.len(), 0);
}

#[test]
fn test_delete_by_file() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    store.insert_nodes_batch(vec![
        make_test_node("file:f1", NodeType::File, "f1"),
        make_test_node("chunk:c1", NodeType::Chunk, "f1"),
        make_test_node("file:f2", NodeType::File, "f2"),
    ]).unwrap();

    store.insert_edges_batch(vec![
        make_test_edge("file:f1", "chunk:c1", RelationType::Contains, "f1"),
    ]).unwrap();

    assert_eq!(store.count_nodes().unwrap(), 3);
    assert_eq!(store.count_edges().unwrap(), 1);

    let deleted_nodes = store.delete_nodes_by_file("f1").unwrap();
    assert_eq!(deleted_nodes, 2);

    let deleted_edges = store.delete_edges_by_file("f1").unwrap();
    assert_eq!(deleted_edges, 1);

    assert_eq!(store.count_nodes().unwrap(), 1);
    assert_eq!(store.count_edges().unwrap(), 0);
}

#[test]
fn test_clear() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    store.insert_nodes_batch(vec![
        make_test_node("file:f1", NodeType::File, "f1"),
    ]).unwrap();

    assert_eq!(store.count_nodes().unwrap(), 1);
    store.clear().unwrap();
    assert_eq!(store.count_nodes().unwrap(), 0);
    assert_eq!(store.count_edges().unwrap(), 0);
}

#[test]
fn test_schema_idempotency() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();
    store.initialize_schema().unwrap();
    store.initialize_schema().unwrap();
}

#[test]
fn test_node_not_found() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    let result = store.get_node("nonexistent").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_get_edges_direction() {
    let store = GraphStore::new_memory().unwrap();
    store.initialize_schema().unwrap();

    store.insert_nodes_batch(vec![
        make_test_node("n1", NodeType::Chunk, "f1"),
        make_test_node("n2", NodeType::Chunk, "f1"),
        make_test_node("n3", NodeType::Chunk, "f1"),
    ]).unwrap();

    store.insert_edges_batch(vec![
        make_test_edge("n1", "n2", RelationType::References, "f1"),
        make_test_edge("n3", "n1", RelationType::References, "f1"),
    ]).unwrap();

    let forward = store.get_edges("n1", EdgeDirection::Forward).unwrap();
    assert_eq!(forward.len(), 1);
    assert_eq!(forward[0].to, "n2");

    let backward = store.get_edges("n1", EdgeDirection::Backward).unwrap();
    assert_eq!(backward.len(), 1);
    assert_eq!(backward[0].from, "n3");

    let both = store.get_edges("n1", EdgeDirection::Both).unwrap();
    assert_eq!(both.len(), 2);
}
