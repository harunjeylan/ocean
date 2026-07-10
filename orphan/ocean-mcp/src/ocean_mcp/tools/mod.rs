pub mod doc_tools;
pub mod query_tools;
pub mod graph_tools;

pub use doc_tools::*;
pub use query_tools::*;
pub use graph_tools::*;

use rmcp::model::{CallToolResult, ContentBlock};

pub fn to_text(text: String) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text)])
}

pub fn to_error(msg: &str) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.to_string())])
}

pub fn file_not_found(path: &str) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(format!("File does not exist: {}", path))])
}

pub fn dir_not_found(path: &str) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(format!("Directory does not exist: {}", path))])
}
