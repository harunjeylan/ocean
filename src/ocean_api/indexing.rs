use std::path::PathBuf;
use std::sync::Arc;

use crate::ocean_chunk::ChunkConfig;
use crate::ocean_graph::GraphConfig;
use crate::ocean_index::config::{BackpressureConfig, IndexConfig as OceanIndexConfig, RateLimiterConfig};
use crate::ocean_index::progress::ConsoleReporter;
use crate::ocean_index::runtime::RetryPolicy;
use crate::ocean_index::{IndexMode, IndexOrchestrator};
use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::graph_store::GraphStore;
use crate::ocean_storage::vector_store::VectorStore;
use crate::ocean_storage::{SurrealChunkStore, SurrealGraphStore, SurrealStateStore, SurrealVectorStore};

use super::embedding::{api_key, create_embedder, EmbeddingConfig};
use super::types::{ApiError, IndexRequest, IndexResult};

pub fn index_directory(request: IndexRequest) -> Result<IndexResult, ApiError> {
    let dir_path = PathBuf::from(&request.dir);
    if !dir_path.is_dir() {
        return Err(ApiError::DocError(format!("directory not found: {}", request.dir)));
    }

    let files = crate::ocean_cli::walk::walk_supported_files(&dir_path);
    if files.is_empty() {
        return Err(ApiError::DocError(format!("No supported documents found in '{}'.", request.dir)));
    }

    let provider = EmbeddingConfig::resolve_provider(request.provider.as_deref(), None);
    let model = EmbeddingConfig::resolve_model(request.model.as_deref(), None);

    let vector_path = format!("{}/vector.db", request.db_path.as_deref().unwrap_or(""));
    let graph_path = format!("{}/graph.db", request.db_path.as_deref().unwrap_or(""));

    let vconfig = StorageConfig::new(&vector_path);
    let vstore = SurrealVectorStore::new_persistent_at(&vector_path, &vconfig)
        .map_err(|e| ApiError::IndexError(format!("Failed to open store: {}", e)))?;

    let dim = EmbeddingConfig::resolve_dimension(request.dimension, None, &provider, &model);
    vstore.initialize_schema(dim)
        .map_err(|e| ApiError::IndexError(format!("Failed to init schema: {}", e)))?;

    let cstore = SurrealChunkStore::new_persistent_at(&vector_path)
        .map_err(|e| ApiError::IndexError(format!("Failed to open chunk store: {}", e)))?;

    let graph_store: Option<SurrealGraphStore> = if !request.no_graph {
        let gconfig = StorageConfig::new(&graph_path);
        let gs = SurrealGraphStore::new_persistent_at(&graph_path, &gconfig)
            .map_err(|e| ApiError::IndexError(format!("Failed to open graph store: {}", e)))?;
        gs.initialize_schema()
            .map_err(|e| ApiError::IndexError(format!("Failed to init graph schema: {}", e)))?;
        Some(gs)
    } else {
        None
    };

    let state_store = SurrealStateStore::new_persistent(&StorageConfig::new(request.db_path.as_deref().unwrap_or("")))
        .map_err(|e| ApiError::IndexError(format!("Failed to open state store: {}", e)))?;

    let resolved_key = api_key(request.api_key.as_deref(), None, None);
    let base_url = EmbeddingConfig::resolve_base_url(&provider, request.base_url.as_deref(), None);

    let embedder = create_embedder(&provider, &model, &base_url, resolved_key.as_deref())?;
    let embedder: Arc<dyn crate::ocean_vector::embedder::Embedder> = Arc::from(embedder);

    let index_mode = if request.watch {
        IndexMode::Watch
    } else if request.reindex {
        IndexMode::Full
    } else {
        IndexMode::Incremental
    };

    let chunk_config = request.chunk_config.unwrap_or_default();

    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let io_threads = request.io_threads.or(Some(cpus * 2));
    let cpu_threads = request.cpu_threads.or(Some(cpus));

    let retry_policy = RetryPolicy::new(
        request.max_retries.unwrap_or(3),
        request.retry_backoff_ms.unwrap_or(100),
        30_000,
    );

    let rate_limiter = RateLimiterConfig {
        max_concurrent: request.max_ai_concurrent.unwrap_or(2),
        requests_per_minute: None,
    };

    let backpressure = BackpressureConfig {
        max_queue_size: request.max_queue_size.unwrap_or(10_000),
        max_in_flight: request.max_in_flight.unwrap_or(10),
        max_ai_concurrent: request.max_ai_concurrent.unwrap_or(2),
        pause_check_ms: 1_000,
    };

    let index_config = OceanIndexConfig {
        mode: index_mode,
        dir: request.dir,
        chunk_config: ChunkConfig {
            min_tokens: chunk_config.min_tokens,
            max_tokens: chunk_config.max_tokens,
            overlap_sentences: chunk_config.overlap_sentences,
            include_images: chunk_config.include_images,
            rows_per_sheet_chunk: chunk_config.rows_per_sheet_chunk,
            token_estimator: None,
        },
        graph_config: GraphConfig {
            extract_references: !request.no_references,
            extract_entities: !request.no_entities,
            ..Default::default()
        },
        batch_size: request.batch_size,
        retry_policy,
        rate_limiter,
        backpressure,
        io_threads,
        cpu_threads,
        no_graph: request.no_graph,
    };

    let orchestrator = IndexOrchestrator::new(
        Arc::new(vstore),
        Arc::new(cstore),
        graph_store.map(|gs| Arc::new(gs) as Arc<dyn GraphStore>),
        Arc::new(state_store),
        embedder,
        Arc::new(ConsoleReporter),
    );

    let embed_cache_path = format!("{}/cache", request.db_path.as_deref().unwrap_or(""));
    let embed_cache = crate::ocean_cache::EmbeddingCache::new(1000, Some(&embed_cache_path));
    let orchestrator = orchestrator.with_embed_cache(embed_cache);

    orchestrator.run(index_config).map_err(|e| ApiError::IndexError(e.to_string()))
}
