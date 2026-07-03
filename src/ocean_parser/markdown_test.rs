use crate::ocean_parser::markdown::MarkdownDocument;

#[test]
fn test_atx_heading_parsing() {
    assert_eq!(MarkdownDocument::atx_heading_level("# Heading 1"), Some(1));
    assert_eq!(MarkdownDocument::atx_heading_level("## Heading 2"), Some(2));
    assert_eq!(MarkdownDocument::atx_heading_level("### Heading 3"), Some(3));
    assert_eq!(MarkdownDocument::atx_heading_level("#Heading"), None); // No space
    assert_eq!(MarkdownDocument::atx_heading_level("####### Heading"), None); // Too many
}

#[test]
fn test_metadata_parsing() {
    let lines = vec![
        "---".to_string(),
        "title: My Document".to_string(),
        "author: John Doe".to_string(),
        "---".to_string(),
        "# Content".to_string(),
    ];

    let (metadata, start) = MarkdownDocument::parse_metadata(&lines);
    assert!(metadata.is_some());
    assert_eq!(start, 4);
}
