use std::path::PathBuf;

use crate::ocean_chunk::{chunk, ChunkConfig};
use crate::ocean_fs::generate_file_id;
use crate::ocean_parser::{open, read_all_blocks, Match, ReadResult};

use super::types::{ApiError, DocResult, FileMatches, GrepResult, ReadRequest};

pub fn open_doc(path: &str) -> Result<DocResult, ApiError> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(ApiError::DocError(format!("file not found: {}", path)));
    }
    let doc = open(path)?;
    let metadata = doc.metadata();
    let outline = doc.outline();
    Ok(DocResult { metadata, outline })
}

pub fn metadata(path: &str) -> Result<crate::ocean_parser::DocumentMetadata, ApiError> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(ApiError::DocError(format!("file not found: {}", path)));
    }
    let doc = open(path)?;
    Ok(doc.metadata())
}

pub fn outline(path: &str) -> Result<crate::ocean_parser::Outline, ApiError> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(ApiError::DocError(format!("file not found: {}", path)));
    }
    let doc = open(path)?;
    Ok(doc.outline())
}

pub fn page_count(path: &str) -> Result<Option<u32>, ApiError> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(ApiError::DocError(format!("file not found: {}", path)));
    }
    let doc = open(path)?;
    Ok(doc.page_count())
}

pub fn search_doc(path: &str, query: &str) -> Result<Vec<Match>, ApiError> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(ApiError::DocError(format!("file not found: {}", path)));
    }
    let doc = open(path)?;
    Ok(doc.search(query))
}

pub fn grep_docs(dir: &str, query: &str) -> Result<GrepResult, ApiError> {
    let dir_path = PathBuf::from(dir);
    if !dir_path.is_dir() {
        return Err(ApiError::DocError(format!("directory not found: {}", dir)));
    }
    let files = crate::ocean_cli::walk::walk_supported_files(&dir_path);
    let mut file_matches = Vec::new();
    let mut total_matches = 0u32;

    for path in &files {
        let name = path.to_string_lossy();
        if let Ok(doc) = open(&name) {
            let matches = doc.search(query);
            if !matches.is_empty() {
                total_matches += matches.len() as u32;
                file_matches.push(FileMatches {
                    file: path.to_string_lossy().to_string(),
                    matches,
                });
            }
        }
    }

    Ok(GrepResult {
        total_matches,
        total_files: files.len(),
        file_matches,
    })
}

pub fn read_doc(request: &ReadRequest) -> Result<ReadResult, ApiError> {
    let p = PathBuf::from(&request.file);
    if !p.exists() {
        return Err(ApiError::DocError(format!("file not found: {}", request.file)));
    }
    let doc = open(&request.file)?;
    let result = doc.read(&request.selector)?;
    Ok(result)
}

pub fn chunk_doc(file: &str, config: Option<ChunkConfig>) -> Result<Vec<crate::ocean_chunk::Chunk>, ApiError> {
    let doc = open(file)?;
    let blocks = read_all_blocks(&*doc)?;
    let file_id = generate_file_id();
    let chunks = chunk(blocks, &file_id, config)?;
    Ok(chunks)
}
