use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::graph_store::{Edge, EdgeDirection, GraphStore, Node, NodeType, RelationType};
use crate::ocean_storage::SurrealGraphStore;

fn make_node(id: &str) -> Node {
    Node {
        id: id.to_string(),
        node_type: NodeType::Chunk,
        ref_id: id.to_string(),
        label: None,
    }
}

fn make_edge(from: &str, to: &str) -> Edge {
    Edge {
        from: from.to_string(),
        to: to.to_string(),
        relation: RelationType::Contains,
        weight: 1.0,
        metadata: None,
    }
}

#[test]
fn test_graph_store_insert_and_count() {
    let config = StorageConfig::new(":memory:");
    let store = SurrealGraphStore::new_memory(&config).unwrap();
    store.initialize_schema().unwrap();

    let n = make_node("g1");
    store.insert_node(&n, "f1").unwrap();

    assert_eq!(store.count_nodes().unwrap(), 1);
}

#[test]
fn test_graph_store_insert_edge() {
    let config = StorageConfig::new(":memory:");
    let store = SurrealGraphStore::new_memory(&config).unwrap();
    store.initialize_schema().unwrap();

    let n1 = make_node("g1");
    let n2 = make_node("g2");
    store.insert_node(&n1, "f1").unwrap();
    store.insert_node(&n2, "f1").unwrap();

    let e = make_edge("g1", "g2");
    store.insert_edge(&e, "f1").unwrap();

    assert_eq!(store.count_edges().unwrap(), 1);
}

#[test]
fn test_graph_store_get_edges() {
    let config = StorageConfig::new(":memory:");
    let store = SurrealGraphStore::new_memory(&config).unwrap();
    store.initialize_schema().unwrap();

    let n1 = make_node("g1");
    let n2 = make_node("g2");
    store.insert_node(&n1, "f1").unwrap();
    store.insert_node(&n2, "f1").unwrap();

    let e = make_edge("g1", "g2");
    store.insert_edge(&e, "f1").unwrap();

    let edges = store.get_edges("g1", EdgeDirection::Both).unwrap();
    assert_eq!(edges.len(), 1);
}
