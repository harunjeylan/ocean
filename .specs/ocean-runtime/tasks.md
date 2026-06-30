# Implementation Plan: Ocean Runtime Concurrency & Resilience

## Overview

Introduce a 3-tier `WorkerPool`, priority `JobQueue`, exponential-backoff `RetryPolicy`, rate-limited embedding via `RateLimiter`, and adaptive backpressure into the indexing pipeline. Work is structured as 8 tasks, each 2–4 hours, with testing embedded in each task.

## Pre-requisites

- All Phase 1–9 modules exist and tests pass (`cargo test` green).
- `IndexOrchestrator`, `FileProcessor`, `IndexPipeline` are stable.
- `IndexConfig` exists with the dead `max_retries` field.
- `ProgressEvent` enum exists with existing variants.

## Tasks

- [ ] 1. **Create `RuntimeError` + helper types**
  - Define `RuntimeError` enum in `src/ocean_index/error.rs` (merge with existing `IndexError` or create separate).
  - Add `From<RuntimeError>` impl for `IndexError`.
  - Define `RetryPolicy` struct + `next_delay()` + `is_transient()` in `src/ocean_index/runtime.rs`.
  - Define `RateLimiterConfig` + `BackpressureConfig` in `src/ocean_index/config.rs`.
  - Replace the dead `max_retries: u32` field in `IndexConfig` with `retry_policy: RetryPolicy`.
  - Write unit tests for `RetryPolicy::next_delay()` (geometric sequence) and `is_transient()`.
  - _Requirements: R3, R8_

  - [ ] 1.1 Define `RuntimeError` enum + `Display` + `Error` + `From` for `IndexError`
  - [ ] 1.2 Define `RetryPolicy` with `next_delay()` and `is_transient()`
  - [ ] 1.3 Define `RateLimiterConfig` and `BackpressureConfig`
  - [ ] 1.4 Update `IndexConfig` to use `RetryPolicy` instead of dead `max_retries`
  - [ ] 1.5 Unit tests for `RetryPolicy`
  - [ ] 1.6 Verify `cargo build` succeeds

- [ ] 2. **Implement `JobPriority`, `FileJob`, `JobQueue`**
  - Create `src/ocean_index/job_queue.rs` with `JobPriority` enum, `FileJob` struct, `JobQueue` struct.
  - Implement `enqueue()`, `dequeue()`, `dequeue_batch()`, `len()`, `has_backlog()`, `clear()`.
  - Priority order: High > Normal > Low.
  - `enqueue()` returns `Err(RuntimeError::QueueFull)` if at capacity.
  - Write unit tests for priority ordering, capacity limit, empty dequeue.
  - Register test file: `job_queue_test.rs`.
  - _Requirements: R2_

  - [ ] 2.1 `JobPriority` enum + `FileJob` struct
  - [ ] 2.2 `JobQueue` with priority buffers + all methods
  - [ ] 2.3 Capacity enforcement
  - [ ] 2.4 Unit tests
  - [ ] 2.5 Register in `ocean_index/mod.rs` and `tests.rs`

- [ ] 3. **Implement `WorkerPool` (3-tier)**
  - Create `src/ocean_index/worker_pool.rs` with `WorkerPool` struct.
  - Two `rayon::ThreadPool` instances (io, cpu) + one `std::sync::Semaphore` (ai).
  - Implement `new()`, `run_io()`, `run_cpu()`, `run_ai()`, `Default`.
  - `run_ai()` acquires semaphore permit before executing closure, releases after.
  - Thread counts: `io = io_threads`, `cpu = cpu_threads`, default via `available_parallelism()`.
  - Rayon pool panic → catch and return `RuntimeError::PoolPanic`.
  - Write unit tests: pool creation with specific sizes, panic handling, AI semaphore capacity enforcement.
  - Register test file: `worker_pool_test.rs`.
  - _Requirements: R1_

  - [ ] 3.1 `WorkerPool` struct + constructor + `Default`
  - [ ] 3.2 `run_io()`, `run_cpu()`, `run_ai()` impl
  - [ ] 3.3 AI semaphore acquire/release in `run_ai()`
  - [ ] 3.4 Panic safety (catch rayon panic)
  - [ ] 3.5 Unit tests
  - [ ] 3.6 Register in `ocean_index/mod.rs` and `tests.rs`

- [ ] 4. **Implement `RateLimiter`**
  - Create `src/ocean_index/rate_limiter.rs` with `RateLimiter` struct + `PermitGuard`.
  - Semaphore-based `max_concurrent` cap.
  - Optional sliding-window `requests_per_minute` (uses `Mutex<VecDeque<Instant>>`).
  - `acquire()` blocks until permit available; `try_acquire()` non-blocking.
  - `PermitGuard` implements `Drop` to release semaphore + update window.
  - Write unit tests: concurrent capacity, RPM enforcement, drop releases permit.
  - Register test file: `rate_limiter_test.rs`.
  - _Requirements: R4_

  - [ ] 4.1 `RateLimiter` struct + constructor
  - [ ] 4.2 Semaphore-based `acquire()` / `try_acquire()`
  - [ ] 4.3 Sliding-window RPM enforcement
  - [ ] 4.4 `PermitGuard` with `Drop`
  - [ ] 4.5 Unit tests
  - [ ] 4.6 Register in `ocean_index/mod.rs` and `tests.rs`

- [ ] 5. **Integrate retry into `FileProcessor`**
  - Add `process_with_retry(path, retry_policy, pool)` method to `FileProcessor`.
  - Retry loop calls `try_process()`; on transient error, waits `next_delay()` then retries.
  - Non-transient errors (corrupt file, parse error, auth error) fail immediately.
  - Emit `ProgressEvent::Retrying` on each retry.
  - Integrate AI semaphore: `try_process()` acquires permit before embedding step.
  - Update `FileProcessor::process()` to delegate to `process_with_retry()` with default policy.
  - Write unit tests: retry recovers from N transient failures; non-transient fails fast.
  - _Requirements: R3, R4_

  - [ ] 5.1 Add `process_with_retry()` with retry loop
  - [ ] 5.2 Transient vs. non-transient error classification
  - [ ] 5.3 Emit `ProgressEvent::Retrying`
  - [ ] 5.4 Acquire AI semaphore in `try_process()`
  - [ ] 5.5 Update `process()` to delegate
  - [ ] 5.6 Unit tests

- [ ] 6. **Integrate backpressure + parallel dispatch into `IndexOrchestrator`**
  - Add `WorkerPool` field to `IndexOrchestrator`.
  - Replace sequential `for` loop with batch dispatch: dequeues up to `max_in_flight` jobs, processes parallel on `cpu_pool`.
  - Before each batch, check backpressure (queue depth, AI permits, in-flight count). Pause with sleep loop if exceeded.
  - Emit `BackpressurePaused` / `BackpressureResumed` events.
  - Each file's processing uses `FileProcessor::process_with_retry()`.
  - Aggregate `FileResult`s in `IndexReport`.
  - Write integration test: process 50 files in parallel, verify all complete.
  - _Requirements: R5, R6_

  - [ ] 6.1 Add `WorkerPool` to `IndexOrchestrator`
  - [ ] 6.2 Replace sequential loop with parallel batch dispatch
  - [ ] 6.3 Backpressure check loop before each batch
  - [ ] 6.4 Emit backpressure progress events
  - [ ] 6.5 Integration test
  - [ ] 6.6 Verify `cargo test --lib index` passes

- [ ] 7. **Update `ProgressEvent` with new variants**
  - Add `BackpressurePaused { queue_len, available_ai, in_flight }` to `ProgressEvent`.
  - Add `BackpressureResumed` to `ProgressEvent`.
  - Add `Retrying { path, attempt, max_retries, delay_ms, error }` to `ProgressEvent`.
  - Update `ConsoleReporter` to display new events (backpressure: show yellow warning; retrying: show attempt count).
  - Update `SilentReporter` (no-op for new events).
  - _Requirements: R5, R3_

  - [ ] 7.1 Add new `ProgressEvent` variants
  - [ ] 7.2 Update `ConsoleReporter` display
  - [ ] 7.3 Update `SilentReporter`

- [ ] 8. **CLI + Config integration**
  - Add CLI flags to `IndexArgs`: `--io-threads`, `--cpu-threads`, `--max-ai-concurrent`, `--max-retries`, `--retry-backoff-ms`, `--max-queue-size`, `--max-in-flight`.
  - Add `runtime` section to `OceanConfig` serde struct with matching fields.
  - Resolution order in `cmd_index`: CLI flag > config > auto-detect > hardcoded default.
  - Pass resolved values into `IndexConfig` before constructing orchestrator.
  - Document all new flags in `--help`.
  - Update `cli-docs.md` if it exists.
  - _Requirements: R7_

  - [ ] 8.1 CLI flags in `IndexArgs`
  - [ ] 8.2 `runtime` section in `OceanConfig`
  - [ ] 8.3 Resolution logic in `cmd_index`
  - [ ] 8.4 Update `--help` strings
  - [ ] 8.5 Verify `cargo run --bin ocean -- index --help` output

- [ ] **Validation & Cleanup**
  - Run full test suite: `cargo test` — all 200+ tests must pass.
  - Verify `cargo build --release` succeeds.
  - Ensure backwards compatibility: existing CLI commands without new flags produce identical behavior (defaults match current sequential behavior, plus AI semaphore of 2 is permissive enough to not block sequential processing).
  - _Requirements: R7, R8_

## Notes

- **Task order**: 1→2→3→4→5→6→7→8. Tasks 2, 3, 4 can be done in parallel after task 1.
- **Dependencies**: Task 5 depends on 3+4. Task 6 depends on 2+3+5. Task 7 can be done anytime after task 5.
- **Backwards compatibility critical**: The sequential-for-loop behavior is the default when `cpu_threads = 1` and `max_in_flight = 1`. Default config should preserve current behavior.
- **No async runtime changes**: All pools are synchronous (rayon + std::sync). The existing tokio runtime in storage is unaffected.
- **Performance baseline**: After implementation, 100 simple text files should index in <50% of the time compared to current sequential processing.
