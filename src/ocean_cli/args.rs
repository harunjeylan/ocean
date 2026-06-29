use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "ocean", version, about = "Document runtime — inspect, search, and manage documents")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Info { file: String },
    Metadata { file: String },
    Outline { file: String },
    PageCount { file: String },
    Search { file: String, query: String },
    Grep { dir: String, query: String },
    Read(ReadArgs),
    Scan { dir: String, #[arg(long)] no_hash: bool },
    Hash { file: String },
    Verify { file: String, hash: String },
    Watch { dir: String },
    Chunk(ChunkArgs),
}

#[derive(Args)]
pub struct ReadArgs {
    pub file: String,
    #[arg(long)]
    pub page: Option<u32>,
    #[arg(long)]
    pub heading: Option<String>,
    #[arg(long)]
    pub paragraph: Option<u32>,
    #[arg(long)]
    pub table: Option<u32>,
    #[arg(long)]
    pub slide: Option<u32>,
    #[arg(long)]
    pub sheet: Option<String>,
    #[arg(long)]
    pub cell: Option<String>,
    #[arg(long)]
    pub image: Option<u32>,
    #[arg(long)]
    pub range: Option<String>,
    #[arg(long)]
    pub skip: Option<u32>,
    #[arg(long)]
    pub take: Option<u32>,
}

#[derive(Args)]
pub struct ChunkArgs {
    pub file: String,
    #[arg(long, default_value = "100")]
    pub min_size: usize,
    #[arg(long, default_value = "800")]
    pub max_size: usize,
    #[arg(long, default_value = "1")]
    pub overlap: usize,
    #[arg(long)]
    pub include_images: bool,
    #[arg(long, default_value = "50")]
    pub rows_per_chunk: usize,
}
