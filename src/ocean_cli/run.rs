use std::sync::mpsc;
use std::sync::Arc;

use clap::Parser;

use crate::ocean_api::docs as api_docs;
use crate::ocean_api::embedding::EmbeddingConfig;
use crate::ocean_api::fs as api_fs;
use crate::ocean_api::graph as api_graph;
use crate::ocean_api::indexing as api_index;
use crate::ocean_api::querying as api_query;
use crate::ocean_api::types::*;
use crate::ocean_chunk::ChunkConfig;
use crate::ocean_cli::args::{
    ChunkArgs, Cli, Commands, ConfigArgs, ConfigCommands, GraphArgs, GraphCommands, IndexArgs, QueryArgs, ReadArgs,
    VectorArgs, VectorCommands, VectorSearchArgs,
};
use crate::ocean_cli::config::OceanConfig;

// TODO(v2.0): remove this function and all experimentals guards when vector/graph graduate to stable
fn check_experimental(config: &Option<OceanConfig>, section: &str, cmd: &str) -> Result<(), String> {
    let enabled = match section {
        "vector" => config.as_ref().and_then(|c| c.experimentals.vector).unwrap_or(false),
        "graph" => config.as_ref().and_then(|c| c.experimentals.graph).unwrap_or(false),
        _ => false,
    };
    if !enabled {
        return Err(format!(
            "The '{}' command is experimental. Enable it by setting \"experimentals\": {{ \"{}\": true }} in .ocean/config.json",
            cmd, section
        ));
    }
    Ok(())
}
use crate::ocean_cli::display::*;
use crate::ocean_cli::events::{global_emitter, set_global_emitter, ConsoleEmitter, JsonEmitter, MultiEmitter, OutputTarget, SystemEvent};
use crate::ocean_storage::readonly::ReadOnlyGuard;
use crate::ocean_cli::metrics::{global_metrics, print_metrics};
use crate::ocean_cli::init::cmd_init;
use crate::ocean_cli::mcp_setup::cmd_mcp_setup;
use crate::ocean_cli::runtime::RuntimeMode;
use crate::ocean_fs::*;
use crate::ocean_parser::*;

pub fn run() -> Result<(), String> {
    crate::ocean_cli::config::load_env_files();
    let config: Option<OceanConfig> = OceanConfig::load();

    let cli = Cli::parse();

    let read_only = config.as_ref().and_then(|c| c.security.read_only).unwrap_or(false);
    ReadOnlyGuard::set_global_enabled(read_only);

    match cli.command {
        Commands::Index(ref args) => {
            check_experimental(&config, "vector", "index")?; // TODO(v2.0): remove — graduates with vector
            return cmd_index(args.clone(), &cli, &config);
        }
        Commands::Query(ref args) => {
            check_experimental(&config, "vector", "query")?; // TODO(v2.0): remove — graduates with vector
            return cmd_query(args.clone(), &cli, &config);
        }
        _ => {}
    }

    setup_event_emitters(
        cli.log_format.as_deref()
            .or_else(|| config.as_ref().and_then(|c| c.observability.log_format.as_deref()))
            .unwrap_or("console"),
        cli.log_file.as_deref()
            .or_else(|| config.as_ref().and_then(|c| c.observability.log_file.as_deref())),
    );

    match cli.command {
        Commands::Info { file, metrics } => {
            if metrics {
                cmd_info_metrics();
                Ok(())
            } else {
                cmd_info(file)
            }
        }
        Commands::Metadata { file } => cmd_metadata(file),
        Commands::Outline { file } => cmd_outline(file),
        Commands::PageCount { file } => cmd_page_count(file),
        Commands::Search { file, query } => cmd_search(file, query),
        Commands::Grep { dir, query } => cmd_grep(dir, query),
        Commands::Read(args) => cmd_read(args),
        Commands::Scan { dir, no_hash } => {
            if read_only {
                return Err("scan is disabled in read-only mode".to_string());
            }
            cmd_scan(dir, no_hash)
        }
        Commands::Hash { file } => cmd_hash(file),
        Commands::Verify { file, hash } => cmd_verify(file, hash),
        Commands::Watch { dir, no_sandbox } => {
            if read_only {
                return Err("watch is disabled in read-only mode".to_string());
            }
            cmd_watch(dir, no_sandbox)
        }
        Commands::Chunk(args) => cmd_chunk(args),
        Commands::Index(_) => unreachable!(),
        Commands::Query(_) => unreachable!(),
        Commands::VectorSearch(args) => {
            check_experimental(&config, "vector", "vector-search")?; // TODO(v2.0): remove — graduates with vector
            cmd_vector_search(args, &config)
        }
        Commands::Vector(args) => {
            check_experimental(&config, "vector", "vector")?; // TODO(v2.0): remove — graduates with vector
            cmd_vector(args)
        }
        Commands::Graph(args) => {
            check_experimental(&config, "graph", "graph")?; // TODO(v2.0): remove — graduates with graph
            cmd_graph(args)
        }
        Commands::Config(args) => cmd_config(args, &config),
        Commands::Init(args) => cmd_init(args.dir),
        Commands::McpSetup(args) => cmd_mcp_setup(args.agent, args.write),
    }
}

fn setup_event_emitters(log_format: &str, log_file: Option<&str>) {
    let emitters: Vec<Box<dyn crate::ocean_cli::events::EventEmitter>> = match log_format {
        "json" => {
            let target = if let Some(path) = log_file {
                OutputTarget::File(std::path::PathBuf::from(path))
            } else {
                OutputTarget::Stderr
            };
            vec![Box::new(JsonEmitter::new(target))]
        }
        _ => {
            let mut emitters: Vec<Box<dyn crate::ocean_cli::events::EventEmitter>> = vec![Box::new(ConsoleEmitter)];
            if let Some(path) = log_file {
                let target = OutputTarget::File(std::path::PathBuf::from(path));
                emitters.push(Box::new(JsonEmitter::new(target)));
            }
            emitters
        }
    };

    if emitters.len() == 1 {
        set_global_emitter(emitters.into_iter().next().unwrap());
    } else {
        set_global_emitter(Box::new(MultiEmitter::new(emitters)));
    }
}

fn cmd_info(file: String) -> Result<(), String> {
    let doc = api_docs::open_doc(&file).map_err(|e| e.to_string())?;
    println!();
    print_meta(&doc.metadata);
    println!();
    if !doc.outline.entries.is_empty() {
        println!("Outline:");
        print_outline(&doc.outline, 0);
    } else {
        println!("Outline: (empty)");
    }
    Ok(())
}

fn cmd_info_metrics() {
    let snapshot = global_metrics().snapshot();
    print_metrics(&snapshot);
}

fn cmd_metadata(file: String) -> Result<(), String> {
    let meta = api_docs::metadata(&file).map_err(|e| e.to_string())?;
    print_meta(&meta);
    Ok(())
}

fn cmd_outline(file: String) -> Result<(), String> {
    let outline = api_docs::outline(&file).map_err(|e| e.to_string())?;
    if outline.entries.is_empty() {
        println!("(empty outline)");
    } else {
        print_outline(&outline, 0);
    }
    Ok(())
}

fn cmd_page_count(file: String) -> Result<(), String> {
    let count = api_docs::page_count(&file).map_err(|e| e.to_string())?;
    match count {
        Some(n) => println!("{}", n),
        None => println!("(none)"),
    }
    Ok(())
}

fn cmd_search(file: String, query: String) -> Result<(), String> {
    let matches = api_docs::search_doc(&file, &query).map_err(|e| e.to_string())?;
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
    let result = api_docs::grep_docs(&dir, &query).map_err(|e| e.to_string())?;
    for fm in &result.file_matches {
        println!("{}:", fm.file);
        for m in &fm.matches {
            println!("  {:?}: \"{}\"", m.selector, m.text);
        }
        println!();
    }
    println!("Total: {} match(es) in {} file(s) for '{}'", result.total_matches, result.total_files, query);
    Ok(())
}

fn cmd_read(args: ReadArgs) -> Result<(), String> {
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

    let request = ReadRequest { file: args.file, selector };
    let result = api_docs::read_doc(&request).map_err(|e| e.to_string())?;
    print_read_result(result);
    Ok(())
}

fn cmd_scan(dir: String, no_hash: bool) -> Result<(), String> {
    let metas = api_fs::scan_files(&dir).map_err(|e| e.to_string())?;
    if metas.is_empty() {
        println!("No supported files found in '{}'.", dir);
    } else {
        println!("Found {} file(s) in '{}':", metas.len(), dir);
        if no_hash {
            for meta in &metas {
                let size_kb = meta.size as f64 / 1024.0;
                println!("  {:>8.1} KB  {:4}  {}", size_kb, meta.extension, meta.path);
            }
        } else {
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
    let hash = api_fs::hash_file(&file).map_err(|e| e.to_string())?;
    println!("{}", hash);
    Ok(())
}

fn cmd_verify(file: String, hash: String) -> Result<(), String> {
    let result = api_fs::verify_file(&file, &hash).map_err(|e| e.to_string())?;
    println!("{}", result);
    Ok(())
}

fn cmd_watch(dir: String, no_sandbox: bool) -> Result<(), String> {
    if !no_sandbox {
        let dir_path = std::path::Path::new(&dir);
        let _sandbox = crate::ocean_cli::sandbox::Sandbox::new(dir_path)
            .map_err(|e| format!("Sandbox init failed: {}", e))?;
    }

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

fn cmd_chunk(args: ChunkArgs) -> Result<(), String> {
    let config = ChunkConfig {
        min_tokens: args.min_size,
        max_tokens: args.max_size,
        overlap_sentences: args.overlap,
        include_images: args.include_images,
        rows_per_sheet_chunk: args.rows_per_chunk,
        token_estimator: crate::ocean_chunk::default_token_estimator,
    };

    let chunks = api_docs::chunk_doc(&args.file, Some(config)).map_err(|e| e.to_string())?;
    if chunks.is_empty() {
        println!("No chunks produced.");
        return Ok(());
    }

    println!("{} chunks from '{}':", chunks.len(), args.file);
    for chunk in &chunks {
        let token_est = crate::ocean_chunk::estimate_tokens(&chunk.content);
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

fn cmd_index(args: IndexArgs, cli: &Cli, config: &Option<OceanConfig>) -> Result<(), String> {
    let read_only = config.as_ref().and_then(|c| c.security.read_only).unwrap_or(false);
    if read_only {
        ReadOnlyGuard::set_global_enabled(true);
        return Err("indexing is disabled in read-only mode".to_string());
    }

    setup_event_emitters(
        args.log_format.as_deref()
            .or_else(|| cli.log_format.as_deref())
            .or_else(|| config.as_ref().and_then(|c| c.observability.log_format.as_deref()))
            .unwrap_or("console"),
        args.log_file.as_deref()
            .or_else(|| cli.log_file.as_deref())
            .or_else(|| config.as_ref().and_then(|c| c.observability.log_file.as_deref())),
    );

    let provider = EmbeddingConfig::resolve_provider(args.provider.as_deref(),
        config.as_ref().and_then(|c| c.embedding.provider.as_deref()));
    let model = EmbeddingConfig::resolve_model(args.model.as_deref(),
        config.as_ref().and_then(|c| c.embedding.model.as_deref()));

    let base_path = crate::ocean_cli::config::resolve_db_path(
        args.db_path.as_deref(),
        config.as_ref().and_then(|c| c.index.db_path.as_deref()),
    );

    let dimension = EmbeddingConfig::resolve_dimension(
        args.dimension,
        config.as_ref().and_then(|c| c.embedding.dimension),
        &provider,
        &model,
    );

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

    let runtime_config = config.as_ref().map(|c| &c.runtime);
    let mode = RuntimeMode::resolve(
        args.mode.as_deref(),
        runtime_config.and_then(|r| r.mode.as_deref()),
    );
    let mode_defaults = mode.defaults();

    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let io_threads = args.io_threads
        .or_else(|| runtime_config.and_then(|r| r.io_threads))
        .or(mode_defaults.io_threads)
        .unwrap_or(cpus * 2);
    let cpu_threads = args.cpu_threads
        .or_else(|| runtime_config.and_then(|r| r.cpu_threads))
        .or(mode_defaults.cpu_threads)
        .unwrap_or(cpus);
    let max_ai_concurrent = args.max_ai_concurrent
        .or_else(|| runtime_config.and_then(|r| r.max_ai_concurrent))
        .or(mode_defaults.max_ai_concurrent);
    let max_retries = args.max_retries
        .or_else(|| runtime_config.and_then(|r| r.max_retries));
    let retry_backoff_ms = args.retry_backoff_ms
        .or_else(|| runtime_config.and_then(|r| r.retry_backoff_ms));
    let max_queue_size = args.max_queue_size
        .or_else(|| runtime_config.and_then(|r| r.max_queue_size))
        .or(mode_defaults.max_queue_size);
    let max_in_flight = args.max_in_flight
        .or_else(|| runtime_config.and_then(|r| r.max_in_flight))
        .or(mode_defaults.max_in_flight);
    let embedding_cache_size = mode_defaults.embedding_cache_size_value();
    let batch_size = args.batch_size
        .or_else(|| config.as_ref().and_then(|c| c.index.batch_size))
        .or(mode_defaults.embedding_batch_size)
        .unwrap_or(10);

    let request = IndexRequest {
        dir: args.dir,
        provider: Some(provider),
        model: Some(model),
        dimension: Some(dimension),
        db_path: Some(base_path),
        api_key,
        base_url: Some(base_url),
        batch_size,
        reindex: args.reindex,
        no_graph: args.no_graph,
        no_references: args.no_references,
        no_entities: args.no_entities,
        watch: args.watch,
        chunk_config: None,
        io_threads: Some(io_threads),
        cpu_threads: Some(cpu_threads),
        max_ai_concurrent,
        max_retries,
        retry_backoff_ms,
        max_queue_size,
        max_in_flight,
        mode: Some(format!("{:?}", mode).to_lowercase()),
        no_sandbox: args.no_sandbox,
        embedding_cache_size: Some(embedding_cache_size),
    };

    global_emitter().emit(SystemEvent::IndexStarted {
        timestamp: crate::ocean_cli::events::unix_millis(),
        dir: request.dir.clone(),
        total_files: 0,
    });

    let report = api_index::index_directory(request).map_err(|e| e.to_string())?;

    global_emitter().emit(SystemEvent::IndexComplete {
        timestamp: crate::ocean_cli::events::unix_millis(),
        duration_ms: report.duration_ms,
        indexed: report.indexed,
        skipped: report.skipped,
        failed: report.failed,
    });

    let metrics = global_metrics();
    metrics.files_indexed.fetch_add(report.indexed, std::sync::atomic::Ordering::Relaxed);
    metrics.files_skipped.fetch_add(report.skipped, std::sync::atomic::Ordering::Relaxed);
    metrics.files_failed.fetch_add(report.failed, std::sync::atomic::Ordering::Relaxed);

    if report.failed > 0 {
        Err(format!("Indexing completed with {} failures.", report.failed))
    } else {
        Ok(())
    }
}

fn cmd_query(args: QueryArgs, cli: &Cli, config: &Option<OceanConfig>) -> Result<(), String> {
    setup_event_emitters(
        args.log_format.as_deref()
            .or_else(|| cli.log_format.as_deref())
            .or_else(|| config.as_ref().and_then(|c| c.observability.log_format.as_deref()))
            .unwrap_or("console"),
        args.log_file.as_deref()
            .or_else(|| cli.log_file.as_deref())
            .or_else(|| config.as_ref().and_then(|c| c.observability.log_file.as_deref())),
    );

    let provider = EmbeddingConfig::resolve_provider(args.provider.as_deref(),
        config.as_ref().and_then(|c| c.embedding.provider.as_deref()));
    let model = EmbeddingConfig::resolve_model(args.model.as_deref(),
        config.as_ref().and_then(|c| c.embedding.model.as_deref()));

    let mode = args.mode.clone().or_else(|| {
        config.as_ref().and_then(|c| c.query.mode.clone())
    });

    let read_only = args.read_only ||
        config.as_ref().and_then(|c| c.security.read_only).unwrap_or(false);
    if read_only {
        ReadOnlyGuard::set_global_enabled(true);
    }

    let base_path = crate::ocean_cli::config::resolve_db_path(
        args.db_path.as_deref(),
        config.as_ref().and_then(|c| c.query.db_path.as_deref()),
    );

    let dimension = EmbeddingConfig::resolve_dimension(
        args.dimension,
        config.as_ref().and_then(|c| c.embedding.dimension),
        &provider,
        &model,
    );

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

    let query_text = args.query.clone();
    let start = std::time::Instant::now();

    let request = QueryRequest {
        text: args.query,
        mode,
        top_k: args.top_k,
        expand_depth: args.expand_depth,
        include_context: args.context,
        context_chunks: args.context_chunks,
        no_cache: args.no_cache,
        filter_file_id: args.file_id,
        filter_heading: args.heading,
        filter_block_type: args.block_type,
        rerank_by_heading: args.rerank_by_heading,
        rerank_by_file: args.rerank_by_file,
        model: Some(model),
        provider: Some(provider),
        dimension: Some(dimension),
        api_key,
        base_url: Some(base_url),
        db_path: Some(base_path),
        read_only: Some(read_only),
    };

    let metrics = global_metrics();
    metrics.queries_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let result = api_query::query(request).map_err(|e| e.to_string())?;

    let duration_ms = start.elapsed().as_millis() as u64;

    global_emitter().emit(SystemEvent::QueryExecuted {
        timestamp: crate::ocean_cli::events::unix_millis(),
        query: query_text,
        mode: format!("{:?}", result.execution.query_mode),
        num_results: result.results.len(),
        duration_ms,
        cached: false,
    });

    print_query_result(&result, args.verbose);
    Ok(())
}

fn cmd_vector_search(args: VectorSearchArgs, config: &Option<OceanConfig>) -> Result<(), String> {
    let provider = EmbeddingConfig::resolve_provider(args.provider.as_deref(),
        config.as_ref().and_then(|c| c.embedding.provider.as_deref()));
    let model = EmbeddingConfig::resolve_model(args.model.as_deref(),
        config.as_ref().and_then(|c| c.embedding.model.as_deref()));

    let base_path = crate::ocean_cli::config::resolve_db_path(
        args.db_path.as_deref(),
        config.as_ref().and_then(|c| c.query.db_path.as_deref()),
    );

    let dimension = EmbeddingConfig::resolve_dimension(
        args.dimension,
        config.as_ref().and_then(|c| c.embedding.dimension),
        &provider,
        &model,
    );

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

    let query_text = args.query.clone();

    let request = VectorSearchRequest {
        query: args.query,
        top_k: args.top_k,
        hybrid: args.hybrid,
        expand_depth: args.expand_depth,
        filter_file_id: args.file_id,
        filter_heading: args.heading,
        filter_block_type: args.block_type,
        model: Some(model),
        provider: Some(provider),
        dimension: Some(dimension),
        api_key,
        base_url: Some(base_url),
        db_path: Some(base_path),
    };

    let results = api_query::vector_search(request).map_err(|e| e.to_string())?;
    if results.is_empty() {
        println!("No results found.");
    } else {
        let label = if args.expand_depth > 0 { "expanded results" } else { "results" };
        println!("Top {} {} for '{}':", results.len(), label, query_text);
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
                &r.file_id[..r.file_id.len().min(8)],
                heading,
            );
            println!("     \"{}\"", content_short);
        }
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
        GraphCommands::Status { db_path } => {
            cmd_graph_status(resolve(db_path))
        }
    }
}

fn cmd_vector(args: VectorArgs) -> Result<(), String> {
    match args.command {
        VectorCommands::Status { db_path, provider, model, api_key, ollama_url } => {
            cmd_vector_status(db_path, provider, model, api_key, ollama_url)
        }
    }
}

fn cmd_graph_info(file: String, db_path: String) -> Result<(), String> {
    let info = api_graph::graph_info(&file, &db_path).map_err(|e| e.to_string())?;
    print_graph_info(info.node_count, info.edge_count,
        info.type_breakdown.into_iter().collect());
    Ok(())
}

fn cmd_graph_expand(node_id: String, depth: usize, direction: String, db_path: String) -> Result<(), String> {
    let subgraph = api_graph::graph_expand(&node_id, depth, &direction, &db_path)
        .map_err(|e| e.to_string())?;
    print_graph_expanded(&subgraph);
    Ok(())
}

fn cmd_graph_path(from: String, to: String, max_depth: usize, db_path: String) -> Result<(), String> {
    let path = api_graph::graph_path(&from, &to, max_depth, &db_path)
        .map_err(|e| e.to_string())?;
    match path {
        Some(edges) => print_graph_path(&edges),
        None => println!("No path found between '{}' and '{}' within {} hops.", from, to, max_depth),
    }
    Ok(())
}

fn cmd_config(args: ConfigArgs, config: &Option<OceanConfig>) -> Result<(), String> {
    match args.command {
        ConfigCommands::Show => cmd_config_show(config),
        ConfigCommands::Validate => cmd_config_validate(config),
    }
}

fn cmd_config_show(config: &Option<OceanConfig>) -> Result<(), String> {
    match config {
        Some(cfg) => {
            let json = serde_json::to_string_pretty(cfg)
                .map_err(|e| format!("Failed to serialize config: {}", e))?;
            println!("{}", json);
        }
        None => {
            println!("No config file found. Using defaults.");
            let default_cfg = OceanConfig::default();
            let json = serde_json::to_string_pretty(&default_cfg)
                .map_err(|e| format!("Failed to serialize config: {}", e))?;
            println!("{}", json);
        }
    }
    Ok(())
}

fn cmd_config_validate(config: &Option<OceanConfig>) -> Result<(), String> {
    match config {
        Some(cfg) => {
            match cfg.validate() {
                Ok(()) => {
                    println!("config OK");
                }
                Err(errors) => {
                    for err in &errors {
                        println!("  error: {}", err);
                    }
                    return Err(format!("config has {} error(s)", errors.len()));
                }
            }
        }
        None => {
            println!("No config file found. Using defaults — no validation needed.");
        }
    }
    Ok(())
}

fn cmd_graph_stats(db_path: String) -> Result<(), String> {
    use std::sync::Arc;
    use crate::ocean_storage::config::StorageConfig;
    use crate::ocean_storage::graph_store::{GraphStore, NodeType};
    use crate::ocean_storage::SurrealGraphStore;

    let config = StorageConfig::new(&db_path);
    let store = SurrealGraphStore::new_persistent_at(&db_path, &config)
        .map_err(|e| format!("Failed to open graph store: {}", e))?;
    store.initialize_schema()
        .map_err(|e| format!("Failed to init schema: {}", e))?;
    let store: Arc<dyn GraphStore> = Arc::new(store);

    let total_nodes = store.count_nodes().unwrap_or(0);
    let total_edges = store.count_edges().unwrap_or(0);
    let type_counts = vec![
        ("File".to_string(), store.get_nodes_by_type(NodeType::File).map(|v| v.len() as u64).unwrap_or(0)),
        ("Chunk".to_string(), store.get_nodes_by_type(NodeType::Chunk).map(|v| v.len() as u64).unwrap_or(0)),
        ("Heading".to_string(), store.get_nodes_by_type(NodeType::Heading).map(|v| v.len() as u64).unwrap_or(0)),
        ("Entity".to_string(), store.get_nodes_by_type(NodeType::Entity).map(|v| v.len() as u64).unwrap_or(0)),
        ("Folder".to_string(), store.get_nodes_by_type(NodeType::Folder).map(|v| v.len() as u64).unwrap_or(0)),
    ];
    print_graph_stats(total_nodes, total_edges, type_counts);
    Ok(())
}

fn cmd_graph_status(db_path: String) -> Result<(), String> {
    use std::sync::Arc;
    use crate::ocean_storage::config::StorageConfig;
    use crate::ocean_storage::graph_store::{GraphStore, NodeType};
    use crate::ocean_storage::SurrealGraphStore;

    let config = StorageConfig::new(&db_path);

    let store = match SurrealGraphStore::new_persistent_at(&db_path, &config) {
        Ok(s) => s,
        Err(e) => {
            print_graph_status(&db_path, false, false, 0, 0, vec![]);
            return Err(format!("Failed to open graph store: {}", e));
        }
    };

    let schema_ok = store.initialize_schema().is_ok();
    let store: Arc<dyn GraphStore> = Arc::new(store);

    let (total_nodes, total_edges) = if schema_ok {
        (store.count_nodes().unwrap_or(0), store.count_edges().unwrap_or(0))
    } else {
        (0, 0)
    };

    let type_counts = if total_nodes > 0 {
        vec![
            ("File".to_string(), store.get_nodes_by_type(NodeType::File).map(|v| v.len() as u64).unwrap_or(0)),
            ("Chunk".to_string(), store.get_nodes_by_type(NodeType::Chunk).map(|v| v.len() as u64).unwrap_or(0)),
            ("Heading".to_string(), store.get_nodes_by_type(NodeType::Heading).map(|v| v.len() as u64).unwrap_or(0)),
            ("Entity".to_string(), store.get_nodes_by_type(NodeType::Entity).map(|v| v.len() as u64).unwrap_or(0)),
            ("Folder".to_string(), store.get_nodes_by_type(NodeType::Folder).map(|v| v.len() as u64).unwrap_or(0)),
        ]
    } else {
        vec![]
    };

    print_graph_status(&db_path, true, schema_ok, total_nodes, total_edges, type_counts);
    Ok(())
}

fn cmd_vector_status(
    db_path: Option<String>,
    cli_provider: Option<String>,
    cli_model: Option<String>,
    cli_api_key: Option<String>,
    cli_ollama_url: Option<String>,
) -> Result<(), String> {
    use std::time::Instant;
    use crate::ocean_storage::config::StorageConfig;
    use crate::ocean_storage::vector_store::VectorStore;
    use crate::ocean_storage::SurrealVectorStore;
    use crate::ocean_api::embedding::{create_embedder, EmbeddingConfig};
    use crate::ocean_cli::config::OceanConfig;

    let config = OceanConfig::load();
    let base_path = crate::ocean_cli::config::resolve_db_path(
        db_path.as_deref(),
        config.as_ref().and_then(|c| c.index.db_path.as_deref()),
    );
    let vector_db = crate::ocean_cli::config::resolve_vector_db_path(
        Some(&base_path),
        None,
    );

    let provider = EmbeddingConfig::resolve_provider(
        cli_provider.as_deref(),
        config.as_ref().and_then(|c| c.embedding.provider.as_deref()),
    );
    let model = EmbeddingConfig::resolve_model(
        cli_model.as_deref(),
        config.as_ref().and_then(|c| c.embedding.model.as_deref()),
    );
    let dimension = EmbeddingConfig::resolve_dimension(
        None,
        config.as_ref().and_then(|c| c.embedding.dimension),
        &provider,
        &model,
    );
    let resolved_key = crate::ocean_cli::config::resolve_api_key(
        cli_api_key.as_deref(),
        config.as_ref().and_then(|c| c.embedding.api_key.as_deref()),
        None,
    );
    let base_url = crate::ocean_cli::config::resolve_base_url(
        &provider,
        cli_ollama_url.as_deref(),
        config.as_ref().and_then(|c| c.embedding.base_url.as_deref()),
    );

    let st_config = StorageConfig::new(&base_path);

    let store = match SurrealVectorStore::new_persistent(&st_config) {
        Ok(s) => s,
        Err(e) => {
            print_vector_status(
                &vector_db, false, false, 0,
                &provider, &model, dimension,
                resolved_key.is_some(),
                matches!(provider.as_str(), "openai" | "anthropic" | "gemini"),
                Some("FAILED (open error)".to_string()), None,
            );
            return Err(format!("Failed to open vector store: {}", e));
        }
    };

    let schema_ok = store.initialize_schema(dimension).is_ok();
    let chunk_count = if schema_ok { store.count().unwrap_or(0) } else { 0 };

    let api_key_required = matches!(provider.as_str(), "openai" | "anthropic" | "gemini");
    let api_key_set = resolved_key.is_some();

    let (connection_result, connection_ms): (Option<String>, Option<u64>) =
        if api_key_required && !api_key_set {
            (Some("Skipped (API key not set)".to_string()), None)
        } else {
            match create_embedder(&provider, &model, &base_url, resolved_key.as_deref()) {
                Ok(embedder) => {
                    let start = Instant::now();
                    match embedder.embed("status check") {
                        Ok(_) => {
                            let ms = start.elapsed().as_millis() as u64;
                            (Some(format!("OK")), Some(ms))
                        }
                        Err(e) => {
                            (Some(format!("FAILED: {}", e)), None)
                        }
                    }
                }
                Err(e) => {
                    (Some(format!("Skipped: {}", e)), None)
                }
            }
        };

    print_vector_status(
        &vector_db, true, schema_ok, chunk_count,
        &provider, &model, dimension,
        api_key_set, api_key_required,
        connection_result, connection_ms,
    );
    Ok(())
}
