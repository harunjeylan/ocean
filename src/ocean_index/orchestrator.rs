use std::sync::Arc;
use std::time::Instant;

use rayon::prelude::*;
use sha2::{Digest, Sha256};

use crate::ocean_fs::scan_dir;
use crate::ocean_storage::{ChunkStore, GraphStore, IndexStatus, StateStore, VectorStore};
use crate::ocean_vector::embedder::Embedder;

use super::config::{BackpressureConfig, IndexConfig, IndexMode};
use super::error::IndexError;
use super::job_queue::{FileJob, JobPriority, JobQueue};
use super::processor::FileProcessor;
use super::progress::{ProgressEvent, ProgressReporter};
use super::report::{FileIndexStatus, FileResult, IndexReport};
use super::worker_pool::WorkerPool;

fn stable_state_id(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    hash[..16].to_string()
}

pub struct IndexOrchestrator {
    processor: FileProcessor,
    state_store: Arc<dyn StateStore>,
    reporter: Arc<dyn ProgressReporter>,
    pool: WorkerPool,
}

impl IndexOrchestrator {
    pub fn new(
        vector_store: Arc<dyn VectorStore>,
        chunk_store: Arc<dyn ChunkStore>,
        graph_store: Option<Arc<dyn GraphStore>>,
        state_store: Arc<dyn StateStore>,
        embedder: Arc<dyn Embedder>,
        reporter: Arc<dyn ProgressReporter>,
    ) -> Self {
        let processor = FileProcessor::new(
            embedder,
            vector_store,
            chunk_store,
            graph_store,
        );
        let pool = WorkerPool::default();
        Self {
            processor,
            state_store,
            reporter,
            pool,
        }
    }

    pub fn run(&self, config: IndexConfig) -> Result<IndexReport, IndexError> {
        let start = Instant::now();
        let dir = config.dir.clone();

        let metas = self.pool.run_io(|| scan_dir(&dir))?.map_err(|e| IndexError::ScanError(format!("{}", e)))?;
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

        let priority = match config.mode {
            IndexMode::Watch => JobPriority::High,
            IndexMode::Incremental => JobPriority::Normal,
            IndexMode::Full => JobPriority::Low,
        };

        let mut job_queue = JobQueue::new(config.backpressure.max_queue_size);
        for meta in &metas {
            job_queue.enqueue(FileJob {
                file_id: meta.id.clone(),
                path: meta.path.clone(),
                priority,
                retry_count: 0,
            }).map_err(|e| IndexError::Runtime(e))?;
        }

        let bp = &config.backpressure;
        let mut report = IndexReport::new();
        let mut was_paused = false;

        loop {
            if job_queue.is_empty() {
                break;
            }

            let batch = self.dequeue_with_backpressure(
                &mut job_queue,
                bp,
                &mut was_paused,
            );

            if batch.is_empty() {
                continue;
            }

            let results: Vec<FileResult> = self.pool.cpu_pool.install(|| {
                batch
                    .par_iter()
                    .map(|job| {
                        self.process_file(job, &config)
                    })
                    .collect()
            });

            for fr in results {
                match fr.status {
                    FileIndexStatus::Indexed => {
                        let state_id = stable_state_id(&fr.path);
                        let _ = self.state_store.update_state(&state_id, "", IndexStatus::Indexed);
                        self.reporter.report(ProgressEvent::FileComplete {
                            path: fr.path.clone(),
                            chunks: fr.chunks,
                            embedded: fr.embedded,
                            embed_skipped: fr.embed_skipped,
                            embed_failed: fr.embed_failed,
                            edges: fr.edges,
                            nodes: fr.nodes,
                            duration_ms: fr.duration_ms,
                        });
                    }
                    FileIndexStatus::Skipped => {
                        self.reporter.report(ProgressEvent::FileSkipped {
                            path: fr.path.clone(),
                        });
                    }
                    FileIndexStatus::Failed => {
                        let state_id = stable_state_id(&fr.path);
                        let _ = self.state_store.update_state(&state_id, "", IndexStatus::Failed);
                        let err = fr.error.clone().unwrap_or_default();
                        self.reporter.report(ProgressEvent::FileFailed {
                            path: fr.path.clone(),
                            error: err,
                        });
                    }
                }
                report.merge(fr);
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

    fn dequeue_with_backpressure(
        &self,
        queue: &mut JobQueue,
        bp: &BackpressureConfig,
        was_paused: &mut bool,
    ) -> Vec<FileJob> {
        loop {
            let ai_permits = self.pool.ai_semaphore.available_permits();
            let queue_len = queue.len();
            let under_backlog = !queue.has_backlog();
            let ai_available = ai_permits > 0 || bp.max_ai_concurrent == 0;
            let under_pressure = under_backlog && ai_available;

            if under_pressure {
                if *was_paused {
                    *was_paused = false;
                    self.reporter.report(ProgressEvent::BackpressureResumed);
                }
                let batch_size = bp.max_in_flight.min(queue_len);
                return queue.dequeue_batch(batch_size);
            }

            if !*was_paused {
                *was_paused = true;
                self.reporter.report(ProgressEvent::BackpressurePaused {
                    queue_len,
                    available_ai: ai_permits,
                    in_flight: 0,
                });
            }

            std::thread::sleep(std::time::Duration::from_millis(bp.pause_check_ms));
        }
    }

    fn process_file(&self, job: &FileJob, config: &IndexConfig) -> FileResult {
        let state_id = stable_state_id(&job.path);

        if config.mode == IndexMode::Incremental {
            match self.state_store.get_state(&state_id) {
                Ok(Some(state)) if state.hash.is_empty() || state.status == IndexStatus::Indexed => {
                    return FileResult {
                        path: job.path.clone(),
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
                }
                Ok(_) => {}
                Err(e) => {
                    return FileResult {
                        path: job.path.clone(),
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
                }
            }
        }

        self.reporter.report(ProgressEvent::FileProcessing {
            path: job.path.clone(),
            current: 0,
            total: 0,
        });

        match self.processor.process_with_retry(
            &job.path,
            &job.file_id,
            config,
            Some(&self.pool),
            Some(self.reporter.as_ref()),
        ) {
            Ok(fr) => fr,
            Err(e) => {
                FileResult {
                    path: job.path.clone(),
                    status: FileIndexStatus::Failed,
                    chunks: 0,
                    embedded: 0,
                    embed_skipped: 0,
                    embed_failed: 0,
                    nodes: 0,
                    edges: 0,
                    duration_ms: 0,
                    error: Some(e.to_string()),
                }
            }
        }
    }
}
