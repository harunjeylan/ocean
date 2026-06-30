use crate::ocean_cli::config::OceanConfig;

#[derive(Debug, Clone)]
pub struct McpConfig {
    pub db_path: String,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dimension: usize,
    pub api_key: Option<String>,
    pub base_url: String,
}

impl McpConfig {
    pub fn from_ocean_config(config: Option<&OceanConfig>) -> Self {
        match config {
            Some(cfg) => {
                let provider = cfg.embedding.provider.clone().unwrap_or_else(|| "ollama".to_string());
                let model = cfg.embedding.model.clone().unwrap_or_else(|| "nomic-embed-text".to_string());
                let dimension = cfg.embedding.dimension.unwrap_or(768);
                let api_key = cfg.embedding.api_key.clone();
                let base_url = cfg.embedding.base_url.clone().unwrap_or_else(|| {
                    if provider == "ollama" {
                        "http://localhost:11434".to_string()
                    } else {
                        String::new()
                    }
                });
                let db_path = crate::ocean_cli::config::resolve_db_path(
                    None,
                    cfg.index.db_path.as_deref(),
                );
                Self { db_path, embedding_provider: provider, embedding_model: model, embedding_dimension: dimension, api_key, base_url }
            }
            None => Self::default(),
        }
    }

    pub fn resolve_db_path(cli: Option<&str>) -> String {
        crate::ocean_cli::config::resolve_db_path(cli, None)
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            db_path: crate::ocean_cli::config::resolve_db_path(None, None),
            embedding_provider: "ollama".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            embedding_dimension: 768,
            api_key: None,
            base_url: "http://localhost:11434".to_string(),
        }
    }
}
