use crate::ocean_vector::search::*;

#[test]
fn test_rrf_fusion() {
    let make_result = |id: &str, vs: Option<f32>, fs: Option<f32>| SearchResult {
        chunk_id: id.into(),
        file_id: "f".into(),
        content: "".into(),
        heading: None,
        score: vs.or(fs).unwrap_or(0.0),
        block_type: "Text".into(),
        vector_score: vs,
        fts_score: fs,
    };

    let vector_results = vec![
        make_result("a", Some(0.9), None),
        make_result("b", Some(0.8), None),
        make_result("c", Some(0.7), None),
    ];

    let fts_results = vec![
        make_result("b", None, Some(0.95)),
        make_result("d", None, Some(0.85)),
    ];

    let fused = fuse_rrf(vector_results, fts_results, 60.0, 10);
    assert_eq!(fused.len(), 4);

    assert_eq!(fused[0].chunk_id, "b");

    for r in &fused {
        assert!(r.score > 0.0);
    }
}

#[test]
fn test_rrf_top_k_truncation() {
    let make_result = |id: &str, vs: Option<f32>| SearchResult {
        chunk_id: id.into(),
        file_id: "f".into(),
        content: "".into(),
        heading: None,
        score: vs.unwrap_or(0.0),
        block_type: "Text".into(),
        vector_score: vs,
        fts_score: None,
    };

    let vector_results: Vec<SearchResult> = (0..10)
        .map(|i| make_result(&format!("r{}", i), Some(1.0 - i as f32 * 0.1)))
        .collect();

    let fused = fuse_rrf(vector_results, vec![], 60.0, 3);
    assert_eq!(fused.len(), 3);
    assert_eq!(fused[0].chunk_id, "r0");
}

#[test]
fn test_search_filter_default() {
    let filter = SearchFilter::new();
    assert!(filter.build_where_clause().is_none());
}

#[test]
fn test_search_filter_with_file_id() {
    let filter = SearchFilter::new().with_file_id("abc-123");
    assert_eq!(filter.file_id, Some("abc-123".into()));
}

#[test]
fn test_search_filter_with_heading() {
    let filter = SearchFilter::new().with_heading("Chapter 1");
    assert_eq!(filter.heading_prefix, Some("Chapter 1".into()));
}

#[test]
fn test_search_filter_with_block_type() {
    let filter = SearchFilter::new().with_block_type("Table");
    assert_eq!(filter.block_type, Some("Table".into()));
}

#[test]
fn test_parse_search_results_empty() {
    let rows = vec![];
    let results = parse_search_results_raw(&rows, true, false).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_search_error_display() {
    let e = SearchError::NoResults("nothing found".into());
    assert_eq!(format!("{}", e), "no results: nothing found");
}

#[test]
fn test_search_filter_where_clause_file_id() {
    let f = SearchFilter::new().with_file_id("abc");
    let clause = f.build_where_clause().unwrap();
    assert_eq!(clause, "file_id = 'abc'");
}
