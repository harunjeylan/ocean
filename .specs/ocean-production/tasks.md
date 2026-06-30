# Implementation Plan: Ocean Production Readiness

## Overview

Implement the remaining items from Sections 10–11: runtime modes, structured observability, filesystem sandboxing, read-only mode, and deployment configuration. 7 tasks, each 2–4 hours.

## Pre-requisites

- All Phase 1–9 modules exist and tests pass.
- `ocean-runtime` phase (worker pool, retry, backpressure) is implemented — mode defaults tune these values.
- `ocean-cache` phase is implemented — mode defaults tune cache sizes.
- `OceanConfig` in `ocean_cli::config` exists with its current fields.

## Tasks

- [ ] 1. **Implement `RuntimeMode` enum + mode-based defaults**
  - Define `RuntimeMode` enum with `Desktop`, `Server`, `Embedded` variants in `src/ocean_cli/config.rs` or `src/ocean_index/runtime.rs`.
  - Implement `RuntimeMode::defaults()` returning `ModeDefaults` with tuned thread/cache/batch values per mode.
  - Implement `RuntimeMode::auto_detect()` (heuristic: num_cpus ≤ 2 → Embedded, num_cpus ≥ 16 → Server, else Desktop).
  - Integrate into config resolution in `cmd_index` and `cmd_query`: mode is resolved first, then individual CLI flags override mode defaults.
  - Add `--mode desktop|server|embedded` CLI flag to `IndexArgs` and `QueryArgs`.
  - Add `runtime.mode` field to `OceanConfig` serde struct.
  - Write unit tests: each mode returns correct defaults, auto-detect produces valid mode.
  - _Requirements: R1_

  - [ ] 1.1 Define `RuntimeMode` + `ModeDefaults`
  - [ ] 1.2 Implement `defaults()` and `auto_detect()`
  - [ ] 1.3 CLI flag in `IndexArgs` and `QueryArgs`
  - [ ] 1.4 Config file field
  - [ ] 1.5 Resolution logic (mode → CLI overrides)
  - [ ] 1.6 Unit tests

- [ ] 2. **Implement structured event system**
  - Define `SystemEvent` enum with `IndexStarted`, `IndexComplete`, `FileProcessed`, `QueryExecuted`, `BackpressureEvent`, `ErrorEvent` variants + `Serialize` derive.
  - Define `EventEmitter` trait with `emit()` and `set_output()` methods.
  - Implement `ConsoleEmitter` (human-readable to stderr, same format as current `ConsoleReporter`).
  - Implement `JsonEmitter` (JSON lines to stderr or file).
  - Add `--log-format console|json` and `--log-file <path>` CLI flags.
  - Integrate `EventEmitter` into `IndexOrchestrator`: emit events on start, each file, complete.
  - Integrate `EventEmitter` into `QueryEngine`: emit `QueryExecuted` event.
  - Write unit tests: `JsonEmitter` produces valid JSON lines.
  - _Requirements: R2_

  - [ ] 2.1 `SystemEvent` enum
  - [ ] 2.2 `EventEmitter` trait + `ConsoleEmitter`
  - [ ] 2.3 `JsonEmitter`
  - [ ] 2.4 CLI flags + config
  - [ ] 2.5 Integration into `IndexOrchestrator`
  - [ ] 2.6 Integration into `QueryEngine`
  - [ ] 2.7 Unit tests

- [ ] 3. **Implement metrics counters**
  - Create `MetricsCollector` struct with `AtomicU64` counters for all required metrics.
  - Implement `snapshot() -> MetricsSnapshot` with computed `cache_hit_rate` and `uptime_seconds`.
  - Add `global_metrics()` singleton (using `OnceLock`).
  - Integrate counters into: `IndexOrchestrator` (files indexed/skipped/failed), `QueryEngine` (queries total/cached), `EmbeddingCache` (embedding calls/cached), `GraphCache` (graph expansions).
  - Add `ocean info --metrics` CLI command to display `MetricsSnapshot`.
  - Write unit tests: counters increment correctly, snapshot returns consistent data.
  - _Requirements: R3_

  - [ ] 3.1 `MetricsCollector` + `MetricsSnapshot`
  - [ ] 3.2 Global singleton
  - [ ] 3.3 Integration into pipeline and query engine
  - [ ] 3.4 `ocean info --metrics` display
  - [ ] 3.5 Unit tests

- [ ] 4. **Implement filesystem sandboxing**
  - Define `Sandbox` struct with `workspace_root` and `allowed_extensions`.
  - Implement `Sandbox::validate(path)` using canonicalized path comparison.
  - Define `SecurityError` enum with `Display` + `Error`.
  - Integrate `Sandbox` into `IndexOrchestrator::run()` — validate each file before processing.
  - Integrate `Sandbox` into `FileWatcher` — validate each watch event path.
  - Add `--no-sandbox` CLI flag to `ocean index` and `ocean watch` commands.
  - Add `security.sandbox` config field.
  - Write unit tests: sandbox accepts/allows/rejects correct paths.
  - _Requirements: R4_

  - [ ] 4.1 `Sandbox` struct + `validate()`
  - [ ] 4.2 `SecurityError` enum
  - [ ] 4.3 Integration into `IndexOrchestrator`
  - [ ] 4.4 Integration into `FileWatcher`
  - [ ] 4.5 CLI flags + config
  - [ ] 4.6 Unit tests

- [ ] 5. **Implement read-only mode**
  - Define `ReadOnlyGuard` struct with `AtomicBool`.
  - Integrate into storage write methods: `FileStore::upsert_file`, `ChunkStore::insert_chunk`, `VectorStore::insert`, `GraphStore::insert_node/edge` — check guard before writing.
  - Add `--read-only` CLI flag to `ocean query`.
  - `ocean index`, `ocean scan`, `ocean watch` commands refuse to run in read-only mode.
  - Write unit tests: writes fail in read-only mode, reads succeed.
  - _Requirements: R5_

  - [ ] 5.1 `ReadOnlyGuard` struct
  - [ ] 5.2 Integration into storage sub-stores
  - [ ] 5.3 CLI flags + command blocking
  - [ ] 5.4 Unit tests

- [ ] 6. **Add config show/validate commands**
  - Add `Config` subcommand to `Commands` enum with `Show` and `Validate` variants.
  - Implement `cmd_config_show()`: load and print merged config as JSON.
  - Implement `cmd_config_validate()`: load config file, parse with serde, run `OceanConfig::validate()`.
  - Add `validate()` method to `OceanConfig` that checks field types, ranges, and required paths.
  - Define `ConfigError` enum for validation failures.
  - Write unit tests: valid config passes, invalid config reports specific errors.
  - _Requirements: R7_

  - [ ] 6.1 CLI subcommand definition
  - [ ] 6.2 `cmd_config_show()` — print merged config
  - [ ] 6.3 `cmd_config_validate()` — validate config file
  - [ ] 6.4 `OceanConfig::validate()` method
  - [ ] 6.5 `ConfigError` enum
  - [ ] 6.6 Unit tests

- [ ] 7. **Extend `OceanConfig` with new sections**
  - Add `runtime: Option<RuntimeConfig>` to `OceanConfig` serde struct.
  - Add `security: Option<SecurityConfig>` to `OceanConfig`.
  - Add `observability: Option<ObservabilityConfig>` to `OceanConfig`.
  - Update `resolve_env_vars()` to handle new fields.
  - Update `load()` to merge local + global config for new sections.
  - Ensure backwards compatibility: old config files without new sections still load correctly (all fields are `Option`).
  - Write tests: load config with new sections, load config without new sections (backwards compat).
  - _Requirements: R7_

  - [ ] 7.1 Add serde structs for new sections
  - [ ] 7.2 Update `load()` and merge logic
  - [ ] 7.3 Backwards compatibility test
  - [ ] 7.4 `cargo build` succeeds

- [ ] **Validation & Cleanup**
  - Run full test suite: `cargo test` — all tests must pass.
  - Verify `cargo build --release` succeeds.
  - Manual smoke test: `ocean config show`, `ocean config validate`, `ocean index --mode desktop`, `ocean query --read-only`.
  - Update `cli-docs.md` with new commands and flags.
  - _Requirements: All_

## Notes

- **Task order**: 1→(2,3,4,5,7 in parallel)→6. Task 6 depends on 7.
- **Dependencies on other phases**: Task 1 (runtime modes) tunes values from ocean-runtime and ocean-cache phases. If those aren't implemented, mode defaults adjust the hardcoded values directly.
- **Backwards compatibility**: All new config fields are `Option` — old config files work unchanged.
- **No new major dependencies**: `serde_json` and `serde` are already dependencies.
- **Estimated total**: ~14–20 hours for all 7 tasks.
