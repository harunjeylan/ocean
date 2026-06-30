use clap::{Args, Parser, Subcommand};

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
    Index(IndexArgs),
    VectorSearch(VectorSearchArgs),
    Graph(GraphArgs),
}

#[derive(Args)]
pub struct GraphArgs {
    #[command(subcommand)]
    pub command: GraphCommands,
}

#[derive(Subcommand)]
pub enum GraphCommands {
    Info {
        file: String,
        #[arg(long, default_value = "ocean.db")]
        db_path: String,
    },
    Expand {
        node_id: String,
        #[arg(long, default_value_t = 2)]
        depth: usize,
        #[arg(long, default_value = "both")]
        direction: String,
        #[arg(long, default_value = "ocean.db")]
        db_path: String,
    },
    Path {
        from: String,
        to: String,
        #[arg(long, default_value_t = 5)]
        max_depth: usize,
        #[arg(long, default_value = "ocean.db")]
        db_path: String,
    },
    Stats {
        #[arg(long, default_value = "ocean.db")]
        db_path: String,
    },
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

#[derive(Args)]
pub struct IndexArgs {
    pub dir: String,
    #[arg(long, default_value = "nomic-embed-text")]
    pub model: String,
    #[arg(long, default_value = "ollama")]
    pub provider: String,
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,
    #[arg(long)]
    pub openai_key: Option<String>,
    #[arg(long)]
    pub openai_url: Option<String>,
    #[arg(long)]
    pub anthropic_key: Option<String>,
    #[arg(long)]
    pub anthropic_url: Option<String>,
    #[arg(long)]
    pub gemini_key: Option<String>,
    #[arg(long, default_value = "ocean.db")]
    pub db_path: String,
    #[arg(long, default_value_t = 10)]
    pub batch_size: usize,
    #[arg(long)]
    pub reindex: bool,
    #[arg(long)]
    pub no_graph: bool,
    #[arg(long)]
    pub no_references: bool,
    #[arg(long)]
    pub no_entities: bool,
}

#[derive(Args)]
pub struct VectorSearchArgs {
    pub query: String,
    #[arg(long, default_value_t = 10)]
    pub top_k: usize,
    #[arg(long)]
    pub hybrid: bool,
    #[arg(long)]
    pub file_id: Option<String>,
    #[arg(long)]
    pub heading: Option<String>,
    #[arg(long)]
    pub block_type: Option<String>,
    #[arg(long, default_value = "nomic-embed-text")]
    pub model: String,
    #[arg(long, default_value = "ollama")]
    pub provider: String,
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,
    #[arg(long)]
    pub openai_key: Option<String>,
    #[arg(long)]
    pub openai_url: Option<String>,
    #[arg(long)]
    pub anthropic_key: Option<String>,
    #[arg(long)]
    pub anthropic_url: Option<String>,
    #[arg(long)]
    pub gemini_key: Option<String>,
    #[arg(long, default_value = "ocean.db")]
    pub db_path: String,
    #[arg(long, default_value_t = 0)]
    pub expand_depth: usize,
}
