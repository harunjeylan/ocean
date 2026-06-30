pub use crate::ocean_storage::graph_store::{
    Edge, EdgeDirection, Node, NodeType, RelationType,
};

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
