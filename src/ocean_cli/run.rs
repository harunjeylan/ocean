use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;

use clap::Parser;

use crate::ocean_chunk::*;
use crate::ocean_cli::args::{
    ChunkArgs, Cli, Commands, GraphArgs, GraphCommands, IndexArgs, QueryArgs, ReadArgs,
    VectorSearchArgs,
};
use crate::ocean_cli::config::OceanConfig;
use crate::ocean_cli::display::*;
use crate::ocean_cli::walk::*;
use crate::ocean_fs::*;
use crate::ocean_graph::*;
use crate::ocean_parser::*;
use crate::ocean_query::*;
use crate::ocean_storage::config::StorageConfig;
use crate::ocean_storage::graph_store::{EdgeDirection, GraphStore};
use crate::ocean_storage::vector_store::VectorStore;
use crate::ocean_storage::{SurrealChunkStore, SurrealGraphStore, SurrealVectorStore};
use crate::ocean_vector::*;

pub fn run() -> Result<(), String> {
    crate::ocean_cli::config::load_env_files();
    let config: Option<OceanConfig> = OceanConfig::load();

    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => cmd_info(file),
        Commands::Metadata { file } => cmd_metadata(file),
        Commands::Outline { file } => cmd_outline(file),
        Commands::PageCount { file } => cmd_page_count(file),
        Commands::Search { file, query } => cmd_search(file, query),
        Commands::Grep { dir, query } => cmd_grep(dir, query),
        Commands::Read(args) => cmd_read(args),
        Commands::Scan { dir, no_hash } => cmd_scan(dir, no_hash),
        Commands::Hash { file } => cmd_hash(file),
        Commands::Verify { file, hash } => cmd_verify(file, hash),
        Commands::Watch { dir } => cmd_watch(dir),
        Commands::Chunk(args) => cmd_chunk(args),
        Commands::Index(args) => cmd_index(args, &config),
        Commands::Query(args) => cmd_query(args, &config),
        Commands::VectorSearch(args) => cmd_vector_search(args, &config),
        Commands::Graph(args) => cmd_graph(args),
    }
}

fn cmd_info(file: String) -> Result<(), String> {
    let _ = check_exists(&file)?;
    let doc = open(&file).map_err(|e| format!("Failed to open: {}", e))?;
    println!();
    print_meta(&doc.metadata());
    println!();
    let outline = doc.outline();
    if !outline.entries.is_empty() {
        println!("Outline:");
        print_outline(&outline, 0);
    } else {
        println!("Outline: (empty)");
    }
    Ok(())
}

fn cmd_metadata(file: String) -> Result<(), String> {
    let _ = check_exists(&file)?;
    let doc = open(&file).map_err(|e| format!("Failed to open: {}", e))?;
    print_meta(&doc.metadata());
    Ok(())
}

fn cmd_outline(file: String) -> Result<(), String> {
    let _ = check_exists(&file)?;
    let doc = open(&file).map_err(|e| format!("Failed to open: {}", e))?;
    let outline = doc.outline();
    if outline.entries.is_empty() {
        println!("(empty outline)");
    } else {
        print_outline(&outline, 0);
    }
    Ok(())
}

fn cmd_page_count(file: String) -> Result<(), String> {
    let _ = check_exists(&file)?;
    let doc = open(&file).map_err(|e| format!("Failed to open: {}", e))?;
    match doc.page_count() {
        Some(n) => println!("{}", n),
        None => println!("(none)"),
    }
    Ok(())
}

fn cmd_search(file: String, query: String) -> Result<(), String> {
    let _ = check_exists(&file)?;
    let doc = open(&file).map_err(|e| format!("Failed to open: {}", e))?;
    let matches = doc.search(&query);
    if matches.is_empty() {
        println!("No matches found for '{}'.", query);
    } else {
        println!("{} match(es) for '{}':", matches.len(), query);
        for m in &matches {
            println!("  {:?}: \"{}\"", m.selector, m.text);
            if !m.context.is_empty() {
                println!("    context: {}", m.context);
            }
        }
    }
    Ok(())
}

fn cmd_grep(dir: String, query: String) -> Result<(), String> {
    let dir_path = PathBuf::from(&dir);
    if !dir_path.is_dir() {
        return Err(format!("directory not found: {}", dir));
    }
    let files = walk_supported_files(&dir_path);
    if files.is_empty() {
        println!("No supported documents found in '{}'.", dir);
        return Ok(());
    }
    let mut total_matches = 0u32;
    for path in &files {
        let name = path.to_string_lossy();
        match open(&name) {
            Ok(doc) => {
                let matches = doc.search(&query);
                if !matches.is_empty() {
                    println!("{}:", path.display());
                    for m in &matches {
                        println!("  {:?}: \"{}\"", m.selector, m.text);
                        total_matches += 1;
                    }
                    println!();
                }
            }
            Err(_) => {}
        }
    }
    println!("Total: {} match(es) in {} file(s) for '{}'", total_matches, files.len(), query);
    Ok(())
}

fn cmd_read(args: ReadArgs) -> Result<(), String> {
    let _ = check_exists(&args.file)?;
    let doc = open(&args.file).map_err(|e| format!("Failed to open: {}", e))?;

    let selector = if args.skip.is_some() || args.take.is_some() {
        let skip = args.skip.unwrap_or(0);
        let take = args.take.ok_or("--take is required when using --skip")?;
        if take == 0 {
            return Err("--take must be greater than 0".to_string());
        }
        Selector::Slice { skip, take }
    } else if let Some(n) = args.page {
        Selector::Page(n)
    } else if let Some(h) = args.heading {
        Selector::Heading(h)
    } else if let Some(n) = args.paragraph {
        Selector::Paragraph(n)
    } else if let Some(n) = args.table {
        Selector::Table(n)
    } else if let Some(n) = args.slide {
        Selector::Slide(n)
    } else if let Some(s) = args.sheet {
        Selector::Sheet(s)
    } else if let Some(c) = args.cell {
        Selector::Cell(c)
    } else if let Some(n) = args.image {
        Selector::Image(n)
    } else if let Some(range_str) = args.range {
        let parts: Vec<&str> = range_str.splitn(2, '-').collect();
        if parts.len() != 2 {
            return Err("Range must be in format <start>-<end> (e.g. 0-100)".to_string());
        }
        let start: usize = parts[0].parse().map_err(|_| "Invalid range start".to_string())?;
        let end: usize = parts[1].parse().map_err(|_| "Invalid range end".to_string())?;
        Selector::Range { start, end }
    } else {
        return Err("No selector specified. Use --skip/--take, --page, --heading, --paragraph, --table, --slide, --sheet, --cell, --image, or --range".to_string());
    };

    let result = doc.read(&selector).map_err(|e| format!("Read failed: {}", e))?;
    print_read_result(result);
    Ok(())
}

fn check_exists(file: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(file);
    if !p.exists() {
        return Err(format!("file not found: {}", file));
    }
    Ok(p)
}

fn cmd_scan(dir: String, no_hash: bool) -> Result<(), String> {
    if no_hash {
        let metas = scan_dir(&dir).map_err(|e| format!("Scan failed: {}", e))?;
        if metas.is_empty() {
            println!("No supported files found in '{}'.", dir);
        } else {
            println!("Found {} file(s) in '{}':", metas.len(), dir);
            for meta in &metas {
                let size_kb = meta.size as f64 / 1024.0;
                println!("  {:>8.1} KB  {:4}  {}", size_kb, meta.extension, meta.path);
            }
        }
    } else {
        let metas = scan_dir(&dir).map_err(|e| format!("Scan failed: {}", e))?;
        if metas.is_empty() {
            println!("No supported files found in '{}'.", dir);
        } else {
            println!("Found {} file(s) in '{}':", metas.len(), dir);
            for meta in &metas {
                let size_kb = meta.size as f64 / 1024.0;
                let short_id = &meta.id[..8];
                println!("  {}  {:>8.1} KB  {:4}  {}", short_id, size_kb, meta.extension, meta.path);
            }
        }
    }
    Ok(())
}

fn cmd_hash(file: String) -> Result<(), String> {
    let hash = hash_file(&file).map_err(|e| format!("Hash failed: {}", e))?;
    println!("{}", hash);
    Ok(())
}

fn cmd_verify(file: String, hash: String) -> Result<(), String> {
    let result = verify_hash(&file, &hash);
    println!("{}", result);
    Ok(())
}

fn cmd_watch(dir: String) -> Result<(), String> {
    let watcher = FileWatcher::new();
    let (tx, rx) = mpsc::channel::<FileEvent>();

    let callback = Arc::new(move |event: FileEvent| {
        let _ = tx.send(event);
    });

    let handle = watcher
        .watch(&dir, callback)
        .map_err(|e| format!("Watch failed: {}", e))?;

    println!("Watching '{}'... Press Ctrl+C to stop.", dir);
    for event in rx {
        match event {
            FileEvent::Created(meta) => println!("[CREATED]  {}", meta.path),
            FileEvent::Modified(meta) => println!("[MODIFIED] {}", meta.path),
            FileEvent::Deleted(id) => println!("[DELETED]  id={}", id),
            FileEvent::Renamed { old_path, new_path, .. } => {
                println!("[RENAMED]  {} -> {}", old_path, new_path);
            }
            FileEvent::Moved { old_path, new_path, .. } => {
                println!("[MOVED]    {} -> {}", old_path, new_path);
            }
        }
    }

    watcher.unwatch(handle).map_err(|e| format!("Unwatch failed: {}", e))
}

fn resolve_provider(cli: Option<&str>, config: Option<&OceanConfig>) -> String {
    cli.or_else(|| config.and_then(|c| c.embedding.provider.as_deref()))
        .unwrap_or("ollama")
        .to_string()
}

fn resolve_model(cli: Option<&str>, config: Option<&OceanConfig>) -> String {
    cli.or_else(|| config.and_then(|c| c.embedding.model.as_deref()))
        .unwrap_or("nomic-embed-text")
        .to_string()
}

fn resolve_index_dimension(
    cli_dim: Option<usize>,
    cli_provider: &str,
    cli_model: &str,
    config: Option<&OceanConfig>,
) -> usize {
    if let Some(d) = cli_dim {
        return d;
    }
    if let Some(d) = config.and_then(|c| c.embedding.dimension) {
        return d;
    }
    match cli_provider {
        "openai" if cli_model.contains("large") => 3072,
        "openai" if cli_model.contains("small") => 1536,
        "openai" => 1536,
        "gemini" => 3072,
        _ => 768,
    }
}

fn resolve_query_dimension(
    cli_dim: Option<usize>,
    cli_provider: &str,
    cli_model: &str,
    config: Option<&OceanConfig>,
) -> usize {
    if let Some(d) = cli_dim {
        return d;
    }
    if let Some(d) = config.and_then(|c| c.embedding.dimension) {
        return d;
    }
    match cli_provider {
        "openai" if cli_model.contains("large") => 3072,
        "openai" if cli_model.contains("small") => 1536,
        "openai" => 1536,
        "gemini" => 3072,
        _ => 768,
    }
}

fn create_embedder(
    provider: &str,
    model: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Box<dyn Embedder>, String> {
    match provider {
        "ollama" => {
            let url = if base_url.is_empty() { "http://localhost:11434" } else { base_url };
            Ok(Box::new(
                OllamaEmbedder::new(model, url)
                    .map_err(|e| format!("Failed to create Ollama embedder: {}", e))?,
            ))
        }
        "openai" => {
            let key = api_key.ok_or_else(|| "--api-key is required for openai provider")?;
            let url = if base_url.is_empty() { "https://api.openai.com/v1" } else { base_url };
            Ok(Box::new(
                OpenAIEmbedder::new(model, url, key)
                    .map_err(|e| format!("Failed to create OpenAI embedder: {}", e))?,
            ))
        }
        "anthropic" => {
            let key = api_key.ok_or_else(|| "--api-key is required for anthropic provider")?;
            let url = if base_url.is_empty() { "https://api.anthropic.com/v1" } else { base_url };
            Ok(Box::new(
                AnthropicEmbedder::new(model, url, key)
                    .map_err(|e| format!("Failed to create Anthropic embedder: {}", e))?,
            ))
        }
        "gemini" => {
            let key = api_key.ok_or_else(|| "--api-key is required for gemini provider")?;
            Ok(Box::new(
                GeminiEmbedder::new(model, key)
                    .map_err(|e| format!("Failed to create Gemini embedder: {}", e))?,
            ))
        }
        other => Err(format!("unsupported provider '{}'. Use: ollama, openai, anthropic, gemini", other)),
    }
}

fn cmd_index(args: IndexArgs, config: &Option<OceanConfig>) -> Result<(), String> {
    let dir_path = std::path::PathBuf::from(&args.dir);
    if !dir_path.is_dir() {
        return Err(format!("directory not found: {}", args.dir));
    }

    let files = walk_supported_files(&dir_path);
    if files.is_empty() {
        println!("No supported documents found in '{}'.", args.dir);
        return Ok(());
    }
    println!("Found {} supported file(s) in '{}'. Indexing...", files.len(), args.dir);

    let provider = resolve_provider(args.provider.as_deref(), config.as_ref());
    let model = resolve_model(args.model.as_deref(), config.as_ref());

    let base_path = crate::ocean_cli::config::resolve_db_path(
        args.db_path.as_deref(),
        config.as_ref().and_then(|c| c.index.db_path.as_deref()),
    );

    let vector_path = format!("{}/vector.db", base_path);
    let graph_path = format!("{}/graph.db", base_path);

    let vconfig = StorageConfig::new(&vector_path);
    let vstore = SurrealVectorStore::new_persistent_at(&vector_path, &vconfig)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    let dim = resolve_index_dimension(
        args.dimension,
        &provider,
        &model,
        config.as_ref(),
    );
    vstore.initialize_schema(dim)
        .map_err(|e| format!("Failed to init schema: {}", e))?;

    let cstore = SurrealChunkStore::new_persistent_at(&vector_path)
        .map_err(|e| format!("Failed to open chunk store: {}", e))?;

    let graph_store: Option<SurrealGraphStore> = if !args.no_graph {
        let gconfig = StorageConfig::new(&graph_path);
        let gs = SurrealGraphStore::new_persistent_at(&graph_path, &gconfig)
            .map_err(|e| format!("Failed to open graph store: {}", e))?;
        gs.initialize_schema()
            .map_err(|e| format!("Failed to init graph schema: {}", e))?;
        Some(gs)
    } else {
        None
    };

    let api_key = crate::ocean_cli::config::resolve_api_key(
        args.api_key.as_deref(),
        config.as_ref().and_then(|c| c.embedding.api_key.as_deref()),
        None,
    );

    let base_url = crate::ocean_cli::config::resolve_base_url(
        &provider,
        args.ollama_url.as_deref(),
        config.as_ref().and_then(|c| c.embedding.base_url.as_deref()),
    );

    let embedder = create_embedder(&provider, &model, &base_url,
        api_key.as_deref())?;

    let pipeline = IndexPipeline::new(Arc::new(vstore), Arc::new(cstore));

    let config = IndexConfig {
        batch_size: args.batch_size,
        reindex: args.reindex,
        model: model.clone(),
        dimension: embedder.dimension(),
        ollama_url: Some(base_url.clone()),
        openai_api_key: api_key,
        db_path: vector_path.clone(),
    };

    let graph_config = GraphConfig {
        extract_references: !args.no_references,
        extract_entities: !args.no_entities,
        ..Default::default()
    };

    let file_count = files.len();
    let mut total_graph_nodes = 0usize;
    let mut total_graph_edges = 0usize;

    for (i, path) in files.iter().enumerate() {
        let name = path.to_string_lossy();
        println!("[{}/{}] Processing: {}", i + 1, file_count, name);

        let doc = match open(&name) {
            Ok(d) => d,
            Err(e) => {
                println!("  Skipping (open failed: {})", e);
                continue;
            }
        };

        let blocks = match read_all_blocks(&*doc) {
            Ok(b) => b,
            Err(e) => {
                println!("  Skipping (read failed: {})", e);
                continue;
            }
        };

        let chunk_config = ChunkConfig {
            min_tokens: 100,
            max_tokens: 800,
            overlap_sentences: 1,
            include_images: false,
            rows_per_sheet_chunk: 50,
            token_estimator: None,
        };

        let file_id = generate_file_id();
        let chunks = match crate::ocean_chunk::chunk(blocks, &file_id, Some(chunk_config)) {
            Ok(c) => c,
            Err(e) => {
                println!("  Skipping (chunk failed: {})", e);
                continue;
            }
        };

        if chunks.is_empty() {
            println!("  No chunks produced.");
            continue;
        }

        match pipeline.index_chunks(chunks.clone(), &*embedder, &config) {
            Ok(report) => {
                println!(
                    "  Indexed: {} embedded, {} skipped, {} failed ({}ms)",
                    report.embedded, report.skipped, report.failed, report.duration_ms
                );
                for err in report.errors.iter().take(3) {
                    println!("    error: {}", err);
                }
                if report.errors.len() > 3 {
                    println!("    ... and {} more errors", report.errors.len() - 3);
                }
                let revision_err = report.errors.iter().any(|e| {
                    let s = e.to_string();
                    s.contains("Invalid revision") || s.contains("revision")
                });
                if revision_err && report.embedded == 0 && report.failed > 0 {
                    println!(
                        "    hint: database revision mismatch. Delete and re-run:\n    \
                         Remove-Item -Recurse -Force \"{}\"",
                        base_path
                    );
                }
            }
            Err(e) => {
                println!("  Index error: {}", e);
            }
        }

        if let Some(ref gs) = graph_store {
            let _ = gs.delete_by_file(&file_id);

            let (nodes, edges) = GraphBuilder::from_chunks(&chunks, &file_id, &graph_config);
            let node_count = nodes.len();
            let edge_count = edges.len();
            total_graph_nodes += node_count;
            total_graph_edges += edge_count;

            let node_pairs: Vec<(Node, String)> = nodes.into_iter()
                .map(|n| (n, file_id.clone()))
                .collect();
            let edge_pairs: Vec<(Edge, String)> = edges.into_iter()
                .map(|e| (e, file_id.clone()))
                .collect();

            if let Err(e) = gs.insert_nodes_batch(node_pairs) {
                println!("  Graph node insert error: {}", e);
            }
            if let Err(e) = gs.insert_edges_batch(edge_pairs) {
                println!("  Graph edge insert error: {}", e);
            }
            println!("  Graph: {} nodes, {} edges", node_count, edge_count);
        }
    }

    if !args.no_graph {
        println!("Graph total: {} nodes, {} edges", total_graph_nodes, total_graph_edges);
    }

    println!("Indexing complete.");
    Ok(())
}

fn cmd_query(args: QueryArgs, config: &Option<OceanConfig>) -> Result<(), String> {
    let provider = resolve_provider(args.provider.as_deref(), config.as_ref());
    let model = resolve_model(args.model.as_deref(), config.as_ref());

    let mode = match args.mode.as_deref().or(config.as_ref().and_then(|c| c.query.mode.as_deref())) {
        None | Some("auto") => QueryMode::Auto,
        Some("vector") => QueryMode::Vector,
        Some("hybrid") => QueryMode::Hybrid,
        Some("expand") => QueryMode::Expand,
        Some(other) => return Err(format!("invalid mode '{}'. Use: auto, vector, hybrid, expand", other)),
    };

    let base_path = crate::ocean_cli::config::resolve_db_path(
        args.db_path.as_deref(),
        config.as_ref().and_then(|c| c.query.db_path.as_deref()),
    );

    let dimension = resolve_query_dimension(
        args.dimension,
        &provider,
        &model,
        config.as_ref(),
    );
    let engine = QueryEngine::new_with_paths(
        &format!("{}/vector.db", base_path),
        &format!("{}/graph.db", base_path),
        dimension,
    )
    .map_err(|e| format!("Failed to create query engine: {}", e))?;

    let api_key = crate::ocean_cli::config::resolve_api_key(
        args.api_key.as_deref(),
        config.as_ref().and_then(|c| c.embedding.api_key.as_deref()),
        None,
    );

    let base_url = crate::ocean_cli::config::resolve_base_url(
        &provider,
        args.ollama_url.as_deref(),
        config.as_ref().and_then(|c| c.embedding.base_url.as_deref()),
    );

    let embedder = create_embedder(&provider, &model, &base_url,
        api_key.as_deref())?;

    let mut filter = crate::ocean_vector::search::SearchFilter::new();
    if let Some(ref fid) = args.file_id {
        filter = filter.with_file_id(fid);
    }
    if let Some(ref h) = args.heading {
        filter = filter.with_heading(h);
    }
    if let Some(ref bt) = args.block_type {
        filter = filter.with_block_type(bt);
    }

    let has_filter = filter.file_id.is_some()
        || filter.heading_prefix.is_some()
        || filter.block_type.is_some();

    let q = Query {
        text: args.query.clone(),
        mode,
        top_k: args.top_k,
        expand_depth: args.expand_depth,
        filter: if has_filter { Some(filter) } else { None },
        include_context: args.context,
        context_chunks: args.context_chunks.unwrap_or(3),
        rerank_by_heading: args.rerank_by_heading,
        rerank_by_file: args.rerank_by_file,
    };

    let result = engine.query(q, &*embedder)
        .map_err(|e| format!("Query failed: {}", e))?;

    print_query_result(&result, args.verbose);
    Ok(())
}

fn cmd_vector_search(args: VectorSearchArgs, config: &Option<OceanConfig>) -> Result<(), String> {
    let provider = resolve_provider(args.provider.as_deref(), config.as_ref());
    let model = resolve_model(args.model.as_deref(), config.as_ref());

    let base_path = crate::ocean_cli::config::resolve_db_path(
        args.db_path.as_deref(),
        config.as_ref().and_then(|c| c.query.db_path.as_deref()),
    );

    let vector_path = format!("{}/vector.db", base_path);
    let graph_path = format!("{}/graph.db", base_path);

    let vconfig = StorageConfig::new(&vector_path);
    let vstore = SurrealVectorStore::new_persistent_at(&vector_path, &vconfig)
        .map_err(|e| format!("Failed to open store: {}", e))?;
    let engine = SearchEngine::new(Arc::new(vstore));

    let api_key = crate::ocean_cli::config::resolve_api_key(
        args.api_key.as_deref(),
        config.as_ref().and_then(|c| c.embedding.api_key.as_deref()),
        None,
    );

    let base_url = crate::ocean_cli::config::resolve_base_url(
        &provider,
        args.ollama_url.as_deref(),
        config.as_ref().and_then(|c| c.embedding.base_url.as_deref()),
    );

    let embedder = create_embedder(&provider, &model, &base_url,
        api_key.as_deref())?;

    let results = if args.hybrid {
        let mut filter = SearchFilter::new();
        if let Some(ref fid) = args.file_id {
            filter = filter.with_file_id(fid);
        }
        if let Some(ref h) = args.heading {
            filter = filter.with_heading(h);
        }
        if let Some(ref bt) = args.block_type {
            filter = filter.with_block_type(bt);
        }

        if filter.file_id.is_some() || filter.heading_prefix.is_some() || filter.block_type.is_some() {
            engine.hybrid_filtered_search(&args.query, &*embedder, &filter, args.top_k)
        } else {
            engine.hybrid_search(&args.query, &*embedder, args.top_k)
        }
    } else {
        let mut filter = SearchFilter::new();
        if let Some(ref fid) = args.file_id {
            filter = filter.with_file_id(fid);
        }
        if let Some(ref h) = args.heading {
            filter = filter.with_heading(h);
        }
        if let Some(ref bt) = args.block_type {
            filter = filter.with_block_type(bt);
        }

        if filter.file_id.is_some() || filter.heading_prefix.is_some() || filter.block_type.is_some() {
            engine.filtered_search(&args.query, &*embedder, &filter, args.top_k)
        } else {
            engine.search(&args.query, &*embedder, args.top_k)
        }
    };

    let results = match results {
        Ok(mut results) => {
            if args.expand_depth > 0 {
                let gconfig = StorageConfig::new(&graph_path);
                if let Ok(gs) = SurrealGraphStore::new_persistent_at(&graph_path, &gconfig) {
                    if gs.initialize_schema().is_ok() {
                        let expansion = ExpansionEngine::new(Arc::new(gs));
                        if let Ok(expanded) = engine.expand_results(&results, &expansion, args.expand_depth) {
                            results = expanded;
                        }
                    }
                }
            }
            results
        }
        Err(e) => return Err(format!("Search failed: {}", e)),
    };

    if results.is_empty() {
        println!("No results found.");
    } else {
        let label = if args.expand_depth > 0 { "expanded results" } else { "results" };
        println!("Top {} {} for '{}':", results.len(), label, args.query);
        for (i, r) in results.iter().enumerate() {
            let content_short = if r.content.len() > 120 {
                format!("{}...", &r.content[..120])
            } else {
                r.content.clone()
            };
            let heading = r.heading.as_deref().unwrap_or("(none)");
            let graph_info = match r.graph_score {
                Some(gs) => format!(" graph={:.4}", gs),
                None => String::new(),
            };
            let score_info = if let (Some(vs), Some(fs)) = (r.vector_score, r.fts_score) {
                format!("score={:.4} (vec={:.4}, fts={:.4}){}", r.score, vs, fs, graph_info)
            } else {
                format!("score={:.4}{}", r.score, graph_info)
            };
            println!(
                "  {}. {}  file={}  heading=\"{}\"",
                i + 1,
                score_info,
                &r.file_id[..8],
                heading,
            );
            println!("     \"{}\"", content_short);
        }
    }

    Ok(())
}

fn cmd_chunk(args: ChunkArgs) -> Result<(), String> {
    let doc = open(&args.file).map_err(|e| format!("Failed to open: {}", e))?;
    let blocks = read_all_blocks(&*doc).map_err(|e| format!("Read failed: {}", e))?;

    let config = ChunkConfig {
        min_tokens: args.min_size,
        max_tokens: args.max_size,
        overlap_sentences: args.overlap,
        include_images: args.include_images,
        rows_per_sheet_chunk: args.rows_per_chunk,
        token_estimator: None,
    };

    let file_id = crate::ocean_fs::generate_file_id();
    let chunks = crate::ocean_chunk::chunk(blocks, &file_id, Some(config))
        .map_err(|e| format!("Chunking failed: {}", e))?;

    if chunks.is_empty() {
        println!("No chunks produced.");
        return Ok(());
    }

    println!("{} chunks from '{}':", chunks.len(), args.file);
    for chunk in &chunks {
        let token_est = estimate_tokens(&chunk.content);
        let heading = chunk.heading.as_deref().unwrap_or("");
        let short_id = &chunk.id[..8];
        println!(
            "  [{}] {:8}  h=\"{}\"  {} tokens",
            short_id,
            format!("{:?}", chunk.block_type),
            heading,
            token_est,
        );
    }

    Ok(())
}

fn cmd_graph(args: GraphArgs) -> Result<(), String> {
    let resolve = |db: Option<String>| -> String {
        let base = crate::ocean_cli::config::resolve_db_path(db.as_deref(), None);
        format!("{}/graph.db", base)
    };
    match args.command {
        GraphCommands::Info { file, db_path } => {
            cmd_graph_info(file, resolve(db_path))
        }
        GraphCommands::Expand { node_id, depth, direction, db_path } => {
            cmd_graph_expand(node_id, depth, direction, resolve(db_path))
        }
        GraphCommands::Path { from, to, max_depth, db_path } => {
            cmd_graph_path(from, to, max_depth, resolve(db_path))
        }
        GraphCommands::Stats { db_path } => {
            cmd_graph_stats(resolve(db_path))
        }
    }
}

fn cmd_graph_info(file: String, db_path: String) -> Result<(), String> {
    let config = StorageConfig::new(&db_path);
    let store = SurrealGraphStore::new_persistent_at(&db_path, &config)
        .map_err(|e| format!("Failed to open graph store: {}", e))?;

    let metas = scan_dir(&file).map_err(|e| format!("Scan failed: {}", e))?;
    if metas.is_empty() {
        return Err(format!("No supported files found matching: {}", file));
    }
    let file_id = &metas[0].id;

    let subgraph = ExpansionEngine::new(Arc::new(store))
        .get_file_graph(file_id)
        .map_err(|e| format!("Failed to get file graph: {}", e))?;

    let mut type_counts: std::collections::HashMap<NodeType, usize> = std::collections::HashMap::new();
    for node in &subgraph.nodes {
        *type_counts.entry(node.node_type.clone()).or_insert(0) += 1;
    }

    print_graph_info(
        subgraph.nodes.len() as u64,
        subgraph.edges.len() as u64,
        type_counts.into_iter().collect(),
    );
    Ok(())
}

fn cmd_graph_expand(node_id: String, depth: usize, direction: String, db_path: String) -> Result<(), String> {
    let config = StorageConfig::new(&db_path);
    let store = SurrealGraphStore::new_persistent_at(&db_path, &config)
        .map_err(|e| format!("Failed to open graph store: {}", e))?;

    let dir = match direction.to_lowercase().as_str() {
        "forward" => EdgeDirection::Forward,
        "backward" => EdgeDirection::Backward,
        "both" => EdgeDirection::Both,
        other => return Err(format!("invalid direction '{}'. Use: forward, backward, both", other)),
    };

    let engine = ExpansionEngine::new(Arc::new(store));
    let subgraph = engine
        .expand(&node_id, depth, dir)
        .map_err(|e| format!("Expansion failed: {}", e))?;

    print_graph_expanded(&subgraph);
    Ok(())
}

fn cmd_graph_path(from: String, to: String, max_depth: usize, db_path: String) -> Result<(), String> {
    let config = StorageConfig::new(&db_path);
    let store = SurrealGraphStore::new_persistent_at(&db_path, &config)
        .map_err(|e| format!("Failed to open graph store: {}", e))?;

    let engine = ExpansionEngine::new(Arc::new(store));
    let path = engine
        .find_path(&from, &to, max_depth)
        .map_err(|e| format!("Path find failed: {}", e))?;

    match path {
        Some(edges) => print_graph_path(&edges),
        None => println!("No path found between '{}' and '{}' within {} hops.", from, to, max_depth),
    }
    Ok(())
}

fn cmd_graph_stats(db_path: String) -> Result<(), String> {
    let config = StorageConfig::new(&db_path);
    let store = SurrealGraphStore::new_persistent_at(&db_path, &config)
        .map_err(|e| format!("Failed to open graph store: {}", e))?;

    let node_count = store.count_nodes().map_err(|e| format!("Count failed: {}", e))?;
    let edge_count = store.count_edges().map_err(|e| format!("Count failed: {}", e))?;

    let type_counts = vec![
        ("File".to_string(), 0u64),
        ("Chunk".to_string(), 0u64),
        ("Heading".to_string(), 0u64),
        ("Entity".to_string(), 0u64),
        ("Folder".to_string(), 0u64),
    ];

    print_graph_stats(node_count, edge_count, type_counts);
    Ok(())
}
