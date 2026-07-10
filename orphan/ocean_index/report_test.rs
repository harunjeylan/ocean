use crate::ocean_index::report::*;

#[test]
fn file_result_indexed() {
    let fr = FileResult {
        path: "/tmp/test.txt".into(),
        status: FileIndexStatus::Indexed,
        chunks: 5,
        embedded: 3,
        embed_skipped: 2,
        embed_failed: 0,
        nodes: 10,
        edges: 3,
        duration_ms: 100,
        error: None,
    };
    assert!(matches!(fr.status, FileIndexStatus::Indexed));
    assert_eq!(fr.chunks, 5);
    assert_eq!(fr.duration_ms, 100);
}

#[test]
fn file_result_skipped() {
    let fr = FileResult {
        path: "/tmp/skip.txt".into(),
        status: FileIndexStatus::Skipped,
        chunks: 0,
        embedded: 0,
        embed_skipped: 0,
        embed_failed: 0,
        nodes: 0,
        edges: 0,
        duration_ms: 0,
        error: None,
    };
    assert!(matches!(fr.status, FileIndexStatus::Skipped));
}

#[test]
fn file_result_failed() {
    let fr = FileResult {
        path: "/tmp/fail.txt".into(),
        status: FileIndexStatus::Failed,
        chunks: 0,
        embedded: 0,
        embed_skipped: 0,
        embed_failed: 0,
        nodes: 0,
        edges: 0,
        duration_ms: 0,
        error: Some("something broke".into()),
    };
    assert!(matches!(fr.status, FileIndexStatus::Failed));
    assert_eq!(fr.error, Some("something broke".into()));
}

#[test]
fn index_report_new() {
    let r = IndexReport::new();
    assert_eq!(r.total_files, 0);
    assert_eq!(r.indexed, 0);
    assert_eq!(r.skipped, 0);
    assert_eq!(r.failed, 0);
    assert_eq!(r.total_chunks, 0);
    assert_eq!(r.total_edges, 0);
    assert_eq!(r.total_nodes, 0);
    assert_eq!(r.duration_ms, 0);
    assert!(r.per_file.is_empty());
}

#[test]
fn index_report_merge_indexed() {
    let mut r = IndexReport::new();
    r.merge(FileResult {
        path: "a.txt".into(),
        status: FileIndexStatus::Indexed,
        chunks: 5,
        embedded: 3,
        embed_skipped: 2,
        embed_failed: 0,
        nodes: 10,
        edges: 3,
        duration_ms: 100,
        error: None,
    });
    assert_eq!(r.total_files, 1);
    assert_eq!(r.indexed, 1);
    assert_eq!(r.skipped, 0);
    assert_eq!(r.failed, 0);
    assert_eq!(r.total_chunks, 5);
    assert_eq!(r.total_nodes, 10);
    assert_eq!(r.total_edges, 3);
    assert_eq!(r.per_file.len(), 1);
}

#[test]
fn index_report_merge_skipped() {
    let mut r = IndexReport::new();
    r.merge(FileResult {
        path: "b.txt".into(),
        status: FileIndexStatus::Skipped,
        chunks: 0,
        embedded: 0,
        embed_skipped: 0,
        embed_failed: 0,
        nodes: 0,
        edges: 0,
        duration_ms: 0,
        error: None,
    });
    assert_eq!(r.total_files, 1);
    assert_eq!(r.indexed, 0);
    assert_eq!(r.skipped, 1);
    assert_eq!(r.failed, 0);
}

#[test]
fn index_report_merge_failed() {
    let mut r = IndexReport::new();
    r.merge(FileResult {
        path: "c.txt".into(),
        status: FileIndexStatus::Failed,
        chunks: 0,
        embedded: 0,
        embed_skipped: 0,
        embed_failed: 0,
        nodes: 0,
        edges: 0,
        duration_ms: 0,
        error: Some("err".into()),
    });
    assert_eq!(r.total_files, 1);
    assert_eq!(r.indexed, 0);
    assert_eq!(r.skipped, 0);
    assert_eq!(r.failed, 1);
}

#[test]
fn index_report_aggregation_math() {
    let mut r = IndexReport::new();
    r.merge(FileResult { path: "a.txt".into(), status: FileIndexStatus::Indexed, chunks: 3, embedded: 3, embed_skipped: 0, embed_failed: 0, nodes: 5, edges: 1, duration_ms: 50, error: None });
    r.merge(FileResult { path: "b.txt".into(), status: FileIndexStatus::Skipped, chunks: 0, embedded: 0, embed_skipped: 0, embed_failed: 0, nodes: 0, edges: 0, duration_ms: 0, error: None });
    r.merge(FileResult { path: "c.txt".into(), status: FileIndexStatus::Failed, chunks: 0, embedded: 0, embed_skipped: 0, embed_failed: 0, nodes: 0, edges: 0, duration_ms: 0, error: Some("err".into()) });
    r.merge(FileResult { path: "d.txt".into(), status: FileIndexStatus::Indexed, chunks: 7, embedded: 5, embed_skipped: 2, embed_failed: 0, nodes: 8, edges: 2, duration_ms: 150, error: None });
    assert_eq!(r.total_files, 4);
    assert_eq!(r.indexed, 2);
    assert_eq!(r.skipped, 1);
    assert_eq!(r.failed, 1);
    assert_eq!(r.total_files, r.indexed + r.skipped + r.failed);
    assert_eq!(r.total_chunks, 10);
    assert_eq!(r.total_nodes, 13);
    assert_eq!(r.total_edges, 3);
}

#[test]
fn index_report_default() {
    let r = IndexReport::default();
    assert_eq!(r.total_files, 0);
}
