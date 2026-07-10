use crate::doc::{DocumentError, Match, open};
use crate::find::find_files;
use std::path::Path;

pub struct FileMatch {
    pub file: String,
    pub matches: Vec<Match>,
}

pub struct GrepResult {
    pub total_matches: u32,
    pub total_files: usize,
    pub file_matches: Vec<FileMatch>,
}

/// Search `query` across all supported documents in `dir`.
pub fn grep_docs(dir: &str, query: &str) -> Result<GrepResult, DocumentError> {
    let dir_path = Path::new(dir);
    let files = find_files(dir_path);
    let mut file_matches = Vec::new();
    let mut total_matches = 0u32;

    for path in &files {
        let name = path.to_string_lossy();
        if let Ok(doc) = open(&name) {
            let matches = doc.search(query);
            if !matches.is_empty() {
                total_matches += matches.len() as u32;
                file_matches.push(FileMatch {
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
