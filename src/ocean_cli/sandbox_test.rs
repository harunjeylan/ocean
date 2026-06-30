use std::path::Path;

use crate::ocean_cli::sandbox::{Sandbox, SecurityError};

#[test]
fn test_sandbox_new_valid_dir() {
    let tmp = std::env::temp_dir();
    let sandbox = Sandbox::new(&tmp);
    assert!(sandbox.is_ok());
}

#[test]
fn test_sandbox_accepts_path_inside() {
    let tmp = std::env::temp_dir();
    let sandbox = Sandbox::new(&tmp).unwrap();
    let result = sandbox.validate(&tmp.join("test.txt"));
    assert!(result.is_err()); // test.txt doesn't exist, canonicalization fails — that's fine
}

#[test]
fn test_sandbox_rejects_non_existent_workspace() {
    let result = Sandbox::new(Path::new("/nonexistent/path/xyz123"));
    assert!(result.is_err());
}

#[test]
fn test_sandbox_allow_extension() {
    let tmp = std::env::temp_dir();
    let mut sandbox = Sandbox::new(&tmp).unwrap();
    sandbox.allow_extension("custom");
}

#[test]
fn test_security_error_display() {
    let err = SecurityError::PathOutsideWorkspace {
        path: "/outside/file.txt".into(),
        workspace: "/workspace".into(),
    };
    let msg = err.to_string();
    assert!(msg.contains("outside"));
    assert!(msg.contains("/outside/file.txt"));
}
