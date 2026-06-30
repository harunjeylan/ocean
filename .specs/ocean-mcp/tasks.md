# Implementation Plan: ocean-mcp

## Overview

Implement the MCP server in three phases: foundation (Cargo.toml + module structure + config), tool implementations (all 11 tools + 1 verify grouped into doc/query/graph categories), and integration (resources, server struct, binary entry point, tests). All new code lives under `src/ocean_mcp/`.

## Dependency: Cargo.toml additions

```toml
rmcp = { version = "2", features = ["server", "macros", "transport-io", "schemars"] }
```

No other new dependencies — all Ocean internals are already in the same crate.

## Tasks

### Sub-Phase A: Foundation — Module Structure + Config

- [ ] 1. Create module structure (`src/ocean_mcp/`)
  - `src/ocean_mcp/mod.rs` — `pub mod tools;` `pub mod server;` `pub mod config;` `pub mod resources;` + `pub fn run()`
  - `src/ocean_mcp/config.rs` — `McpConfig` struct with `from_ocean_config()`
  - `src/ocean_mcp/tools/mod.rs` — all parameter structs with `JsonSchema` + `Deserialize`
  - `src/ocean_mcp/tools/doc_tools.rs` — placeholder `mod doc_tools;`
  - `src/ocean_mcp/tools/query_tools.rs` — placeholder
  - `src/ocean_mcp/tools/graph_tools.rs` — placeholder
  - `src/ocean_mcp/server.rs` — `OceanMcpServer` struct (stub, no tools yet)
  - `src/ocean_mcp/resources.rs` — resource handler stub
  - `src/mcp.rs` — thin `fn main() { ocean::ocean_mcp::run(); }`
  - `src/lib.rs` — add `pub mod ocean_mcp;`
  - _Requirements: R1, R13_

  - [ ] 1.1 Create directory `src/ocean_mcp/tools/`
  - [ ] 1.2 Write `src/ocean_mcp/mod.rs` with `run()` that loads config
  - [ ] 1.3 Write `src/ocean_mcp/config.rs` with `McpConfig` + `from_ocean_config()`
  - [ ] 1.4 Write `src/ocean_mcp/tools/mod.rs` with all 12 parameter structs
  - [ ] 1.5 Write `src/mcp.rs` entry point
  - [ ] 1.6 Register `pub mod ocean_mcp;` in `src/lib.rs`

- [ ] 2. Add `rmcp` dependency and `[[bin]]` to `Cargo.toml`
  - Add `rmcp = { version = "2", features = ["server", "macros", "transport-io", "schemars"] }`
  - Add `[[bin]] name = "mcp", path = "src/mcp.rs"`
  - Verify `cargo check --bin mcp` compiles (stub functions only)
  - _Requirements: R1_

### Sub-Phase B: Tool Implementations

- [ ] 3. Implement Tier 1 — Document Tools (`doc_tools.rs`)
  - Each tool is a free async function returning `CallToolResult`
  - Each uses `tokio::task::spawn_blocking` → `ocean_api::docs::*`
  - All use `ocean_api` error types → `CallToolResult::error(...)`
  - _Requirements: R3–R8, R12_

  - [ ] 3.1 `handle_read(params)` — `ocean_api::docs::read_doc()`
  - [ ] 3.2 `handle_search(params)` — `ocean_api::docs::search_doc()`
  - [ ] 3.3 `handle_grep(params)` — `ocean_api::docs::grep_docs()`
  - [ ] 3.4 `handle_info(params)` — `ocean_api::docs::open_doc()` + `outline()`
  - [ ] 3.5 `handle_scan(params)` — `ocean_api::fs::scan_files()`
  - [ ] 3.6 `handle_chunk(params)` — `ocean_api::docs::chunk_doc()`
  - [ ] 3.7 `handle_verify(params)` — `ocean_api::fs::verify_file()`

- [ ] 4. Implement Tier 2 — Query Tools (`query_tools.rs`)
  - _Requirements: R9, R10_

  - [ ] 4.1 `handle_query(params)` — `ocean_api::querying::query()`
        - Map `mode` string → `QueryMode` enum
        - Build `QueryRequest` from params, resolve defaults from config
        - Handle DB not found gracefully → `CallToolResult::error`
  - [ ] 4.2 `handle_vector_status(params)` — check DB accessibility, schema, chunk count, embedder ping
        - Open store → check accessible
        - Query chunk count
        - Attempt test embed → report connection status
        - Never panic on DB failure → report "Not accessible"

- [ ] 5. Implement Tier 3 — Graph Tools (`graph_tools.rs`)
  - _Requirements: R11_

  - [ ] 5.1 `handle_graph_info(params)` — `ocean_api::graph::graph_info()`
  - [ ] 5.2 `handle_graph_expand(params)` — `ocean_api::graph::graph_expand()`
  - [ ] 5.3 `handle_graph_stats(params)` — `ocean_api::graph::graph_stats()`

### Sub-Phase C: Server Integration + Binary

- [ ] 6. Implement `OceanMcpServer` struct + `#[tool_handler]` block (`server.rs`)
  - The `#[tool_handler]` impl block ties all tool methods to their MCP names
  - Each tool method extracts params, calls the `handle_*` function, returns result
  - _Requirements: R2, R14_

  - [ ] 6.1 Add `get_info()` → `ServerInfo { name: "ocean-mcp", version: ... }`
  - [ ] 6.2 Add `read` tool method (with `#[tool(name = "read", ...)]`)
  - [ ] 6.3 Add `search` tool method
  - [ ] 6.4 Add `grep` tool method
  - [ ] 6.5 Add `info` tool method
  - [ ] 6.6 Add `scan` tool method
  - [ ] 6.7 Add `chunk` tool method
  - [ ] 6.8 Add `verify` tool method
  - [ ] 6.9 Add `query` tool method
  - [ ] 6.10 Add `vector_status` tool method
  - [ ] 6.11 Add `graph_info` tool method
  - [ ] 6.12 Add `graph_expand` tool method
  - [ ] 6.13 Add `graph_stats` tool method

- [ ] 7. Implement Resource Handler (`resources.rs`)
  - _Requirements: R15_

  - [ ] 7.1 Parse `document://` URI → file path (URL decode)
  - [ ] 7.2 `read_resource` dispatcher for `document://` scheme
  - [ ] 7.3 IF resource not found → `Err(McpError::resource_not_found())`

- [ ] 8. Wire `run()` function in `mod.rs`
  - Load env files and config
  - Create `OceanMcpServer`
  - Create stdio transport `rmcp::transport::io::stdio()`
  - Serve server via `server.serve(transport).await`
  - _Requirements: R1, R13_

  - [ ] 8.1 Implement config loading chain
  - [ ] 8.2 Implement stdio transport setup
  - [ ] 8.3 Implement graceful shutdown on Ctrl+C
  - [ ] 8.4 Verify `cargo build --bin mcp` compiles end-to-end

- [ ] 9. Write unit tests
  - _Requirements: All_

  - [ ] 9.1 Test `handle_read` with valid/invalid file paths
  - [ ] 9.2 Test `handle_search` with matching/non-matching queries
  - [ ] 9.3 Test `handle_grep` with valid/invalid directories
  - [ ] 9.4 Test `handle_info` with various formats
  - [ ] 9.5 Test `handle_scan` with valid/invalid directories
  - [ ] 9.6 Test `handle_chunk` with valid/invalid config
  - [ ] 9.7 Test `handle_verify` with matching/non-matching hashes
  - [ ] 9.8 Test `handle_query` parameter mapping
  - [ ] 9.9 Test `handle_vector_status` — db unavailable case
  - [ ] 9.10 Test `handle_graph_info` — db unavailable case
  - [ ] 9.11 Test `handle_graph_expand` — parameter mapping
  - [ ] 9.12 Test `handle_graph_stats` — db unavailable case
  - [ ] 9.13 Test resource URI parsing (encode/decode round-trip)
  - [ ] 9.14 Test config loading with mock config files
  - [ ] 9.15 Register all test modules in `src/tests.rs`

- [ ] 10. Integration test — full server lifecycle
  - _Validates: Properties P1–P6_

  - [ ] 10.1 Start `OceanMcpServer`, connect in-process test client
  - [ ] 10.2 Verify `list_tools` returns 12 tools with correct names
  - [ ] 10.3 Verify `call_tool` for each tool with valid params returns `CallToolResult::success`
  - [ ] 10.4 Verify `call_tool` with unknown name returns `Err(McpError)`
  - [ ] 10.5 Verify `read_resource` with valid `document://` URI
  - [ ] 10.6 Verify `read_resource` with missing file returns `Err(McpError)`

- [ ] 11. Checkpoint — Ensure all tests pass
  - Run `cargo test --lib` — all unit tests pass
  - Run `cargo build --bin mcp` — release build succeeds
  - Fix any compilation or test failures

## Notes

### Dependencies
- Task 2 depends on Task 1 (need module structure before Cargo.toml change can compile)
- Tasks 3–5 are independent of each other — can be built in parallel
- Task 6 depends on Tasks 3–5 (server needs tool implementations)
- Task 7 depends on Task 1 (module structure)
- Task 8 depends on Tasks 6 and 7 (binary needs server + resources)
- Tasks 9 and 10 depend on Tasks 3–5
- Task 11 is final validation

### File size estimates
- `src/ocean_mcp/mod.rs`: ~40 lines
- `src/ocean_mcp/config.rs`: ~50 lines
- `src/ocean_mcp/tools/mod.rs`: ~180 lines (12 parameter structs)
- `src/ocean_mcp/tools/doc_tools.rs`: ~200 lines (7 handlers)
- `src/ocean_mcp/tools/query_tools.rs`: ~120 lines (2 handlers)
- `src/ocean_mcp/tools/graph_tools.rs`: ~80 lines (3 handlers)
- `src/ocean_mcp/server.rs`: ~200 lines (12 tool methods + init)
- `src/ocean_mcp/resources.rs`: ~60 lines
- `src/mcp.rs`: ~5 lines
- Tests: ~300 lines across spec files

### Risk Mitigation
- **`rmcp` API surface is large** — use only `#[tool]`, `#[tool_handler]`, `CallToolResult`, `Content::Text`, `Parameters`, `McpError`, `transport::io::stdio()`. Ignore prompts, tasks, sampling, elicitation.
- **`#[tool_handler]` is `!dyn`-compatible** — the server struct cannot be `Box<dyn ServerHandler>`. It must be used as a concrete type. This is fine for the binary target.
- **`schemars` may extend compile times** — acceptable for a release build. Parameter structs are small.
- **Windows stdio** — MCP works with binary stdio on Windows. Test with `npx @modelcontextprotocol/inspector`.
- **`rmcp` v2 uses `#[tool_handler]` on `impl` blocks** — each `#[tool(...)]` method must be `async fn` returning `CallToolResult`. Do NOT mix manual `ServerHandler` methods with the macro.

### Testing approach
- Use `rmcp`'s in-process testing capabilities (service + transport) for integration tests
- Unit tests call `handle_*` functions directly with mock params
- Real file tests use `tests/test-cwd/` fixtures
- Database-dependent tests (`handle_query`, `handle_vector_status`, graph tools) test both "db exists" and "db doesn't exist" paths
