use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use ocean::ocean_parser::*;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_temp_file(content: &str, ext: &str) -> String {
    let dir = Path::new("tests").join("test-cwd");
    std::fs::create_dir_all(&dir).unwrap();
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = dir.join(format!("test_{}.{}", n, ext));
    std::fs::write(&path, content).unwrap();
    path.to_string_lossy().to_string()
}

#[test]
fn test_open_txt() {
    let path = create_temp_file("Hello, world!\nLine two.", "txt");
    let doc = open(&path).expect("should open txt");
    let meta = doc.metadata();
    assert_eq!(meta.format, DocumentFormat::Text);
    assert!(meta.size > 0);
    assert!(meta.path.to_string_lossy().ends_with(".txt"));
}

#[test]
fn test_open_markdown() {
    let content = "# Title\n\nSome content.\n\n## Subtitle\n\nMore content.";
    let path = create_temp_file(content, "md");
    let doc = open(&path).expect("should open md");
    assert_eq!(doc.metadata().format, DocumentFormat::Markdown);

    let outline = doc.outline();
    assert_eq!(outline.entries.len(), 1);
    assert_eq!(outline.entries[0].label, "Title");
    assert_eq!(outline.entries[0].level, 1);
    assert_eq!(outline.entries[0].children.len(), 1);
    assert_eq!(outline.entries[0].children[0].label, "Subtitle");
    assert_eq!(outline.entries[0].children[0].level, 2);
}

#[test]
fn test_open_html() {
    let content = "<html><body><h1>Title</h1><p>Para</p></body></html>";
    let path = create_temp_file(content, "html");
    let doc = open(&path).expect("should open html");
    assert_eq!(doc.metadata().format, DocumentFormat::Html);

    let result = read_heading(&*doc, "Title").expect("should read heading");
    match result {
        ReadResult::Text(t) => assert!(t.contains("Para")),
        _ => panic!("expected Text variant"),
    }
}

#[test]
fn test_read_txt_paragraph() {
    let path = create_temp_file("line1\nline2\nline3", "txt");
    let doc = open(&path).unwrap();

    let p0 = read_paragraph(&*doc, 0).unwrap();
    assert_eq!(p0, ReadResult::Text("line1".to_string()));

    let p2 = read_paragraph(&*doc, 2).unwrap();
    assert_eq!(p2, ReadResult::Text("line3".to_string()));
}

#[test]
fn test_read_txt_range() {
    let path = create_temp_file("Hello World", "txt");
    let doc = open(&path).unwrap();
    let result = read_range(&*doc, 0, 5).unwrap();
    assert_eq!(result, ReadResult::Text("Hello".to_string()));
}

#[test]
fn test_search_txt() {
    let path = create_temp_file("apple\nbanana\ncherry\napple pie", "txt");
    let doc = open(&path).unwrap();
    let matches = doc.search("apple");
    assert_eq!(matches.len(), 2);
}

#[test]
fn test_unsupported_format() {
    let path = create_temp_file("test", "xyz");
    let result = open(&path);
    assert!(result.is_err());
    match result {
        Err(DocumentError::UnsupportedFormat(_)) => {}
        _ => panic!("expected UnsupportedFormat"),
    }
}

#[test]
fn test_invalid_selector_txt() {
    let path = create_temp_file("hello", "txt");
    let doc = open(&path).unwrap();
    let result = doc.read(&Selector::Slide(1));
    assert!(result.is_err());
    match result {
        Err(DocumentError::InvalidSelector(_)) => {}
        other => panic!("expected InvalidSelector, got {:?}", other),
    }
}

#[test]
fn test_markdown_heading_read() {
    let content = "# Intro\n\nWelcome text.\n\n# Main\n\nBody text.\n\n## Detail\n\nNested.";
    let path = create_temp_file(content, "md");
    let doc = open(&path).unwrap();

    let intro = read_heading(&*doc, "Intro").unwrap();
    match intro {
        ReadResult::Text(t) => assert!(t.contains("Welcome")),
        _ => panic!("expected Text"),
    }

    let detail = read_heading(&*doc, "Detail").unwrap();
    match detail {
        ReadResult::Text(t) => assert!(t.contains("Nested")),
        _ => panic!("expected Text"),
    }
}

#[test]
fn test_markdown_outline_hierarchy() {
    let content = "# A\n\n## B\n\n### C\n\n# D\n\n## E";
    let path = create_temp_file(content, "md");
    let doc = open(&path).unwrap();
    let outline = doc.outline();

    assert_eq!(outline.entries.len(), 2);

    assert_eq!(outline.entries[0].label, "A");
    assert_eq!(outline.entries[0].children.len(), 1);
    assert_eq!(outline.entries[0].children[0].label, "B");
    assert_eq!(outline.entries[0].children[0].children.len(), 1);
    assert_eq!(outline.entries[0].children[0].children[0].label, "C");

    assert_eq!(outline.entries[1].label, "D");
    assert_eq!(outline.entries[1].children.len(), 1);
    assert_eq!(outline.entries[1].children[0].label, "E");
}

#[test]
fn test_read_api_functions() {
    let path = create_temp_file("paragraph0\nparagraph1", "txt");
    let doc = open(&path).unwrap();

    let r0 = read(&*doc, &Selector::Paragraph(0)).unwrap();
    let r1 = read(&*doc, &Selector::Paragraph(1)).unwrap();
    assert_eq!(r0, ReadResult::Text("paragraph0".to_string()));
    assert_eq!(r1, ReadResult::Text("paragraph1".to_string()));
}

#[test]
fn test_html_table() {
    let content = "<html><body><table><tr><th>H1</th><th>H2</th></tr><tr><td>A</td><td>B</td></tr></table></body></html>";
    let path = create_temp_file(content, "html");
    let doc = open(&path).unwrap();

    let result = read_table(&*doc, 0).unwrap();
    match result {
        ReadResult::Table { headers, rows } => {
            assert_eq!(headers, vec!["H1".to_string(), "H2".to_string()]);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0], vec!["A".to_string(), "B".to_string()]);
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn test_html_image() {
    let content = r#"<html><body><img src="pic.png" alt="Photo"></body></html>"#;
    let path = create_temp_file(content, "html");
    let doc = open(&path).unwrap();

    let result = read_image(&*doc, 0).unwrap();
    match result {
        ReadResult::Image { caption, .. } => {
            assert_eq!(caption, Some("Photo".to_string()));
        }
        _ => panic!("expected Image"),
    }
}

#[test]
fn test_document_metadata() {
    let path = create_temp_file("test content", "txt");
    let doc = open(&path).unwrap();
    let meta = doc.metadata();
    assert_eq!(meta.format, DocumentFormat::Text);
    assert!(meta.size > 0);
    assert!(Path::new(&meta.path).exists());
}

#[test]
fn test_outline_empty_txt() {
    let path = create_temp_file("no headings here", "txt");
    let doc = open(&path).unwrap();
    let outline = doc.outline();
    assert!(outline.entries.is_empty());
}

#[test]
fn test_page_count_none_txt() {
    let path = create_temp_file("content", "txt");
    let doc = open(&path).unwrap();
    assert_eq!(doc.page_count(), None);
}

#[test]
fn test_missing_file() {
    let result = open("C:\\nonexistent_file_12345.xyz");
    assert!(result.is_err());
}

#[test]
fn test_search_case_insensitive() {
    let path = create_temp_file("Hello World\nhello there\nHELLO AGAIN", "txt");
    let doc = open(&path).unwrap();
    let matches = doc.search("hello");
    assert_eq!(matches.len(), 3);
}

#[test]
fn test_read_markdown_paragraph() {
    let content = "# Heading\n\nPara one.\n\nPara two.";
    let path = create_temp_file(content, "md");
    let doc = open(&path).unwrap();

    let p0 = read_paragraph(&*doc, 0).unwrap();
    assert_eq!(p0, ReadResult::Text("Para one.".to_string()));

    let p1 = read_paragraph(&*doc, 1).unwrap();
    assert_eq!(p1, ReadResult::Text("Para two.".to_string()));
}

#[test]
fn test_invalid_markdown_heading() {
    let path = create_temp_file("# Real", "md");
    let doc = open(&path).unwrap();
    let result = read_heading(&*doc, "Fake");
    assert!(matches!(result, Err(DocumentError::InvalidSelector(_))));
}

#[test]
fn test_html_heading_outline() {
    let content = "<html><body><h1>Top</h1><h2>Sub</h2><h1>Second</h1></body></html>";
    let path = create_temp_file(content, "html");
    let doc = open(&path).unwrap();
    let outline = doc.outline();
    assert_eq!(outline.entries.len(), 2);
    assert_eq!(outline.entries[0].label, "Top");
    assert_eq!(outline.entries[0].children.len(), 1);
    assert_eq!(outline.entries[0].children[0].label, "Sub");
    assert_eq!(outline.entries[1].label, "Second");
}
