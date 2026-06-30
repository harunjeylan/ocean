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
#[path = "ocean_vector/store_test.rs"]
mod store_spec;
#[path = "ocean_vector/pipeline_test.rs"]
mod pipeline_spec;
#[path = "ocean_vector/search_test.rs"]
mod search_spec;

#[path = "ocean_graph/store_test.rs"]
mod graph_store_spec;
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
