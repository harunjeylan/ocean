use rmcp::model::{CallToolResult, ContentBlock};
use serde::Deserialize;

use crate::ocean_api::{self};
use crate::ocean_api::types::QueryRequest;
use crate::ocean_storage::vector_store::VectorStore;

use super::to_text;

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub query: String,
    pub mode: Option<String>,
    pub top_k: Option<usize>,
    pub expand_depth: Option<usize>,
    pub include_context: Option<bool>,
    pub db_path: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub filter_file_id: Option<String>,
    pub filter_heading: Option<String>,
    pub filter_block_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VectorStatusParams {
    pub db_path: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
}

pub async fn handle_query(params: QueryParams) -> CallToolResult {
    let db_path = crate::ocean_cli::config::resolve_db_path(params.db_path.as_deref(), None);
    let provider = params.provider.clone().unwrap_or_else(|| "ollama".to_string());
    let model = params.model.clone().unwrap_or_else(|| "nomic-embed-text".to_string());

    let query_text = params.query.clone();
    let request = QueryRequest {
        text: query_text,
        mode: params.mode.clone(),
        top_k: params.top_k.unwrap_or(10),
        expand_depth: params.expand_depth.unwrap_or(0),
        include_context: params.include_context.unwrap_or(false),
        context_chunks: None,
        no_cache: false,
        filter_file_id: params.filter_file_id.clone(),
        filter_heading: params.filter_heading.clone(),
        filter_block_type: params.filter_block_type.clone(),
        rerank_by_heading: false,
        rerank_by_file: false,
        model: Some(model),
        provider: Some(provider),
        dimension: None,
        api_key: None,
        base_url: None,
        db_path: Some(db_path.clone()),
        read_only: None,
    };

    match tokio::task::spawn_blocking(move || ocean_api::querying::query(request)).await {
        Ok(Ok(query_result)) => {
            let mut lines = Vec::new();
            lines.push(format!("Query returned {} result(s)", query_result.results.len()));
            lines.push(format!("Mode: {:?}", query_result.execution.query_mode));
            lines.push(format!("Duration: {}ms", query_result.execution.total_time_ms));
            lines.push(String::new());
            for (i, chunk) in query_result.results.iter().enumerate() {
                lines.push(format!("Result #{} (score: {:.4})", i + 1, chunk.score));
                if let Some(ref heading) = chunk.heading {
                    lines.push(format!("  Heading: {}", heading));
                }
                lines.push(format!("  File: {}", chunk.file_id));
                let preview: String = chunk.content.chars().take(300).collect();
                lines.push(format!("  Content: {}...", preview));
                lines.push(String::new());
            }
            if !query_result.context_windows.is_empty() {
                lines.push(format!("Context windows: {}", query_result.context_windows.len()));
                for (j, cw) in query_result.context_windows.iter().enumerate() {
                    lines.push(format!("  Window #{}: {} chunks", j + 1, cw.chunks.len()));
                }
            }
            to_text(lines.join("\n"))
        }
        Ok(Err(e)) => {
            let msg = e.to_string();
            if msg.contains("Not accessible") || msg.contains("Failed to open store") || msg.contains("No such file") {
                CallToolResult::error(vec![ContentBlock::text(format!("{}. Run 'ocean index .' first to index documents.", msg))])
            } else {
                CallToolResult::error(vec![ContentBlock::text(msg)])
            }
        }
        Err(e) => CallToolResult::error(vec![ContentBlock::text(format!("Task failed: {}", e))]),
    }
}

pub async fn handle_vector_status(params: VectorStatusParams) -> CallToolResult {
    let db_path = crate::ocean_cli::config::resolve_db_path(params.db_path.as_deref(), None);
    let vector_db = format!("{}/vector.db", db_path);

    if !std::path::PathBuf::from(&vector_db).exists() {
        return to_text(format!(
            "Vector Status\n  Database: {}\n  Accessible: No\n  Schema: Not initialized\n  Indexed chunks: 0\n  Suggestion: Run 'ocean index .' first.", vector_db
        ));
    }

    let provider = params.provider.clone().unwrap_or_else(|| "ollama".to_string());
    let model = params.model.clone().unwrap_or_else(|| "nomic-embed-text".to_string());

    let mut lines = vec![format!("Vector Status")];
    lines.push(format!("  Database: {}", vector_db));
    lines.push(format!("  Accessible: Yes"));

    match crate::ocean_storage::SurrealVectorStore::new_persistent_at(
        &vector_db,
        &crate::ocean_storage::config::StorageConfig::new(&db_path),
    ) {
        Ok(store) => {
            match store.initialize_schema(768) {
                Ok(_) => lines.push(format!("  Schema: Initialized")),
                Err(_) => lines.push(format!("  Schema: Not initialized")),
            }
            match store.count() {
                Ok(count) => lines.push(format!("  Indexed chunks: {}", count)),
                Err(e) => lines.push(format!("  Indexed chunks: error ({})", e)),
            }
            lines.push(format!("  Embedder: {} / {} (dim {})", provider, model, 768));
            match connection_check(&provider, &model).await {
                Ok(ms) => lines.push(format!("  Connection: OK ({}ms)", ms)),
                Err(e) => lines.push(format!("  Connection: FAILED ({})", e)),
            }
        }
        Err(e) => {
            lines.push(format!("  Schema: Not initialized"));
            lines.push(format!("  Indexed chunks: 0"));
            lines.push(format!("  Connection: FAILED ({})", e));
        }
    }

    to_text(lines.join("\n"))
}

async fn connection_check(provider: &str, model: &str) -> Result<u64, String> {
    let start = std::time::Instant::now();
    let url = match provider {
        "ollama" => format!("http://localhost:11434/api/embed"),
        "openai" => return Err("API key required".to_string()),
        "anthropic" => return Err("API key required".to_string()),
        "gemini" => return Err("API key required".to_string()),
        _ => return Err(format!("Unknown provider: {}", provider)),
    };

    let client = reqwest::blocking::Client::new();
    let payload = serde_json::json!({
        "model": model,
        "input": ["test"]
    });
    match client.post(&url).json(&payload).send() {
        Ok(_) => Ok(start.elapsed().as_millis() as u64),
        Err(e) => Err(format!("{}", e)),
    }
}
