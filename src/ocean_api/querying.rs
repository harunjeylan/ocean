use crate::ocean_query::{Query, QueryEngine, QueryMode};
use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::graph_store::GraphStore;
use crate::ocean_storage::{SurrealGraphStore, SurrealVectorStore};
use crate::ocean_vector::search::{SearchEngine, SearchFilter, SearchResult};

use super::embedding::{api_key, create_embedder, EmbeddingConfig};
use super::types::{ApiError, QueryRequest, VectorSearchRequest};

pub fn query(request: QueryRequest) -> Result<crate::ocean_query::QueryResult, ApiError> {
    let provider = EmbeddingConfig::resolve_provider(request.provider.as_deref(), None);
    let model = EmbeddingConfig::resolve_model(request.model.as_deref(), None);

    let mode = match request.mode.as_deref() {
        None | Some("auto") => QueryMode::Auto,
        Some("vector") => QueryMode::Vector,
        Some("hybrid") => QueryMode::Hybrid,
        Some("expand") => QueryMode::Expand,
        Some(other) => return Err(ApiError::ConfigError(format!("invalid mode '{}'. Use: auto, vector, hybrid, expand", other))),
    };

    let dimension = EmbeddingConfig::resolve_dimension(request.dimension, None, &provider, &model);

    let base_path = request.db_path.as_deref().unwrap_or("");
    let engine = QueryEngine::new_with_paths(
        &format!("{}/vector.db", base_path),
        &format!("{}/graph.db", base_path),
        dimension,
    )?;

    let resolved_key = api_key(request.api_key.as_deref(), None, None);
    let base_url = EmbeddingConfig::resolve_base_url(&provider, request.base_url.as_deref(), None);
    let embedder = create_embedder(&provider, &model, &base_url, resolved_key.as_deref())?;

    let mut filter = SearchFilter::new();
    if let Some(ref fid) = request.filter_file_id {
        filter = filter.with_file_id(fid);
    }
    if let Some(ref h) = request.filter_heading {
        filter = filter.with_heading(h);
    }
    if let Some(ref bt) = request.filter_block_type {
        filter = filter.with_block_type(bt);
    }

    let has_filter = filter.file_id.is_some()
        || filter.heading_prefix.is_some()
        || filter.block_type.is_some();

    let q = Query {
        text: request.text,
        mode,
        top_k: request.top_k,
        expand_depth: request.expand_depth,
        filter: if has_filter { Some(filter) } else { None },
        include_context: request.include_context,
        context_chunks: request.context_chunks.unwrap_or(3),
        rerank_by_heading: request.rerank_by_heading,
        rerank_by_file: request.rerank_by_file,
    };

    engine.query(q, &*embedder).map_err(|e| ApiError::QueryError(e.to_string()))
}

pub fn vector_search(request: VectorSearchRequest) -> Result<Vec<SearchResult>, ApiError> {
    let provider = EmbeddingConfig::resolve_provider(request.provider.as_deref(), None);
    let model = EmbeddingConfig::resolve_model(request.model.as_deref(), None);

    let base_path = request.db_path.as_deref().unwrap_or("");
    let vector_path = format!("{}/vector.db", base_path);
    let graph_path = format!("{}/graph.db", base_path);

    let vconfig = StorageConfig::new(&vector_path);
    let vstore = SurrealVectorStore::new_persistent_at(&vector_path, &vconfig)
        .map_err(|e| ApiError::QueryError(format!("Failed to open store: {}", e)))?;
    let engine = SearchEngine::new(std::sync::Arc::new(vstore));

    let resolved_key = api_key(request.api_key.as_deref(), None, None);
    let base_url = EmbeddingConfig::resolve_base_url(&provider, request.base_url.as_deref(), None);
    let embedder = create_embedder(&provider, &model, &base_url, resolved_key.as_deref())?;

    let results = if request.hybrid {
        let mut filter = SearchFilter::new();
        if let Some(ref fid) = request.filter_file_id {
            filter = filter.with_file_id(fid);
        }
        if let Some(ref h) = request.filter_heading {
            filter = filter.with_heading(h);
        }
        if let Some(ref bt) = request.filter_block_type {
            filter = filter.with_block_type(bt);
        }
        if filter.file_id.is_some() || filter.heading_prefix.is_some() || filter.block_type.is_some() {
            engine.hybrid_filtered_search(&request.query, &*embedder, &filter, request.top_k)
        } else {
            engine.hybrid_search(&request.query, &*embedder, request.top_k)
        }
    } else {
        let mut filter = SearchFilter::new();
        if let Some(ref fid) = request.filter_file_id {
            filter = filter.with_file_id(fid);
        }
        if let Some(ref h) = request.filter_heading {
            filter = filter.with_heading(h);
        }
        if let Some(ref bt) = request.filter_block_type {
            filter = filter.with_block_type(bt);
        }
        if filter.file_id.is_some() || filter.heading_prefix.is_some() || filter.block_type.is_some() {
            engine.filtered_search(&request.query, &*embedder, &filter, request.top_k)
        } else {
            engine.search(&request.query, &*embedder, request.top_k)
        }
    };

    let mut results = results.map_err(|e| ApiError::QueryError(format!("Search failed: {}", e)))?;

    if request.expand_depth > 0 {
        let gconfig = StorageConfig::new(&graph_path);
        if let Ok(gs) = SurrealGraphStore::new_persistent_at(&graph_path, &gconfig) {
            if gs.initialize_schema().is_ok() {
                let expansion = crate::ocean_graph::ExpansionEngine::new(std::sync::Arc::new(gs));
                if let Ok(expanded) = engine.expand_results(&results, &expansion, request.expand_depth) {
                    results = expanded;
                }
            }
        }
    }

    Ok(results)
}
