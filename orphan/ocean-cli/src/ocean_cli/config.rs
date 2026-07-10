use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn resolve_env_vars(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next();
            let mut var_name = String::new();
            for ch in chars.by_ref() {
                if ch == '}' {
                    break;
                }
                var_name.push(ch);
            }
            let val = std::env::var(&var_name).unwrap_or_default();
            result.push_str(&val);
        } else {
            result.push(c);
        }
    }
    result
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EmbeddingConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub dimension: Option<usize>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IndexConfigOpt {
    pub batch_size: Option<usize>,
    pub db_path: Option<String>,
    pub reindex: Option<bool>,
    pub no_graph: Option<bool>,
    pub no_references: Option<bool>,
    pub no_entities: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QueryConfigOpt {
    pub top_k: Option<usize>,
    pub db_path: Option<String>,
    pub mode: Option<String>,
    pub expand_depth: Option<usize>,
    pub context: Option<bool>,
    pub context_chunks: Option<usize>,
    pub verbose: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfigOpt {
    pub embedding_cache_size: Option<usize>,
    pub query_cache_size: Option<usize>,
    pub graph_cache_size: Option<usize>,
    pub query_ttl_secs: Option<u64>,
    pub embedding_cache_path: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfigOpt {
    pub mode: Option<String>,
    pub io_threads: Option<usize>,
    pub cpu_threads: Option<usize>,
    pub max_ai_concurrent: Option<usize>,
    pub max_retries: Option<u32>,
    pub retry_backoff_ms: Option<u64>,
    pub max_queue_size: Option<usize>,
    pub max_in_flight: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExperimentalsConfig {
    pub vector: Option<bool>,
    pub graph: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfigOpt {
    pub sandbox: Option<bool>,
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ObservabilityConfigOpt {
    pub log_format: Option<String>,
    pub log_file: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OceanConfig {
    pub embedding: EmbeddingConfig,
    pub index: IndexConfigOpt,
    pub query: QueryConfigOpt,
    pub runtime: RuntimeConfigOpt,
    pub cache: CacheConfigOpt,
    pub security: SecurityConfigOpt,
    pub observability: ObservabilityConfigOpt,
    pub experimentals: ExperimentalsConfig,
}

impl OceanConfig {
    fn resolve_env_vars_mut(&mut self) {
        let resolve = |s: &mut Option<String>| {
            *s = s.as_deref().map(resolve_env_vars);
        };
        resolve(&mut self.embedding.api_key);
        resolve(&mut self.embedding.base_url);
        resolve(&mut self.index.db_path);
        resolve(&mut self.query.db_path);
    }

    pub fn load() -> Option<Self> {
        let mut cfg: OceanConfig = OceanConfig::default();
        let mut found = false;

        for path in [local_config_path(), global_config_path()].iter().flatten() {
            if path.exists() {
                let content = std::fs::read_to_string(path).ok()?;
                match serde_json::from_str::<OceanConfig>(&content) {
                    Ok(partial) => {
                        merge_config(&mut cfg, partial);
                        found = true;
                    }
                    Err(e) => {
                        eprintln!("Warning: invalid config at {}: {}", path.display(), e);
                    }
                }
            }
        }

        if found {
            cfg.resolve_env_vars_mut();
            Some(cfg)
        } else {
            None
        }
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if let Some(ref mode) = self.runtime.mode {
            let valid = matches!(mode.to_lowercase().as_str(), "desktop" | "server" | "embedded");
            if !valid {
                errors.push(format!("runtime.mode: invalid value '{}'. Use: desktop, server, embedded", mode));
            }
        }

        if let Some(ref fmt) = self.observability.log_format {
            let valid = matches!(fmt.to_lowercase().as_str(), "console" | "json");
            if !valid {
                errors.push(format!("observability.log_format: invalid value '{}'. Use: console, json", fmt));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    ParseError { path: PathBuf, detail: String },
    ValidationError { fields: Vec<String> },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NotFound => write!(f, "config file not found"),
            ConfigError::ParseError { path, detail } => {
                write!(f, "failed to parse config at '{}': {}", path.display(), detail)
            }
            ConfigError::ValidationError { fields } => {
                write!(f, "config validation failed: {}", fields.join(", "))
            }
        }
    }
}

impl std::error::Error for ConfigError {}

fn merge_config(base: &mut OceanConfig, partial: OceanConfig) {
    macro_rules! merge_opt {
        ($dst:expr, $src:expr) => {
            if let Some(v) = $src {
                $dst = Some(v);
            }
        };
    }

    merge_opt!(base.embedding.provider, partial.embedding.provider);
    merge_opt!(base.embedding.model, partial.embedding.model);
    merge_opt!(base.embedding.dimension, partial.embedding.dimension);
    merge_opt!(base.embedding.api_key, partial.embedding.api_key);
    merge_opt!(base.embedding.base_url, partial.embedding.base_url);
    merge_opt!(base.index.batch_size, partial.index.batch_size);
    merge_opt!(base.index.db_path, partial.index.db_path);
    merge_opt!(base.index.reindex, partial.index.reindex);
    merge_opt!(base.index.no_graph, partial.index.no_graph);
    merge_opt!(base.index.no_references, partial.index.no_references);
    merge_opt!(base.index.no_entities, partial.index.no_entities);
    merge_opt!(base.query.top_k, partial.query.top_k);
    merge_opt!(base.query.db_path, partial.query.db_path);
    merge_opt!(base.query.mode, partial.query.mode);
    merge_opt!(base.query.expand_depth, partial.query.expand_depth);
    merge_opt!(base.query.context, partial.query.context);
    merge_opt!(base.query.context_chunks, partial.query.context_chunks);
    merge_opt!(base.query.verbose, partial.query.verbose);
    merge_opt!(base.runtime.mode, partial.runtime.mode);
    merge_opt!(base.runtime.io_threads, partial.runtime.io_threads);
    merge_opt!(base.runtime.cpu_threads, partial.runtime.cpu_threads);
    merge_opt!(base.runtime.max_ai_concurrent, partial.runtime.max_ai_concurrent);
    merge_opt!(base.runtime.max_retries, partial.runtime.max_retries);
    merge_opt!(base.runtime.retry_backoff_ms, partial.runtime.retry_backoff_ms);
    merge_opt!(base.runtime.max_queue_size, partial.runtime.max_queue_size);
    merge_opt!(base.runtime.max_in_flight, partial.runtime.max_in_flight);
    merge_opt!(base.cache.embedding_cache_size, partial.cache.embedding_cache_size);
    merge_opt!(base.cache.query_cache_size, partial.cache.query_cache_size);
    merge_opt!(base.cache.graph_cache_size, partial.cache.graph_cache_size);
    merge_opt!(base.cache.query_ttl_secs, partial.cache.query_ttl_secs);
    merge_opt!(base.cache.embedding_cache_path, partial.cache.embedding_cache_path);
    merge_opt!(base.cache.enabled, partial.cache.enabled);
    merge_opt!(base.security.sandbox, partial.security.sandbox);
    merge_opt!(base.security.read_only, partial.security.read_only);
    merge_opt!(base.observability.log_format, partial.observability.log_format);
    merge_opt!(base.observability.log_file, partial.observability.log_file);
    merge_opt!(base.experimentals.vector, partial.experimentals.vector);
    merge_opt!(base.experimentals.graph, partial.experimentals.graph);
}

pub fn resolve_api_key(cli_key: Option<&str>, config_key: Option<&str>, env_var: Option<&str>) -> Option<String> {
    if let Some(k) = cli_key {
        return Some(k.to_string());
    }
    if let Some(k) = config_key {
        return Some(k.to_string());
    }
    env_var.map(|s| s.to_string())
}

pub fn resolve_db_path(cli: Option<&str>, config: Option<&str>) -> String {
    if let Some(p) = cli.filter(|s| !s.is_empty()) {
        return p.to_string();
    }
    if let Some(p) = config.filter(|s| !s.is_empty()) {
        return p.to_string();
    }
    default_db_path()
}

pub fn resolve_base_url(provider: &str, cli_ollama_url: Option<&str>, config_base_url: Option<&str>) -> String {
    let from_config = config_base_url.filter(|s| !s.is_empty());
    let from_cli_ollama = cli_ollama_url.filter(|s| !s.is_empty());

    // CLI --ollama-url is used only when provider is ollama
    if provider == "ollama" {
        if let Some(url) = from_cli_ollama {
            return url.to_string();
        }
        if let Some(url) = from_config {
            return url.to_string();
        }
        return "http://localhost:11434".to_string();
    }

    // For non-ollama providers, config base_url takes priority
    if let Some(url) = from_config {
        return url.to_string();
    }

    match provider {
        "openai" => "https://api.openai.com/v1".to_string(),
        "anthropic" => "https://api.anthropic.com/v1".to_string(),
        _ => String::new(),
    }
}

pub fn load_env_files() {
    let paths = [
        global_env_path(),
        Some(PathBuf::from(".env")),
        local_env_path(),
    ];

    for path in paths.iter().flatten() {
        if path.exists() {
            let _ = dotenvy::from_path(path);
        }
    }
}

fn local_config_path() -> Option<PathBuf> {
    Some(PathBuf::from(".ocean").join("config.json"))
}

fn local_env_path() -> Option<PathBuf> {
    Some(PathBuf::from(".ocean").join(".env"))
}

fn global_config_path() -> Option<PathBuf> {
    base_ocean_dir().map(|p| p.join("config.json"))
}

fn global_env_path() -> Option<PathBuf> {
    base_ocean_dir().map(|p| p.join(".env"))
}

fn base_ocean_dir() -> Option<PathBuf> {
    let home = std::env::var_os(if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    })?;
    Some(PathBuf::from(home).join(".ocean"))
}

fn default_db_path() -> String {
    let dir_name = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "default".to_string());

    let kebab = to_kebab_case(&dir_name);

    base_ocean_dir()
        .map(|p| p.join("database").join(&kebab).to_string_lossy().to_string())
        .unwrap_or_else(|| kebab)
}

pub fn resolve_ocean_db_path(cli: Option<&str>, config: Option<&str>) -> String {
    format!("{}/ocean.db", resolve_db_path(cli, config))
}

pub fn resolve_vector_db_path(cli: Option<&str>, config: Option<&str>) -> String {
    format!("{}/vector.db", resolve_db_path(cli, config))
}

pub fn resolve_graph_db_path(cli: Option<&str>, config: Option<&str>) -> String {
    format!("{}/graph.db", resolve_db_path(cli, config))
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_sep = false;
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            prev_is_sep = false;
        } else if !prev_is_sep {
            result.push('-');
            prev_is_sep = true;
        }
    }
    result.trim_end_matches('-').to_string()
}
