use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ocean", version, about = "Document runtime — inspect, search, and manage documents")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    #[arg(long, global = true)]
    pub log_format: Option<String>,
    #[arg(long, global = true)]
    pub log_file: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Info {
        file: String,
        #[arg(long)]
        metrics: bool,
    },
    Metadata { file: String },
    Outline { file: String },
    PageCount { file: String },
    Search { file: String, query: String },
    Grep { dir: String, query: String },
    Read(ReadArgs),
    Scan { dir: String, #[arg(long)] no_hash: bool },
    Hash { file: String },
    Verify { file: String, hash: String },
    Watch {
        dir: String,
        #[arg(long)]
        no_sandbox: bool,
    },
    Chunk(ChunkArgs),
    Index(IndexArgs),
    Query(QueryArgs),
    VectorSearch(VectorSearchArgs),
    Graph(GraphArgs),
    Vector(VectorArgs),
    Config(ConfigArgs),
    Init(InitArgs),
    McpSetup(McpSetupArgs),
}

#[derive(Args)]
pub struct McpSetupArgs {
    pub agent: Option<String>,
    #[arg(long)]
    pub write: bool,
}

#[derive(Args)]
pub struct InitArgs {
    #[arg(long)]
    pub dir: Option<String>,
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
        #[arg(long)]
        db_path: Option<String>,
    },
    Expand {
        node_id: String,
        #[arg(long, default_value_t = 2)]
        depth: usize,
        #[arg(long, default_value = "both")]
        direction: String,
        #[arg(long)]
        db_path: Option<String>,
    },
    Path {
        from: String,
        to: String,
        #[arg(long, default_value_t = 5)]
        max_depth: usize,
        #[arg(long)]
        db_path: Option<String>,
    },
    Stats {
        #[arg(long)]
        db_path: Option<String>,
    },
    Status {
        #[arg(long)]
        db_path: Option<String>,
    },
}

#[derive(Args)]
pub struct VectorArgs {
    #[command(subcommand)]
    pub command: VectorCommands,
}

#[derive(Subcommand)]
pub enum VectorCommands {
    Status {
        #[arg(long)]
        db_path: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        api_key: Option<String>,
        #[arg(long)]
        ollama_url: Option<String>,
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

#[derive(Args, Clone)]
pub struct IndexArgs {
    pub dir: String,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub ollama_url: Option<String>,
    #[arg(long)]
    pub api_key: Option<String>,
    #[arg(long)]
    pub dimension: Option<usize>,
    #[arg(long)]
    pub db_path: Option<String>,
    #[arg(long)]
    pub batch_size: Option<usize>,
    #[arg(long)]
    pub reindex: bool,
    #[arg(long)]
    pub no_graph: bool,
    #[arg(long)]
    pub no_references: bool,
    #[arg(long)]
    pub no_entities: bool,
    #[arg(long)]
    pub watch: bool,
    #[arg(long)]
    pub mode: Option<String>,
    #[arg(long)]
    pub no_sandbox: bool,
    #[arg(long)]
    pub io_threads: Option<usize>,
    #[arg(long)]
    pub cpu_threads: Option<usize>,
    #[arg(long)]
    pub max_ai_concurrent: Option<usize>,
    #[arg(long)]
    pub max_retries: Option<u32>,
    #[arg(long)]
    pub retry_backoff_ms: Option<u64>,
    #[arg(long)]
    pub max_queue_size: Option<usize>,
    #[arg(long)]
    pub max_in_flight: Option<usize>,
    #[arg(long)]
    pub log_format: Option<String>,
    #[arg(long)]
    pub log_file: Option<String>,
}

#[derive(Args, Clone)]
pub struct QueryArgs {
    pub query: String,
    #[arg(long)]
    pub mode: Option<String>,
    #[arg(long)]
    pub read_only: bool,
    #[arg(long, default_value_t = 10)]
    pub top_k: usize,
    #[arg(long, default_value_t = 0)]
    pub expand_depth: usize,
    #[arg(long)]
    pub context: bool,
    #[arg(long)]
    pub context_chunks: Option<usize>,
    #[arg(long)]
    pub no_cache: bool,
    #[arg(long)]
    pub file_id: Option<String>,
    #[arg(long)]
    pub heading: Option<String>,
    #[arg(long)]
    pub block_type: Option<String>,
    #[arg(long)]
    pub rerank_by_heading: bool,
    #[arg(long)]
    pub rerank_by_file: bool,
    #[arg(long)]
    pub verbose: bool,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub ollama_url: Option<String>,
    #[arg(long)]
    pub api_key: Option<String>,
    #[arg(long)]
    pub dimension: Option<usize>,
    #[arg(long)]
    pub db_path: Option<String>,
    #[arg(long)]
    pub log_format: Option<String>,
    #[arg(long)]
    pub log_file: Option<String>,
}

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    Show,
    Validate,
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
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub ollama_url: Option<String>,
    #[arg(long)]
    pub api_key: Option<String>,
    #[arg(long)]
    pub dimension: Option<usize>,
    #[arg(long)]
    pub db_path: Option<String>,
    #[arg(long, default_value_t = 0)]
    pub expand_depth: usize,
}
