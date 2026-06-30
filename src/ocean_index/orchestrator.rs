use std::sync::Arc;
use std::time::Instant;

use sha2::{Digest, Sha256};

use crate::ocean_fs::scan_dir;
use crate::ocean_storage::{ChunkStore, GraphStore, IndexStatus, StateStore, VectorStore};
use crate::ocean_vector::embedder::Embedder;

use super::config::{IndexConfig, IndexMode};
use super::error::IndexError;
use super::processor::FileProcessor;
use super::progress::{ProgressEvent, ProgressReporter};
use super::report::{FileIndexStatus, FileResult, IndexReport};

fn stable_state_id(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    hash[..16].to_string()
}

pub struct IndexOrchestrator {
    processor: FileProcessor,
    state_store: Arc<dyn StateStore>,
    reporter: Box<dyn ProgressReporter>,
}

impl IndexOrchestrator {
    pub fn new(
        vector_store: Arc<dyn VectorStore>,
        chunk_store: Arc<dyn ChunkStore>,
        graph_store: Option<Arc<dyn GraphStore>>,
        state_store: Arc<dyn StateStore>,
        embedder: Arc<dyn Embedder>,
        reporter: Box<dyn ProgressReporter>,
    ) -> Self {
        let processor = FileProcessor::new(
            embedder,
            vector_store,
            chunk_store,
            graph_store,
        );
        Self {
            processor,
            state_store,
            reporter,
        }
    }

    pub fn run(&self, config: IndexConfig) -> Result<IndexReport, IndexError> {
        let start = Instant::now();
        let dir = config.dir.clone();

        let metas = scan_dir(&dir).map_err(|e| IndexError::ScanError(format!("{}", e)))?;
        let total_files = metas.len() as u64;

        self.reporter.report(ProgressEvent::ScanStarted {
            dir: dir.clone(),
            total: total_files,
        });

        if total_files == 0 {
            let mut report = IndexReport::new();
            report.duration_ms = start.elapsed().as_millis() as u64;
            self.reporter.report(ProgressEvent::IndexComplete(report.clone()));
            return Ok(report);
        }

        let mut report = IndexReport::new();

        for (i, meta) in metas.iter().enumerate() {
            let current = (i + 1) as u64;
            let path_str = &meta.path;

            if config.mode == IndexMode::Incremental {
                let state_id = stable_state_id(&meta.path);
                match self.state_store.get_state(&state_id) {
                    Ok(Some(state)) if state.hash == meta.hash => {
                        let fr = FileResult {
                            path: path_str.clone(),
                            status: FileIndexStatus::Skipped,
                            chunks: 0,
                            embedded: 0,
                            embed_skipped: 0,
                            embed_failed: 0,
                            nodes: 0,
                            edges: 0,
                            duration_ms: 0,
                            error: None,
                        };
                        self.reporter.report(ProgressEvent::FileSkipped {
                            path: path_str.clone(),
                        });
                        report.merge(fr);
                        continue;
                    }
                    Err(e) => {
                        let fr = FileResult {
                            path: path_str.clone(),
                            status: FileIndexStatus::Failed,
                            chunks: 0,
                            embedded: 0,
                            embed_skipped: 0,
                            embed_failed: 0,
                            nodes: 0,
                            edges: 0,
                            duration_ms: 0,
                            error: Some(format!("state check: {}", e)),
                        };
                        self.reporter.report(ProgressEvent::FileFailed {
                            path: path_str.clone(),
                            error: e.to_string(),
                        });
                        let _ = self.state_store.update_state(&state_id, &meta.hash, IndexStatus::Failed);
                        report.merge(fr);
                        continue;
                    }
                    _ => {}
                }
            }

            self.reporter.report(ProgressEvent::FileProcessing {
                path: path_str.clone(),
                current,
                total: total_files,
            });

            match self.processor.process(path_str, &meta.id, &config) {
                Ok(fr) => {
                    let _ = self.state_store.update_state(&stable_state_id(&meta.path), &meta.hash, IndexStatus::Indexed);
                    self.reporter.report(ProgressEvent::FileComplete {
                        path: path_str.clone(),
                        chunks: fr.chunks,
                        embedded: fr.embedded,
                        embed_skipped: fr.embed_skipped,
                        embed_failed: fr.embed_failed,
                        edges: fr.edges,
                        nodes: fr.nodes,
                        duration_ms: fr.duration_ms,
                    });
                    report.merge(fr);
                }
                Err(e) => {
                    let _ = self.state_store.update_state(&stable_state_id(&meta.path), &meta.hash, IndexStatus::Failed);
                    self.reporter.report(ProgressEvent::FileFailed {
                        path: path_str.clone(),
                        error: e.to_string(),
                    });
                    let fr = FileResult {
                        path: path_str.clone(),
                        status: FileIndexStatus::Failed,
                        chunks: 0,
                        embedded: 0,
                        embed_skipped: 0,
                        embed_failed: 0,
                        nodes: 0,
                        edges: 0,
                        duration_ms: 0,
                        error: Some(e.to_string()),
                    };
                    report.merge(fr);
                }
            }
        }

        if report.total_nodes > 0 || report.total_edges > 0 {
            self.reporter.report(ProgressEvent::GraphProgress {
                total_nodes: report.total_nodes,
                total_edges: report.total_edges,
            });
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        self.reporter.report(ProgressEvent::IndexComplete(report.clone()));

        Ok(report)
    }
}
