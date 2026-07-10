use crate::hash::hash_file;
use crate::shared::types::{self, FileEvent, FileMeta};
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
    WatchFailed(String),
    ChannelError,
}

impl std::fmt::Display for WatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatchError::WatchFailed(msg) => write!(f, "watch failed: {}", msg),
            WatchError::ChannelError => write!(f, "channel error"),
        }
    }
}

impl std::error::Error for WatchError {}

impl From<notify::Error> for WatchError {
    fn from(e: notify::Error) -> Self {
        WatchError::WatchFailed(e.to_string())
    }
}

#[allow(dead_code)]
struct WatcherInner {
    watcher: RecommendedWatcher,
    running: Arc<AtomicBool>,
}

pub struct FileWatcher {
    inner: Arc<Mutex<WatcherInner>>,
}

impl FileWatcher {
    pub fn new(path: &str) -> Result<Self, WatchError> {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let (tx, rx): (Sender<FileEvent>, Receiver<FileEvent>) =
            crossbeam_channel::bounded(MAX_BATCH_SIZE);
        let tx_callback: Arc<dyn Fn(FileEvent) + Send + Sync> = Arc::new(move |event| {
            let _ = tx.send(event);
        });
        let cb_for_watcher = tx_callback.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if !running_clone.load(Ordering::SeqCst) {
                return;
            }
            match res {
                Ok(event) => {
                    if let Some(file_event) = map_notify_event(&event) {
                        flush_batch(&[file_event], &cb_for_watcher);
                    }
                }
                Err(_) => {}
            }
        })?;

        watcher.watch(Path::new(path), RecursiveMode::Recursive)?;

        let inner = Arc::new(Mutex::new(WatcherInner { watcher, running }));

        let inner_clone = inner.clone();
        thread::spawn(move || {
            let mut batch: Vec<FileEvent> = Vec::new();
            let mut last_flush = Instant::now();

            while let Ok(event) = rx.recv() {
                batch.push(event);
                if batch.len() >= MAX_BATCH_SIZE
                    || last_flush.elapsed() > Duration::from_millis(DEBOUNCE_WINDOW_MS)
                {
                    let inner = inner_clone.lock().unwrap();
                    let batch_copy = batch.clone();
                    drop(inner);
                    flush_batch(&batch_copy, &tx_callback);
                    batch.clear();
                    last_flush = Instant::now();
                }
            }
        });

        Ok(FileWatcher { inner })
    }

    pub fn stop(&self) {
        let inner = self.inner.lock().unwrap();
        inner.running.store(false, Ordering::SeqCst);
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        FileWatcher {
            inner: Arc::new(Mutex::new(WatcherInner {
                watcher: notify::recommended_watcher(|_| {}).unwrap(),
                running: Arc::new(AtomicBool::new(false)),
            })),
        }
    }
}

fn flush_batch(events: &[FileEvent], callback: &Arc<dyn Fn(FileEvent) + Send + Sync>) {
    for event in events {
        callback(event.clone());
    }
}

fn map_notify_event(event: &Event) -> Option<FileEvent> {
    let path = event.paths.first()?;
    let path_str = path.to_str()?.to_string();

    match event.kind {
        EventKind::Create(_) => Some(FileEvent::Created(path_str)),
        EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Name(_)) => {
            Some(FileEvent::Modified(path_str))
        }
        EventKind::Remove(_) => Some(FileEvent::Deleted(path_str)),
        _ => None,
    }
}

#[allow(dead_code)]
fn extract_file_meta(path: &str) -> Option<FileMeta> {
    let p = Path::new(path);
    let metadata = std::fs::metadata(p).ok()?;

    let hash = hash_file(path).ok()?;
    let ext = p.extension()?.to_str()?.to_lowercase();
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
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
