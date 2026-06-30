use std::io::{self, Write};
use std::path::PathBuf;

fn prompt(prompt: &str, default: &str) -> String {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() { default.to_string() } else { trimmed }
}

fn prompt_optional(prompt: &str) -> Option<String> {
    print!("{}: ", prompt);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

fn prompt_masked(prompt: &str) -> Option<String> {
    print!("{}: ", prompt);
    io::stdout().flush().ok();
    match rpassword::read_password() {
        Ok(p) if p.is_empty() => None,
        Ok(p) => Some(p),
        Err(_) => {
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            let trimmed = input.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
    }
}

pub fn default_model(provider: &str) -> &'static str {
    match provider {
        "openai" => "text-embedding-3-small",
        "gemini" => "gemini-embedding-001",
        "anthropic" => "cohere-embed-multilingual-v3",
        _ => "nomic-embed-text",
    }
}

pub fn default_dimension(provider: &str, model: &str) -> usize {
    match (provider, model) {
        ("openai", m) if m.contains("large") => 3072,
        ("openai", _) => 1536,
        ("gemini", _) => 3072,
        _ => 768,
    }
}

pub fn default_base_url(provider: &str) -> &'static str {
    match provider {
        "ollama" => "http://localhost:11434",
        "openai" => "https://api.openai.com/v1",
        "anthropic" => "https://api.anthropic.com/v1",
        "gemini" => "",
        _ => "",
    }
}

pub fn section_exists(content: &str, marker: &str) -> bool {
    content.lines().any(|l| l.trim() == marker)
}

pub fn write_config(dir: &PathBuf, provider: &str, model: &str, dimension: usize, api_key: &Option<String>, base_url: &str) -> Result<(), String> {
    let ocean_dir = dir.join(".ocean");
    std::fs::create_dir_all(&ocean_dir).map_err(|e| format!("Failed to create {}: {}", ocean_dir.display(), e))?;

    let config = serde_json::json!({
        "embedding": {
            "provider": provider,
            "model": model,
            "dimension": dimension,
            "api_key": api_key.as_deref().unwrap_or(""),
            "base_url": base_url,
        },
        "index": {
            "batch_size": 10,
        },
        "query": {
            "top_k": 10,
            "mode": "auto",
        },
    });

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    let config_path = ocean_dir.join("config.json");
    std::fs::write(&config_path, &json)
        .map_err(|e| format!("Failed to write {}: {}", config_path.display(), e))?;

    println!("  Created {}", config_path.display());
    Ok(())
}

pub fn ensure_claude_md(dir: &PathBuf) -> Result<(), String> {
    let path = dir.join("CLAUDE.md");
    let marker = "## Ocean CLI";
    let content = if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?
    } else {
        String::new()
    };

    if section_exists(&content, marker) {
        println!("  CLAUDE.md already has an Ocean CLI section (skipped)");
        return Ok(());
    }

    let section = format!(
        r#"
## Ocean CLI

This project uses `ocean` — a document intelligence tool for parsing, chunking, indexing, and semantic search.

### Commands
- `ocean info <file>` — Show metadata and outline of a document
- `ocean index <dir>` — Index documents for semantic search (requires embedding service)
- `ocean query <query>` — Search indexed documents
- `ocean chunk <file>` — Chunk a document into semantic segments
- `ocean scan <dir>` — List supported files in a directory
- `ocean watch <dir>` — Watch a directory for file changes
- `ocean help` — Show full help

### Configuration
Configuration file: `.ocean/config.json`
- Embedding provider, model, dimension
- Index batch size, db path
- Query top_k, mode, cache settings

### Usage Pattern
1. `ocean init` to initialize a project
2. `ocean scan .` to see supported files
3. `ocean index .` to index all documents
4. `ocean query "your question"` to search
"#,
    );

    let mut new_content = content;
    new_content.push_str(&section);
    std::fs::write(&path, &new_content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    println!("  Appended Ocean CLI section to {}", path.display());
    Ok(())
}

pub fn ensure_agents_md(dir: &PathBuf) -> Result<(), String> {
    let path = dir.join("AGENTS.md");
    let marker = "## Ocean CLI Usage";
    let content = if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?
    } else {
        String::new()
    };

    if section_exists(&content, marker) {
        println!("  AGENTS.md already has an Ocean CLI Usage section (skipped)");
        return Ok(());
    }

    let section = format!(
        r#"
## Ocean CLI Usage

The `ocean` binary provides document intelligence features.

### Key Commands
- `ocean info <file>` — metadata + outline
- `ocean index <dir>` — parse, chunk, embed, and store
- `ocean query <query>` — search indexed documents (vector/hybrid/expand)
- `ocean chunk <file>` — semantic chunking
- `ocean scan <dir>` — list supported files with metadata
- `ocean watch <dir>` — monitor directory for changes
- `ocean graph expand <node>` — traverse the knowledge graph
- `ocean graph path <from> <to>` — find shortest path

### Supported Formats
PDF, DOCX, PPTX, XLSX, TXT, MD, HTML, PNG, JPG/JPEG

### Output
- Commands print to stdout
- Errors printed to stderr
- Structured JSON logs via `--log-format json`
"#,
    );

    let mut new_content = content;
    new_content.push_str(&section);
    std::fs::write(&path, &new_content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    println!("  Appended Ocean CLI Usage section to {}", path.display());
    Ok(())
}

pub fn ensure_ocean_cli_skill(dir: &PathBuf) -> Result<(), String> {
    let skill_dir = dir.join(".agents").join("skills").join("ocean-cli");
    let path = skill_dir.join("SKILL.md");

    if path.exists() {
        println!("  {} already exists (skipped)", path.display());
        return Ok(());
    }

    std::fs::create_dir_all(&skill_dir)
        .map_err(|e| format!("Failed to create {}: {}", skill_dir.display(), e))?;

    let skill = format!(
        r#"# ocean-cli

Provides document intelligence capabilities via the `ocean` CLI tool.

## Overview

`ocean` is a document runtime for parsing, chunking, indexing with embeddings, semantic search, and knowledge graph construction. It supports PDF, DOCX, PPTX, XLSX, TXT, Markdown, and HTML formats.

## Commands

### `ocean info <file> [--metrics]`
Show metadata and outline of a document file. Use `--metrics` to display global usage metrics.

### `ocean index <dir> [--model] [--provider] [--db-path] [--reindex] [--no-graph] [--mode]`
Scan, parse, chunk, embed with a vector model, and store in SurrealDB. Supports Ollama, OpenAI, Anthropic, and Gemini providers.

### `ocean query <query> [--mode auto|vector|hybrid|expand] [--top-k N] [--context] [--expand-depth N] [--provider]`
Unified semantic search. Auto mode selects strategy based on query length.

### `ocean chunk <file> [--min-size N] [--max-size N] [--overlap N] [--include-images] [--rows-per-chunk N]`
Split a document into semantic chunks with configurable token bounds and overlap.

### `ocean scan <dir> [--no-hash]`
List all supported files with size, hash, and file extension.

### `ocean search <file> <query>`
Full-text search within a single document.

### `ocean grep <dir> <query>`
Recursive full-text search across all supported documents in a directory.

### `ocean read <file> [--page N | --heading S | --slide N | --sheet S | --skip N --take N]`
Read content by selector (page, heading, paragraph, table, slide, sheet, cell, image, range, or slice).

### `ocean watch <dir> [--no-sandbox]`
Monitor a directory for file changes (create, modify, delete, rename). Ctrl+C to stop.

### `ocean hash <file>`
Compute SHA-256 hex digest.

### `ocean verify <file> <hash>`
Verify a file's SHA-256 hash. Prints true/false.

### `ocean vector-search <query> [--top-k N] [--hybrid] [--expand-depth N]`
Backwards-compatible vector search with optional hybrid (vector + FTS) and graph expansion.

### `ocean graph info <file>`
Display graph stats for a file (node count, edge count, type breakdown).

### `ocean graph expand <node-id> [--depth N] [--direction both|in|out]`
Traverse the knowledge graph from a node up to depth N.

### `ocean graph path <from> <to> [--max-depth N]`
Find shortest path between two graph nodes.

### `ocean graph stats`
Display global graph statistics across all indexed files.

### `ocean config show`
Display current configuration as JSON.

### `ocean config validate`
Validate current configuration for correctness.

### `ocean init [--dir PATH]`
Interactive project initialization (prompts for embedding config, creates .ocean/config.json, updates CLAUDE.md and AGENTS.md).

## Configuration

Configuration is stored in `.ocean/config.json` with sections for embedding, index, query, runtime, cache, security, and observability.

Resolution order: CLI flags > `.ocean/config.json` > `~/.ocean/config.json` > `.env` > defaults.

## Supported Embedding Providers

| Provider | Default Model | Default Dim | Default URL |
|----------|---------------|-------------|-------------|
| Ollama | nomic-embed-text | 768 | http://localhost:11434 |
| OpenAI | text-embedding-3-small | 1536 | https://api.openai.com/v1 |
| Anthropic | cohere-embed-multilingual-v3 | 768 | https://api.anthropic.com/v1 |
| Gemini | gemini-embedding-001 | 3072 | (built-in) |

## Usage Examples

```bash
# Initialize a project
ocean init

# Index all documents
ocean index .

# Search with default (auto) mode
ocean query "what is this document about"

# Search with hybrid mode
ocean query --mode hybrid "key findings"

# Chunk a specific file
ocean chunk report.docx --min-size 50 --max-size 500

# View graph relationships
ocean graph info report.docx

# File watcher
ocean watch ./docs
```
"#,
    );

    std::fs::write(&path, &skill).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    println!("  Created {}", path.display());
    Ok(())
}

pub fn cmd_init(dir: Option<String>) -> Result<(), String> {
    let cwd = match dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?,
    };

    if !cwd.is_dir() {
        return Err(format!("'{}' is not a directory", cwd.display()));
    }

    println!("Initializing ocean in {}", cwd.display());
    println!();

    let provider = prompt("Embedding provider (ollama/openai/anthropic/gemini)", "ollama").to_lowercase();
    let default_model = default_model(&provider);
    let model = prompt("Embedding model", default_model);
    let default_dim = default_dimension(&provider, &model);
    let dim_str = prompt(&format!("Embedding dimension [{}]", default_dim), &default_dim.to_string());
    let dimension: usize = dim_str.parse().map_err(|_| format!("Invalid dimension: {}", dim_str))?;

    println!("(API key is optional for ollama, recommended for others)");
    let api_key = prompt_masked("API key (leave empty for none)");

    let default_url = default_base_url(&provider);
    let base_url = if provider == "gemini" {
        prompt_optional("Base URL (leave empty for default)")
    } else {
        Some(prompt("Base URL", default_url))
    };
    let base_url = base_url.unwrap_or_default();

    println!();
    println!("Configuration:");
    println!("  Provider:  {}", provider);
    println!("  Model:     {}", model);
    println!("  Dimension: {}", dimension);
    println!("  API key:   {}", if api_key.is_some() { "(set)" } else { "(none)" });
    println!("  Base URL:  {}", if base_url.is_empty() { "(default)" } else { &base_url });
    println!();

    let confirm = prompt("Write configuration?", "y");
    if confirm.to_lowercase() != "y" {
        println!("Aborted.");
        return Ok(());
    }

    write_config(&cwd, &provider, &model, dimension, &api_key, &base_url)?;
    ensure_claude_md(&cwd)?;
    ensure_agents_md(&cwd)?;
    ensure_ocean_cli_skill(&cwd)?;

    println!();
    println!("Ocean initialized in {}", cwd.display());
    println!();
    println!("Next steps:");
    println!("  1. Place supported documents in this directory");
    println!("  2. Run: ocean scan .");
    println!("  3. Run: ocean index .");
    println!("  4. Run: ocean query \"your question\"");

    Ok(())
}
