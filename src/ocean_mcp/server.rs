use std::sync::Arc;

use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::ErrorData as McpError;
use rmcp::service::{RequestContext, RoleServer};

use crate::ocean_cli::config::OceanConfig;

use super::config::McpConfig;
use super::resources;
use super::tools;

#[derive(Debug, Clone)]
pub struct OceanMcpServer {
    pub config: McpConfig,
    pub ocean_config: Option<OceanConfig>,
}

impl OceanMcpServer {
    pub fn new(config: McpConfig) -> Self {
        Self {
            ocean_config: OceanConfig::load(),
            config,
        }
    }
}

impl ServerHandler for OceanMcpServer {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_resources()
                .build(),
        )
        .with_server_info(
            Implementation::new("ocean-mcp", env!("CARGO_PKG_VERSION"))
                .with_description("Document intelligence MCP server"),
        )
        .with_instructions(
            "Document intelligence MCP server. Provides tools to read, search, chunk, index, query, and explore document graphs.",
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        // TODO(v2.0): remove experimentals checks — vector and graph graduate to stable
        let vector_enabled = self.ocean_config.as_ref().and_then(|c| c.experimentals.vector).unwrap_or(false);
        let graph_enabled = self.ocean_config.as_ref().and_then(|c| c.experimentals.graph).unwrap_or(false);

        let mut tools = vec![
            Tool::new("read", "Read content from any supported document (PDF, DOCX, XLSX, PPTX, TXT, MD, HTML)", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the document file" },
                    "selector_type": { "type": "string", "enum": ["page", "heading", "paragraph", "table", "slide", "sheet", "cell", "range", "skip"], "description": "Type of selector" },
                    "selector_value": { "type": "string" },
                    "skip": { "type": "integer", "minimum": 0 },
                    "take": { "type": "integer", "minimum": 1 }
                },
                "required": ["file_path"]
            }).as_object().unwrap().clone())),
            Tool::new("search", "Full-text search within a single document file", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the document file" },
                    "query": { "type": "string", "description": "Search query text" }
                },
                "required": ["file_path", "query"]
            }).as_object().unwrap().clone())),
            Tool::new("grep", "Full-text search across all supported documents in a directory", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "directory": { "type": "string", "description": "Directory to search" },
                    "query": { "type": "string", "description": "Search query text" }
                },
                "required": ["directory", "query"]
            }).as_object().unwrap().clone())),
            Tool::new("info", "Get document metadata and outline (table of contents)", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the document file" }
                },
                "required": ["file_path"]
            }).as_object().unwrap().clone())),
            Tool::new("scan", "List all supported documents in a directory", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "directory": { "type": "string", "description": "Directory to scan" },
                    "include_hash": { "type": "boolean", "description": "Include SHA-256 hash" }
                },
                "required": ["directory"]
            }).as_object().unwrap().clone())),
            Tool::new("chunk", "Split a document into semantic chunks with configurable token bounds", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the document file" },
                    "min_size": { "type": "integer", "minimum": 50, "default": 100 },
                    "max_size": { "type": "integer", "minimum": 100, "default": 800 },
                    "overlap": { "type": "integer", "minimum": 0, "default": 1 }
                },
                "required": ["file_path"]
            }).as_object().unwrap().clone())),
            Tool::new("verify", "Compute and verify a file's SHA-256 hash against an expected value", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the file" },
                    "expected_hash": { "type": "string", "description": "Expected SHA-256 hash" }
                },
                "required": ["file_path", "expected_hash"]
            }).as_object().unwrap().clone())),
        ];

        // TODO(v2.0): remove this conditional — vector tools graduate to stable (always include)
        if vector_enabled {
            tools.push(Tool::new("query", "Semantic search over indexed documents (requires 'ocean index' to have run)", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" },
                    "mode": { "type": "string", "enum": ["auto", "vector", "hybrid", "expand"], "default": "auto" },
                    "top_k": { "type": "integer", "minimum": 1, "default": 10 },
                    "expand_depth": { "type": "integer", "minimum": 0, "default": 0 },
                    "include_context": { "type": "boolean", "default": false },
                    "db_path": { "type": "string" },
                    "filter_file_id": { "type": "string" },
                    "filter_heading": { "type": "string" },
                    "filter_block_type": { "type": "string" }
                },
                "required": ["query"]
            }).as_object().unwrap().clone())));
            tools.push(Tool::new("vector_status", "Check vector index health — database access, schema state, chunk count, embedder connectivity", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "db_path": { "type": "string" },
                    "provider": { "type": "string" },
                    "model": { "type": "string" }
                }
            }).as_object().unwrap().clone())));
        }

        // TODO(v2.0): remove this conditional — graph tools graduate to stable (always include)
        if graph_enabled {
            tools.push(Tool::new("graph_info", "Get node/edge counts and type breakdown for a file's subgraph", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the document file" },
                    "db_path": { "type": "string" }
                },
                "required": ["file_path"]
            }).as_object().unwrap().clone())));
            tools.push(Tool::new("graph_expand", "BFS traversal from a seed node, return connected nodes and edges", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "node_id": { "type": "string", "description": "Seed node ID" },
                    "depth": { "type": "integer", "minimum": 1, "default": 2 },
                    "direction": { "type": "string", "enum": ["forward", "backward", "both"], "default": "both" },
                    "db_path": { "type": "string" }
                },
                "required": ["node_id"]
            }).as_object().unwrap().clone())));
            tools.push(Tool::new("graph_stats", "Return global node/edge counts by type", Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "db_path": { "type": "string" }
                }
            }).as_object().unwrap().clone())));
        }
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let name = &*request.name;
        let args = request.arguments.unwrap_or_default();

        // TODO(v2.0): remove experimentals checks — vector and graph graduate to stable
        let vector_enabled = self.ocean_config.as_ref().and_then(|c| c.experimentals.vector).unwrap_or(false);
        let graph_enabled = self.ocean_config.as_ref().and_then(|c| c.experimentals.graph).unwrap_or(false);

        let json_args = serde_json::Value::Object(args);

        // TODO(v2.0): remove experimental_err and all fallthrough error arms below
        let experimental_err = |section: &str| -> McpError {
            McpError::invalid_params(
                format!("This tool is experimental. Enable it by setting \"experimentals\": {{ \"{}\": true }} in .ocean/config.json", section),
                None,
            )
        };

        match name {
            "read" => Ok(tools::doc_tools::handle_read(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await),
            "search" => Ok(tools::doc_tools::handle_search(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await),
            "grep" => Ok(tools::doc_tools::handle_grep(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await),
            "info" => Ok(tools::doc_tools::handle_info(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await),
            "scan" => Ok(tools::doc_tools::handle_scan(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await),
            "chunk" => Ok(tools::doc_tools::handle_chunk(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await),
            "verify" => Ok(tools::doc_tools::handle_verify(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await),
            "query" if vector_enabled => Ok(tools::query_tools::handle_query(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await), // TODO(v2.0): remove guard, always dispatch
            "vector_status" if vector_enabled => Ok(tools::query_tools::handle_vector_status(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await), // TODO(v2.0): remove guard
            "graph_info" if graph_enabled => Ok(tools::graph_tools::handle_graph_info(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await), // TODO(v2.0): remove guard
            "graph_expand" if graph_enabled => Ok(tools::graph_tools::handle_graph_expand(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await), // TODO(v2.0): remove guard
            "graph_stats" if graph_enabled => Ok(tools::graph_tools::handle_graph_stats(serde_json::from_value(json_args).map_err(|e| McpError::invalid_params(format!("Invalid params: {}", e), None))?).await), // TODO(v2.0): remove guard
            "query" | "vector_status" => Err(experimental_err("vector")), // TODO(v2.0): remove fallthrough arms
            "graph_info" | "graph_expand" | "graph_stats" => Err(experimental_err("graph")), // TODO(v2.0): remove fallthrough arms
            _ => Err(McpError::method_not_found::<CallToolRequestMethod>()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(vec![]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        resources::handle_read_resource(request)
    }
}
