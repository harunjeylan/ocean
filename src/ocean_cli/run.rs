use std::path::PathBuf;

use clap::Parser;

use crate::ocean_cli::args::{Cli, Commands, ReadArgs};
use crate::ocean_cli::display::*;
use crate::ocean_cli::walk::*;
use crate::ocean_parser::*;

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
