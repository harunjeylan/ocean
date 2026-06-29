use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, Args};

use ocean::ocean_parser::*;

const SUPPORTED_EXTS: &[&str] = &["pdf", "docx", "xlsx", "pptx", "txt", "md", "html", "htm"];

fn walk_supported_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = vec![];
    if !dir.is_dir() {
        return files;
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if SUPPORTED_EXTS.contains(&ext.to_lowercase().as_str()) {
                            files.push(path);
                        }
                    }
                }
            }
        }
    }
    files.sort();
    files
}

#[derive(Parser)]
#[command(name = "ocean-cli", version, about = "Document reader — open, inspect, search any document")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show document summary (metadata + outline)
    Info { file: String },

    /// Show all document metadata fields
    Metadata { file: String },

    /// Show document outline/table of contents
    Outline { file: String },

    /// Show page/slide count
    PageCount { file: String },

    /// Search for text across the document
    Search {
        file: String,
        query: String,
    },

    /// Search all supported documents in a directory recursively
    Grep {
        dir: String,
        query: String,
    },

    /// Read content from the document
    Read(ReadArgs),
}

#[derive(Args)]
struct ReadArgs {
    file: String,

    /// Read by page number
    #[arg(long)]
    page: Option<u32>,

    /// Read heading content (by heading text)
    #[arg(long)]
    heading: Option<String>,

    /// Read by paragraph index
    #[arg(long)]
    paragraph: Option<u32>,

    /// Read by table index
    #[arg(long)]
    table: Option<u32>,

    /// Read by slide number
    #[arg(long)]
    slide: Option<u32>,

    /// Read sheet by name
    #[arg(long)]
    sheet: Option<String>,

    /// Read cell reference (e.g. "B12")
    #[arg(long)]
    cell: Option<String>,

    /// Read by image index
    #[arg(long)]
    image: Option<u32>,

    /// Read byte range (e.g. "0-100")
    #[arg(long)]
    range: Option<String>,
}

fn print_meta(meta: &DocumentMetadata) {
    println!("Metadata:");
    println!("  Path:    {}", meta.path.display());
    println!("  Format:  {:?}", meta.format);
    println!("  Size:    {} bytes", meta.size);
    if let Some(ref t) = meta.title { println!("  Title:   {}", t); }
    if let Some(ref a) = meta.author { println!("  Author:  {}", a); }
    if let Some(ref c) = meta.created { println!("  Created: {}", c); }
    if let Some(ref m) = meta.modified { println!("  Modified: {}", m); }
    if let Some(ref p) = meta.page_count { println!("  Pages:   {}", p); }
}

fn print_outline(outline: &Outline, indent: usize) {
    for entry in &outline.entries {
        println!("{:indent$}- [L{}] {}  ({:?})", "", entry.level, entry.label, entry.selector, indent = indent);
        if !entry.children.is_empty() {
            let child_outline = Outline { entries: entry.children.clone() };
            print_outline(&child_outline, indent + 2);
        }
    }
}

fn print_read_result(result: ReadResult) {
    match result {
        ReadResult::Text(t) => println!("{}", t),
        ReadResult::Table { headers, rows } => {
            if !headers.is_empty() {
                println!("{}", headers.join(" | "));
                println!("{}", vec!["---"; headers.len()].join(" | "));
            }
            for row in &rows {
                println!("{}", row.join(" | "));
            }
        }
        ReadResult::Image { bytes, format, caption } => {
            println!("Image: {} bytes, format: {:?}", bytes.len(), format);
            if let Some(ref cap) = caption { println!("Caption: {}", cap); }
        }
        ReadResult::Metadata(meta) => print_meta(&meta),
        ReadResult::Outline(ref outline) => print_outline(outline, 0),
        ReadResult::Page { number, text } => {
            println!("--- Page {} ---", number);
            println!("{}", text);
        }
        ReadResult::Slide { number, title, content } => {
            println!("--- Slide {} ---", number);
            if let Some(ref t) = title { println!("Title: {}", t); }
            println!("{}", content);
        }
        ReadResult::Sheet { name, rows } => {
            println!("--- Sheet: {} ---", name);
            for row in &rows {
                println!("{}", row.join(" | "));
            }
        }
        ReadResult::CellValue(v) => println!("{}", v),
        ReadResult::MatchResult(matches) => {
            for m in matches {
                println!("  {:?}: \"{}\" (score: {})", m.selector, m.text, m.score);
                if !m.context.is_empty() {
                    println!("    context: {}", m.context);
                }
            }
        }
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => {
            let p = PathBuf::from(&file);
            if !p.exists() {
                return Err(format!("file not found: {}", file));
            }
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

        Commands::Metadata { file } => {
            let p = PathBuf::from(&file);
            if !p.exists() {
                return Err(format!("file not found: {}", file));
            }
            let doc = open(&file).map_err(|e| format!("Failed to open: {}", e))?;
            print_meta(&doc.metadata());
            Ok(())
        }

        Commands::Outline { file } => {
            let p = PathBuf::from(&file);
            if !p.exists() {
                return Err(format!("file not found: {}", file));
            }
            let doc = open(&file).map_err(|e| format!("Failed to open: {}", e))?;
            let outline = doc.outline();
            if outline.entries.is_empty() {
                println!("(empty outline)");
            } else {
                print_outline(&outline, 0);
            }
            Ok(())
        }

        Commands::PageCount { file } => {
            let p = PathBuf::from(&file);
            if !p.exists() {
                return Err(format!("file not found: {}", file));
            }
            let doc = open(&file).map_err(|e| format!("Failed to open: {}", e))?;
            match doc.page_count() {
                Some(n) => println!("{}", n),
                None => println!("(none)"),
            }
            Ok(())
        }

        Commands::Search { file, query } => {
            let p = PathBuf::from(&file);
            if !p.exists() {
                return Err(format!("file not found: {}", file));
            }
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

        Commands::Grep { dir, query } => {
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

        Commands::Read(args) => {
            let p = PathBuf::from(&args.file);
            if !p.exists() {
                return Err(format!("file not found: {}", args.file));
            }
            let doc = open(&args.file).map_err(|e| format!("Failed to open: {}", e))?;

            let selector = if let Some(n) = args.page {
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
                return Err("No selector specified. Use --page, --heading, --paragraph, --table, --slide, --sheet, --cell, --image, or --range".to_string());
            };

            let result = doc.read(&selector)
                .map_err(|e| format!("Read failed: {}", e))?;
            print_read_result(result);
            Ok(())
        }
    }
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(msg) => {
            eprintln!("Error: {}", msg);
            std::process::exit(1);
        }
    }
}
