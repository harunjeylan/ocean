# Requirements Document: Ocean Runtime Concurrency & Resilience

## Introduction

The indexing pipeline currently processes files **sequentially** in a single `for` loop (`IndexOrchestrator::run()`), has **no retry logic** despite a `max_retries: u32` field existing in `IndexConfig`, applies **no rate limiting** to embedding API calls (OpenAI, Anthropic, Gemini), and has **no backpressure** mechanism. This phase introduces a 3-tier worker pool, a priority job queue, rate-limited embedding, exponential-backoff retry, and adaptive backpressure — turning the indexing pipeline from a fragile single-threaded process into a resilient, concurrent system capable of handling 10k–100k files.

The scope is production-grade concurrency for the indexing pipeline only (query-time concurrency is a future concern).

---

## Glossary

- **Worker Pool**: A set of threads that execute file-processing jobs concurrently. Three tiers: IO (high parallelism for filesystem/disk), CPU (bounded for parse/chunk/graph), AI (rate-limited for embedding API calls).
- **Job Queue**: A priority queue holding `FileJob` units that are dispatched to workers. Three priority levels: High (watch events), Normal (incremental index), Low (bulk full-index).
- **Backpressure**: The mechanism that slows or pauses job ingestion when system resources (CPU, memory, queue depth) exceed thresholds.
- **Rate Limiter**: A token-bucket or semaphore that limits the number of concurrent embedding API calls within a time window.
- **Exponential Backoff**: A retry strategy where wait time increases geometrically (100ms, 500ms, 2s, 10s) after each consecutive failure, up to a configurable max retries.
- **FileJob**: A unit of work representing one file to process through the pipeline. Contains file ID, path, priority, and retry count.
- **3-Tier Concurrency**: IO tier (rayon/async for scanning/DB), CPU tier (bounded thread pool for parse/chunk/graph), AI tier (semaphore-limited for embedding API calls).

---

## Requirements

### R1: 3-Tier Worker Pool

**User Story:** As the index orchestrator, I want a 3-tier worker pool (IO, CPU, AI) so that filesystem scanning, CPU-bound parsing/chunking, and rate-limited embedding API calls do not compete for the same threads.

#### Acceptance Criteria

1. THE `WorkerPool` struct SHALL expose three pools: `io_pool: rayon::ThreadPool`, `cpu_pool: rayon::ThreadPool`, `ai_semaphore: std::sync::Semaphore` (or `tokio::sync::Semaphore`).
2. THE IO pool SHALL be sized to `num_cpus * 2` (default) for high-parallelism filesystem and database operations.
3. THE CPU pool SHALL be sized to `num_cpus` (default) for CPU-bound parse/chunk/graph operations.
4. THE AI semaphore SHALL limit concurrent embedding calls to a configurable `max_concurrent_embeddings` (default 2).
5. THE `WorkerPool` SHALL be constructable via `WorkerPool::new(io_threads, cpu_threads, max_ai_concurrent)` with sensible defaults.
6. THE `WorkerPool` SHALL provide `run_io(fn)`, `run_cpu(fn)`, `run_ai(fn)` methods that dispatch closures to the appropriate tier.
7. WHEN a closure panics in any tier, the pool SHALL catch the panic and return an error rather than crashing the process.

---

### R2: Priority Job Queue

**User Story:** As the index orchestrator, I want a priority-based job queue so that watch-triggered updates are processed before bulk indexing jobs.

#### Acceptance Criteria

1. A `FileJob` struct SHALL exist with fields: `file_id: String`, `path: String`, `priority: JobPriority`, `retry_count: u32`.
2. A `JobPriority` enum SHALL exist with variants: `High`, `Normal`, `Low`.
3. A `JobQueue` struct SHALL exist with three internal `VecDeque<FileJob>` buffers (high, normal, low).
4. `JobQueue::enqueue(job)` SHALL insert into the appropriate priority buffer.
5. `JobQueue::dequeue() -> Option<FileJob>` SHALL return from high first, then normal, then low.
6. `JobQueue::dequeue_batch(max: usize) -> Vec<FileJob>` SHALL return up to `max` jobs, respecting priority order.
7. `JobQueue::len() -> usize` SHALL return total queued jobs.
8. `JobQueue::has_backlog() -> bool` SHALL return true if total queued jobs exceed a configurable `max_queue_size` (default 10,000).

---

### R3: Exponential-Backoff Retry

**User Story:** As the file processor, I want automatic retry with exponential backoff on transient failures (embedder timeout, storage conflict) so that temporary network/storage issues do not abort indexing.

#### Acceptance Criteria

1. THE `FileProcessor::process()` method SHALL accept a `max_retries: u32` and `initial_backoff_ms: u64` parameter (the existing dead `max_retries` field SHALL be used).
2. ON transient failure (embedder timeout, storage conflict), THE processor SHALL retry with backoff: `initial_backoff_ms * 2^retry_count`, clamped to a max of 30 seconds.
3. ON non-transient failure (corrupt file, unsupported format), THE processor SHALL fail immediately without retry.
4. A `RetryPolicy` struct SHALL exist with fields: `max_retries: u32`, `initial_backoff_ms: u64`, `max_backoff_ms: u64`.
5. THE `RetryPolicy` SHALL implement `next_delay(retry_count: u32) -> Duration`.
6. AFTER all retries exhausted, THE processor SHALL emit a `FileFailed` event with the final error and retry count.
7. THE `IndexConfig` SHALL include a `retry_policy: RetryPolicy` field replacing the current dead `max_retries` field.

---

### R4: Rate-Limited Embedding

**User Story:** As an embedder consumer, I want rate-limited API calls so that remote embedding providers (OpenAI, Anthropic, Gemini) are not overwhelmed and API keys are not banned.

#### Acceptance Criteria

1. A `RateLimiter` struct SHALL exist with configurable `max_concurrent: usize` (semaphore-based) and optional `requests_per_minute: u64`.
2. THE `IndexPipeline::index_chunks()` method SHALL acquire a semaphore permit before calling `embedder.embed_batch()`.
3. THE semaphore permit SHALL be released after the embedding call completes (success or failure).
4. WHEN `requests_per_minute` is set, THE rate limiter SHALL enforce at most N batch calls per 60-second window using a sliding-window counter.
5. THE `IndexPipeline::embed_batch()` SHALL have an explicit timeout (default 60s) per call to prevent hanging on unresponsive APIs.
6. THE `IndexConfig` SHALL include a `rate_limiter: RateLimiterConfig` field with `max_concurrent` and optional `requests_per_minute`.

---

### R5: Adaptive Backpressure

**User Story:** As the index orchestrator, I want the system to automatically slow down job ingestion when resources are strained, so that the process does not OOM or overwhelm the storage layer.

#### Acceptance Criteria

1. THE `IndexOrchestrator::run()` SHALL check system pressure BEFORE dequeuing each batch: queue depth, memory pressure (via `sys-info` or heuristic).
2. WHEN ANY of the following thresholds are exceeded, ingestion SHALL pause for 1 second before re-checking:
   - Queue depth > 10,000 (configurable via `max_queue_size`)
   - Concurrent AI embeddings at capacity
   - Total jobs in-flight > 50 (configurable via `max_in_flight`)
3. WHEN paused, THE orchestrator SHALL emit a `BackpressurePaused` progress event.
4. WHEN pressure subsides, THE orchestrator SHALL emit a `BackpressureResumed` event and continue.
5. THE backpressure thresholds SHALL be configurable via `IndexConfig.backpressure`.

---

### R6: Parallel File Processing with Safety

**User Story:** As the index orchestrator, I want to process multiple files in parallel within safe concurrency limits so that multi-core CPUs are utilized efficiently.

#### Acceptance Criteria

1. THE `IndexOrchestrator::run()` SHALL process files using the CPU worker pool with parallelism bounded by `cpu_pool.num_threads()`.
2. EACH file's pipeline (parse → chunk → embed → graph → store) SHALL run on the CPU pool, with the embed step acquiring an AI semaphore permit.
3. Storage writes SHALL be serialized (single writer) OR use the IO pool for parallel reads with a mutex around writes.
4. THE number of in-flight files SHALL be bounded by `max_in_flight` (default 10) to prevent excessive memory use.
5. WHEN a file's processing panics, OTHER files SHALL continue unaffected.

---

### R7: CLI & Config Integration

**User Story:** As a CLI user, I want to configure concurrency parameters via CLI flags and config file so that I can tune performance for my hardware.

#### Acceptance Criteria

1. THE `ocean index` command SHALL accept optional flags: `--io-threads`, `--cpu-threads`, `--max-ai-concurrent`, `--max-retries`, `--retry-backoff-ms`, `--max-queue-size`, `--max-in-flight`.
2. THESE values SHALL default to sensible hardware-detected values (using `std::thread::available_parallelism()`).
3. THE config file (`~/.ocean/config.json`) SHALL support a `runtime` section with these values.
4. Resolution order: CLI flag > config file > hardware detection > hardcoded defaults.
5. THE `ocean index --help` SHALL document all new flags.

---

### R8: Error Handling

**User Story:** As a system operator, I want clear, typed error variants for all runtime operations so that I can handle concurrency failures appropriately.

#### Acceptance Criteria

1. A `RuntimeError` enum SHALL exist with variants: `PoolPanic(String)`, `QueueFull(usize)`, `RetryExhausted { path, retries, last_error }`, `RateLimitExceeded`, `BackpressureTimeout`.
2. THE `RuntimeError` SHALL implement `Display`, `Error`, `Send`, `Sync`.
3. THE existing `IndexError` SHALL gain `From<RuntimeError>` so retry/queue failures propagate to the orchestrator's error type.
