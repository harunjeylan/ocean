use std::sync::Arc;

use crate::ocean_index::report::IndexReport;

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    ScanStarted {
        dir: String,
        total: u64,
    },
    FileProcessing {
        path: String,
        current: u64,
        total: u64,
    },
    FileComplete {
        path: String,
        chunks: u64,
        embedded: u64,
        embed_skipped: u64,
        embed_failed: u64,
        edges: u64,
        nodes: u64,
        duration_ms: u64,
    },
    FileSkipped {
        path: String,
    },
    FileFailed {
        path: String,
        error: String,
    },
    GraphProgress {
        total_nodes: u64,
        total_edges: u64,
    },
    IndexComplete(IndexReport),
    BackpressurePaused {
        queue_len: usize,
        available_ai: usize,
        in_flight: u32,
    },
    BackpressureResumed,
    Retrying {
        path: String,
        attempt: u32,
        max_retries: u32,
        delay_ms: u64,
        error: String,
    },
}

pub trait ProgressReporter: Send + Sync {
    fn report(&self, event: ProgressEvent);
}

impl ProgressReporter for Box<dyn ProgressReporter> {
    fn report(&self, event: ProgressEvent) {
        (**self).report(event);
    }
}

impl ProgressReporter for Arc<dyn ProgressReporter> {
    fn report(&self, event: ProgressEvent) {
        (**self).report(event);
    }
}

pub struct ConsoleReporter;

impl ProgressReporter for ConsoleReporter {
    fn report(&self, event: ProgressEvent) {
        match event {
            ProgressEvent::ScanStarted { dir, total } => {
                println!("Found {} supported file(s) in '{}'. Indexing...", total, dir);
            }
            ProgressEvent::FileProcessing { path, current, total } => {
                println!("[{}/{}] Processing: {}", current, total, path);
            }
            ProgressEvent::FileComplete { path: _, chunks: _, embedded, embed_skipped, embed_failed, edges, nodes, duration_ms } => {
                println!("  Indexed: {} embedded, {} skipped, {} failed ({}ms)", embedded, embed_skipped, embed_failed, duration_ms);
                if nodes > 0 || edges > 0 {
                    println!("  Graph: {} nodes, {} edges", nodes, edges);
                }
            }
            ProgressEvent::FileSkipped { path } => {
                println!("  Skipped (unchanged): {}", path);
            }
            ProgressEvent::FileFailed { path, error } => {
                println!("  Failed: {} ({})", path, error);
            }
            ProgressEvent::GraphProgress { total_nodes, total_edges } => {
                println!("Graph total: {} nodes, {} edges", total_nodes, total_edges);
            }
            ProgressEvent::IndexComplete(report) => {
                let total = report.total_files;
                let indexed = report.indexed;
                let skipped = report.skipped;
                let failed = report.failed;
                let duration = report.duration_ms;
                println!(
                    "Indexing complete: {} indexed, {} skipped, {} failed ({} files, {}ms)",
                    indexed, skipped, failed, total, duration
                );
            }
            ProgressEvent::BackpressurePaused { queue_len, available_ai, in_flight } => {
                println!("  ⚠ Backpressure: queue={}, ai_permits={}, in_flight={}", queue_len, available_ai, in_flight);
            }
            ProgressEvent::BackpressureResumed => {
                println!("  ✓ Backpressure resolved, resuming");
            }
            ProgressEvent::Retrying { path, attempt, max_retries, delay_ms, error } => {
                println!("  Retry {}/{} for '{}' in {}ms: {}", attempt + 1, max_retries + 1, path, delay_ms, error);
            }
        }
    }
}

pub struct SilentReporter;

impl ProgressReporter for SilentReporter {
    fn report(&self, _event: ProgressEvent) {}
}
