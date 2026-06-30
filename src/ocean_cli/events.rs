use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;

use crate::ocean_index::report::FileIndexStatus;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum SystemEvent {
    IndexStarted {
        timestamp: u64,
        dir: String,
        total_files: u64,
    },
    IndexComplete {
        timestamp: u64,
        duration_ms: u64,
        indexed: u64,
        skipped: u64,
        failed: u64,
    },
    FileProcessed {
        timestamp: u64,
        path: String,
        status: String,
        duration_ms: u64,
    },
    QueryExecuted {
        timestamp: u64,
        query: String,
        mode: String,
        num_results: usize,
        duration_ms: u64,
        cached: bool,
    },
    BackpressureEvent {
        timestamp: u64,
        action: String,
        queue_len: usize,
        in_flight: u32,
    },
    ErrorEvent {
        timestamp: u64,
        severity: String,
        module: String,
        message: String,
    },
}

pub fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone)]
pub enum OutputTarget {
    Stderr,
    File(PathBuf),
    Both(PathBuf),
}

pub trait EventEmitter: Send + Sync {
    fn emit(&self, event: SystemEvent);
    fn set_output(&mut self, target: OutputTarget);
}

pub struct ConsoleEmitter;

impl EventEmitter for ConsoleEmitter {
    fn emit(&self, event: SystemEvent) {
        match event {
            SystemEvent::IndexStarted { timestamp, dir, total_files } => {
                eprintln!("[{}] IndexStarted: {} ({} files)", timestamp, dir, total_files);
            }
            SystemEvent::IndexComplete { timestamp, duration_ms, indexed, skipped, failed } => {
                eprintln!("[{}] IndexComplete: {} indexed, {} skipped, {} failed ({}ms)", timestamp, indexed, skipped, failed, duration_ms);
            }
            SystemEvent::FileProcessed { timestamp, path, status, duration_ms } => {
                eprintln!("[{}] FileProcessed: {} ({}, {}ms)", timestamp, path, status, duration_ms);
            }
            SystemEvent::QueryExecuted { timestamp, query, mode, num_results, duration_ms, cached } => {
                let cache_tag = if cached { " (cached)" } else { "" };
                eprintln!("[{}] QueryExecuted: \"{}\" mode={} {} results{}{}", timestamp, query, mode, num_results, cache_tag, duration_ms);
            }
            SystemEvent::BackpressureEvent { timestamp, action, queue_len, in_flight } => {
                eprintln!("[{}] Backpressure: {} (queue={}, in_flight={})", timestamp, action, queue_len, in_flight);
            }
            SystemEvent::ErrorEvent { timestamp, severity, module, message } => {
                eprintln!("[{}] {} [{}]: {}", timestamp, severity.to_uppercase(), module, message);
            }
        }
    }

    fn set_output(&mut self, _target: OutputTarget) {}
}

pub struct JsonEmitter {
    output: Mutex<OutputTarget>,
    file: Mutex<Option<File>>,
}

impl JsonEmitter {
    pub fn new(target: OutputTarget) -> Self {
        let file = match &target {
            OutputTarget::File(p) | OutputTarget::Both(p) => {
                OpenOptions::new().create(true).append(true).open(p).ok()
            }
            OutputTarget::Stderr => None,
        };
        Self {
            output: Mutex::new(target),
            file: Mutex::new(file),
        }
    }

    fn write_line(&self, line: &str) {
        let output = self.output.lock().unwrap_or_else(|e| e.into_inner());
        match &*output {
            OutputTarget::Stderr => {
                eprintln!("{}", line);
            }
            OutputTarget::File(_) => {
                if let Ok(mut file) = self.file.lock() {
                    if let Some(ref mut f) = *file {
                        let _ = writeln!(f, "{}", line);
                    }
                }
            }
            OutputTarget::Both(_) => {
                eprintln!("{}", line);
                if let Ok(mut file) = self.file.lock() {
                    if let Some(ref mut f) = *file {
                        let _ = writeln!(f, "{}", line);
                    }
                }
            }
        }
    }
}

impl EventEmitter for JsonEmitter {
    fn emit(&self, event: SystemEvent) {
        if let Ok(json) = serde_json::to_string(&event) {
            self.write_line(&json);
        }
    }

    fn set_output(&mut self, target: OutputTarget) {
        let file = match &target {
            OutputTarget::File(p) | OutputTarget::Both(p) => {
                OpenOptions::new().create(true).append(true).open(p).ok()
            }
            OutputTarget::Stderr => None,
        };
        if let Ok(mut f) = self.file.lock() {
            *f = file;
        }
        if let Ok(mut o) = self.output.lock() {
            *o = target;
        }
    }
}

pub struct MultiEmitter {
    emitters: Vec<Box<dyn EventEmitter>>,
}

impl MultiEmitter {
    pub fn new(emitters: Vec<Box<dyn EventEmitter>>) -> Self {
        Self { emitters }
    }
}

impl EventEmitter for MultiEmitter {
    fn emit(&self, event: SystemEvent) {
        for emitter in &self.emitters {
            emitter.emit(event.clone());
        }
    }

    fn set_output(&mut self, target: OutputTarget) {
        for emitter in &mut self.emitters {
            emitter.set_output(target.clone());
        }
    }
}

pub fn file_index_status_to_string(status: &FileIndexStatus) -> String {
    match status {
        FileIndexStatus::Indexed => "indexed".to_string(),
        FileIndexStatus::Skipped => "skipped".to_string(),
        FileIndexStatus::Failed => "failed".to_string(),
    }
}

static CONSOLE_EMITTER: ConsoleEmitter = ConsoleEmitter;
static GLOBAL_EMITTER: std::sync::OnceLock<&'static dyn EventEmitter> = std::sync::OnceLock::new();

pub fn global_emitter() -> &'static dyn EventEmitter {
    GLOBAL_EMITTER.get().copied().unwrap_or(&CONSOLE_EMITTER)
}

pub fn set_global_emitter(emitter: Box<dyn EventEmitter>) {
    let leaked: &'static dyn EventEmitter = Box::leak(emitter);
    let _ = GLOBAL_EMITTER.set(leaked);
}
