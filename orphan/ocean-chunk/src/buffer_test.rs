use crate::ocean_chunk::buffer::ChunkBuffer;
use crate::ocean_chunk::types::{ChunkConfig, ChunkType};

#[test]
fn new_buffer_is_empty() {
    let buf = ChunkBuffer::new();
    assert!(buf.is_empty());
    assert_eq!(buf.char_count(), 0);
}

#[test]
fn append_adds_text() {
    let mut buf = ChunkBuffer::new();
    let config = ChunkConfig::default();
    buf.append("Hello", ChunkType::Text, &config);
    assert!(!buf.is_empty());
}

#[test]
fn flush_creates_chunk() {
    let mut buf = ChunkBuffer::new();
    let config = ChunkConfig::default();
    buf.append("Hello world.", ChunkType::Text, &config);
    buf.flush(Some("file-1"), &config);
    assert_eq!(buf.chunks.len(), 1);
    assert_eq!(buf.chunks[0].content, "Hello world.");
    assert_eq!(buf.chunks[0].file_id, "file-1");
}

#[test]
fn flush_empty_buffer_does_nothing() {
    let mut buf = ChunkBuffer::new();
    let config = ChunkConfig::default();
    buf.flush(Some("file-1"), &config);
    assert!(buf.chunks.is_empty());
}

#[test]
fn final_flush_emits_even_below_min() {
    let mut buf = ChunkBuffer::new();
    let config = ChunkConfig::default();
    buf.append("Hi", ChunkType::Text, &config);
    buf.final_flush("file-1", &config);
    assert_eq!(buf.chunks.len(), 1);
}

#[test]
fn emit_atomic_creates_chunk_without_clearing_buffer() {
    let mut buf = ChunkBuffer::new();
    buf.emit_atomic("table data".into(), ChunkType::Table, "file-1");
    assert_eq!(buf.chunks.len(), 1);
    assert_eq!(buf.chunks[0].block_type, ChunkType::Table);
}
