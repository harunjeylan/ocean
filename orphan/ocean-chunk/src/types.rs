use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum ChunkType {
    Text,
    Table,
    Page,
    Slide,
    Sheet,
    Cell,
    Image,
    Metadata,
    Heading,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub id: String,
    pub file_id: String,
    pub content: String,
    pub heading: Option<String>,
    pub page: Option<u32>,
    pub slide: Option<u32>,
    pub sheet: Option<String>,
    pub block_type: ChunkType,
    pub start_offset: Option<usize>,
    pub end_offset: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct ChunkConfig {
    pub min_tokens: usize,
    pub max_tokens: usize,
    pub overlap_sentences: usize,
    pub include_images: bool,
    pub rows_per_sheet_chunk: usize,
    pub token_estimator: fn(&str) -> usize,
}

pub fn default_token_estimator(text: &str) -> usize {
    text.split_whitespace().count() + text.matches(|c: char| c.is_ascii_punctuation()).count() / 2
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_tokens: 100,
            max_tokens: 800,
            overlap_sentences: 1,
            include_images: false,
            rows_per_sheet_chunk: 50,
            token_estimator: default_token_estimator,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChunkError {
    EmptyInput,
    InvalidConfig(String),
    ContentTooLarge(String),
}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkError::EmptyInput => write!(f, "empty input: no blocks provided"),
            ChunkError::InvalidConfig(msg) => write!(f, "invalid config: {}", msg),
            ChunkError::ContentTooLarge(msg) => write!(f, "content too large: {}", msg),
        }
    }
}

impl std::error::Error for ChunkError {}

impl PartialEq for ChunkConfig {
    fn eq(&self, other: &Self) -> bool {
        self.min_tokens == other.min_tokens
            && self.max_tokens == other.max_tokens
            && self.overlap_sentences == other.overlap_sentences
            && self.include_images == other.include_images
            && self.rows_per_sheet_chunk == other.rows_per_sheet_chunk
    }
}

impl ChunkConfig {
    pub fn with_token_estimator(mut self, estimator: fn(&str) -> usize) -> Self {
        self.token_estimator = estimator;
        self
    }
}

pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}
