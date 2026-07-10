use uuid::Uuid;

use crate::ocean_chunk::types::{Chunk, ChunkConfig, ChunkType};

pub(crate) struct ChunkBuffer {
    pub content: String,
    pub heading: Option<String>,
    pub page: Option<u32>,
    pub slide: Option<u32>,
    pub sheet: Option<String>,
    pub block_type: ChunkType,
    pub chunks: Vec<Chunk>,
}

impl ChunkBuffer {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            heading: None,
            page: None,
            slide: None,
            sheet: None,
            block_type: ChunkType::Text,
            chunks: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    pub fn char_count(&self) -> usize {
        self.content.len()
    }

    pub fn estimated_tokens(&self, estimator: &fn(&str) -> usize) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        estimator(&self.content)
    }

    pub fn append(&mut self, text: &str, block_type: ChunkType, config: &ChunkConfig) {
        let mut text = text.to_string();

        if !text.ends_with(' ') && !text.ends_with('\n') {
            text.push(' ');
        }

        if self.content.is_empty() {
            self.block_type = block_type;
        } else if self.block_type != block_type && matches!(block_type, ChunkType::Cell) {
            // Cell values merge into text context, don't update block_type
        }

        self.content.push_str(&text);

        if self.estimated_tokens(&config.token_estimator) > config.max_tokens
            && self.char_count() > config.max_tokens * 4
        {
            self.flush(None, config);
        }
    }

    pub fn flush(&mut self, file_id: Option<&str>, config: &ChunkConfig) {
        let content = self.content.trim().to_string();
        if content.is_empty() {
            return;
        }

        let estimated = (config.token_estimator)(&content);

        if estimated < config.min_tokens && !self.chunks.is_empty() {
            if let Some(last) = self.chunks.last_mut() {
                if last.heading == self.heading && last.block_type == self.block_type {
                    last.content.push(' ');
                    last.content.push_str(&content);
                    last.end_offset = last.end_offset.map(|o| o + content.len() + 1);
                    self.clear();
                    return;
                }
            }
        }

        let chunk = Chunk {
            id: Uuid::now_v7().to_string(),
            file_id: file_id.unwrap_or("").to_string(),
            content,
            heading: self.heading.clone(),
            page: self.page,
            slide: self.slide,
            sheet: self.sheet.clone(),
            block_type: self.block_type.clone(),
            start_offset: None,
            end_offset: None,
        };

        self.chunks.push(chunk);
        self.clear();
    }

    pub fn final_flush(&mut self, file_id: &str, _config: &ChunkConfig) {
        let content = self.content.trim().to_string();
        if content.is_empty() {
            return;
        }

        let chunk = Chunk {
            id: Uuid::now_v7().to_string(),
            file_id: file_id.to_string(),
            content,
            heading: self.heading.clone(),
            page: self.page,
            slide: self.slide,
            sheet: self.sheet.clone(),
            block_type: self.block_type.clone(),
            start_offset: None,
            end_offset: None,
        };

        self.chunks.push(chunk);
        self.clear();
    }

    pub fn emit_atomic(
        &mut self,
        content: String,
        block_type: ChunkType,
        file_id: &str,
    ) {
        let chunk = Chunk {
            id: Uuid::now_v7().to_string(),
            file_id: file_id.to_string(),
            content,
            heading: self.heading.clone(),
            page: self.page,
            slide: self.slide,
            sheet: self.sheet.clone(),
            block_type,
            start_offset: None,
            end_offset: None,
        };

        self.chunks.push(chunk);
    }

    pub fn set_heading(&mut self, heading: Option<String>) {
        self.heading = heading;
    }

    pub fn set_page(&mut self, page: Option<u32>) {
        self.page = page;
    }

    pub fn set_slide(&mut self, slide: Option<u32>) {
        self.slide = slide;
    }

    pub fn set_sheet(&mut self, sheet: Option<String>) {
        self.sheet = sheet;
    }

    fn clear(&mut self) {
        self.content.clear();
        self.block_type = ChunkType::Text;
    }
}
