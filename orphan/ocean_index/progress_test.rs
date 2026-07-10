use crate::ocean_index::progress::*;
use crate::ocean_index::report::IndexReport;

#[test]
fn silent_reporter_discards_events() {
    let reporter = SilentReporter;
    reporter.report(ProgressEvent::ScanStarted { dir: "/tmp".into(), total: 5 });
    reporter.report(ProgressEvent::FileProcessing { path: "a.txt".into(), current: 1, total: 5 });
    reporter.report(ProgressEvent::FileComplete { path: "a.txt".into(), chunks: 3, embedded: 2, embed_skipped: 1, embed_failed: 0, edges: 1, nodes: 5, duration_ms: 50 });
    reporter.report(ProgressEvent::FileSkipped { path: "b.txt".into() });
    reporter.report(ProgressEvent::FileFailed { path: "c.txt".into(), error: "err".into() });
    let report = IndexReport::new();
    reporter.report(ProgressEvent::IndexComplete(report));
}

#[test]
fn progress_event_scan_started() {
    let event = ProgressEvent::ScanStarted { dir: "/tmp".into(), total: 42 };
    match event {
        ProgressEvent::ScanStarted { dir, total } => {
            assert_eq!(dir, "/tmp");
            assert_eq!(total, 42);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn progress_event_file_processing() {
    let event = ProgressEvent::FileProcessing { path: "test.txt".into(), current: 1, total: 10 };
    match event {
        ProgressEvent::FileProcessing { path, current, total } => {
            assert_eq!(path, "test.txt");
            assert_eq!(current, 1);
            assert_eq!(total, 10);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn progress_event_file_complete() {
    let event = ProgressEvent::FileComplete {
        path: "test.txt".into(),
        chunks: 5,
        embedded: 3,
        embed_skipped: 2,
        embed_failed: 0,
        edges: 2,
        nodes: 8,
        duration_ms: 100,
    };
    match event {
        ProgressEvent::FileComplete { path, chunks, embedded, embed_skipped, embed_failed, .. } => {
            assert_eq!(path, "test.txt");
            assert_eq!(chunks, 5);
            assert_eq!(embedded, 3);
            assert_eq!(embed_skipped, 2);
            assert_eq!(embed_failed, 0);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn progress_event_file_skipped() {
    let event = ProgressEvent::FileSkipped { path: "skip.txt".into() };
    match event {
        ProgressEvent::FileSkipped { path } => assert_eq!(path, "skip.txt"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn progress_event_file_failed() {
    let event = ProgressEvent::FileFailed { path: "fail.txt".into(), error: "error msg".into() };
    match event {
        ProgressEvent::FileFailed { path, error } => {
            assert_eq!(path, "fail.txt");
            assert_eq!(error, "error msg");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn progress_event_graph_progress() {
    let event = ProgressEvent::GraphProgress { total_nodes: 42, total_edges: 7 };
    match event {
        ProgressEvent::GraphProgress { total_nodes, total_edges } => {
            assert_eq!(total_nodes, 42);
            assert_eq!(total_edges, 7);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn progress_event_index_complete() {
    let report = IndexReport::new();
    let event = ProgressEvent::IndexComplete(report.clone());
    match event {
        ProgressEvent::IndexComplete(r) => assert_eq!(r.total_files, 0),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn progress_event_backpressure_paused() {
    let event = ProgressEvent::BackpressurePaused { queue_len: 50, available_ai: 1, in_flight: 5 };
    match event {
        ProgressEvent::BackpressurePaused { queue_len, available_ai, in_flight } => {
            assert_eq!(queue_len, 50);
            assert_eq!(available_ai, 1);
            assert_eq!(in_flight, 5);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn progress_event_backpressure_resumed() {
    let event = ProgressEvent::BackpressureResumed;
    match event {
        ProgressEvent::BackpressureResumed => {}
        _ => panic!("wrong variant"),
    }
}

#[test]
fn progress_event_retrying() {
    let event = ProgressEvent::Retrying {
        path: "test.txt".into(),
        attempt: 1,
        max_retries: 3,
        delay_ms: 200,
        error: "timeout".into(),
    };
    match event {
        ProgressEvent::Retrying { path, attempt, max_retries, delay_ms, error } => {
            assert_eq!(path, "test.txt");
            assert_eq!(attempt, 1);
            assert_eq!(max_retries, 3);
            assert_eq!(delay_ms, 200);
            assert_eq!(error, "timeout");
        }
        _ => panic!("wrong variant"),
    }
}
