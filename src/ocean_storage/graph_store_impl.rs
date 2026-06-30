use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::Surreal;
use tokio::runtime::Runtime;

use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::error::StorageError;
use crate::ocean_storage::graph_store::{
    Edge, EdgeDirection, GraphStore, Node, NodeType, RelationType,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphNodeRecord {
    node_id: String,
    node_type: String,
    ref_id: String,
    label: Option<String>,
    file_id: String,
    created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphEdgeRecord {
    edge_id: String,
    from_id: String,
    to_id: String,
    relation: String,
    weight: f64,
    metadata: Option<String>,
    file_id: String,
    created_at: i64,
}

pub struct SurrealGraphStore {
    db: Surreal<Db>,
    rt: Runtime,
}

impl SurrealGraphStore {
    pub fn new_persistent(config: &StorageConfig) -> Result<Self, StorageError> {
        let path = config.graph_path();
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("GraphStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<SurrealKv>(&path).await
                    .map_err(|e| StorageError::ConnectionFailed("GraphStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("GraphStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;
        Ok(Self { db, rt })
    }

    pub fn new_persistent_at(path: &str, _config: &StorageConfig) -> Result<Self, StorageError> {
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("GraphStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<SurrealKv>(path).await
                    .map_err(|e| StorageError::ConnectionFailed("GraphStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("GraphStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;
        Ok(Self { db, rt })
    }

    pub fn new_memory(_config: &StorageConfig) -> Result<Self, StorageError> {
        let rt = Runtime::new()
            .map_err(|e| StorageError::ConnectionFailed("GraphStore".into(), e.to_string()))?;
        let db = rt
            .block_on(async {
                let db = Surreal::new::<Mem>(()).await
                    .map_err(|e| StorageError::ConnectionFailed("GraphStore".into(), e.to_string()))?;
                db.use_ns("ocean").use_db("ocean").await
                    .map_err(|e| StorageError::ConnectionFailed("GraphStore".into(), e.to_string()))?;
                Ok::<_, StorageError>(db)
            })?;
        Ok(Self { db, rt })
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    fn node_to_record(node: &Node, file_id: &str) -> GraphNodeRecord {
        GraphNodeRecord {
            node_id: node.id.clone(),
            node_type: format!("{:?}", node.node_type),
            ref_id: node.ref_id.clone(),
            label: node.label.clone(),
            file_id: file_id.to_string(),
            created_at: Self::now(),
        }
    }

    fn record_to_node(record: GraphNodeRecord) -> Result<Node, StorageError> {
        let node_type = match record.node_type.as_str() {
            "File" => NodeType::File,
            "Chunk" => NodeType::Chunk,
            "Heading" => NodeType::Heading,
            "Entity" => NodeType::Entity,
            "Folder" => NodeType::Folder,
            other => return Err(StorageError::QueryFailed("GraphStore".into(), format!("unknown node type: {}", other))),
        };
        Ok(Node {
            id: record.node_id,
            node_type,
            ref_id: record.ref_id,
            label: record.label,
        })
    }

    fn edge_to_record(edge: &Edge, file_id: &str, edge_id: &str) -> GraphEdgeRecord {
        GraphEdgeRecord {
            edge_id: edge_id.to_string(),
            from_id: edge.from.clone(),
            to_id: edge.to.clone(),
            relation: format!("{:?}", edge.relation),
            weight: edge.weight as f64,
            metadata: edge.metadata.clone(),
            file_id: file_id.to_string(),
            created_at: Self::now(),
        }
    }

    fn record_to_edge(record: GraphEdgeRecord) -> Result<Edge, StorageError> {
        let relation = match record.relation.as_str() {
            "Contains" => RelationType::Contains,
            "References" => RelationType::References,
            "Mentions" => RelationType::Mentions,
            "BelongsTo" => RelationType::BelongsTo,
            "DerivedFrom" => RelationType::DerivedFrom,
            "SimilarTo" => RelationType::SimilarTo,
            "CrossReference" => RelationType::CrossReference,
            other => return Err(StorageError::QueryFailed("GraphStore".into(), format!("unknown relation type: {}", other))),
        };
        Ok(Edge {
            from: record.from_id,
            to: record.to_id,
            relation,
            weight: record.weight as f32,
            metadata: record.metadata,
        })
    }
}

impl GraphStore for SurrealGraphStore {
    fn insert_node(&self, node: &Node, file_id: &str) -> Result<(), StorageError> {
        let record = Self::node_to_record(node, file_id);
        self.rt.block_on(async {
            let _: Option<GraphNodeRecord> = self
                .db
                .create(("graph_node", &record.node_id))
                .content(record)
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::insert_node".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }

    fn insert_edge(&self, edge: &Edge, file_id: &str) -> Result<(), StorageError> {
        let edge_id = format!("{}_{}_{:?}", edge.from, edge.to, edge.relation);
        let record = Self::edge_to_record(edge, file_id, &edge_id);
        self.rt.block_on(async {
            let _: Option<GraphEdgeRecord> = self
                .db
                .create(("graph_edge", &edge_id))
                .content(record)
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::insert_edge".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }

    fn insert_nodes_batch(&self, nodes: Vec<(Node, String)>) -> Result<(), StorageError> {
        self.rt.block_on(async {
            for (node, file_id) in nodes {
                let record = SurrealGraphStore::node_to_record(&node, &file_id);
                self.db
                    .query(format!("UPSERT graph_node:`{}` CONTENT $data", record.node_id))
                    .bind(("data", record))
                    .await
                    .map_err(|e| StorageError::QueryFailed("GraphStore::insert_nodes_batch".into(), e.to_string()))?;
            }
            Ok::<_, StorageError>(())
        })
    }

    fn insert_edges_batch(&self, edges: Vec<(Edge, String)>) -> Result<(), StorageError> {
        self.rt.block_on(async {
            for (edge, file_id) in edges {
                let edge_id = format!("{}_{}_{:?}", edge.from, edge.to, edge.relation);
                let record = SurrealGraphStore::edge_to_record(&edge, &file_id, &edge_id);
                self.db
                    .query(format!("UPSERT graph_edge:`{}` CONTENT $data", edge_id))
                    .bind(("data", record))
                    .await
                    .map_err(|e| StorageError::QueryFailed("GraphStore::insert_edges_batch".into(), e.to_string()))?;
            }
            Ok::<_, StorageError>(())
        })
    }

    fn get_node(&self, id: &str) -> Result<Option<Node>, StorageError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT * FROM graph_node WHERE node_id = $id")
                .bind(("id", id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_node".into(), e.to_string()))?;
            let record: Option<GraphNodeRecord> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_node".into(), e.to_string()))?;
            match record {
                Some(r) => Ok(Some(SurrealGraphStore::record_to_node(r)?)),
                None => Ok(None),
            }
        })
    }

    fn get_node_by_ref(&self, ref_id: &str) -> Result<Option<Node>, StorageError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT * FROM graph_node WHERE ref_id = $ref_id")
                .bind(("ref_id", ref_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_node_by_ref".into(), e.to_string()))?;
            let record: Option<GraphNodeRecord> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_node_by_ref".into(), e.to_string()))?;
            match record {
                Some(r) => Ok(Some(SurrealGraphStore::record_to_node(r)?)),
                None => Ok(None),
            }
        })
    }

    fn get_nodes_by_type(&self, node_type: NodeType) -> Result<Vec<Node>, StorageError> {
        let type_str = format!("{:?}", node_type);
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT * FROM graph_node WHERE node_type = $t")
                .bind(("t", type_str))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_nodes_by_type".into(), e.to_string()))?;
            let records: Vec<GraphNodeRecord> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_nodes_by_type".into(), e.to_string()))?;
            records.into_iter().map(SurrealGraphStore::record_to_node).collect()
        })
    }

    fn get_neighbors(&self, node_id: &str) -> Result<Vec<(Node, Edge)>, StorageError> {
        self.rt.block_on(async {
            let mut edge_results = self
                .db
                .query("SELECT * FROM graph_edge WHERE from_id = $id OR to_id = $id")
                .bind(("id", node_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_neighbors".into(), e.to_string()))?;
            let edge_records: Vec<GraphEdgeRecord> = edge_results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_neighbors".into(), e.to_string()))?;

            let mut neighbor_ids = Vec::new();
            let mut edges = Vec::new();
            for er in &edge_records {
                let neighbor_id = if er.from_id == node_id {
                    er.to_id.clone()
                } else {
                    er.from_id.clone()
                };
                neighbor_ids.push(neighbor_id);
                edges.push(SurrealGraphStore::record_to_edge(er.clone())?);
            }

            if neighbor_ids.is_empty() {
                return Ok(Vec::new());
            }

            let mut results = self
                .db
                .query("SELECT * FROM graph_node WHERE node_id IN $ids")
                .bind(("ids", neighbor_ids))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_neighbors".into(), e.to_string()))?;
            let node_records: Vec<GraphNodeRecord> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_neighbors".into(), e.to_string()))?;

            let mut node_map = std::collections::HashMap::new();
            for nr in node_records {
                if let Ok(n) = SurrealGraphStore::record_to_node(nr) {
                    node_map.insert(n.id.clone(), n);
                }
            }

            let mut neighbors = Vec::new();
            for edge in edges.into_iter() {
                let neighbor_id = if edge.from == node_id {
                    edge.to.clone()
                } else {
                    edge.from.clone()
                };
                if let Some(node) = node_map.remove(&neighbor_id) {
                    neighbors.push((node, edge));
                }
            }
            Ok(neighbors)
        })
    }

    fn get_edges(&self, node_id: &str, direction: EdgeDirection) -> Result<Vec<Edge>, StorageError> {
        self.rt.block_on(async {
            let query = match direction {
                EdgeDirection::Forward => "SELECT * FROM graph_edge WHERE from_id = $id",
                EdgeDirection::Backward => "SELECT * FROM graph_edge WHERE to_id = $id",
                EdgeDirection::Both => "SELECT * FROM graph_edge WHERE from_id = $id OR to_id = $id",
            };
            let mut results = self
                .db
                .query(query)
                .bind(("id", node_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_edges".into(), e.to_string()))?;
            let records: Vec<GraphEdgeRecord> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_edges".into(), e.to_string()))?;
            records.into_iter().map(SurrealGraphStore::record_to_edge).collect()
        })
    }

    fn get_edges_by_relation(&self, relation: RelationType) -> Result<Vec<Edge>, StorageError> {
        let rel_str = format!("{:?}", relation);
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT * FROM graph_edge WHERE relation = $r")
                .bind(("r", rel_str))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_edges_by_relation".into(), e.to_string()))?;
            let records: Vec<GraphEdgeRecord> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_edges_by_relation".into(), e.to_string()))?;
            records.into_iter().map(SurrealGraphStore::record_to_edge).collect()
        })
    }

    fn get_nodes_by_file(&self, file_id: &str) -> Result<Vec<Node>, StorageError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT * FROM graph_node WHERE file_id = $fid")
                .bind(("fid", file_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_nodes_by_file".into(), e.to_string()))?;
            let records: Vec<GraphNodeRecord> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_nodes_by_file".into(), e.to_string()))?;
            records.into_iter().map(SurrealGraphStore::record_to_node).collect()
        })
    }

    fn get_edges_by_file(&self, file_id: &str) -> Result<Vec<Edge>, StorageError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT * FROM graph_edge WHERE file_id = $fid")
                .bind(("fid", file_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_edges_by_file".into(), e.to_string()))?;
            let records: Vec<GraphEdgeRecord> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::get_edges_by_file".into(), e.to_string()))?;
            records.into_iter().map(SurrealGraphStore::record_to_edge).collect()
        })
    }

    fn delete_by_file(&self, file_id: &str) -> Result<u64, StorageError> {
        let node_count = self.count_nodes()?;
        self.rt.block_on(async {
            self.db
                .query("DELETE graph_node WHERE file_id = $fid")
                .bind(("fid", file_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::delete_by_file".into(), e.to_string()))?;
            self.db
                .query("DELETE graph_edge WHERE file_id = $fid")
                .bind(("fid", file_id.to_string()))
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::delete_by_file".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })?;
        let remaining = self.count_nodes()?;
        Ok(node_count.saturating_sub(remaining))
    }

    fn count_nodes(&self) -> Result<u64, StorageError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT count() AS total FROM graph_node GROUP BY count")
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::count_nodes".into(), e.to_string()))?;
            #[derive(serde::Deserialize)]
            struct CountResult { total: u64 }
            let rows: Vec<CountResult> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::count_nodes".into(), e.to_string()))?;
            Ok(rows.first().map(|r| r.total).unwrap_or(0))
        })
    }

    fn count_edges(&self) -> Result<u64, StorageError> {
        self.rt.block_on(async {
            let mut results = self
                .db
                .query("SELECT count() AS total FROM graph_edge GROUP BY count")
                .await
                .map_err(|e| StorageError::QueryFailed("GraphStore::count_edges".into(), e.to_string()))?;
            #[derive(serde::Deserialize)]
            struct CountResult { total: u64 }
            let rows: Vec<CountResult> = results.take(0)
                .map_err(|e| StorageError::QueryFailed("GraphStore::count_edges".into(), e.to_string()))?;
            Ok(rows.first().map(|r| r.total).unwrap_or(0))
        })
    }

    fn clear(&self) -> Result<(), StorageError> {
        self.rt.block_on(async {
            self.db.query("DELETE graph_node").await
                .map_err(|e| StorageError::QueryFailed("GraphStore::clear".into(), e.to_string()))?;
            self.db.query("DELETE graph_edge").await
                .map_err(|e| StorageError::QueryFailed("GraphStore::clear".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }

    fn initialize_schema(&self) -> Result<(), StorageError> {
        self.rt.block_on(async {
            let surql = "
                DEFINE TABLE IF NOT EXISTS graph_node SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS node_id ON TABLE graph_node TYPE string;
                DEFINE FIELD IF NOT EXISTS node_type ON TABLE graph_node TYPE string;
                DEFINE FIELD IF NOT EXISTS ref_id ON TABLE graph_node TYPE string;
                DEFINE FIELD IF NOT EXISTS label ON TABLE graph_node TYPE option<string>;
                DEFINE FIELD IF NOT EXISTS file_id ON TABLE graph_node TYPE string;
                DEFINE FIELD IF NOT EXISTS created_at ON TABLE graph_node TYPE int;
                DEFINE INDEX IF NOT EXISTS idx_node_id ON TABLE graph_node FIELDS node_id UNIQUE;
                DEFINE INDEX IF NOT EXISTS idx_node_ref ON TABLE graph_node FIELDS ref_id;
                DEFINE INDEX IF NOT EXISTS idx_node_type ON TABLE graph_node FIELDS node_type;
                DEFINE INDEX IF NOT EXISTS idx_node_file ON TABLE graph_node FIELDS file_id;

                DEFINE TABLE IF NOT EXISTS graph_edge SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS edge_id ON TABLE graph_edge TYPE string;
                DEFINE FIELD IF NOT EXISTS from_id ON TABLE graph_edge TYPE string;
                DEFINE FIELD IF NOT EXISTS to_id ON TABLE graph_edge TYPE string;
                DEFINE FIELD IF NOT EXISTS relation ON TABLE graph_edge TYPE string;
                DEFINE FIELD IF NOT EXISTS weight ON TABLE graph_edge TYPE float;
                DEFINE FIELD IF NOT EXISTS metadata ON TABLE graph_edge TYPE option<string>;
                DEFINE FIELD IF NOT EXISTS file_id ON TABLE graph_edge TYPE string;
                DEFINE FIELD IF NOT EXISTS created_at ON TABLE graph_edge TYPE int;
                DEFINE INDEX IF NOT EXISTS idx_edge_id ON TABLE graph_edge FIELDS edge_id UNIQUE;
                DEFINE INDEX IF NOT EXISTS idx_edge_from ON TABLE graph_edge FIELDS from_id;
                DEFINE INDEX IF NOT EXISTS idx_edge_to ON TABLE graph_edge FIELDS to_id;
                DEFINE INDEX IF NOT EXISTS idx_edge_relation ON TABLE graph_edge FIELDS relation;
                DEFINE INDEX IF NOT EXISTS idx_edge_file ON TABLE graph_edge FIELDS file_id;
            ";
            self.db.query(surql).await
                .map_err(|e| StorageError::SchemaError("GraphStore".into(), e.to_string()))?;
            Ok::<_, StorageError>(())
        })
    }
}
