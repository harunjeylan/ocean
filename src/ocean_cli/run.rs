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
    ChunkArgs, Cli, Commands, GraphArgs, GraphCommands, IndexArgs, QueryArgs, ReadArgs,
    VectorSearchArgs,
};
use crate::ocean_cli::config::OceanConfig;
use crate::ocean_cli::display::*;
use crate::ocean_fs::*;
use crate::ocean_parser::*;

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

fn cmd_chunk(args: ChunkArgs) -> Result<(), String> {
    let config = ChunkConfig {
        min_tokens: args.min_size,
        max_tokens: args.max_size,
        overlap_sentences: args.overlap,
        include_images: args.include_images,
        rows_per_sheet_chunk: args.rows_per_chunk,
        token_estimator: None,
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

fn cmd_index(args: IndexArgs, config: &Option<OceanConfig>) -> Result<(), String> {
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

    let runtime = config.as_ref().map(|c| &c.runtime);
    let io_threads = args.io_threads.or_else(|| runtime.and_then(|r| r.io_threads));
    let cpu_threads = args.cpu_threads.or_else(|| runtime.and_then(|r| r.cpu_threads));
    let max_ai_concurrent = args.max_ai_concurrent.or_else(|| runtime.and_then(|r| r.max_ai_concurrent));
    let max_retries = args.max_retries.or_else(|| runtime.and_then(|r| r.max_retries));
    let retry_backoff_ms = args.retry_backoff_ms.or_else(|| runtime.and_then(|r| r.retry_backoff_ms));
    let max_queue_size = args.max_queue_size.or_else(|| runtime.and_then(|r| r.max_queue_size));
    let max_in_flight = args.max_in_flight.or_else(|| runtime.and_then(|r| r.max_in_flight));

    let request = IndexRequest {
        dir: args.dir,
        provider: Some(provider),
        model: Some(model),
        dimension: Some(dimension),
        db_path: Some(base_path),
        api_key,
        base_url: Some(base_url),
        batch_size: args.batch_size,
        reindex: args.reindex,
        no_graph: args.no_graph,
        no_references: args.no_references,
        no_entities: args.no_entities,
        watch: args.watch,
        chunk_config: None,
        io_threads,
        cpu_threads,
        max_ai_concurrent,
        max_retries,
        retry_backoff_ms,
        max_queue_size,
        max_in_flight,
    };

    let report = api_index::index_directory(request).map_err(|e| e.to_string())?;
    if report.failed > 0 {
        Err(format!("Indexing completed with {} failures.", report.failed))
    } else {
        Ok(())
    }
}

fn cmd_query(args: QueryArgs, config: &Option<OceanConfig>) -> Result<(), String> {
    let provider = EmbeddingConfig::resolve_provider(args.provider.as_deref(),
        config.as_ref().and_then(|c| c.embedding.provider.as_deref()));
    let model = EmbeddingConfig::resolve_model(args.model.as_deref(),
        config.as_ref().and_then(|c| c.embedding.model.as_deref()));

    let mode = args.mode.clone().or_else(|| {
        config.as_ref().and_then(|c| c.query.mode.clone())
    });

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

    let request = QueryRequest {
        text: args.query,
        mode,
        top_k: args.top_k,
        expand_depth: args.expand_depth,
        include_context: args.context,
        context_chunks: args.context_chunks,
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
    };

    let result = api_query::query(request).map_err(|e| e.to_string())?;
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
    }
}

fn cmd_graph_info(file: String, db_path: String) -> Result<(), String> {
    let info = api_graph::graph_info(&file, &db_path).map_err(|e| e.to_string())?;
    print_graph_info(info.node_count, info.edge_count,
        info.type_breakdown.into_iter().map(|(nt, c)| (nt, c)).collect());
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

fn cmd_graph_stats(db_path: String) -> Result<(), String> {
    let stats = api_graph::graph_stats(&db_path).map_err(|e| e.to_string())?;
    let type_counts = vec![
        ("File".to_string(), 0u64),
        ("Chunk".to_string(), 0u64),
        ("Heading".to_string(), 0u64),
        ("Entity".to_string(), 0u64),
        ("Folder".to_string(), 0u64),
    ];
    print_graph_stats(stats.node_count, stats.edge_count, type_counts);
    Ok(())
}
