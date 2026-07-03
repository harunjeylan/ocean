use std::path::Path;
use std::sync::atomic::AtomicU64;

use ocean_doc::ocean_parser::*;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_temp_file(content: &str, ext: &str) -> String {
    let count = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = format!(
        "{}\\test_{}_{}.{}",
        std::env::temp_dir().display(),
        std::process::id(),
        count,
        ext
    );
    std::fs::write(&path, content).expect("failed to write temp file");
    path
}

#[test]
fn test_open_txt() {
    let path = create_temp_file("Hello World", "txt");
    let doc = open(&path).expect("should open txt");
    assert_eq!(doc.metadata().format, DocumentFormat::Text);
}

#[test]
fn test_open_markdown() {
    let content = "# Title\n\nBody.";
    let path = create_temp_file(content, "md");
    let doc = open(&path).expect("should open md");
    assert_eq!(doc.metadata().format, DocumentFormat::Markdown);
}

#[test]
fn test_open_html() {
    let content = "<html><body><h1>Title</h1><p>Para</p></body></html>";
    let path = create_temp_file(content, "html");
    let doc = open(&path).expect("should open html");
    assert_eq!(doc.metadata().format, DocumentFormat::Html);

    let result = doc.read(&Selector::Slice { skip: 0, take: 10 }).expect("should read slice");
    match result {
        ReadResult::Text(t) => assert!(t.contains("Para")),
        _ => panic!("expected Text variant"),
    }
}

#[test]
fn test_read_txt_paragraph() {
    let path = create_temp_file("line1\nline2\nline3", "txt");
    let doc = open(&path).unwrap();

    let p0 = doc.read(&Selector::Slice { skip: 0, take: 1 }).unwrap();
    match p0 {
        ReadResult::Text(t) => assert!(t.starts_with("line1")),
        other => panic!("expected Text, got {:?}", other),
    }

    let p2 = doc.read(&Selector::Slice { skip: 2, take: 1 }).unwrap();
    match p2 {
        ReadResult::Text(t) => assert!(t.starts_with("line3")),
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn test_read_txt_range() {
    let path = create_temp_file("Hello World", "txt");
    let doc = open(&path).unwrap();
    let result = doc.read(&Selector::Range { start: 0, end: 5 }).unwrap();
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

    let intro = doc.read(&Selector::Slice { skip: 0, take: 10 }).unwrap();
    match intro {
        ReadResult::Text(t) => assert!(t.contains("Welcome")),
        _ => panic!("expected Text"),
    }

    let detail = doc.read(&Selector::Slice { skip: 4, take: 10 }).unwrap();
    match detail {
        ReadResult::Text(t) => assert!(t.contains("Nested")),
        _ => panic!("expected Text"),
    }
}

#[test]
fn test_read_api_functions() {
    let path = create_temp_file("paragraph0\nparagraph1", "txt");
    let doc = open(&path).unwrap();

    let r0 = read(&*doc, &Selector::Slice { skip: 0, take: 1 }).unwrap();
    let r1 = read(&*doc, &Selector::Slice { skip: 1, take: 1 }).unwrap();
    match r0 {
        ReadResult::Text(t) => assert!(t.starts_with("paragraph0")),
        other => panic!("expected Text, got {:?}", other),
    }
    match r1 {
        ReadResult::Text(t) => assert!(t.starts_with("paragraph1")),
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn test_html_table() {
    let content = "<html><body><p>before</p><table><tr><th>H1</th><th>H2</th></tr><tr><td>A</td><td>B</td></tr></table><p>after</p></body></html>";
    let path = create_temp_file(content, "html");
    let doc = open(&path).unwrap();

    let result = doc.read(&Selector::Slice { skip: 0, take: 10 }).unwrap();
    match result {
        ReadResult::Text(t) => assert!(t.contains("before") && t.contains("after")),
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn test_html_image() {
    let content = r#"<html><body><p>text</p><img src="pic.png" alt="Photo"></body></html>"#;
    let path = create_temp_file(content, "html");
    let doc = open(&path).unwrap();

    let result = doc.read(&Selector::Slice { skip: 0, take: 10 }).unwrap();
    match result {
        ReadResult::Text(_) => {}
        other => panic!("expected Text, got {:?}", other),
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

    let result = doc.read(&Selector::Slice { skip: 0, take: 10 }).unwrap();
    match result {
        ReadResult::Text(t) => {
            assert!(t.contains("Para one"));
            assert!(t.contains("Para two"));
        }
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn test_invalid_markdown_heading() {
    let path = create_temp_file("# Real", "md");
    let doc = open(&path).unwrap();
    let result = doc.read(&Selector::Page(99));
    assert!(matches!(result, Err(DocumentError::InvalidSelector(_))));
}





