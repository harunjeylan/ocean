use std::collections::{HashSet, VecDeque, HashMap};

use crate::ocean_graph::error::GraphError;
use crate::ocean_graph::store::GraphStore;
use crate::ocean_graph::types::{Edge, EdgeDirection, Node, Subgraph};

pub struct ExpansionEngine {
    store: GraphStore,
}

impl ExpansionEngine {
    pub fn new(store: GraphStore) -> Self {
        Self { store }
    }

    pub fn expand(
        &self,
        node_id: &str,
        depth: usize,
        _direction: EdgeDirection,
    ) -> Result<Subgraph, GraphError> {
        if depth == 0 || depth > 5 {
            return Err(GraphError::InvalidDepth(format!(
                "depth must be between 1 and 5, got {}",
                depth
            )));
        }

        let seed_node = self
            .store
            .get_node(node_id)?
            .ok_or_else(|| GraphError::NodeNotFound(node_id.to_string()))?;

        let mut visited: HashSet<String> = HashSet::new();
        let mut result_nodes: Vec<Node> = Vec::new();
        let mut result_edges: Vec<Edge> = Vec::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        visited.insert(seed_node.id.clone());
        queue.push_back((seed_node.id.clone(), 0));

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }

            let neighbors = self.store.get_neighbors(&current_id)?;

            for (neighbor_node, edge) in neighbors {
                if edge.weight <= 0.0 {
                    continue;
                }

                result_edges.push(edge);

                if visited.insert(neighbor_node.id.clone()) {
                    result_nodes.push(neighbor_node.clone());
                    queue.push_back((neighbor_node.id.clone(), current_depth + 1));
                }
            }
        }

        result_nodes.insert(0, seed_node);

        Ok(Subgraph {
            seed_id: node_id.to_string(),
            nodes: result_nodes,
            edges: result_edges,
            depth,
        })
    }

    pub fn expand_from_chunks(
        &self,
        chunk_ids: &[String],
        depth: usize,
    ) -> Result<Subgraph, GraphError> {
        if depth == 0 || depth > 5 {
            return Err(GraphError::InvalidDepth(format!(
                "depth must be between 1 and 5, got {}",
                depth
            )));
        }

        let mut all_nodes: HashMap<String, Node> = HashMap::new();
        let mut all_edges: Vec<Edge> = Vec::new();
        let mut edge_set: HashSet<String> = HashSet::new();
        let seed_id = chunk_ids.join(",");

        for chunk_id in chunk_ids {
            let node_id = format!("chunk:{}", chunk_id);
            let subgraph = self.expand(&node_id, depth, EdgeDirection::Both)?;

            for node in subgraph.nodes {
                all_nodes.entry(node.id.clone()).or_insert(node);
            }

            for edge in subgraph.edges {
                let edge_key = format!("{}_{}_{:?}", edge.from, edge.to, edge.relation);
                if edge_set.insert(edge_key) {
                    all_edges.push(edge);
                }
            }
        }

        Ok(Subgraph {
            seed_id,
            nodes: all_nodes.into_values().collect(),
            edges: all_edges,
            depth,
        })
    }

    pub fn find_path(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: usize,
    ) -> Result<Option<Vec<Edge>>, GraphError> {
        if max_depth == 0 || max_depth > 10 {
            return Err(GraphError::InvalidDepth(format!(
                "max_depth must be between 1 and 10, got {}",
                max_depth
            )));
        }

        if from_id == to_id {
            return Ok(Some(Vec::new()));
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut parent: HashMap<String, (String, Edge)> = HashMap::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        visited.insert(from_id.to_string());
        queue.push_back((from_id.to_string(), 0));

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if current_depth >= max_depth {
                continue;
            }

            let neighbors = self.store.get_neighbors(&current_id)?;

            for (neighbor_node, edge) in &neighbors {
                if visited.insert(neighbor_node.id.clone()) {
                    parent.insert(
                        neighbor_node.id.clone(),
                        (current_id.clone(), edge.clone()),
                    );

                    if neighbor_node.id == to_id {
                        let mut path = Vec::new();
                        let mut node = to_id.to_string();
                        while let Some((prev, e)) = parent.get(&node) {
                            path.push(e.clone());
                            node = prev.clone();
                            if node == from_id {
                                break;
                            }
                        }
                        path.reverse();
                        return Ok(Some(path));
                    }

                    queue.push_back((neighbor_node.id.clone(), current_depth + 1));
                }
            }
        }

        Ok(None)
    }

    pub fn get_file_graph(&self, file_id: &str) -> Result<Subgraph, GraphError> {
        let nodes = self.store.get_nodes_by_file(file_id)?;
        let edges = self.store.get_edges_by_file(file_id)?;

        Ok(Subgraph {
            seed_id: format!("file:{}", file_id),
            nodes,
            edges,
            depth: 0,
        })
    }
}
