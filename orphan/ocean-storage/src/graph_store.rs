use crate::ocean_storage::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum NodeType {
    File,
    Chunk,
    Heading,
    Entity,
    Folder,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RelationType {
    Contains,
    References,
    Mentions,
    BelongsTo,
    DerivedFrom,
    SimilarTo,
    CrossReference,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EdgeDirection {
    Forward,
    Backward,
    Both,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub id: String,
    pub node_type: NodeType,
    pub ref_id: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: RelationType,
    pub weight: f32,
    pub metadata: Option<String>,
}

pub trait GraphStore: Send + Sync {
    fn insert_node(&self, node: &Node, file_id: &str) -> Result<(), StorageError>;
    fn insert_edge(&self, edge: &Edge, file_id: &str) -> Result<(), StorageError>;
    fn insert_nodes_batch(&self, nodes: Vec<(Node, String)>) -> Result<(), StorageError>;
    fn insert_edges_batch(&self, edges: Vec<(Edge, String)>) -> Result<(), StorageError>;
    fn get_node(&self, id: &str) -> Result<Option<Node>, StorageError>;
    fn get_node_by_ref(&self, ref_id: &str) -> Result<Option<Node>, StorageError>;
    fn get_nodes_by_type(&self, node_type: NodeType) -> Result<Vec<Node>, StorageError>;
    fn get_neighbors(&self, node_id: &str) -> Result<Vec<(Node, Edge)>, StorageError>;
    fn get_edges(&self, node_id: &str, direction: EdgeDirection) -> Result<Vec<Edge>, StorageError>;
    fn get_edges_by_relation(&self, relation: RelationType) -> Result<Vec<Edge>, StorageError>;
    fn get_nodes_by_file(&self, file_id: &str) -> Result<Vec<Node>, StorageError>;
    fn get_edges_by_file(&self, file_id: &str) -> Result<Vec<Edge>, StorageError>;
    fn delete_by_file(&self, file_id: &str) -> Result<u64, StorageError>;
    fn count_nodes(&self) -> Result<u64, StorageError>;
    fn count_edges(&self) -> Result<u64, StorageError>;
    fn clear(&self) -> Result<(), StorageError>;
    fn initialize_schema(&self) -> Result<(), StorageError>;
}
