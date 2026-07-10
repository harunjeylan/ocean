use crate::ocean_chunk::{Chunk, ChunkType};
use crate::ocean_graph::builder::GraphBuilder;
use crate::ocean_graph::types::{GraphConfig, NodeType, RelationType};

fn make_chunk(id: &str, file_id: &str, content: &str, heading: Option<&str>) -> Chunk {
    Chunk {
        id: id.to_string(),
        file_id: file_id.to_string(),
        content: content.to_string(),
        heading: heading.map(|s| s.to_string()),
        page: None,
        slide: None,
        sheet: None,
        block_type: ChunkType::Text,
        start_offset: None,
        end_offset: None,
    }
}

#[test]
fn test_structural_empty_chunks() {
    let chunks: Vec<Chunk> = vec![];
    let (nodes, edges) = GraphBuilder::structural(&chunks, "file-1");

    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].node_type, NodeType::File);
    assert_eq!(nodes[0].ref_id, "file-1");
    assert_eq!(edges.len(), 0);
}

#[test]
fn test_structural_single_chunk_with_heading() {
    let chunks = vec![make_chunk("chunk-1", "file-1", "content", Some("Introduction"))];
    let (nodes, edges) = GraphBuilder::structural(&chunks, "file-1");

    assert_eq!(nodes.len(), 3);
    assert!(nodes.iter().any(|n| n.node_type == NodeType::File));
    assert!(nodes.iter().any(|n| n.node_type == NodeType::Chunk));
    assert!(nodes.iter().any(|n| n.node_type == NodeType::Heading));

    assert_eq!(edges.len(), 3);
    assert!(edges.iter().any(|e| e.relation == RelationType::Contains));
    assert!(edges.iter().any(|e| e.relation == RelationType::BelongsTo));

    let contains = edges.iter().find(|e| e.relation == RelationType::Contains).unwrap();
    assert_eq!(contains.from, "file:file-1");
    assert_eq!(contains.to, "chunk:chunk-1");
}

#[test]
fn test_structural_multiple_chunks_same_heading() {
    let chunks = vec![
        make_chunk("chunk-1", "file-1", "content1", Some("Overview")),
        make_chunk("chunk-2", "file-1", "content2", Some("Overview")),
    ];
    let (nodes, edges) = GraphBuilder::structural(&chunks, "file-1");

    let heading_nodes: Vec<_> = nodes.iter().filter(|n| n.node_type == NodeType::Heading).collect();
    assert_eq!(heading_nodes.len(), 1);

    let belongs_to: Vec<_> = edges.iter().filter(|e| e.relation == RelationType::BelongsTo && e.from.starts_with("chunk:") && e.to.starts_with("heading:")).collect();
    assert_eq!(belongs_to.len(), 2);
}

#[test]
fn test_structural_multiple_files() {
    let chunks_a = vec![make_chunk("c1", "f1", "content", None)];
    let chunks_b = vec![make_chunk("c2", "f2", "content", None)];

    let (nodes_a, _) = GraphBuilder::structural(&chunks_a, "f1");
    let (nodes_b, _) = GraphBuilder::structural(&chunks_b, "f2");

    assert_eq!(nodes_a.iter().filter(|n| n.node_type == NodeType::File).count(), 1);
    assert_eq!(nodes_b.iter().filter(|n| n.node_type == NodeType::File).count(), 1);
    assert!(nodes_a.iter().any(|n| n.id == "file:f1"));
    assert!(nodes_b.iter().any(|n| n.id == "file:f2"));
}

#[test]
fn test_structural_deterministic_ids() {
    let chunks = vec![make_chunk("c1", "f1", "content", Some("Methods"))];

    let (nodes1, edges1) = GraphBuilder::structural(&chunks, "f1");
    let (nodes2, edges2) = GraphBuilder::structural(&chunks, "f1");

    let mut ids1: Vec<String> = nodes1.iter().map(|n| n.id.clone()).collect();
    let mut ids2: Vec<String> = nodes2.iter().map(|n| n.id.clone()).collect();
    ids1.sort();
    ids2.sort();
    assert_eq!(ids1, ids2);

    let mut eids1: Vec<String> = edges1.iter().map(|e| format!("{}_{}_{:?}", e.from, e.to, e.relation)).collect();
    let mut eids2: Vec<String> = edges2.iter().map(|e| format!("{}_{}_{:?}", e.from, e.to, e.relation)).collect();
    eids1.sort();
    eids2.sort();
    assert_eq!(eids1, eids2);
}

#[test]
fn test_from_chunks_with_references() {
    let config = GraphConfig {
        extract_references: true,
        extract_entities: false,
        ..Default::default()
    };

    let chunks = vec![make_chunk("c1", "f1", "See Document X for details.", None)];
    let (nodes, edges) = GraphBuilder::from_chunks(&chunks, "f1", &config);

    assert!(nodes.iter().any(|n| n.node_type == NodeType::File));
    assert!(nodes.iter().any(|n| n.node_type == NodeType::Chunk));

    let ref_edges: Vec<_> = edges.iter().filter(|e| e.relation == RelationType::References).collect();
    assert!(!ref_edges.is_empty());
}

#[test]
fn test_from_chunks_with_entities() {
    let config = GraphConfig {
        extract_references: false,
        extract_entities: true,
        entity_min_frequency: 1,
        ..Default::default()
    };

    let chunks = vec![
        make_chunk("c1", "f1", "Human Resources Department manages staffing. Human Resources Department handles employee relations.", None),
    ];
    let (nodes, edges) = GraphBuilder::from_chunks(&chunks, "f1", &config);

    let entity_nodes: Vec<_> = nodes.iter().filter(|n| n.node_type == NodeType::Entity).collect();
    assert!(!entity_nodes.is_empty());

    let mention_edges: Vec<_> = edges.iter().filter(|e| e.relation == RelationType::Mentions).collect();
    assert!(!mention_edges.is_empty());
}

#[test]
fn test_reference_extraction_empty_input() {
    let config = GraphConfig {
        extract_references: true,
        extract_entities: false,
        ..Default::default()
    };
    let (_, edges) = GraphBuilder::from_chunks(&[], "f1", &config);
    assert_eq!(edges.len(), 0);
}
