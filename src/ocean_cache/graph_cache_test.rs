use crate::ocean_cache::graph_cache::GraphCache;
use crate::ocean_storage::graph_store::{Edge, Node, NodeType, RelationType};

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
        relation: RelationType::References,
        weight: 1.0,
        metadata: None,
    }
}

#[test]
fn test_graph_cache_hit() {
    let cache = GraphCache::new(10);
    let node = make_node("node1");
    let edge = make_edge("node1", "node2");
    let neighbors = vec![(node.clone(), edge.clone())];
    cache.set("node1".into(), neighbors.clone());
    let result = cache.get("node1");
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_graph_cache_miss() {
    let cache = GraphCache::new(10);
    assert!(cache.get("nonexistent").is_none());
}

#[test]
fn test_graph_cache_invalidate_node() {
    let cache = GraphCache::new(10);
    cache.set("node1".into(), vec![(make_node("n1"), make_edge("n1", "n2"))]);
    assert!(cache.get("node1").is_some());
    cache.invalidate_node("node1");
    assert!(cache.get("node1").is_none());
}

#[test]
fn test_graph_cache_invalidate_all() {
    let cache = GraphCache::new(10);
    cache.set("node1".into(), vec![(make_node("n1"), make_edge("n1", "n2"))]);
    cache.set("node2".into(), vec![(make_node("n2"), make_edge("n2", "n3"))]);
    cache.invalidate_all();
    assert!(cache.get("node1").is_none());
    assert!(cache.get("node2").is_none());
}

#[test]
fn test_graph_cache_lru_eviction() {
    let cache = GraphCache::new(2);
    cache.set("n1".into(), vec![(make_node("n1"), make_edge("n1", "n2"))]);
    cache.set("n2".into(), vec![(make_node("n2"), make_edge("n2", "n3"))]);
    assert!(cache.get("n1").is_some());
    assert!(cache.get("n2").is_some());
    cache.set("n3".into(), vec![(make_node("n3"), make_edge("n3", "n4"))]);
    assert!(cache.get("n3").is_some());
}
