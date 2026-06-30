#[derive(Debug, Clone)]
pub struct StagedWrite {
    pub store_name: String,
    pub table: String,
    pub record_id: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct TransactionStaging {
    pub staged_writes: Vec<StagedWrite>,
}

impl TransactionStaging {
    pub fn new() -> Self {
        Self {
            staged_writes: Vec::new(),
        }
    }

    pub fn add_write(&mut self, store_name: &str, table: &str, record_id: &str, data: serde_json::Value) {
        self.staged_writes.push(StagedWrite {
            store_name: store_name.to_string(),
            table: table.to_string(),
            record_id: record_id.to_string(),
            data,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.staged_writes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.staged_writes.len()
    }

    pub fn clear(&mut self) {
        self.staged_writes.clear();
    }

    pub fn drain(&mut self) -> Vec<StagedWrite> {
        std::mem::take(&mut self.staged_writes)
    }
}

#[derive(Debug, Clone)]
pub struct TransactionError {
    pub succeeded: Vec<String>,
    pub failed: Vec<(String, String)>,
}

impl std::fmt::Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TransactionError: {} succeeded, {} failed: {:?}",
            self.succeeded.len(),
            self.failed.len(),
            self.failed
        )
    }
}

impl std::error::Error for TransactionError {}
