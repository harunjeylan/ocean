use crate::ocean_chunk::types::ChunkConfig;

pub fn find_sentence_boundary(text: &str, limit: usize) -> Option<usize> {
    if text.len() <= limit {
        return None;
    }

    let search_end = limit.min(text.len());
    let search_start = (search_end / 2).max(1);

    let text_before = &text[..search_end];

    let mut best_boundary: Option<usize> = None;

    for (idx, _) in text_before.char_indices().rev() {
        if idx < search_start {
            break;
        }

        let remainder = &text_before[idx..];

        if let Some(next) = remainder.chars().next() {
            if next == '\n' {
                if idx > 0 && text_before[..idx].ends_with('\n') {
                    if let Some(boundary) = best_boundary {
                        if idx > search_end / 2 {
                            return Some(idx);
                        }
                        return Some(boundary);
                    }
                }
                best_boundary = Some(idx);
            } else if next == '.' || next == '!' || next == '?' {
                if idx + 1 < text_before.len() {
                    let after = text_before[idx + 1..].chars().next().unwrap_or(' ');
                    if after == ' ' || after == '\n' || after == '\r' || after == '\t' {
                        best_boundary = Some(idx + 1);
                    }
                }
            }
        }
    }

    if best_boundary.is_some() {
        return best_boundary;
    }

    let mid = search_end / 2;
    let midpoint = &text[..search_end];
    if let Some(pos) = midpoint[mid..].find(' ') {
        Some(mid + pos + 1)
    } else {
        Some(search_end)
    }
}

pub fn extract_last_sentences(text: &str, n: usize) -> String {
    if n == 0 || text.is_empty() {
        return String::new();
    }

    let sentences: Vec<&str> = text
        .split_inclusive(|c| c == '.' || c == '!' || c == '?' || c == '\n')
        .filter(|s| !s.trim().is_empty())
        .collect();

    let count = sentences.len();
    if count == 0 {
        return String::new();
    }

    let take = n.min(count);
    let result: String = sentences[count - take..].concat();
    result.trim_start().to_string()
}

pub fn split_with_overlap(text: &str, config: &ChunkConfig) -> Vec<String> {
    let max_chars = config.max_tokens * 4;

    if text.len() <= max_chars {
        return vec![text.to_string()];
    }

    let mut segments = Vec::new();
    let mut remaining = text;
    let mut prev_segment: Option<String> = None;

    while !remaining.is_empty() {
        if remaining.len() <= max_chars {
            let segment = remaining.to_string();
            if let Some(ref prev) = prev_segment {
                let overlap = extract_last_sentences(prev, config.overlap_sentences);
                segments.push(format!("{}{}", overlap, segment));
            } else {
                segments.push(segment);
            }
            break;
        }

        let boundary = find_sentence_boundary(remaining, max_chars);
        let split_at = boundary.unwrap_or(max_chars);

        let segment = &remaining[..split_at];

        if let Some(ref prev) = prev_segment {
            let overlap = extract_last_sentences(prev, config.overlap_sentences);
            segments.push(format!("{}{}", overlap, segment));
        } else {
            segments.push(segment.to_string());
        }

        prev_segment = Some(segment.to_string());
        remaining = &remaining[split_at..];
    }

    segments
}
