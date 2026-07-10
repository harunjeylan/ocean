#[derive(Debug, Clone)]
pub enum FileIndexStatus {
    Indexed,
    Skipped,
    Failed,
}

#[derive(Debug, Clone)]
pub struct FileResult {
    pub path: String,
    pub status: FileIndexStatus,
    pub chunks: u64,
    pub embedded: u64,
    pub embed_skipped: u64,
    pub embed_failed: u64,
    pub nodes: u64,
    pub edges: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IndexReport {
    pub total_files: u64,
    pub indexed: u64,
    pub skipped: u64,
    pub failed: u64,
    pub total_chunks: u64,
    pub total_edges: u64,
    pub total_nodes: u64,
    pub duration_ms: u64,
    pub per_file: Vec<FileResult>,
}

impl IndexReport {
    pub fn new() -> Self {
        Self {
            total_files: 0,
            indexed: 0,
            skipped: 0,
            failed: 0,
            total_chunks: 0,
            total_edges: 0,
            total_nodes: 0,
            duration_ms: 0,
            per_file: Vec::new(),
        }
    }

    pub fn merge(&mut self, result: FileResult) {
        self.total_files += 1;
        self.total_chunks += result.chunks;
        self.total_edges += result.edges;
        self.total_nodes += result.nodes;
        match result.status {
            FileIndexStatus::Indexed => self.indexed += 1,
            FileIndexStatus::Skipped => self.skipped += 1,
            FileIndexStatus::Failed => self.failed += 1,
        }
        self.per_file.push(result);
    }
}

impl Default for IndexReport {
    fn default() -> Self {
        Self::new()
    }
}
