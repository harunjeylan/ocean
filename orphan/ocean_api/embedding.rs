use crate::ocean_vector::embedder::Embedder;
use crate::ocean_vector::{AnthropicEmbedder, GeminiEmbedder, OllamaEmbedder, OpenAIEmbedder};

use super::types::ApiError;

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub dimension: usize,
    pub api_key: Option<String>,
    pub base_url: String,
}

impl EmbeddingConfig {
    pub fn new(
        provider: &str,
        model: &str,
        dimension: usize,
        api_key: Option<String>,
        base_url: &str,
    ) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.to_string(),
            dimension,
            api_key,
            base_url: base_url.to_string(),
        }
    }

    pub fn resolve_provider(cli: Option<&str>, provider: Option<&str>) -> String {
        cli.or(provider).unwrap_or("ollama").to_string()
    }

    pub fn resolve_model(cli: Option<&str>, model: Option<&str>) -> String {
        cli.or(model).unwrap_or("nomic-embed-text").to_string()
    }

    pub fn resolve_dimension(
        cli_dim: Option<usize>,
        config_dim: Option<usize>,
        provider: &str,
        model: &str,
    ) -> usize {
        if let Some(d) = cli_dim {
            return d;
        }
        if let Some(d) = config_dim {
            return d;
        }
        match provider {
            "openai" if model.contains("large") => 3072,
            "openai" if model.contains("small") => 1536,
            "openai" => 1536,
            "gemini" => 3072,
            _ => 768,
        }
    }

    pub fn resolve_base_url(
        provider: &str,
        cli_url: Option<&str>,
        config_url: Option<&str>,
    ) -> String {
        let from_config = config_url.filter(|s| !s.is_empty());
        let from_cli = cli_url.filter(|s| !s.is_empty());

        if provider == "ollama" {
            if let Some(url) = from_cli {
                return url.to_string();
            }
            if let Some(url) = from_config {
                return url.to_string();
            }
            return "http://localhost:11434".to_string();
        }

        if let Some(url) = from_config {
            return url.to_string();
        }

        match provider {
            "openai" => "https://api.openai.com/v1".to_string(),
            "anthropic" => "https://api.anthropic.com/v1".to_string(),
            _ => String::new(),
        }
    }
}

pub fn create_embedder(
    provider: &str,
    model: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Box<dyn Embedder>, ApiError> {
    match provider {
        "ollama" => {
            let url = if base_url.is_empty() { "http://localhost:11434" } else { base_url };
            Ok(Box::new(
                OllamaEmbedder::new(model, url)
                    .map_err(|e| ApiError::EmbedderError(format!("Failed to create Ollama embedder: {}", e)))?,
            ))
        }
        "openai" => {
            let key = api_key.ok_or_else(|| ApiError::EmbedderError("--api-key is required for openai provider".into()))?;
            let url = if base_url.is_empty() { "https://api.openai.com/v1" } else { base_url };
            Ok(Box::new(
                OpenAIEmbedder::new(model, url, key)
                    .map_err(|e| ApiError::EmbedderError(format!("Failed to create OpenAI embedder: {}", e)))?,
            ))
        }
        "anthropic" => {
            let key = api_key.ok_or_else(|| ApiError::EmbedderError("--api-key is required for anthropic provider".into()))?;
            let url = if base_url.is_empty() { "https://api.anthropic.com/v1" } else { base_url };
            Ok(Box::new(
                AnthropicEmbedder::new(model, url, key)
                    .map_err(|e| ApiError::EmbedderError(format!("Failed to create Anthropic embedder: {}", e)))?,
            ))
        }
        "gemini" => {
            let key = api_key.ok_or_else(|| ApiError::EmbedderError("--api-key is required for gemini provider".into()))?;
            Ok(Box::new(
                GeminiEmbedder::new(model, key)
                    .map_err(|e| ApiError::EmbedderError(format!("Failed to create Gemini embedder: {}", e)))?,
            ))
        }
        other => Err(ApiError::EmbedderError(format!("unsupported provider '{}'. Use: ollama, openai, anthropic, gemini", other))),
    }
}

pub fn api_key(cli_key: Option<&str>, config_key: Option<&str>, env_var: Option<&str>) -> Option<String> {
    if let Some(k) = cli_key {
        return Some(k.to_string());
    }
    if let Some(k) = config_key {
        return Some(k.to_string());
    }
    env_var.map(|s| s.to_string())
}
