use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub base_path: String,
    pub files_subdir: String,
    pub chunks_subdir: String,
    pub vectors_subdir: String,
    pub graph_subdir: String,
    pub state_subdir: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_path: String::new(),
            files_subdir: "files.db".into(),
            chunks_subdir: "chunks.db".into(),
            vectors_subdir: "vectors.db".into(),
            graph_subdir: "graph.db".into(),
            state_subdir: "state.db".into(),
        }
    }
}

impl StorageConfig {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
            ..Default::default()
        }
    }

    pub fn files_path(&self) -> String {
        PathBuf::from(&self.base_path)
            .join(&self.files_subdir)
            .to_string_lossy()
            .to_string()
    }

    pub fn chunks_path(&self) -> String {
        PathBuf::from(&self.base_path)
            .join(&self.chunks_subdir)
            .to_string_lossy()
            .to_string()
    }

    pub fn vectors_path(&self) -> String {
        PathBuf::from(&self.base_path)
            .join(&self.vectors_subdir)
            .to_string_lossy()
            .to_string()
    }

    pub fn graph_path(&self) -> String {
        PathBuf::from(&self.base_path)
            .join(&self.graph_subdir)
            .to_string_lossy()
            .to_string()
    }

    pub fn state_path(&self) -> String {
        PathBuf::from(&self.base_path)
            .join(&self.state_subdir)
            .to_string_lossy()
            .to_string()
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        for p in [&self.files_path(), &self.chunks_path(), &self.vectors_path(), &self.graph_path(), &self.state_path()] {
            if let Some(parent) = PathBuf::from(p).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        Ok(())
    }
}
