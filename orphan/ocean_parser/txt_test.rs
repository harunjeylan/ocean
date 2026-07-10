use crate::ocean_fs::parser::txt::TxtDocument;

#[test]
fn test_paragraph_detection() {
    let boundaries = vec![
        crate::ocean_fs::parser::txt::LineBoundary {
            line_num: 0,
            content: "hello".to_string(),
        },
        crate::ocean_fs::parser::txt::LineBoundary {
            line_num: 1,
            content: "world".to_string(),
        },
        crate::ocean_fs::parser::txt::LineBoundary {
            line_num: 2,
            content: "".to_string(),
        },
        crate::ocean_fs::parser::txt::LineBoundary {
            line_num: 3,
            content: "test2".to_string(),
        },
    ];

    let paragraphs = TxtDocument::detect_paragraphs(&boundaries);
    assert_eq!(paragraphs.len(), 2);
}

#[test]
fn test_stats_calculation() {
    let boundaries = vec![
        crate::ocean_fs::parser::txt::LineBoundary {
            line_num: 0,
            content: "hello".to_string(),
        },
        crate::ocean_fs::parser::txt::LineBoundary {
            line_num: 1,
            content: "world".to_string(),
        },
    ];

    let stats = TxtDocument::calculate_stats("hello\nworld", &boundaries);
    assert_eq!(stats.line_count, 2);
}
