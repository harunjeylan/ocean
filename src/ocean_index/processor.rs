use std::sync::Arc;
use std::time::Instant;

use crate::ocean_chunk::chunk;
use crate::ocean_graph::GraphBuilder;
use crate::ocean_parser::read_all_blocks;
use crate::ocean_storage::{ChunkStore, GraphStore, VectorStore};
use crate::ocean_vector::embedder::Embedder;
use crate::ocean_vector::pipeline::{IndexConfig as PipelineIndexConfig, IndexPipeline};

use super::config::IndexConfig;
use super::error::IndexError;
use super::report::FileResult;

pub(crate) struct FileProcessor {
    embedder: Arc<dyn Embedder>,
    pipeline: IndexPipeline,
    graph_store: Option<Arc<dyn GraphStore>>,
}

impl FileProcessor {
    pub fn new(
        embedder: Arc<dyn Embedder>,
        vector_store: Arc<dyn VectorStore>,
        chunk_store: Arc<dyn ChunkStore>,
        graph_store: Option<Arc<dyn GraphStore>>,
    ) -> Self {
        let pipeline = IndexPipeline::new(vector_store, chunk_store);
        Self {
            embedder,
            pipeline,
            graph_store,
        }
    }

    pub fn process(
        &self,
        path: &str,
        file_id: &str,
        config: &IndexConfig,
    ) -> Result<FileResult, IndexError> {
        let start = Instant::now();

        let doc = crate::ocean_parser::open(path).map_err(|e| {
            IndexError::FileProcessError {
                file_id: file_id.to_string(),
                stage: "parse".into(),
                error: format!("Failed to open: {}", e),
            }
        })?;

        let blocks = read_all_blocks(&*doc).map_err(|e| {
            IndexError::FileProcessError {
                file_id: file_id.to_string(),
                stage: "parse".into(),
                error: format!("read failed: {}", e),
            }
        })?;

        let chunks = chunk(blocks, file_id, Some(config.chunk_config.clone())).map_err(|e| {
            IndexError::FileProcessError {
                file_id: file_id.to_string(),
                stage: "chunk".into(),
                error: e.to_string(),
            }
        })?;

        let chunk_count = chunks.len() as u64;

        let pipeline_config = PipelineIndexConfig {
            batch_size: config.batch_size,
            reindex: config.mode == super::config::IndexMode::Full,
            model: self.embedder.model_name().to_string(),
            dimension: self.embedder.dimension(),
            ..Default::default()
        };

        let pipeline_report = self.pipeline.index_chunks(
            chunks.clone(),
            &*self.embedder,
            &pipeline_config,
        ).map_err(|e| {
            IndexError::FileProcessError {
                file_id: file_id.to_string(),
                stage: "embed".into(),
                error: e.to_string(),
            }
        })?;

        let embedded = pipeline_report.embedded as u64;
        let embed_skipped = pipeline_report.skipped as u64;
        let embed_failed = pipeline_report.failed as u64;

        let mut node_count = 0u64;
        let mut edge_count = 0u64;
        if let Some(ref gs) = self.graph_store {
            if !config.no_graph {
                let (nodes, edges) = GraphBuilder::from_chunks(&chunks, file_id, &config.graph_config);
                node_count = nodes.len() as u64;
                edge_count = edges.len() as u64;

                if !nodes.is_empty() {
                    let node_pairs: Vec<(crate::ocean_storage::Node, String)> = nodes
                        .into_iter()
                        .map(|n| (n, file_id.to_string()))
                        .collect();
                    gs.insert_nodes_batch(node_pairs).map_err(|e| {
                        IndexError::FileProcessError {
                            file_id: file_id.to_string(),
                            stage: "graph".into(),
                            error: format!("node insert: {}", e),
                        }
                    })?;
                }

                if !edges.is_empty() {
                    let edge_pairs: Vec<(crate::ocean_storage::Edge, String)> = edges
                        .into_iter()
                        .map(|e| (e, file_id.to_string()))
                        .collect();
                    gs.insert_edges_batch(edge_pairs).map_err(|e| {
                        IndexError::FileProcessError {
                            file_id: file_id.to_string(),
                            stage: "graph".into(),
                            error: format!("edge insert: {}", e),
                        }
                    })?;
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(FileResult {
            path: path.to_string(),
            status: super::report::FileIndexStatus::Indexed,
            chunks: chunk_count,
            embedded,
            embed_skipped,
            embed_failed,
            nodes: node_count,
            edges: edge_count,
            duration_ms,
            error: None,
        })
    }
}
