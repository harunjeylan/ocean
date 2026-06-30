use crate::ocean_fs::path_resolver::PathResolver;
use tempfile::tempdir;

#[test]
fn test_record_and_resolve_move() {
    let resolver = PathResolver::in_memory().unwrap();
    let file_id = "test-id-123";

    resolver
        .record_move(file_id, "/old/path/file.txt", "/new/path/file.txt")
        .unwrap();

    let resolved = resolver.resolve_path(file_id);
    assert_eq!(resolved, Some("/new/path/file.txt".to_string()));
}

#[test]
fn test_resolve_unknown_id() {
    let resolver = PathResolver::in_memory().unwrap();
    let resolved = resolver.resolve_path("nonexistent-id");
    assert_eq!(resolved, None);
}

#[test]
fn test_move_history_single() {
    let resolver = PathResolver::in_memory().unwrap();
    let file_id = "test-id-456";

    resolver
        .record_move(file_id, "/old/path/a.txt", "/new/path/a.txt")
        .unwrap();

    let history = resolver.get_move_history(file_id);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].old_path, "/old/path/a.txt");
    assert_eq!(history[0].new_path, "/new/path/a.txt");
}

#[test]
fn test_move_history_chain() {
    let resolver = PathResolver::in_memory().unwrap();
    let file_id = "chain-id-789";

    resolver
        .record_move(file_id, "/a/b.txt", "/c/b.txt")
        .unwrap();
    resolver
        .record_move(file_id, "/c/b.txt", "/d/b.txt")
        .unwrap();
    resolver
        .record_move(file_id, "/d/b.txt", "/e/b.txt")
        .unwrap();

    let history = resolver.get_move_history(file_id);
    assert_eq!(history.len(), 3);

    let resolved = resolver.resolve_path(file_id);
    assert_eq!(resolved, Some("/e/b.txt".to_string()));
}

#[test]
fn test_resolve_with_multiple_moves() {
    let resolver = PathResolver::in_memory().unwrap();
    let file_id = "multi-id";

    resolver
        .record_move(file_id, "/v1/file.txt", "/v2/file.txt")
        .unwrap();
    resolver
        .record_move(file_id, "/v2/file.txt", "/v3/file.txt")
        .unwrap();

    let resolved = resolver.resolve_path(file_id);
    assert_eq!(resolved, Some("/v3/file.txt".to_string()));
}

#[test]
fn test_history_of_unknown_id() {
    let resolver = PathResolver::in_memory().unwrap();
    let history = resolver.get_move_history("ghost");
    assert!(history.is_empty());
}

#[test]
fn test_database_creation() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_paths");
    let db_str = db_path.to_str().unwrap();

    let resolver = PathResolver::new(db_str).unwrap();
    resolver.record_move("f1", "/old", "/new").unwrap();

    assert!(db_path.exists());
}
