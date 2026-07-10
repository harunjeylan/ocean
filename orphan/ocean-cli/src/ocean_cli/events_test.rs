use crate::ocean_cli::events::{ConsoleEmitter, EventEmitter, JsonEmitter, OutputTarget, SystemEvent, unix_millis};

#[test]
fn test_console_emit_does_not_panic() {
    let emitter = ConsoleEmitter;
    emitter.emit(SystemEvent::IndexStarted {
        timestamp: unix_millis(),
        dir: "/test".into(),
        total_files: 5,
    });
}

#[test]
fn test_json_emitter_stderr() {
    let emitter = JsonEmitter::new(OutputTarget::Stderr);
    emitter.emit(SystemEvent::IndexComplete {
        timestamp: unix_millis(),
        duration_ms: 100,
        indexed: 5,
        skipped: 1,
        failed: 0,
    });
}

#[test]
fn test_json_emitter_produces_valid_json() {
    let event = SystemEvent::QueryExecuted {
        timestamp: 1234567890,
        query: "test query".into(),
        mode: "hybrid".into(),
        num_results: 3,
        duration_ms: 50,
        cached: false,
    };
    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["event"], "QueryExecuted");
    assert_eq!(parsed["data"]["query"], "test query");
}

#[test]
fn test_json_emitter_output_target_switch() {
    let mut emitter = JsonEmitter::new(OutputTarget::Stderr);
    emitter.set_output(OutputTarget::Stderr);
}
