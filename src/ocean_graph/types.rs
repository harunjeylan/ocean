use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    File,
    Chunk,
    Heading,
    Entity,
    Folder,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationType {
    Contains,
    References,
    Mentions,
    BelongsTo,
    DerivedFrom,
    SimilarTo,
    CrossReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeDirection {
    Forward,
    Backward,
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub node_type: NodeType,
    pub ref_id: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: RelationType,
    pub weight: f32,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Subgraph {
    pub seed_id: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct GraphConfig {
    pub extract_references: bool,
    pub extract_entities: bool,
    pub max_expansion_depth: usize,
    pub entity_min_frequency: usize,
    pub default_edge_weight: f32,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            extract_references: true,
            extract_entities: true,
            max_expansion_depth: 3,
            entity_min_frequency: 3,
            default_edge_weight: 1.0,
        }
    }
}
