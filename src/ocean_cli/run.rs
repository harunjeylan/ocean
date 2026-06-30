use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;

use clap::Parser;

use crate::ocean_chunk::*;
use crate::ocean_cli::args::{ChunkArgs, Cli, Commands, IndexArgs, ReadArgs, VectorSearchArgs};
use crate::ocean_cli::display::*;
use crate::ocean_cli::walk::*;
use crate::ocean_fs::*;
use crate::ocean_parser::*;
use crate::ocean_vector::*;

pub fn run() -> Result<(), String> {
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
        Commands::Index(args) => cmd_index(args),
        Commands::VectorSearch(args) => cmd_vector_search(args),
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

fn create_embedder(
    provider: &str,
    model: &str,
    ollama_url: &str,
    openai_key: Option<&str>,
    openai_url: Option<&str>,
    anthropic_key: Option<&str>,
    anthropic_url: Option<&str>,
    gemini_key: Option<&str>,
) -> Result<Box<dyn Embedder>, String> {
    match provider {
        "ollama" => {
            Ok(Box::new(
                OllamaEmbedder::new(model, ollama_url)
                    .map_err(|e| format!("Failed to create Ollama embedder: {}", e))?,
            ))
        }
        "openai" => {
            let key = openai_key.ok_or_else(|| "--openai-key is required for openai provider")?;
            let url = openai_url.unwrap_or("https://api.openai.com/v1");
            Ok(Box::new(
                OpenAIEmbedder::new(model, url, key)
                    .map_err(|e| format!("Failed to create OpenAI embedder: {}", e))?,
            ))
        }
        "anthropic" => {
            let key = anthropic_key.ok_or_else(|| "--anthropic-key is required for anthropic provider")?;
            let url = anthropic_url.unwrap_or("https://api.anthropic.com/v1");
            Ok(Box::new(
                AnthropicEmbedder::new(model, url, key)
                    .map_err(|e| format!("Failed to create Anthropic embedder: {}", e))?,
            ))
        }
        "gemini" => {
            let key = gemini_key.ok_or_else(|| "--gemini-key is required for gemini provider")?;
            Ok(Box::new(
                GeminiEmbedder::new(model, key)
                    .map_err(|e| format!("Failed to create Gemini embedder: {}", e))?,
            ))
        }
        other => Err(format!("unsupported provider '{}'. Use: ollama, openai, anthropic, gemini", other)),
    }
}

fn cmd_index(args: IndexArgs) -> Result<(), String> {
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

    let store = VectorStore::new_persistent(&args.db_path)
        .map_err(|e| format!("Failed to open store: {}", e))?;
    let dim = match args.provider.as_str() {
        "openai" if args.model.contains("large") => 3072,
        "openai" if args.model.contains("small") => 1536,
        _ => 768,
    };
    store.initialize_schema(dim)
        .map_err(|e| format!("Failed to init schema: {}", e))?;

    let embedder = create_embedder(&args.provider, &args.model, &args.ollama_url,
        args.openai_key.as_deref(), args.openai_url.as_deref(),
        args.anthropic_key.as_deref(), args.anthropic_url.as_deref(),
        args.gemini_key.as_deref())?;

    let pipeline = IndexPipeline::new(store);

    let config = IndexConfig {
        batch_size: args.batch_size,
        reindex: args.reindex,
        model: args.model.clone(),
        dimension: embedder.dimension(),
        ollama_url: Some(args.ollama_url.clone()),
        openai_api_key: args.openai_key.clone(),
        db_path: args.db_path.clone(),
    };

    let file_count = files.len();
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

        match pipeline.index_chunks(chunks, &*embedder, &config) {
            Ok(report) => {
                println!(
                    "  Indexed: {} embedded, {} skipped, {} failed ({}ms)",
                    report.embedded, report.skipped, report.failed, report.duration_ms
                );
            }
            Err(e) => {
                println!("  Index error: {}", e);
            }
        }
    }

    println!("Indexing complete.");
    Ok(())
}

fn cmd_vector_search(args: VectorSearchArgs) -> Result<(), String> {
    let store = VectorStore::new_persistent(&args.db_path)
        .map_err(|e| format!("Failed to open store: {}", e))?;
    let engine = SearchEngine::new(store);

    let embedder = create_embedder(&args.provider, &args.model, &args.ollama_url,
        args.openai_key.as_deref(), args.openai_url.as_deref(),
        args.anthropic_key.as_deref(), args.anthropic_url.as_deref(),
        args.gemini_key.as_deref())?;

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

    match results {
        Ok(results) => {
            if results.is_empty() {
                println!("No results found.");
            } else {
                println!("Top {} results for '{}':", results.len(), args.query);
                for (i, r) in results.iter().enumerate() {
                    let content_short = if r.content.len() > 120 {
                        format!("{}...", &r.content[..120])
                    } else {
                        r.content.clone()
                    };
                    let heading = r.heading.as_deref().unwrap_or("(none)");
                    let score_info = if let (Some(vs), Some(fs)) = (r.vector_score, r.fts_score) {
                        format!("score={:.4} (vec={:.4}, fts={:.4})", r.score, vs, fs)
                    } else {
                        format!("score={:.4}", r.score)
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
        }
        Err(e) => return Err(format!("Search failed: {}", e)),
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
