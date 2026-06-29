use crate::ocean_chunk::buffer::ChunkBuffer;
use crate::ocean_chunk::heading::detect_heading;
use crate::ocean_chunk::split::split_with_overlap;
use crate::ocean_chunk::types::{Chunk, ChunkConfig, ChunkError, ChunkType, estimate_tokens};
use crate::ocean_parser::*;

pub fn chunk(
    blocks: Vec<ReadResult>,
    file_id: &str,
    config: Option<ChunkConfig>,
) -> Result<Vec<Chunk>, ChunkError> {
    if blocks.is_empty() {
        return Err(ChunkError::EmptyInput);
    }

    let config = config.unwrap_or_default();

    if config.min_tokens > config.max_tokens {
        return Err(ChunkError::InvalidConfig(
            "min_tokens must be <= max_tokens".into(),
        ));
    }

    let mut buffer = ChunkBuffer::new();

    for block in &blocks {
        process_block(block, &mut buffer, file_id, &config);
    }

    buffer.final_flush(file_id, &config);

    post_process(&mut buffer.chunks, &config);

    Ok(buffer.chunks)
}

pub fn chunk_stream(
    blocks: impl IntoIterator<Item = ReadResult>,
    file_id: &str,
    config: Option<ChunkConfig>,
) -> Result<Vec<Chunk>, ChunkError> {
    let blocks: Vec<ReadResult> = blocks.into_iter().collect();
    chunk(blocks, file_id, config)
}

fn process_block(
    block: &ReadResult,
    buffer: &mut ChunkBuffer,
    file_id: &str,
    config: &ChunkConfig,
) {
    match block {
        ReadResult::Text(text) => process_text(text, buffer, file_id, config),
        ReadResult::Page { number, text } => process_page(*number, text, buffer, file_id, config),
        ReadResult::Slide { number, title, content } => {
            process_slide(*number, title, content, buffer, file_id, config)
        }
        ReadResult::Sheet { name, rows } => process_sheet(name, rows, buffer, file_id, config),
        ReadResult::Table { headers, rows } => process_table(headers, rows, buffer, file_id, config),
        ReadResult::CellValue(value) => {
            buffer.append(value, ChunkType::Cell, config);
        }
        ReadResult::Image { caption, .. } => process_image(caption, buffer, file_id, config),
        ReadResult::Metadata(_) | ReadResult::Outline(_) => {}
        ReadResult::MatchResult(matches) => {
            for m in matches {
                buffer.append(&m.text, ChunkType::Text, config);
            }
        }
    }
}

fn process_text(
    text: &str,
    buffer: &mut ChunkBuffer,
    file_id: &str,
    config: &ChunkConfig,
) {
    if text.trim().is_empty() {
        return;
    }

    if let Some((_level, heading_text)) = detect_heading(text) {
        buffer.flush(Some(file_id), config);

        buffer.emit_atomic(heading_text.clone(), ChunkType::Heading, file_id);

        buffer.set_heading(Some(heading_text));
        return;
    }

    buffer.append(text, ChunkType::Text, config);
}

fn process_page(
    number: u32,
    text: &str,
    buffer: &mut ChunkBuffer,
    file_id: &str,
    config: &ChunkConfig,
) {
    if text.trim().is_empty() {
        return;
    }

    buffer.set_page(Some(number));

    if let Some((_level, heading_text)) = detect_heading(text) {
        buffer.flush(Some(file_id), config);

        buffer.emit_atomic(heading_text.clone(), ChunkType::Heading, file_id);

        buffer.set_heading(Some(heading_text));
        return;
    }

    buffer.append(text, ChunkType::Page, config);
}

fn process_slide(
    number: u32,
    title: &Option<String>,
    content: &str,
    buffer: &mut ChunkBuffer,
    file_id: &str,
    config: &ChunkConfig,
) {
    buffer.flush(Some(file_id), config);
    buffer.set_slide(Some(number));

    if let Some(t) = title {
        buffer.set_heading(Some(t.clone()));
    }

    let content_heading = detect_heading(content);
    if let Some((_level, ref heading_text)) = content_heading {
        buffer.set_heading(Some(heading_text.clone()));
    }

    let text = if let Some(t) = title {
        if content_heading.is_some() {
            content.to_string()
        } else {
            format!("{}: {}", t, content)
        }
    } else {
        content.to_string()
    };

    let max_chars = config.max_tokens * 4;
    if text.len() > max_chars {
        let segments = split_with_overlap(&text, config);
        for segment in segments {
            buffer.emit_atomic(segment, ChunkType::Slide, file_id);
        }
    } else {
        buffer.emit_atomic(text, ChunkType::Slide, file_id);
    }
}

fn process_sheet(
    name: &str,
    rows: &[Vec<String>],
    buffer: &mut ChunkBuffer,
    file_id: &str,
    config: &ChunkConfig,
) {
    buffer.flush(Some(file_id), config);
    buffer.set_sheet(Some(name.to_string()));
    buffer.set_heading(Some(name.to_string()));

    if rows.is_empty() {
        buffer.emit_atomic(format!("[Sheet: {}]", name), ChunkType::Sheet, file_id);
        return;
    }

    for chunk_rows in rows.chunks(config.rows_per_sheet_chunk) {
        let mut content = String::new();
        for row in chunk_rows {
            content.push_str(&row.join("\t"));
            content.push('\n');
        }

        let max_chars = config.max_tokens * 4;
        if content.len() > max_chars {
            let segments = split_with_overlap(&content, config);
            for segment in segments {
                buffer.emit_atomic(segment, ChunkType::Sheet, file_id);
            }
        } else {
            buffer.emit_atomic(content, ChunkType::Sheet, file_id);
        }
    }
}

fn process_table(
    headers: &[String],
    rows: &[Vec<String>],
    buffer: &mut ChunkBuffer,
    file_id: &str,
    config: &ChunkConfig,
) {
    buffer.flush(Some(file_id), config);

    let mut content = String::new();
    if !headers.is_empty() {
        content.push_str(&headers.join("\t"));
        content.push('\n');
    }
    for row in rows {
        content.push_str(&row.join("\t"));
        content.push('\n');
    }

    let estimated = estimate_tokens(&content);
    let max_tokens = config.max_tokens;
    if estimated > max_tokens {
        // Tables are atomic — emit warning comment and still emit as single chunk
        let warning = format!(
            "[WARNING: table exceeds {} tokens (est. {})]\n{}",
            max_tokens, estimated, content
        );
        buffer.emit_atomic(warning, ChunkType::Table, file_id);
    } else {
        buffer.emit_atomic(content, ChunkType::Table, file_id);
    }
}

fn process_image(
    caption: &Option<String>,
    buffer: &mut ChunkBuffer,
    file_id: &str,
    config: &ChunkConfig,
) {
    if !config.include_images {
        return;
    }

    buffer.flush(Some(file_id), config);

    let content = match caption {
        Some(c) => format!("[Image: {}]", c),
        None => "[Image]".to_string(),
    };

    buffer.emit_atomic(content, ChunkType::Image, file_id);
}

fn post_process(chunks: &mut Vec<Chunk>, config: &ChunkConfig) {
    if chunks.is_empty() {
        return;
    }

    let mut i = 0;
    while i + 1 < chunks.len() {
        let can_merge = chunks[i].block_type == chunks[i + 1].block_type
            && chunks[i].heading == chunks[i + 1].heading
            && matches!(
                chunks[i].block_type,
                ChunkType::Text | ChunkType::Page | ChunkType::Cell
            );

        if can_merge {
            let t1 = estimate_tokens(&chunks[i].content);
            let t2 = estimate_tokens(&chunks[i + 1].content);

            if t1 + t2 <= config.max_tokens {
                let next = chunks.remove(i + 1);
                chunks[i].content.push(' ');
                chunks[i].content.push_str(&next.content);
                continue;
            }
        }
        i += 1;
    }
}
