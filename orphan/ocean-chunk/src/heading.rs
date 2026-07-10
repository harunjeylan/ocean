pub fn detect_heading(text: &str) -> Option<(u8, String)> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (level, content) = if let Some(rest) = trimmed.strip_prefix("###### ") {
        (6u8, rest)
    } else if let Some(rest) = trimmed.strip_prefix("##### ") {
        (5u8, rest)
    } else if let Some(rest) = trimmed.strip_prefix("#### ") {
        (4u8, rest)
    } else if let Some(rest) = trimmed.strip_prefix("### ") {
        (3u8, rest)
    } else if let Some(rest) = trimmed.strip_prefix("## ") {
        (2u8, rest)
    } else if let Some(rest) = trimmed.strip_prefix("# ") {
        (1u8, rest)
    } else {
        return None;
    };

    let heading_text = content.trim().to_string();
    if heading_text.is_empty() {
        return None;
    }

    Some((level, heading_text))
}
