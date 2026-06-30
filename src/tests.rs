#[path = "ocean_chunk/heading_test.rs"]
mod heading_spec;
#[path = "ocean_chunk/split_test.rs"]
mod split_spec;
#[path = "ocean_chunk/buffer_test.rs"]
mod buffer_spec;
#[path = "ocean_chunk/chunker_test.rs"]
mod chunker_spec;

#[path = "ocean_fs/hasher_test.rs"]
mod hasher_spec;
#[path = "ocean_fs/filter_test.rs"]
mod filter_spec;
#[path = "ocean_fs/normalizer_test.rs"]
mod normalizer_spec;
#[path = "ocean_fs/path_resolver_test.rs"]
mod path_resolver_spec;
#[path = "ocean_fs/scanner_test.rs"]
mod scanner_spec;
#[path = "ocean_fs/watcher_test.rs"]
mod watcher_spec;

#[path = "ocean_vector/embedder_test.rs"]
mod embedder_spec;
#[path = "ocean_vector/pipeline_test.rs"]
mod pipeline_spec;
#[path = "ocean_vector/search_test.rs"]
mod search_spec;

#[path = "ocean_graph/builder_test.rs"]
mod graph_builder_spec;
#[path = "ocean_graph/entity_test.rs"]
mod graph_entity_spec;
#[path = "ocean_graph/expansion_test.rs"]
mod graph_expansion_spec;

#[path = "ocean_query/types_test.rs"]
mod query_types_spec;
#[path = "ocean_query/engine_test.rs"]
mod query_engine_spec;
#[path = "ocean_query/context_test.rs"]
mod query_context_spec;

#[path = "ocean_storage/file_store_test.rs"]
mod storage_file_store_spec;
#[path = "ocean_storage/chunk_store_test.rs"]
mod storage_chunk_store_spec;
#[path = "ocean_storage/state_store_test.rs"]
mod storage_state_store_spec;
#[path = "ocean_storage/storage_test.rs"]
mod storage_integration_spec;
#[path = "ocean_storage/vector_store_test.rs"]
mod storage_vector_store_spec;
#[path = "ocean_storage/graph_store_test.rs"]
mod storage_graph_store_spec;

#[path = "ocean_index/config_test.rs"]
mod index_config_spec;
#[path = "ocean_index/report_test.rs"]
mod index_report_spec;
#[path = "ocean_index/progress_test.rs"]
mod index_progress_spec;
#[path = "ocean_index/runtime_test.rs"]
mod index_runtime_spec;
#[path = "ocean_index/job_queue_test.rs"]
mod index_job_queue_spec;
#[path = "ocean_index/worker_pool_test.rs"]
mod index_worker_pool_spec;
#[path = "ocean_index/rate_limiter_test.rs"]
mod index_rate_limiter_spec;
#[path = "ocean_index/orchestrator_test.rs"]
mod index_orchestrator_spec;

#[path = "ocean_cli/runtime_test.rs"]
mod cli_runtime_spec;
#[path = "ocean_cli/events_test.rs"]
mod cli_events_spec;
#[path = "ocean_cli/metrics_test.rs"]
mod cli_metrics_spec;
#[path = "ocean_cli/sandbox_test.rs"]
mod cli_sandbox_spec;
#[path = "ocean_storage/readonly_test.rs"]
mod storage_readonly_spec;

#[path = "ocean_cache/lru_test.rs"]
mod cache_lru_spec;
#[path = "ocean_cache/embedding_cache_test.rs"]
mod cache_embedding_spec;
#[path = "ocean_cache/query_cache_test.rs"]
mod cache_query_spec;
#[path = "ocean_cache/graph_cache_test.rs"]
mod cache_graph_spec;

#[path = "ocean_cli/init_test.rs"]
mod cli_init_spec;
