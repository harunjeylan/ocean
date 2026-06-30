use std::sync::Arc;

use super::embedder_spec::MockEmbedder;

fn test_dir() -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("test-cwd");
    p.to_string_lossy().to_string()
}
use crate::ocean_chunk::ChunkConfig;
use crate::ocean_graph::GraphConfig;
use crate::ocean_index::config::{BackpressureConfig, IndexConfig, IndexMode, RateLimiterConfig};
use crate::ocean_index::orchestrator::IndexOrchestrator;
use crate::ocean_index::progress::SilentReporter;
use crate::ocean_index::runtime::RetryPolicy;
use crate::ocean_storage::{
    ChunkStore, GraphStore, StateStore, StorageConfig,
    SurrealChunkStore, SurrealGraphStore, SurrealStateStore, SurrealVectorStore, VectorStore,
};

fn in_memory_stores(dim: usize) -> (Arc<SurrealVectorStore>, Arc<SurrealChunkStore>, Option<Arc<SurrealGraphStore>>, Arc<SurrealStateStore>) {
    let config = StorageConfig::new(":memory:");
    let vstore = SurrealVectorStore::new_memory(&config).unwrap();
    vstore.initialize_schema(dim).unwrap();

    let cstore = SurrealChunkStore::new_memory().unwrap();

    let gconfig = StorageConfig::new(":memory:");
    let gs = SurrealGraphStore::new_memory(&gconfig).unwrap();
    gs.initialize_schema().unwrap();

    let sstore = SurrealStateStore::new_memory().unwrap();

    (Arc::new(vstore), Arc::new(cstore), Some(Arc::new(gs)), Arc::new(sstore))
}

fn make_orchestrator(
    vstore: Arc<SurrealVectorStore>,
    cstore: Arc<SurrealChunkStore>,
    gstore: Option<Arc<SurrealGraphStore>>,
    sstore: Arc<SurrealStateStore>,
) -> IndexOrchestrator {
    let embedder = Arc::new(MockEmbedder::new(4, "test-model"));
    IndexOrchestrator::new(
        vstore as Arc<dyn VectorStore>,
        cstore as Arc<dyn ChunkStore>,
        gstore.map(|g| g as Arc<dyn GraphStore>),
        sstore as Arc<dyn StateStore>,
        embedder,
        Arc::new(SilentReporter),
    )
}

#[test]
fn orchestrator_run_full_mode() {
    let (v, c, g, s) = in_memory_stores(4);
    let orch = make_orchestrator(v.clone(), c.clone(), g.clone(), s.clone());

    let config = IndexConfig {
        mode: IndexMode::Full,
        dir: test_dir(),
        chunk_config: ChunkConfig {
            min_tokens: 50,
            max_tokens: 500,
            ..Default::default()
        },
        graph_config: GraphConfig::default(),
        batch_size: 10,
        retry_policy: RetryPolicy::default(),
        rate_limiter: RateLimiterConfig::default(),
        backpressure: BackpressureConfig::default(),
        io_threads: None,
        cpu_threads: None,
        no_graph: false,
    };

    let report = orch.run(config).unwrap();
    assert!(report.indexed > 0 || report.total_files > 0);
    assert_eq!(report.skipped, 0);
    assert_eq!(report.total_files, report.indexed + report.failed);

    let node_count = VectorStore::count(&*v).unwrap();
    assert!(node_count > 0);
}

#[test]
fn orchestrator_incremental_skips_unchanged_files() {
    let (v, c, g, s) = in_memory_stores(4);
    let orch = make_orchestrator(v.clone(), c.clone(), g.clone(), s.clone());

    let config = IndexConfig {
        mode: IndexMode::Full,
        dir: test_dir(),
        chunk_config: ChunkConfig {
            min_tokens: 50,
            max_tokens: 500,
            ..Default::default()
        },
        graph_config: GraphConfig::default(),
        batch_size: 10,
        retry_policy: RetryPolicy::default(),
        rate_limiter: RateLimiterConfig::default(),
        backpressure: BackpressureConfig::default(),
        io_threads: None,
        cpu_threads: None,
        no_graph: false,
    };

    let report1 = orch.run(config.clone()).unwrap();
    assert!(report1.indexed > 0);

    let config2 = IndexConfig {
        mode: IndexMode::Incremental,
        ..config.clone()
    };
    let report2 = orch.run(config2).unwrap();
    assert_eq!(report2.indexed, 0);
    assert_eq!(report2.skipped, report1.total_files);
}

#[test]
fn orchestrator_full_reindexes_all_files() {
    let (v, c, g, s) = in_memory_stores(4);
    let orch = make_orchestrator(v.clone(), c.clone(), g.clone(), s.clone());

    let config = IndexConfig {
        mode: IndexMode::Full,
        dir: test_dir(),
        chunk_config: ChunkConfig {
            min_tokens: 50,
            max_tokens: 500,
            ..Default::default()
        },
        graph_config: GraphConfig::default(),
        batch_size: 10,
        retry_policy: RetryPolicy::default(),
        rate_limiter: RateLimiterConfig::default(),
        backpressure: BackpressureConfig::default(),
        io_threads: None,
        cpu_threads: None,
        no_graph: false,
    };

    let report1 = orch.run(config.clone()).unwrap();
    assert!(report1.indexed > 0);

    let report2 = orch.run(config.clone()).unwrap();
    assert!(report2.indexed > 0);
}

#[test]
fn orchestrator_report_aggregation_valid() {
    let (v, c, g, s) = in_memory_stores(4);
    let orch = make_orchestrator(v, c, g, s);

    let config = IndexConfig {
        mode: IndexMode::Full,
        dir: test_dir(),
        chunk_config: ChunkConfig {
            min_tokens: 50,
            max_tokens: 500,
            ..Default::default()
        },
        graph_config: GraphConfig::default(),
        batch_size: 10,
        retry_policy: RetryPolicy::default(),
        rate_limiter: RateLimiterConfig::default(),
        backpressure: BackpressureConfig::default(),
        io_threads: None,
        cpu_threads: None,
        no_graph: false,
    };

    let report = orch.run(config).unwrap();
    assert_eq!(
        report.total_files,
        report.indexed + report.skipped + report.failed,
        "total_files must equal indexed + skipped + failed"
    );
}
