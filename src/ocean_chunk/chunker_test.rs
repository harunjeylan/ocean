use crate::ocean_chunk::chunker::chunk;
use crate::ocean_chunk::types::{ChunkConfig, ChunkError, ChunkType};
use crate::ocean_parser::*;

fn make_text(text: &str) -> ReadResult {
    ReadResult::Text(text.to_string())
}

fn make_table(headers: &[&str], rows: &[&[&str]]) -> ReadResult {
    ReadResult::Table {
        headers: headers.iter().map(|s| s.to_string()).collect(),
        rows: rows
            .iter()
            .map(|r| r.iter().map(|s| s.to_string()).collect())
            .collect(),
    }
}

fn make_slide(number: u32, title: &str, content: &str) -> ReadResult {
    ReadResult::Slide {
        number,
        title: if title.is_empty() {
            None
        } else {
            Some(title.to_string())
        },
        content: content.to_string(),
    }
}

fn make_sheet(name: &str, rows: &[&[&str]]) -> ReadResult {
    ReadResult::Sheet {
        name: name.to_string(),
        rows: rows
            .iter()
            .map(|r| r.iter().map(|s| s.to_string()).collect())
            .collect(),
    }
}

#[test]
fn empty_input_returns_error() {
    let result = chunk(vec![], "file-1", None);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ChunkError::EmptyInput));
}

#[test]
fn invalid_config_returns_error() {
    let config = ChunkConfig {
        min_tokens: 500,
        max_tokens: 100,
        ..Default::default()
    };
    let result = chunk(
        vec![make_text("Hello.")],
        "file-1",
        Some(config),
    );
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ChunkError::InvalidConfig(_)));
}

#[test]
fn simple_text_chunk() {
    let blocks = vec![make_text("Hello world.")];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].content, "Hello world.");
    assert_eq!(result[0].file_id, "file-1");
    assert_eq!(result[0].block_type, ChunkType::Text);
}

#[test]
fn heading_detected_and_chunked() {
    let blocks = vec![
        make_text("# Section 1"),
        make_text("Content under section 1."),
        make_text("## Subsection"),
        make_text("Content under subsection."),
    ];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 4);
    assert_eq!(result[0].block_type, ChunkType::Heading);
    assert_eq!(result[0].content, "Section 1");
    assert_eq!(result[1].heading, Some("Section 1".into()));
    assert_eq!(result[2].block_type, ChunkType::Heading);
    assert_eq!(result[2].content, "Subsection");
    assert_eq!(result[2].heading, Some("Section 1".into()));
}

#[test]
fn table_is_atomic() {
    let blocks = vec![make_table(
        &["Name", "Value"],
        &[&["A", "1"], &["B", "2"]],
    )];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].block_type, ChunkType::Table);
    assert!(result[0].content.contains("Name"));
    assert!(result[0].content.contains("A"));
}

#[test]
fn slide_is_atomic() {
    let blocks = vec![make_slide(1, "Intro", "Welcome to the presentation.")];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].block_type, ChunkType::Slide);
    assert_eq!(result[0].slide, Some(1));
    assert_eq!(result[0].heading, Some("Intro".into()));
}

#[test]
fn sheet_is_chunked() {
    let blocks = vec![make_sheet("Sheet1", &[&["A", "1"], &["B", "2"]])];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].block_type, ChunkType::Sheet);
    assert_eq!(result[0].sheet, Some("Sheet1".into()));
    assert!(result[0].content.contains("A\t1"));
}

#[test]
fn images_skipped_by_default() {
    let blocks = vec![
        ReadResult::Image {
            bytes: vec![],
            format: ImageFormat::Png,
            caption: Some("photo".into()),
        },
        make_text("Some text."),
    ];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].block_type, ChunkType::Text);
}

#[test]
fn images_included_when_configured() {
    let config = ChunkConfig {
        include_images: true,
        ..Default::default()
    };
    let blocks = vec![
        ReadResult::Image {
            bytes: vec![],
            format: ImageFormat::Png,
            caption: Some("photo".into()),
        },
    ];
    let result = chunk(blocks, "file-1", Some(config)).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].block_type, ChunkType::Image);
    assert!(result[0].content.contains("photo"));
}

#[test]
fn adjacent_text_merged_under_same_heading() {
    let blocks = vec![
        make_text("# Title"),
        make_text("Paragraph one."),
        make_text("Paragraph two."),
    ];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].block_type, ChunkType::Heading);
    assert_eq!(result[1].block_type, ChunkType::Text);
    assert!(result[1].content.contains("Paragraph one."));
    assert!(result[1].content.contains("Paragraph two."));
}

#[test]
fn metadata_skipped() {
    let meta = DocumentMetadata {
        path: std::path::PathBuf::from("test.txt"),
        format: DocumentFormat::Text,
        title: Some("Test".into()),
        author: None,
        created: None,
        modified: None,
        page_count: None,
        size: 100,
    };
    let blocks = vec![ReadResult::Metadata(meta), make_text("Hello.")];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn cell_values_merged_into_text() {
    let blocks = vec![
        make_text("Data:"),
        ReadResult::CellValue("42".into()),
        make_text("is the answer."),
    ];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 1);
    assert!(result[0].content.contains("42"));
}

#[test]
fn determinism() {
    let blocks = vec![
        make_text("# Section"),
        make_text("Content."),
        make_table(&["A"], &[&["1"]]),
    ];
    let r1 = chunk(blocks.clone(), "file-1", None).unwrap();
    let r2 = chunk(blocks, "file-1", None).unwrap();

    assert_eq!(r1.len(), r2.len());
    for (c1, c2) in r1.iter().zip(r2.iter()) {
        assert_eq!(c1.content, c2.content);
        assert_eq!(c1.block_type, c2.block_type);
        assert_eq!(c1.heading, c2.heading);
    }
}

#[test]
fn sheet_split_by_row_groups() {
    let rows: Vec<Vec<String>> = (0..120)
        .map(|i| vec![format!("val{}", i)])
        .collect();
    let config = ChunkConfig {
        rows_per_sheet_chunk: 50,
        ..Default::default()
    };
    let blocks = vec![ReadResult::Sheet {
        name: "BigSheet".into(),
        rows,
    }];
    let result = chunk(blocks, "file-1", Some(config)).unwrap();
    assert_eq!(result.len(), 3);
    for c in &result {
        assert_eq!(c.block_type, ChunkType::Sheet);
    }
}

#[test]
fn slide_with_long_content_split() {
    let content = "Sentence. ".repeat(500);
    let blocks = vec![make_slide(1, "Long Slide", &content)];
    let config = ChunkConfig {
        max_tokens: 100,
        ..Default::default()
    };
    let result = chunk(blocks, "file-1", Some(config)).unwrap();
    assert!(result.len() >= 2);
    for c in &result {
        assert_eq!(c.block_type, ChunkType::Slide);
    }
}

#[test]
fn post_process_merges_small_chunks() {
    let blocks = vec![
        make_text("Short A."),
        make_text("Short B."),
    ];
    let config = ChunkConfig {
        min_tokens: 100,
        max_tokens: 800,
        ..Default::default()
    };
    let result = chunk(blocks, "file-1", Some(config)).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn empty_text_blocks_skipped() {
    let blocks = vec![
        make_text(""),
        make_text("  "),
        make_text("Real content."),
    ];
    let result = chunk(blocks, "file-1", None).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].content, "Real content.");
}
