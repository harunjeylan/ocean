use crate::ocean_fs::types::FileEvent;
use crate::ocean_fs::watcher::FileWatcher;
use std::fs;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_watch_create_event() {
    let dir = tempdir().unwrap();
    let watcher = FileWatcher::new();
    let (tx, rx) = mpsc::channel();

    let callback = Arc::new(move |event: FileEvent| {
        let _ = tx.send(event);
    });

    let handle = watcher
        .watch(dir.path().to_str().unwrap(), callback)
        .unwrap();

    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, b"hello").unwrap();

    thread::sleep(Duration::from_millis(300));

    if let Ok(event) = rx.try_recv() {
        match event {
            FileEvent::Created(meta) => {
                assert!(meta.path.ends_with("test.txt"));
            }
            _ => {}
        }
    }

    watcher.unwatch(handle).unwrap();
}

#[test]
fn test_watch_modify_event() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, b"hello").unwrap();

    let watcher = FileWatcher::new();
    let (tx, rx) = mpsc::channel();

    let callback = Arc::new(move |event: FileEvent| {
        let _ = tx.send(event);
    });

    let handle = watcher
        .watch(dir.path().to_str().unwrap(), callback)
        .unwrap();

    fs::write(&file_path, b"modified").unwrap();

    thread::sleep(Duration::from_millis(300));

    if let Ok(event) = rx.try_recv() {
        match event {
            FileEvent::Modified(meta) => {
                assert!(meta.path.ends_with("test.txt"));
            }
            _ => {}
        }
    }

    watcher.unwatch(handle).unwrap();
}

#[test]
fn test_watch_delete_event() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, b"hello").unwrap();

    let watcher = FileWatcher::new();
    let (tx, rx) = mpsc::channel();

    let callback = Arc::new(move |event: FileEvent| {
        let _ = tx.send(event);
    });

    let handle = watcher
        .watch(dir.path().to_str().unwrap(), callback)
        .unwrap();

    fs::remove_file(&file_path).unwrap();

    thread::sleep(Duration::from_millis(300));

    if let Ok(event) = rx.try_recv() {
        match event {
            FileEvent::Deleted(_) => {}
            _ => panic!("expected Deleted event"),
        }
    }

    watcher.unwatch(handle).unwrap();
}

#[test]
fn test_watch_invalid_path() {
    let watcher = FileWatcher::new();
    let callback = Arc::new(|_event: FileEvent| {});

    let result = watcher.watch("C:\\nonexistent_path_watcher_xyz", callback);
    assert!(result.is_err());
}
