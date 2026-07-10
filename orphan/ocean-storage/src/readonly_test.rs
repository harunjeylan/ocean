use crate::ocean_storage::readonly::ReadOnlyGuard;

#[test]
fn test_read_only_guard_allows_writes_by_default() {
    let guard = ReadOnlyGuard::new(false);
    assert!(guard.check_write_allowed().is_ok());
    assert!(!guard.is_read_only());
}

#[test]
fn test_read_only_guard_blocks_writes() {
    let guard = ReadOnlyGuard::new(true);
    assert!(guard.check_write_allowed().is_err());
    assert!(guard.is_read_only());
}

#[test]
fn test_read_only_guard_toggle() {
    let guard = ReadOnlyGuard::new(false);
    assert!(guard.check_write_allowed().is_ok());
    guard.set_enabled(true);
    assert!(guard.check_write_allowed().is_err());
    guard.set_enabled(false);
    assert!(guard.check_write_allowed().is_ok());
}
