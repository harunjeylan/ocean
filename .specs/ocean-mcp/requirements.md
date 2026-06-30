# Requirements Document: ocean-mcp

## Introduction

ocean-mcp is the Model Context Protocol (MCP) server layer for the Ocean Document Runtime. It exposes Ocean's full document intelligence pipeline — reading, searching, chunking, indexing, querying, and graph exploration — as MCP tools that AI assistants (Claude Desktop, Cursor, etc.) can invoke through standardised protocol calls.

Currently, Ocean is only accessible via its CLI (`ocean read`, `ocean query`, etc.). There is no programmatic interface that AI agents can discover and invoke at runtime. The MCP specification (2025-11-25) defines a standard for tool discovery, calling, resource access, and prompt templates. ocean-mcp bridges this gap by wrapping the existing `ocean_api` layer in MCP-compliant tool handlers served over stdio transport.

**Scope:** This phase covers the MCP server implementation — binary entry point, tool definitions for all Tier 1–3 capabilities, resource handlers for document content, config loading, and stdio transport. It does NOT cover Streamable HTTP transport, authentication, or client-side SDK integration (all future scope).

---

## Glossary

- **MCP (Model Context Protocol)**: An open protocol (2025-11-25) that standardises how applications provide context and tools to AI assistants.
- **Tool**: An MCP primitive that represents an invocable function exposed by the server. Tools have names, descriptions, input schemas (JSON Schema), and return structured results.
- **Resource**: An MCP primitive representing data that can be read by the client (e.g., file contents, directory listings). Resources are identified by URI schemes.
- **Prompt**: A pre-defined prompt template that the client can load and present to the user.
- **Transport**: The communication layer between MCP client and server. `stdio` transport uses stdin/stdout; `streamable-http` uses Server-Sent Events.
- **ServerHandler**: The `rmcp` trait that MCP server developers implement to handle `list_tools`, `call_tool`, `list_resources`, `read_resource`, etc.
- **`rmcp`**: The official Rust SDK for MCP (v2.0.0+), providing types, transport, handler traits, and procedural macros (`#[tool]`, `#[tool_handler]`, `#[tool_router]`).
- **Tier 1 tools**: Document reading/searching tools that work without any indexing (just parse + read on-the-fly).
- **Tier 2 tools**: Indexed/semantic tools that require an existing SurrealDB index built by `ocean index`.
- **Tier 3 tools**: Graph exploration tools that require an indexed graph store.
- **`ocean_api`**: The existing synchronous public API layer (`src/ocean_api/`), which wraps all underlying modules (parser, chunker, vector, graph, query) into simple function calls.
- **Content::Text**: The `rmcp` model variant for returning textual content from a tool call. Each tool returns `CallToolResult::success(vec![Content::Text(...)])`.

---

## Requirements

### Requirement R1: Binary Entry Point

**User Story:** As a user, I want to start the MCP server by running a single binary (`ocean mcp`) so that AI assistants can connect to it via stdio.

#### Acceptance Criteria

1. THE system SHALL provide a `[[bin]]` target named `mcp` in Cargo.toml with entry point at `src/mcp.rs`.
2. WHEN invoked with no arguments, the binary SHALL start an MCP server on stdio transport and listen for client messages.
3. THE binary SHALL load Ocean configuration (`OceanConfig::load()`) and MCP-specific config on startup.
4. THE server SHALL NOT exit immediately — it SHALL block until the client disconnects or sends a shutdown signal.
5. THE server SHALL log startup info (version, config status) to stderr (not stdout, which is the MCP transport).

---

### Requirement R2: ServerHandler Implementation

**User Story:** As an AI assistant client, I want the server to correctly implement the MCP handshake and capability negotiation so that I can discover its tools.

#### Acceptance Criteria

1. THE system SHALL implement `rmcp::handler::server::ServerHandler` for the server struct.
2. THE `initialize` method SHALL return server info with name `"ocean-mcp"`, version matching the crate version, and capability flags for `tools`, `resources`, and optionally `prompts`.
3. THE `list_tools` method SHALL return all registered tool definitions with names, descriptions, and JSON Schema input parameters.
4. THE `call_tool` method SHALL dispatch to the correct handler based on the tool name and return `CallToolResult`.
5. The default `ping`, `complete`, `set_level`, `subscribe`, `unsubscribe` implementations SHALL be sufficient (no custom behaviour needed).

---

### Requirement R3: Tier 1 — Document Read Tool

**User Story:** As an AI assistant, I want to read content from any supported document file so that I can answer questions about its contents.

#### Acceptance Criteria

1. THE tool SHALL be named `read` with description `"Read content from any supported document (PDF, DOCX, XLSX, PPTX, TXT, MD, HTML)"`.
2. THE tool SHALL accept parameters: `file_path` (string, required), `selector_type` (string, optional, one of: `page`, `heading`, `paragraph`, `table`, `slide`, `sheet`, `cell`, `range`, `skip`), `selector_value` (string, optional), `skip` (integer, optional), `take` (integer, optional).
3. IF `file_path` does not exist, THE tool SHALL return `CallToolResult::error(...)` with a descriptive message.
4. IF the file format is unsupported, THE tool SHALL return a clear error.
5. ON success, THE tool SHALL return the document content as `Content::Text(...)`.
6. THE tool SHALL call `ocean_api::docs::read_doc()` internally.

---

### Requirement R4: Tier 1 — Search Tool

**User Story:** As an AI assistant, I want to search for a phrase within a single document so that I can find relevant passages.

#### Acceptance Criteria

1. THE tool SHALL be named `search` with description `"Full-text search within a single document file"`.
2. Parameters: `file_path` (string, required), `query` (string, required).
3. IF no matches found, SHALL return a message indicating zero results.
4. ON success, SHALL return match locations with surrounding context text.
5. THE tool SHALL call `ocean_api::docs::search_doc()` internally.

---

### Requirement R5: Tier 1 — Grep Tool

**User Story:** As an AI assistant, I want to search for a phrase across all supported documents in a directory so that I can find which files contain relevant information.

#### Acceptance Criteria

1. THE tool SHALL be named `grep` with description `"Full-text search across all supported documents in a directory"`.
2. Parameters: `directory` (string, required), `query` (string, required).
3. IF the directory does not exist, SHALL return an error.
4. ON success, SHALL return per-file match summaries with match counts.
5. THE tool SHALL call `ocean_api::docs::grep_docs()` internally.

---

### Requirement R6: Tier 1 — Info Tool

**User Story:** As an AI assistant, I want to get metadata and outline (table of contents) of a document so that I can understand its structure before reading.

#### Acceptance Criteria

1. THE tool SHALL be named `info` with description `"Get document metadata and outline (table of contents)"`.
2. Parameters: `file_path` (string, required).
3. ON success, SHALL return formatted metadata fields (title, author, page count, etc.) plus the hierarchical outline.
4. THE tool SHALL call `ocean_api::docs::open_doc()` and `ocean_api::docs::outline()` internally.

---

### Requirement R7: Tier 1 — Scan Tool

**User Story:** As an AI assistant, I want to list all supported documents in a directory so that I can discover what files are available.

#### Acceptance Criteria

1. THE tool SHALL be named `scan` with description `"List all supported documents in a directory"`.
2. Parameters: `directory` (string, required), `include_hash` (boolean, optional, default `false`).
3. ON success, SHALL return a formatted list of files with size, extension, and optionally SHA-256 hash.
4. THE tool SHALL call `ocean_api::fs::scan_files()` internally.

---

### Requirement R8: Tier 1 — Chunk Tool

**User Story:** As an AI assistant, I want to split a document into semantic chunks so that I can understand its topical structure and section boundaries.

#### Acceptance Criteria

1. THE tool SHALL be named `chunk` with description `"Split a document into semantic chunks with configurable token bounds"`.
2. Parameters: `file_path` (string, required), `min_size` (integer, optional, default 100), `max_size` (integer, optional, default 800), `overlap` (integer, optional, default 1).
3. ON success, SHALL return each chunk with its ID, type, heading context, content, and token estimate.
4. THE tool SHALL call `ocean_api::docs::chunk_doc()` internally.

---

### Requirement R9: Tier 2 — Query Tool

**User Story:** As an AI assistant, I want to perform semantic/hybrid/expand queries over an indexed document corpus so that I can find relevant information beyond keyword matching.

#### Acceptance Criteria

1. THE tool SHALL be named `query` with description `"Semantic search over indexed documents (requires 'ocean index' to have run)"`.
2. Parameters: `query` (string, required), `mode` (string, optional, one of `auto`, `vector`, `hybrid`, `expand`, default `auto`), `top_k` (integer, optional, default 10), `expand_depth` (integer, optional, default 0), `include_context` (boolean, optional, default `false`), `db_path` (string, optional), `provider` (string, optional), `model` (string, optional), `filter_file_id` (string, optional), `filter_heading` (string, optional), `filter_block_type` (string, optional).
3. IF the database is inaccessible or empty, SHALL return a helpful error suggesting to run `ocean index`.
4. ON success, SHALL return ranked results with scores, file IDs, heading context, content excerpts, and optional context windows.
5. THE tool SHALL call `ocean_api::querying::query()` internally.

---

### Requirement R10: Tier 2 — Vector Status Tool

**User Story:** As an AI assistant or user, I want to check whether the vector index is healthy (accessible, populated, embedder reachable) before attempting a query.

#### Acceptance Criteria

1. THE tool SHALL be named `vector_status` with description `"Check vector index health — database access, schema state, chunk count, embedder connectivity"`.
2. Parameters: `db_path` (string, optional), `provider` (string, optional), `model` (string, optional).
3. ON success, SHALL return a formatted health report including: database accessible (yes/no), schema initialised (yes/no), indexed chunk count, embedder model/dimension, connection status (OK/FAILED).
4. THE tool SHALL NOT crash if the database doesn't exist — SHALL report "Not accessible".
5. THE tool SHALL attempt a live test embed to verify embedder connectivity.

---

### Requirement R11: Tier 3 — Graph Tools

**User Story:** As an AI assistant, I want to explore the knowledge graph built during indexing so that I can understand relationships between documents, chunks, headings, and entities.

#### Acceptance Criteria

1. THE system SHALL expose three graph tools: `graph_info`, `graph_expand`, `graph_stats`.
2. **`graph_info(file_path, db_path?)`** — Return node/edge counts and type breakdown for a file's subgraph.
3. **`graph_expand(node_id, depth?, direction?, db_path?)`** — BFS traversal from a seed node, return connected nodes and edges.
4. **`graph_stats(db_path?)`** — Return global node/edge counts by type.
5. All three SHALL call `ocean_api::graph::*` functions internally.
6. IF the graph store is inaccessible, SHALL return a descriptive error.

---

### Requirement R12: Tier 1 — Verify Tool

**User Story:** As an AI assistant, I want to verify a file's SHA-256 hash against an expected value so that I can check file integrity.

#### Acceptance Criteria

1. THE tool SHALL be named `verify` with description `"Compute and verify a file's SHA-256 hash against an expected value"`.
2. Parameters: `file_path` (string, required), `expected_hash` (string, required).
3. ON success, SHALL return `"true"` or `"false"` indicating whether the hash matches.
4. IF the file does not exist, SHALL return an error.

---

### Requirement R13: Config and Setup

**User Story:** As a user, I want the MCP server to pick up my existing Ocean configuration (embedding provider, model, DB path, API keys) so that I don't have to reconfigure.

#### Acceptance Criteria

1. THE server SHALL load `.ocean/config.json` and `~/.ocean/config.json` using `OceanConfig::load()` on startup.
2. THE server SHALL load `.env` files using `load_env_files()` before reading config.
3. THE server SHALL support `${VAR}` environment variable syntax in config values.
4. Config values SHALL be injectable via environment variables for tools that accept optional connection parameters.
5. The default DB path SHALL follow the existing convention: `~/.ocean/database/{cwd-kebab-case}.db`.

---

### Requirement R14: Error Handling

**User Story:** As an AI assistant, I want clear, structured error responses so that I can diagnose and report problems to the user.

#### Acceptance Criteria

1. ALL tool handlers SHALL return `CallToolResult::error(...)` for user-facing failures (bad file paths, missing index, embedder unreachable).
2. ONLY protocol-level issues (unknown tool name, malformed request) SHALL return `Err(McpError)`.
3. Error messages SHALL be human-readable and suggest remediation where possible (e.g., "File not found: ...", "Database not initialised. Run 'ocean index .' first.").
4. The server SHALL NOT panic on any input — all code paths SHALL return a `Result`.
5. Panic hooks SHALL catch any unexpected panics and convert them to error responses.

---

### Requirement R15: MCP Resources

**User Story:** As an AI assistant, I want to read document contents via MCP resources (with `document://` URIs) so that I can fetch content without calling a tool.

#### Acceptance Criteria

1. THE system SHALL expose a `document://{path}` resource URI scheme where `{path}` is a URL-encoded file path.
2. THE `list_resources` method SHALL NOT enumerate all documents (could be many files) — SHALL return empty.
3. THE `read_resource` method SHALL read the file content when a `document://` URI is provided.
4. THE resource content SHALL be text (not binary) for supported document formats.
5. IF the file does not exist or the URI scheme is unknown, SHALL return `Err(McpError::resource_not_found())`.

---

### Requirement R16: Prompts (Optional)

**User Story:** As a user, I want pre-built prompt templates for common document tasks so that I don't have to craft prompts from scratch.

#### Acceptance Criteria

1. THE system MAY expose a `summarize-document` prompt that takes a `file_path` argument.
2. THE system MAY expose an `analyze-document` prompt that takes `file_path` and optional `focus` arguments.
3. Implementations SHALL be trivial — just a template string with `{{file_path}}` placeholders.
4. IF prompts are not implemented, `list_prompts` SHALL return an empty list (no error).
