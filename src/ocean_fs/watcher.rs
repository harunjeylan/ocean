use crate::ocean_fs::types::{self, FileEvent, FileMeta};
use crossbeam_channel::{Receiver, Sender};
use notify::event::ModifyKind;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const DEBOUNCE_WINDOW_MS: u64 = 100;
const MAX_BATCH_SIZE: usize = 100;

#[derive(Debug, Clone)]
pub struct WatchHandle {
    pub id: String,
}

#[derive(Debug, Clone)]
pub enum WatchError {
    NotifyError(String),
    AlreadyWatching,
    InvalidPath(String),
}

impl std::fmt::Display for WatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatchError::NotifyError(msg) => write!(f, "notify error: {}", msg),
            WatchError::AlreadyWatching => write!(f, "already watching"),
            WatchError::InvalidPath(p) => write!(f, "invalid path: {}", p),
        }
    }
}

impl std::error::Error for WatchError {}

impl From<notify::Error> for WatchError {
    fn from(e: notify::Error) -> Self {
        WatchError::NotifyError(e.to_string())
    }
}

struct WatcherInner {
    handle: Option<notify::Result<RecommendedWatcher>>,
    running: Arc<AtomicBool>,
}

pub struct FileWatcher {
    inner: Arc<Mutex<WatcherInner>>,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(WatcherInner {
                handle: None,
                running: Arc::new(AtomicBool::new(false)),
            })),
        }
    }

    pub fn watch(
        &self,
        path: &str,
        callback: Arc<dyn Fn(FileEvent) + Send + Sync>,
    ) -> Result<WatchHandle, WatchError> {
        let watch_path = Path::new(path);
        if !watch_path.exists() {
            return Err(WatchError::InvalidPath(path.to_string()));
        }
        if !watch_path.is_dir() {
            return Err(WatchError::InvalidPath(format!("{} is not a directory", path)));
        }

        let mut inner = self.inner.lock().unwrap();
        if inner.handle.is_some() {
            return Err(WatchError::AlreadyWatching);
        }

        let running = Arc::new(AtomicBool::new(true));
        inner.running = running.clone();

        let (tx, rx): (Sender<FileEvent>, Receiver<FileEvent>) = crossbeam_channel::unbounded();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if let Some(fe) = map_notify_event(&event) {
                    let _ = tx.send(fe);
                }
            }
        })
        .map_err(WatchError::from)?;

        watcher
            .watch(watch_path, RecursiveMode::Recursive)
            .map_err(WatchError::from)?;

        let handle_id = format!("watcher-{}", types::generate_file_id());
        let handle = WatchHandle {
            id: handle_id.clone(),
        };

        let rx_clone = rx;
        let running_clone = running.clone();

        thread::spawn(move || {
            let mut buffer: Vec<FileEvent> = Vec::new();
            let mut last_flush = Instant::now();

            loop {
                if !running_clone.load(Ordering::Relaxed) {
                    break;
                }

                if let Ok(event) = rx_clone.recv_timeout(Duration::from_millis(50)) {
                    buffer.push(event);

                    if buffer.len() >= MAX_BATCH_SIZE {
                        flush_batch(&buffer, &callback);
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }

                if !buffer.is_empty()
                    && last_flush.elapsed() >= Duration::from_millis(DEBOUNCE_WINDOW_MS)
                {
                    flush_batch(&buffer, &callback);
                    buffer.clear();
                    last_flush = Instant::now();
                }
            }

            if !buffer.is_empty() {
                flush_batch(&buffer, &callback);
            }
        });

        inner.handle = Some(Ok(watcher));

        Ok(handle)
    }

    pub fn unwatch(&self, _handle: WatchHandle) -> Result<(), WatchError> {
        let mut inner = self.inner.lock().unwrap();
        inner.running.store(false, Ordering::Relaxed);
        inner.handle = None;
        Ok(())
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

fn flush_batch(events: &[FileEvent], callback: &Arc<dyn Fn(FileEvent) + Send + Sync>) {
    for event in events {
        callback(event.clone());
    }
}

fn map_notify_event(event: &Event) -> Option<FileEvent> {
    let path = event.paths.first()?.to_str()?.to_string();
    let meta = extract_file_meta(&path);

    match event.kind {
        EventKind::Create(_) => meta.map(FileEvent::Created),
        EventKind::Modify(modify_kind) => {
            if matches!(modify_kind, ModifyKind::Name(_)) {
                if let Some(new_path) = extract_new_path(event) {
                    Some(FileEvent::Renamed {
                        file_id: types::generate_file_id(),
                        old_path: path,
                        new_path,
                    })
                } else {
                    None
                }
            } else if matches!(
                modify_kind,
                ModifyKind::Data(_) | ModifyKind::Any
            ) {
                meta.map(FileEvent::Modified)
            } else {
                None
            }
        }
        EventKind::Remove(_) => {
            let file_id = types::generate_file_id();
            Some(FileEvent::Deleted(file_id))
        }
        _ => None,
    }
}

fn extract_file_meta(path: &str) -> Option<FileMeta> {
    let p = Path::new(path);
    let metadata = std::fs::metadata(p).ok()?;

    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let hash = crate::ocean_fs::hasher::hash_file(path).ok()?;

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Some(FileMeta {
        id: types::generate_file_id(),
        path: path.to_string(),
        hash,
        size: metadata.len(),
        modified,
        extension: ext,
    })
}

fn extract_new_path(event: &Event) -> Option<String> {
    for path in &event.paths {
        if path.exists() {
            return path.to_str().map(|s| s.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::mpsc;
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
}
