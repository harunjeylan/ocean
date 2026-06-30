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
}

pub trait ProgressReporter: Send {
    fn report(&self, event: ProgressEvent);
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
        }
    }
}

pub struct SilentReporter;

impl ProgressReporter for SilentReporter {
    fn report(&self, _event: ProgressEvent) {}
}
