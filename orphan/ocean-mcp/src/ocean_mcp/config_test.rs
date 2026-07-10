use crate::ocean_mcp::config::McpConfig;
use crate::ocean_cli::config::OceanConfig;

#[test]
fn test_mcp_config_defaults() {
    let cfg = McpConfig::default();
    assert_eq!(cfg.embedding_provider, "ollama");
    assert_eq!(cfg.embedding_model, "nomic-embed-text");
    assert_eq!(cfg.embedding_dimension, 768);
    assert!(cfg.base_url.contains("localhost"));
}

#[test]
fn test_mcp_config_from_ocean_config() {
    let mut ocean = OceanConfig::default();
    ocean.embedding.provider = Some("openai".to_string());
    ocean.embedding.model = Some("text-embedding-3-small".to_string());
    ocean.embedding.dimension = Some(1536);

    let cfg = McpConfig::from_ocean_config(Some(&ocean));
    assert_eq!(cfg.embedding_provider, "openai");
    assert_eq!(cfg.embedding_model, "text-embedding-3-small");
    assert_eq!(cfg.embedding_dimension, 1536);
}

#[test]
fn test_mcp_config_from_none() {
    let cfg = McpConfig::from_ocean_config(None);
    assert_eq!(cfg.embedding_provider, "ollama");
    assert_eq!(cfg.embedding_dimension, 768);
}

#[test]
fn test_mcp_config_resolve_db_path() {
    let path = McpConfig::resolve_db_path(Some("/custom/path"));
    assert_eq!(path, "/custom/path");
}

#[test]
fn test_mcp_config_with_api_key() {
    let mut ocean = OceanConfig::default();
    ocean.embedding.api_key = Some("sk-test-key".to_string());

    let cfg = McpConfig::from_ocean_config(Some(&ocean));
    assert_eq!(cfg.api_key.as_deref(), Some("sk-test-key"));
}

#[test]
fn test_mcp_config_with_custom_base_url() {
    let mut ocean = OceanConfig::default();
    ocean.embedding.base_url = Some("http://custom:8080".to_string());
    ocean.embedding.provider = Some("ollama".to_string());

    let cfg = McpConfig::from_ocean_config(Some(&ocean));
    assert_eq!(cfg.base_url, "http://custom:8080");
}
