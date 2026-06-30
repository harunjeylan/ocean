use std::collections::HashMap;
use std::sync::Arc;

use crate::ocean_cache::{EmbeddingCache, QueryCache};
use crate::ocean_graph::expansion::ExpansionEngine;
use crate::ocean_graph::types::EdgeDirection;
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

pub fn expand_results(
    results: &[SearchResult],
    expansion: &ExpansionEngine,
    depth: usize,
    store: &dyn VectorStore,
) -> Vec<SearchResult> {
    if depth == 0 {
        return results.to_vec();
    }

    let mut expanded: Vec<SearchResult> = results.to_vec();
    let mut seen_chunks: std::collections::HashSet<String> =
        results.iter().map(|r| r.chunk_id.clone()).collect();

    for result in results {
        let chunk_node_id = format!("chunk:{}", result.chunk_id);
        let subgraph = match expansion.expand(&chunk_node_id, depth, EdgeDirection::Both) {
            Ok(sg) => sg,
            Err(_) => continue,
        };

        for node in &subgraph.nodes {
            let nt = format!("{:?}", node.node_type);
            if nt != "Chunk" {
                continue;
            }
            let chunk_id = node.ref_id.clone();
            if seen_chunks.insert(chunk_id.clone()) {
                if let Ok(Some(record)) = store.get_chunk(&chunk_id) {
                    let combined_score =
                        0.7 * result.score + 0.3 * (1.0 / (1.0 + 1.0));

                    expanded.push(SearchResult {
                        chunk_id,
                        file_id: record.file_id,
                        content: record.content,
                        heading: record.heading,
                        score: combined_score,
                        block_type: record.block_type,
                        vector_score: result.vector_score,
                        fts_score: result.fts_score,
                        graph_score: Some(combined_score),
                    });
                }
            }
        }
    }

    expanded.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    expanded
}

pub struct QueryEngine {
    store: Arc<dyn VectorStore>,
    chunk_store: Arc<dyn ChunkStore>,
    search: SearchEngine,
    graph: Option<ExpansionEngine>,
    embed_cache: Option<EmbeddingCache>,
    query_cache: Option<QueryCache>,
    no_cache: bool,
}

impl QueryEngine {
    pub fn with_caches(mut self, embed_cache: Option<EmbeddingCache>, query_cache: Option<QueryCache>, no_cache: bool) -> Self {
        self.embed_cache = embed_cache;
        self.query_cache = query_cache;
        self.no_cache = no_cache;
        self
    }

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
            embed_cache: None,
            query_cache: None,
            no_cache: false,
        })
    }

    pub fn new_persistent(config: &StorageConfig, dimension: usize) -> Result<Self, QueryError> {
        let vstore_impl = SurrealVectorStore::new_persistent(config)
            .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?;
        vstore_impl.initialize_schema(dimension)
            .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?;
        let vstore: Arc<dyn VectorStore> = Arc::new(vstore_impl);

        let cstore: Arc<dyn ChunkStore> = Arc::new(
            SurrealChunkStore::new_persistent(config)
                .map_err(|e| QueryError::VectorSearchFailed(e.to_string()))?,
        );

        let graph = match SurrealGraphStore::new_persistent(config) {
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
            embed_cache: None,
            query_cache: None,
            no_cache: false,
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
            embed_cache: None,
            query_cache: None,
            no_cache: false,
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
            embed_cache: None,
            query_cache: None,
            no_cache: false,
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

        if !self.no_cache {
            if let Some(ref qcache) = self.query_cache {
                let filter_hash: u64 = query.filter.as_ref().map(|f| {
                    use std::hash::{Hash, Hasher};
                    let mut s = std::collections::hash_map::DefaultHasher::new();
                    f.file_id.hash(&mut s);
                    f.heading_prefix.hash(&mut s);
                    f.block_type.hash(&mut s);
                    s.finish()
                }).unwrap_or(0);

                let cache_key = crate::ocean_cache::QueryCacheKey {
                    query_text: query.text.clone(),
                    mode: mode.clone(),
                    top_k: query.top_k,
                    filter_hash,
                };

                if let Some(cached) = qcache.get(&cache_key) {
                    return Ok(QueryResult {
                        results: cached,
                        context_windows: Vec::new(),
                        execution: ExecutionMeta {
                            query_mode: mode.clone(),
                            total_results: 0,
                            vector_search_time_ms: 0,
                            graph_expand_time_ms: None,
                            fusion_time_ms: 0,
                            total_time_ms: 0,
                        },
                    });
                }
            }
        }

        let plan = self.build_execution_plan(&query, &mode);
        let result = self.execute(&plan, &query, &mode, embedder)?;

        if !self.no_cache {
            if let Some(ref qcache) = self.query_cache {
                let filter_hash: u64 = query.filter.as_ref().map(|f| {
                    use std::hash::{Hash, Hasher};
                    let mut s = std::collections::hash_map::DefaultHasher::new();
                    f.file_id.hash(&mut s);
                    f.heading_prefix.hash(&mut s);
                    f.block_type.hash(&mut s);
                    s.finish()
                }).unwrap_or(0);

                let cache_key = crate::ocean_cache::QueryCacheKey {
                    query_text: query.text.clone(),
                    mode: mode.clone(),
                    top_k: query.top_k,
                    filter_hash,
                };

                qcache.set(cache_key, result.results.clone());
            }
        }

        Ok(result)
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
                        expand_results(&fused_results, expansion, *depth, &*self.store);
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
