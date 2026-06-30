use std::collections::HashMap;
use std::sync::Arc;

use crate::ocean_graph::expansion::ExpansionEngine;
use crate::ocean_query::context::ContextWindowBuilder;
use crate::ocean_query::error::QueryError;
use crate::ocean_query::types::*;
use crate::ocean_storage::chunk_store::ChunkStore;
use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::graph_store::GraphStore;
use crate::ocean_storage::vector_store::VectorStore;
use crate::ocean_storage::{SurrealChunkStore, SurrealGraphStore, SurrealVectorStore, Storage};
use crate::ocean_vector::embedder::Embedder;
use crate::ocean_vector::search::{parse_search_results_raw, SearchEngine, SearchResult};

#[derive(Debug, Clone)]
enum SubQuery {
    Vector { top_k: usize },
    Fts { top_k: usize },
    RrfFusion { k: f32 },
    GraphExpand { depth: usize },
    RerankByHeading,
    RerankByFile,
    #[allow(dead_code)]
    BuildContext { context_chunks: usize },
}

#[derive(Debug, Clone)]
struct ExecutionPlan {
    steps: Vec<SubQuery>,
}

pub fn select_mode(text: &str, expand_depth: usize) -> QueryMode {
    if expand_depth > 0 {
        return QueryMode::Expand;
    }

    let text = text.trim();
    if text.is_empty() {
        return QueryMode::Hybrid;
    }

    let word_count = text.split_whitespace().count();

    let cross_ref_phrases = [
        "related to",
        "connected to",
        "reference",
        "associated with",
    ];
    let lower = text.to_lowercase();
    for phrase in &cross_ref_phrases {
        if lower.contains(phrase) {
            return QueryMode::Expand;
        }
    }

    if word_count < 3 {
        QueryMode::Vector
    } else {
        QueryMode::Hybrid
    }
}

pub struct QueryEngine {
    store: Arc<dyn VectorStore>,
    chunk_store: Arc<dyn ChunkStore>,
    search: SearchEngine,
    graph: Option<ExpansionEngine>,
}

impl QueryEngine {
    pub fn from_storage(storage: Arc<dyn Storage>) -> Result<Self, QueryError> {
        let config = StorageConfig::new(storage.storage_path());
        let vstore: Arc<dyn VectorStore> = Arc::new(
            SurrealVectorStore::new_persistent(&config)
                .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?,
        );
        let cstore: Arc<dyn ChunkStore> = Arc::new(
            SurrealChunkStore::new_persistent(&config)
                .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?,
        );
        let gstore: Arc<dyn GraphStore> = Arc::new(
            SurrealGraphStore::new_persistent(&config)
                .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?,
        );

        let search = SearchEngine::new(vstore.clone());
        let graph = Some(ExpansionEngine::new(gstore));

        Ok(Self {
            store: vstore,
            chunk_store: cstore,
            search,
            graph,
        })
    }

    pub fn new(db_path: &str) -> Result<Self, QueryError> {
        Self::new_with_dimension(db_path, 768)
    }

    pub fn new_with_dimension(db_path: &str, dimension: usize) -> Result<Self, QueryError> {
        let graph_path = format!("{}_graph", db_path);
        Self::new_with_paths(db_path, &graph_path, dimension)
    }

    pub fn new_with_paths(
        vector_path: &str,
        graph_path: &str,
        dimension: usize,
    ) -> Result<Self, QueryError> {
        let vconfig = StorageConfig::new(vector_path);
        let gconfig = StorageConfig::new(graph_path);

        let vstore_impl = SurrealVectorStore::new_persistent_at(vector_path, &vconfig)
            .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?;
        vstore_impl.initialize_schema(dimension)
            .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?;
        let vstore: Arc<dyn VectorStore> = Arc::new(vstore_impl);

        let cstore: Arc<dyn ChunkStore> = Arc::new(
            SurrealChunkStore::new_persistent_at(vector_path)
                .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?,
        );

        let graph = match SurrealGraphStore::new_persistent_at(graph_path, &gconfig) {
            Ok(gs) => {
                let _ = gs.initialize_schema();
                Some(ExpansionEngine::new(Arc::new(gs) as Arc<dyn GraphStore>))
            }
            Err(_) => None,
        };

        let search = SearchEngine::new(vstore.clone());

        Ok(Self {
            store: vstore,
            chunk_store: cstore,
            search,
            graph,
        })
    }

    pub fn new_memory() -> Result<Self, QueryError> {
        Self::new_memory_with_dimension(768)
    }

    pub fn new_memory_with_dimension(dimension: usize) -> Result<Self, QueryError> {
        let config = StorageConfig::new(":memory:");

        let vstore_impl = SurrealVectorStore::new_memory(&config)
            .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?;
        vstore_impl.initialize_schema(dimension)
            .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?;
        let vstore: Arc<dyn VectorStore> = Arc::new(vstore_impl);

        let cstore: Arc<dyn ChunkStore> = Arc::new(
            SurrealChunkStore::new_memory()
                .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?,
        );

        let graph = match SurrealGraphStore::new_memory(&config) {
            Ok(gs) => {
                let _ = gs.initialize_schema();
                Some(ExpansionEngine::new(Arc::new(gs) as Arc<dyn GraphStore>))
            }
            Err(_) => None,
        };

        let search = SearchEngine::new(vstore.clone());

        Ok(Self {
            store: vstore,
            chunk_store: cstore,
            search,
            graph,
        })
    }

    fn resolve_mode(&self, query: &Query) -> QueryMode {
        if query.mode == QueryMode::Auto {
            select_mode(&query.text, query.expand_depth)
        } else {
            query.mode.clone()
        }
    }

    fn build_execution_plan(&self, query: &Query, mode: &QueryMode) -> ExecutionPlan {
        let mut steps = Vec::new();

        match mode {
            QueryMode::Vector => {
                steps.push(SubQuery::Vector {
                    top_k: query.top_k,
                });
            }
            QueryMode::Hybrid => {
                steps.push(SubQuery::Vector {
                    top_k: query.top_k,
                });
                steps.push(SubQuery::Fts {
                    top_k: query.top_k,
                });
                steps.push(SubQuery::RrfFusion { k: 60.0 });
            }
            QueryMode::Expand => {
                steps.push(SubQuery::Vector {
                    top_k: query.top_k,
                });
                steps.push(SubQuery::Fts {
                    top_k: query.top_k,
                });
                steps.push(SubQuery::RrfFusion { k: 60.0 });
                steps.push(SubQuery::GraphExpand {
                    depth: query.expand_depth,
                });
            }
            QueryMode::Auto => {
                unreachable!("auto mode should have been resolved already");
            }
        }

        if query.rerank_by_heading {
            steps.push(SubQuery::RerankByHeading);
        }
        if query.rerank_by_file {
            steps.push(SubQuery::RerankByFile);
        }
        if query.include_context {
            steps.push(SubQuery::BuildContext {
                context_chunks: query.context_chunks.max(1).min(10),
            });
        }

        ExecutionPlan { steps }
    }

    pub fn query(
        &self,
        query: Query,
        embedder: &dyn Embedder,
    ) -> Result<QueryResult, QueryError> {
        if query.text.trim().is_empty() {
            return Err(QueryError::InvalidQuery("query text is empty".into()));
        }

        let mode = self.resolve_mode(&query);
        let plan = self.build_execution_plan(&query, &mode);
        self.execute(&plan, &query, &mode, embedder)
    }

    fn execute(
        &self,
        plan: &ExecutionPlan,
        query: &Query,
        mode: &QueryMode,
        embedder: &dyn Embedder,
    ) -> Result<QueryResult, QueryError> {
        let start = std::time::Instant::now();

        let mut vector_results: Vec<SearchResult> = Vec::new();
        let mut fts_results: Vec<SearchResult> = Vec::new();
        let mut fused_results: Vec<SearchResult> = Vec::new();
        let mut final_results: Vec<SearchResult> = Vec::new();

        let mut vector_time = 0u64;
        let mut fusion_time = 0u64;
        let mut graph_time: Option<u64> = None;

        for step in &plan.steps {
            match step {
                SubQuery::Vector { top_k } => {
                    let t = std::time::Instant::now();

                    let results = self.search.search(&query.text, embedder, *top_k);
                    vector_results = match results {
                        Ok(r) => r,
                        Err(e) => {
                            return Err(QueryError::VectorSearchFailed(e.to_string()));
                        }
                    };
                    vector_time = t.elapsed().as_millis() as u64;
                }
                SubQuery::Fts { top_k } => {
                    let fts_raw = self
                        .store
                        .fts_search(&query.text, *top_k, None)
                        .unwrap_or_default();
                    fts_results =
                        parse_search_results_raw(&fts_raw, false, true).unwrap_or_default();
                }
                SubQuery::RrfFusion { k } => {
                    let t = std::time::Instant::now();
                    fused_results = crate::ocean_vector::search::fuse_rrf(
                        vector_results.clone(),
                        fts_results.clone(),
                        *k,
                        query.top_k,
                    );
                    fusion_time = t.elapsed().as_millis() as u64;
                }
                SubQuery::GraphExpand { depth } => {
                    let t = std::time::Instant::now();
                    let expansion = match self.graph {
                        Some(ref engine) => engine,
                        None => {
                            graph_time = Some(t.elapsed().as_millis() as u64);
                            continue;
                        }
                    };

                    final_results =
                        match self.search.expand_results(&fused_results, expansion, *depth) {
                            Ok(r) => r,
                            Err(_) => fused_results.clone(),
                        };
                    graph_time = Some(t.elapsed().as_millis() as u64);
                }
                SubQuery::RerankByHeading => {
                    let mut heading_counts: HashMap<Option<String>, usize> = HashMap::new();
                    let results = if !final_results.is_empty() {
                        &final_results
                    } else if !fused_results.is_empty() {
                        &fused_results
                    } else {
                        &vector_results
                    };
                    let mut reranked = results.clone();
                    for r in &reranked {
                        *heading_counts.entry(r.heading.clone()).or_insert(0) += 1;
                    }
                    for r in &mut reranked {
                        let count = heading_counts.get(&r.heading).copied().unwrap_or(1) as f32;
                        r.score *= 1.0 / (1.0 + 0.15 * (count - 1.0));
                    }
                    reranked.sort_by(|a, b| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    final_results = reranked;
                }
                SubQuery::RerankByFile => {
                    let mut file_counts: HashMap<String, usize> = HashMap::new();
                    let results = if !final_results.is_empty() {
                        &final_results
                    } else if !fused_results.is_empty() {
                        &fused_results
                    } else {
                        &vector_results
                    };
                    let mut reranked = results.clone();
                    for r in &reranked {
                        *file_counts.entry(r.file_id.clone()).or_insert(0) += 1;
                    }
                    for r in &mut reranked {
                        let count = file_counts.get(&r.file_id).copied().unwrap_or(1) as f32;
                        r.score *= 1.0 / (1.0 + 0.1 * (count - 1.0));
                    }
                    reranked.sort_by(|a, b| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    final_results = reranked;
                }
                SubQuery::BuildContext { context_chunks: _ } => {}
            }
        }

        let source_results = if !final_results.is_empty() {
            &final_results
        } else if !fused_results.is_empty() {
            &fused_results
        } else {
            &vector_results
        };

        if source_results.is_empty() {
            return Err(QueryError::NoResults);
        }

        let ranked: Vec<RankedChunk> = source_results
            .iter()
            .map(|r| RankedChunk {
                chunk_id: r.chunk_id.clone(),
                file_id: r.file_id.clone(),
                content: r.content.clone(),
                heading: r.heading.clone(),
                score: r.score,
                vector_score: r.vector_score,
                fts_score: r.fts_score,
                graph_score: r.graph_score,
                block_type: Some(r.block_type.clone()),
            })
            .collect();

        let total = ranked.len();

        let context_windows = if query.include_context {
            let builder = ContextWindowBuilder::new(self.chunk_store.clone());
            let n = query.context_chunks.max(1).min(10);
            let mut windows = Vec::new();
            for chunk in &ranked {
                if let Ok(cw) = builder.build(chunk, n) {
                    windows.push(cw);
                }
            }
            windows
        } else {
            Vec::new()
        };

        let total_time = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            results: ranked,
            context_windows,
            execution: ExecutionMeta {
                query_mode: mode.clone(),
                total_results: total,
                vector_search_time_ms: vector_time,
                graph_expand_time_ms: graph_time,
                fusion_time_ms: fusion_time,
                total_time_ms: total_time,
            },
        })
    }

    pub fn query_stream<'a>(
        &'a self,
        query: Query,
        embedder: &'a dyn Embedder,
    ) -> Result<impl Iterator<Item = Result<RankedChunk, QueryError>> + 'a, QueryError> {
        let result = self.query(query, embedder)?;
        let iter = result.results.into_iter().map(Ok);
        Ok(iter)
    }
}
