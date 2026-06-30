use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum RuntimeMode {
    #[serde(rename = "desktop")]
    Desktop,
    #[serde(rename = "server")]
    Server,
    #[serde(rename = "embedded")]
    Embedded,
}

impl RuntimeMode {
    pub fn defaults(&self) -> ModeDefaults {
        match self {
            RuntimeMode::Desktop => ModeDefaults {
                io_threads: None,
                cpu_threads: None,
                max_ai_concurrent: Some(2),
                embedding_cache_size: Some(1000),
                query_cache_size: Some(100),
                max_in_flight: Some(10),
                max_queue_size: Some(10_000),
                embedding_batch_size: Some(10),
            },
            RuntimeMode::Server => ModeDefaults {
                io_threads: None,
                cpu_threads: None,
                max_ai_concurrent: Some(4),
                embedding_cache_size: Some(5000),
                query_cache_size: Some(500),
                max_in_flight: Some(50),
                max_queue_size: Some(100_000),
                embedding_batch_size: Some(32),
            },
            RuntimeMode::Embedded => ModeDefaults {
                io_threads: Some(2),
                cpu_threads: Some(1),
                max_ai_concurrent: Some(1),
                embedding_cache_size: Some(100),
                query_cache_size: Some(20),
                max_in_flight: Some(3),
                max_queue_size: Some(1_000),
                embedding_batch_size: Some(4),
            },
        }
    }

    pub fn auto_detect() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        if cpus <= 2 {
            RuntimeMode::Embedded
        } else if cpus >= 16 {
            RuntimeMode::Server
        } else {
            RuntimeMode::Desktop
        }
    }

    pub fn from_mode_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "desktop" => Some(RuntimeMode::Desktop),
            "server" => Some(RuntimeMode::Server),
            "embedded" => Some(RuntimeMode::Embedded),
            _ => None,
        }
    }

    pub fn resolve(cli: Option<&str>, config: Option<&str>) -> Self {
        if let Some(s) = cli {
                if let Some(mode) = Self::from_mode_str(s) {
                return mode;
            }
        }
        if let Some(s) = config {
                if let Some(mode) = Self::from_mode_str(s) {
                return mode;
            }
        }
        Self::auto_detect()
    }
}

impl Default for RuntimeMode {
    fn default() -> Self {
        RuntimeMode::Desktop
    }
}

static AUTO_DETECTED_MODE: OnceLock<RuntimeMode> = OnceLock::new();

pub fn detected_mode() -> RuntimeMode {
    *AUTO_DETECTED_MODE.get_or_init(RuntimeMode::auto_detect)
}

#[derive(Debug, Clone)]
pub struct ModeDefaults {
    pub io_threads: Option<usize>,
    pub cpu_threads: Option<usize>,
    pub max_ai_concurrent: Option<usize>,
    pub embedding_cache_size: Option<usize>,
    pub query_cache_size: Option<usize>,
    pub max_in_flight: Option<usize>,
    pub max_queue_size: Option<usize>,
    pub embedding_batch_size: Option<usize>,
}

impl Default for ModeDefaults {
    fn default() -> Self {
        RuntimeMode::Desktop.defaults()
    }
}

impl ModeDefaults {
    pub fn io_threads_value(&self, cpus: usize) -> usize {
        self.io_threads.unwrap_or_else(|| cpus * 2)
    }

    pub fn cpu_threads_value(&self, cpus: usize) -> usize {
        self.cpu_threads.unwrap_or(cpus)
    }

    pub fn embedding_cache_size_value(&self) -> usize {
        self.embedding_cache_size.unwrap_or(1000)
    }

    pub fn query_cache_size_value(&self) -> usize {
        self.query_cache_size.unwrap_or(100)
    }

    pub fn embedding_batch_size_value(&self) -> usize {
        self.embedding_batch_size.unwrap_or(10)
    }
}
