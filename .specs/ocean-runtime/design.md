# Design Document: Ocean Runtime Concurrency & Resilience

## Overview

The indexing pipeline currently runs single-threaded with no retry logic, no rate limiting, and no backpressure. This design introduces a 3-tier worker pool, priority job queue, exponential-backoff retry, rate-limited embedding, and adaptive backpressure — all integrated into the existing `IndexOrchestrator`, `FileProcessor`, and `IndexPipeline` without breaking their public APIs.

The core insight is that Ocean has three fundamentally different kinds of work: IO-bound (filesystem scan, DB reads), CPU-bound (parse, chunk, graph build), and API-bound (embedding calls). Each needs independent concurrency controls. A unified "one pool fits all" approach would either waste IO throughput or overload remote APIs.

### Key Design Decisions

1. **rayon::ThreadPool for IO and CPU tiers** — rayon provides work-stealing, panic-safety, and is already a dependency. Two separate pools prevent CPU-bound parsing from starving IO.
2. **std::sync::Semaphore for AI tier** — Embedding calls are typically network IO, not CPU. A counting semaphore limits concurrency to avoid rate limits. Simple, no additional dependencies.
3. **No async runtime change** — The existing `tokio::runtime::Runtime` (used by `PathResolver` and storage) is not replaced. The worker pools are synchronous (`rayon`), which matches the existing sync pipeline. Async migration is a future concern.
4. **Priority queue over channel-based dispatch** — A simple `VecDeque`-based priority queue is sufficient. No need for a full job-scheduler library. The orchestrator pulls jobs and dispatches to pools.
5. **Application-level backpressure** — Rather than relying on OS signals, the orchestrator checks thresholds before each batch and pauses with a sleep loop. Simple, predictable, and cross-platform.

---

## Architecture

### High-level component diagram

```text
Filesystem Events (watch)
         │
         ▼
   JobQueue (3 priorities)
         │
         ▼
IndexOrchestrator::run()
         │
         ├── dequeues batch (respecting priority)
         ├── checks backpressure thresholds
         ├── dispatches to cpu_pool via parallel iter
         │
         ▼
   WorkerPool (3 tiers)
         │
         ├── io_pool (rayon)     — scan, DB reads
         ├── cpu_pool (rayon)    — parse, chunk, graph build
         └── ai_semaphore        — rate-limits embedding calls
                                    │
                                    ▼
                              FileProcessor::process()
                                    │
                              with RetryPolicy wrapping
                              each transient-fallible step
```

### Data flow per file

```text
FileJob
  │  [cpu_pool]
  ├── parse (ocean_parser::read_all_blocks)
  │     └── if Err(transient) → retry with backoff
  ├── chunk (ocean_chunk::chunker::chunk)
  ├── embed [acquires ai_semaphore permit]
  │     └── if Err(transient) → release permit, retry with backoff
  ├── graph (GraphBuilder::from_chunks)
  ├── store (write to storage sub-stores)
  └── state (StateStore::update_state)
```

---

## Components and Interfaces

### 1. WorkerPool

```rust
pub struct WorkerPool {
    pub io_pool: rayon::ThreadPool,
    pub cpu_pool: rayon::ThreadPool,
    pub ai_semaphore: std::sync::Semaphore,
}

impl WorkerPool {
    pub fn new(
        io_threads: usize,
        cpu_threads: usize,
        max_ai_concurrent: usize,
    ) -> Self;

    /// Run a closure on the IO pool.
    pub fn run_io<T: Send>(&self, f: impl FnOnce() -> T + Send) -> Result<T, RuntimeError>;

    /// Run a closure on the CPU pool.
    pub fn run_cpu<T: Send>(&self, f: impl FnOnce() -> T + Send) -> Result<T, RuntimeError>;

    /// Run a closure with an AI semaphore permit acquired.
    pub fn run_ai<T: Send>(&self, f: impl FnOnce() -> T + Send) -> Result<T, RuntimeError>;
}

impl Default for WorkerPool {
    fn default() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(cpus * 2, cpus, 2)
    }
}
```

### 2. JobQueue

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    High,   // watch events
    Normal, // incremental index
    Low,    // bulk full-index
}

#[derive(Debug, Clone)]
pub struct FileJob {
    pub file_id: String,
    pub path: String,
    pub priority: JobPriority,
    pub retry_count: u32,
}

pub struct JobQueue {
    high: VecDeque<FileJob>,
    normal: VecDeque<FileJob>,
    low: VecDeque<FileJob>,
    max_size: usize,
}

impl JobQueue {
    pub fn new(max_size: usize) -> Self;
    pub fn enqueue(&mut self, job: FileJob) -> Result<(), RuntimeError>;
    pub fn enqueue_batch(&mut self, jobs: Vec<FileJob>) -> Result<(), RuntimeError>;
    pub fn dequeue(&mut self) -> Option<FileJob>;
    pub fn dequeue_batch(&mut self, max: usize) -> Vec<FileJob>;
    pub fn len(&self) -> usize;
    pub fn has_backlog(&self) -> bool;
    pub fn clear(&mut self);
}
```

### 3. RetryPolicy

```rust
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl RetryPolicy {
    pub fn new(max_retries: u32, initial_backoff_ms: u64, max_backoff_ms: u64) -> Self;

    /// Returns the delay duration for the given retry count (0-indexed).
    /// delay = min(initial * 2^retry_count, max_backoff)
    pub fn next_delay(&self, retry_count: u32) -> Duration;

    /// Returns true if the error is transient (should be retried).
    pub fn is_transient(&self, error: &dyn std::error::Error) -> bool;
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3, 100, 30_000) // 3 retries, 100ms initial, 30s max
    }
}
```

### 4. RateLimiter

```rust
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    pub max_concurrent: usize,
    pub requests_per_minute: Option<u64>,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 2,
            requests_per_minute: None,
        }
    }
}

pub struct RateLimiter {
    semaphore: std::sync::Semaphore,
    rps: Option<u64>,
    // sliding-window state (only when requests_per_minute is set)
    window_timestamps: Mutex<VecDeque<Instant>>,
}

impl RateLimiter {
    pub fn new(config: &RateLimiterConfig) -> Self;

    /// Acquire a permit, blocking if at capacity.
    pub fn acquire(&self) -> Result<PermitGuard, RuntimeError>;

    /// Non-blocking try-acquire.
    pub fn try_acquire(&self) -> Result<Option<PermitGuard>, RuntimeError>;

    /// Returns the number of currently available permits.
    pub fn available_permits(&self) -> usize;
}

pub struct PermitGuard<'a> {
    // Releases the semaphore permit on drop.
}
```

### 5. Backpressure Config

```rust
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    pub max_queue_size: usize,     // default 10,000
    pub max_in_flight: usize,      // default 10
    pub max_ai_concurrent: usize,  // default 2
    pub pause_check_ms: u64,       // default 1,000 (how long to sleep when paused)
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10_000,
            max_in_flight: 10,
            max_ai_concurrent: 2,
            pause_check_ms: 1_000,
        }
    }
}
```

### 6. Updated IndexConfig

```rust
#[derive(Debug, Clone)]
pub struct IndexConfig {
    // ...existing fields...
    pub retry_policy: RetryPolicy,
    pub rate_limiter: RateLimiterConfig,
    pub backpressure: BackpressureConfig,
    pub io_threads: Option<usize>,   // None = auto-detect
    pub cpu_threads: Option<usize>,  // None = auto-detect
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            // ...existing defaults...
            retry_policy: RetryPolicy::default(),
            rate_limiter: RateLimiterConfig::default(),
            backpressure: BackpressureConfig::default(),
            io_threads: None,
            cpu_threads: None,
        }
    }
}
```

### 7. Updated IndexOrchestrator

```rust
pub struct IndexOrchestrator {
    processor: FileProcessor,
    state_store: Arc<dyn StateStore>,
    reporter: Box<dyn ProgressReporter>,
    pool: WorkerPool,                // NEW
}

impl IndexOrchestrator {
    pub fn run(&self, config: IndexConfig) -> Result<IndexReport, IndexError> {
        let mut job_queue = JobQueue::new(config.backpressure.max_queue_size);
        let mut in_flight: u32 = 0;

        // 1. Scan directory → enqueue with priority based on mode
        let metas = self.pool.run_io(|| scan_dir(&dir))?;
        let priority = match config.mode {
            IndexMode::Watch => JobPriority::High,
            IndexMode::Incremental => JobPriority::Normal,
            IndexMode::Full => JobPriority::Low,
        };
        for meta in metas {
            job_queue.enqueue(FileJob {
                file_id: meta.id.clone(),
                path: meta.path.clone(),
                priority,
                retry_count: 0,
            })?;
        }

        // 2. Process batches with backpressure
        while let Some(batch) = self.dequeue_with_backpressure(&mut job_queue, &config) {
            let results: Vec<FileResult> = self.pool.cpu_pool.install(|| {
                batch
                    .par_iter()
                    .map(|job| self.process_one(job, &config))
                    .collect()
            });
            // aggregate results...
        }
    }

    fn dequeue_with_backpressure(
        &self,
        queue: &mut JobQueue,
        config: &IndexConfig,
    ) -> Option<Vec<FileJob>> {
        loop {
            if !queue.has_backlog()
                && self.pool.ai_semaphore.available_permits() > 0
                && in_flight < config.backpressure.max_in_flight
            {
                return Some(queue.dequeue_batch(config.backpressure.max_in_flight));
            }
            // backpressure: pause and re-check
            self.reporter.report(ProgressEvent::BackpressurePaused {
                queue_len: queue.len(),
                available_ai: self.pool.ai_semaphore.available_permits(),
                in_flight,
            });
            std::thread::sleep(Duration::from_millis(config.backpressure.pause_check_ms));
        }
    }

    fn process_one(&self, job: &FileJob, config: &IndexConfig) -> FileResult {
        self.processor.process_with_retry(
            &job.path,
            &config.retry_policy,
            &|path| {
                // Each embedding call inside the processor will:
                //   1. Acquire AI semaphore permit via pool.run_ai(...)
                //   2. Release on completion
                self.processor.process(path)
            },
        )
    }
}
```

### 8. Updated FileProcessor

```rust
impl FileProcessor {
    pub fn process_with_retry(
        &self,
        path: &str,
        retry_policy: &RetryPolicy,
        pool: &WorkerPool,
    ) -> Result<FileResult, IndexError> {
        let mut last_error = None;
        for attempt in 0..=retry_policy.max_retries {
            match self.try_process(path, pool) {
                Ok(result) => return Ok(result),
                Err(e) if retry_policy.is_transient(&e) && attempt < retry_policy.max_retries => {
                    last_error = Some(e);
                    let delay = retry_policy.next_delay(attempt);
                    std::thread::sleep(delay);
                }
                Err(e) => return Err(e),
            }
        }
        Err(IndexError::RetryExhausted {
            path: path.to_string(),
            retries: retry_policy.max_retries,
            last_error: Box::new(last_error.unwrap()),
        })
    }

    fn try_process(&self, path: &str, pool: &WorkerPool) -> Result<FileResult, IndexError> {
        // Acquire AI permit for embedding step
        let _permit = pool.ai_semaphore.acquire().map_err(|_| {
            IndexError::Runtime(RuntimeError::RateLimitExceeded)
        })?;

        // ... existing pipeline: parse → chunk → embed → graph → store ...
        // Each step that hits a transient error returns Err, which propagates
        // up to the retry loop.
    }
}
```

---

## Data Models

### RuntimeError

```rust
#[derive(Debug)]
pub enum RuntimeError {
    /// A rayon worker panicked.
    PoolPanic(String),
    /// Job queue is full.
    QueueFull(usize),
    /// Transient error exhausted after all retries.
    RetryExhausted {
        path: String,
        retries: u32,
        last_error: String,
    },
    /// Could not acquire AI semaphore permit.
    RateLimitExceeded,
    /// Backpressure paused for too long (optional timeout).
    BackpressureTimeout,
}
```

### ProgressEvent (extended)

```rust
// New variants added to the existing ProgressEvent enum:
pub enum ProgressEvent {
    // ...existing variants...

    // NEW:
    BackpressurePaused {
        queue_len: usize,
        available_ai: usize,
        in_flight: u32,
    },
    BackpressureResumed,
    Retrying {
        path: String,
        attempt: u32,
        max_retries: u32,
        delay_ms: u64,
        error: String,
    },
}
```

---

## Integration Points

### Updated modules

| Module | Change |
|--------|--------|
| `ocean_index::orchestrator` | Add `WorkerPool`, `JobQueue`, backpressure loop |
| `ocean_index::processor` | Add `process_with_retry()`, transient error classification |
| `ocean_index::config` | Add `RetryPolicy`, `RateLimiterConfig`, `BackpressureConfig`, thread counts |
| `ocean_index::progress` | Add `BackpressurePaused`, `BackpressureResumed`, `Retrying` events |
| `ocean_vector::pipeline` | Add AI semaphore acquire/release around `embed_batch()` |
| `ocean_api::indexing` | Pass new config fields through to orchestrator |
| `ocean_cli::args` | Add CLI flags for concurrency parameters |
| `ocean_cli::config` | Add `runtime` section to `OceanConfig` JSON |

### Backwards compatibility

- `IndexOrchestrator::new()` unchanged — `WorkerPool` is created internally with defaults.
- `IndexConfig::default()` unchanged — new fields have sensible defaults.
- `FileProcessor::process()` unchanged — `process_with_retry()` is the new entry point; `process()` delegates with default retry policy.
- All existing tests continue to pass unchanged.

---

## Correctness Properties

### Property 1: At-Most-Once Per File

*For any* file processed by the orchestrator, the file SHALL be processed at most once per `run()` invocation (no duplicate processing if the same file appears in multiple priority levels).

**Validates:** R2, R6

### Property 2: Retry Boundedness

*For any* file that encounters transient errors, the number of processing attempts SHALL NOT exceed `max_retries + 1`.

**Validates:** R3

### Property 3: AI Concurrency Cap

*For any* point in time, the number of concurrent `embed_batch()` calls SHALL NOT exceed `max_ai_concurrent`.

**Validates:** R4

### Property 4: Backpressure Eventual Progress

*For any* finite set of files, the orchestrator SHALL eventually process all files regardless of backpressure state (backpressure does not cause deadlock).

**Validates:** R5

### Property 5: Deterministic Priority Ordering

*For any* two jobs `A` (High) and `B` (Low) enqueued before processing begins, job `A` SHALL be dequeued before job `B`.

**Validates:** R2

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| Worker pool thread panics | `PoolPanic` error caught by rayon, propagated as `RuntimeError::PoolPanic` |
| Queue full | `RuntimeError::QueueFull(current_size)` — CLI shows helpful message |
| Embedder timeout (transient) | Retry with exponential backoff up to `max_retries` |
| Embedder returns 401 (non-transient) | Fail immediately, no retry |
| Storage conflict (transient) | Retry with exponential backoff |
| AI semaphore deadlocked | Timeout after `max_backoff_ms * max_retries` → `BackpressureTimeout` |
| Corrupt file | `FileFailed` immediately (non-transient) |

---

## Testing Strategy

### Unit Tests

- `WorkerPool::new()` creates pools with correct thread counts.
- `JobQueue` priority ordering: high dequeued before normal before low.
- `RetryPolicy::next_delay()` produces correct exponential sequence.
- `RetryPolicy::is_transient()` correctly classifies embedder timeout vs. auth error.
- `RateLimiter::acquire()` blocks when at capacity.
- `BackpressureConfig` defaults are reasonable.

### Integration Tests

- `IndexOrchestrator` with `WorkerPool` processes 100 files in parallel without errors.
- Embedder rate limiting prevents >N concurrent calls (measured via mock embedder with delay).
- Retry recovers from transient mock failures.
- Backpressure pauses when `max_in_flight` is reached.

### Property-Based Tests

- Property 2 (retry bounded): Randomize number of transient failures; verify retry count never exceeds `max_retries`.
- Property 3 (AI cap): Use mock embedder that records concurrent calls; verify never exceeds `max_ai_concurrent`.

---

## Performance Considerations

- **Default thread counts**: `io = 2*num_cpus`, `cpu = num_cpus` — these are conservative starting points.
- **AI semaphore default of 2**: Safe for all providers. Users with local Ollama can increase to 4–8.
- **No lock contention**: AI semaphore is the only shared synchronization primitive during indexing.
- **Rayon work-stealing** handles load imbalance between files automatically.
- **Memory**: `max_in_flight = 10` limits total chunks in memory to ~10 files' worth of parsed content.
