use rmcp::model::*;
use rmcp::ErrorData as McpError;

pub fn handle_read_resource(request: ReadResourceRequestParams) -> Result<ReadResourceResult, McpError> {
    let uri = &request.uri;
    if !uri.starts_with("document://") {
        return Err(McpError::resource_not_found("invalid scheme", None));
    }

    let raw_path = uri.trim_start_matches("document://");
    let decoded = urlencoding_decode(raw_path).unwrap_or_else(|_| raw_path.to_string());
    let path = std::path::PathBuf::from(&decoded);

    if !path.exists() {
        return Err(McpError::resource_not_found("file not found", None));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|_| McpError::internal_error(format!("Failed to read file: {}", path.display()), None))?;

    Ok(ReadResourceResult::new(vec![
        ResourceContents::TextResourceContents {
            uri: uri.clone(),
            mime_type: Some(mime_for_path(&path).to_string()),
            text: content,
            meta: None,
        },
    ]))
}

fn urlencoding_decode(s: &str) -> Result<String, ()> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hi = chars.next().ok_or(())?;
            let lo = chars.next().ok_or(())?;
            let byte = u8::from_str_radix(&format!("{}{}", hi, lo), 16).map_err(|_| ())?;
            result.push(byte as char);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    Ok(result)
}

fn mime_for_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "txt" | "md" => "text/plain",
        "html" | "htm" => "text/html",
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}
