# Implementation Plan: ocean-graph

## Overview

Implement the graph relationship layer: data types (Node, Edge, NodeType, RelationType), SurrealDB-backed GraphStore, GraphBuilder (structural + reference + entity extraction), ExpansionEngine (BFS/DFS traversal), and CLI integration. All new code lives under `src/ocean_graph/`. Graph building runs as part of the index pipeline after vector indexing.

## Dependencies

No new Cargo dependencies. `surrealdb`, `tokio`, `serde`, `serde_json`, `sha2`, `chrono` are already present from ocean-vector. The `reqwest` dependency may be needed later for remote entity extraction, but not for MVP.

## Tasks

### Sub-Phase A: Foundation — Types and Module Structure

- [ ] 1. Create module structure (`src/ocean_graph/`)
  - `mod.rs` — `pub mod types;` + `pub mod store;` + `pub mod builder;` + `pub mod entity;` + `pub mod expansion;` + `pub mod error;` + `pub use` re-exports
  - `types.rs` — `Node`, `Edge`, `NodeType`, `RelationType`, `Subgraph`, `GraphConfig`
  - `store.rs` — `GraphStore` (SurrealDB-backed)
  - `builder.rs` — `GraphBuilder`
  - `entity.rs` — `EntityExtractor`
  - `expansion.rs` — `ExpansionEngine`
  - `error.rs` — `GraphError`

  _Requirements: R1, R2, R10_

  - [ ] 1.1 Define `NodeType` enum (File, Chunk, Heading, Entity, Folder)
  - [ ] 1.2 Define `RelationType` enum (Contains, References, Mentions, BelongsTo, DerivedFrom, SimilarTo, CrossReference)
  - [ ] 1.3 Define `Node` struct with `id`, `node_type`, `ref_id`, `label`
  - [ ] 1.4 Define `Edge` struct with `from`, `to`, `relation`, `weight`, `metadata`
  - [ ] 1.5 Define `Subgraph` struct with `seed_id`, `nodes`, `edges`, `depth`
  - [ ] 1.6 Define `GraphConfig` with defaults (extract_references: true, extract_entities: true, max_expansion_depth: 3, entity_min_frequency: 3, default_edge_weight: 1.0)
  - [ ] 1.7 Derive `Clone, Debug, PartialEq, Serialize, Deserialize` on all data types
  - [ ] 1.8 Write unit tests: type creation, serialization roundtrip, config defaults

  _Requirements: R1, R10_

### Sub-Phase B: GraphStore — SurrealDB Persistence

- [ ] 2. Implement `GraphStore` in `store.rs`
  - Follow exact same pattern as `ocean_vector::store::VectorStore`:
    - `new_memory()` — `Surreal::new::<Mem>(())` for tests
    - `new_persistent(path)` — `Surreal::new::<SurrealKv>(path)` for production
    - Sync-to-async bridge via `tokio::runtime::Runtime`
    - Namespace: `ocean`, DB name: `ocean`

  _Requirements: R2_

  - [ ] 2.1 Implement `initialize_schema()` — run SurrealQL to create `graph_node` and `graph_edge` tables with all fields and indexes (see design.md schema)
  - [ ] 2.2 Implement `insert_node(&self, node: Node)` — insert into `graph_node` table using `db.create(("graph_node", &node.id)).content(node)`
  - [ ] 2.3 Implement `insert_edge(&self, edge: Edge)` — insert into `graph_edge` table using `db.create(("graph_edge", &edge_id)).content(edge)`
  - [ ] 2.4 Implement `insert_nodes_batch(&self, nodes: Vec<Node>)` — loop with create (or batch SurrealQL)
  - [ ] 2.5 Implement `insert_edges_batch(&self, edges: Vec<Edge>)` — loop with create
  - [ ] 2.6 Implement `get_node(&self, id: &str)` — `SELECT * FROM graph_node WHERE id = $id`
  - [ ] 2.7 Implement `get_node_by_ref(&self, ref_id: &str)` — `SELECT * FROM graph_node WHERE ref_id = $ref_id`
  - [ ] 2.8 Implement `get_neighbors(&self, node_id: &str)` — query both outgoing (`from_id = $id`) and incoming (`to_id = $id`) edges, fetch connected nodes, return `Vec<(Node, Edge)>`
  - [ ] 2.9 Implement `get_edges(&self, node_id: &str, direction: EdgeDirection)` — filtered query on `from_id` or `to_id`
  - [ ] 2.10 Implement `get_nodes_by_type(&self, node_type: NodeType)` — `SELECT * FROM graph_node WHERE node_type = $t`
  - [ ] 2.11 Implement `get_edges_by_relation(&self, relation: RelationType)` — `SELECT * FROM graph_edge WHERE relation = $r`
  - [ ] 2.12 Implement `delete_nodes_by_file(&self, file_id: &str)` — `DELETE graph_node WHERE file_id = $fid`
  - [ ] 2.13 Implement `delete_edges_by_file(&self, file_id: &str)` — `DELETE graph_edge WHERE file_id = $fid`
  - [ ] 2.14 Implement `count_nodes()` and `count_edges()` — `SELECT count() ... GROUP BY count`
  - [ ] 2.15 Implement `clear()` — `DELETE graph_node; DELETE graph_edge;`

  _Requirements: R2_

  - [ ] 2.16 Write integration tests (in-memory SurrealDB):
    - Insert node → get node roundtrip
    - Insert edge → get edge roundtrip
    - Batch insert nodes/edges
    - Neighbors query (outgoing + incoming + both)
    - Node type query
    - Delete by file removes correct nodes/edges
    - Count returns correct totals
    - Clear removes everything
    - Schema idempotency (initialize_schema called twice)

### Sub-Phase C: GraphBuilder — Structural + Reference + Entity Edges

- [ ] 3. Implement `GraphBuilder` in `builder.rs`

  _Requirements: R3, R4_

  - [ ] 3.1 Implement `structural(chunks, file_id)`:
    - Create File node: `Node { id: format!("file:{}", file_id), node_type: File, ref_id: file_id, label: None }`
    - For each chunk: create Chunk node, edge `File ─Contains─→ Chunk`, edge `Chunk ─BelongsTo─→ File`
    - For each chunk with heading: create/generate Heading node (deduplicated by heading_id), edge `Chunk ─BelongsTo─→ Heading`
    - Heading ID: `format!("heading:{}:{}", file_id, sha256(heading_text))`
    - Return `(Vec<Node>, Vec<Edge>)`

  - [ ] 3.2 Implement `from_chunks(chunks, file_id, config)`:
    - Call `structural(chunks, file_id)`
    - If `config.extract_references`: call `extract_references(chunks, nodes)` and merge edges
    - If `config.extract_entities`: call `EntityExtractor`, create entity nodes/edges, merge
    - Return combined result

  - [ ] 3.3 Implement `extract_references(chunks, nodes)`:
    - Regex patterns: `(?i)(?:see|refer to|as per|per)\s+["'""]?([A-Z][A-Za-z0-9\s]+)`, URLs, quoted document titles
    - For each match, create edge: `Chunk ─References─→ TargetNode` with weight 0.7
    - If target node doesn't exist, create a best-effort label edge with metadata describing the reference text
    - Return `Vec<Edge>`

  _Requirements: R3, R4_

  - [ ] 3.4 Write unit tests for `structural`:
    - Empty chunks: returns File node only, no edges
    - Single chunk with heading: produces File node, 1 Chunk node, 1 Heading node, 3 edges
    - Multiple chunks under same heading: single Heading node, each chunk gets BelongsTo edge
    - Multiple files produce separate subgraphs
    - Deterministic node IDs for same input

  - [ ] 3.5 Write unit tests for `extract_references`:
    - Detects "see Policy Document" pattern
    - Detects URL references
    - Creates edge even when target doesn't exist (tolerance)
    - No matches produces empty edge list

### Sub-Phase D: EntityExtractor — Heuristic Entity Detection

- [ ] 4. Implement `EntityExtractor` in `entity.rs`

  _Requirements: R4_

  - [ ] 4.1 Implement `extract_capitalized(text)`: find sequences of 3+ capitalized words (e.g., `"Human Resources Department"`)
  - [ ] 4.2 Implement `extract_repeated(content_by_chunk, min_freq)`: collect all nouns (split by whitespace, filter by length > 3), count frequency across all chunks, return those appearing >= `min_freq` times
  - [ ] 4.3 Implement `extract(text, min_freq)`: combine results from both methods, deduplicate, return `Vec<String>`

  - [ ] 4.4 Write unit tests:
    - Extracts "Human Resources Department" from text containing it
    - Returns empty for text with no capitalized phrases
    - Repeated word frequency threshold correct
    - Deduplication works (same entity not returned twice)
    - Case-insensitive dedup (e.g., "HR Department" and "Hr Department")

### Sub-Phase E: ExpansionEngine — Graph Traversal

- [ ] 5. Implement `ExpansionEngine` in `expansion.rs`

  _Requirements: R5, R6_

  - [ ] 5.1 Implement `expand(node_id, depth, direction)`:
    - BFS traversal using `GraphStore::get_neighbors`
    - Track visited nodes to avoid cycles
    - Stop when depth reached or queue empty
    - Return `Subgraph { seed_id, nodes, edges, depth }`
    - Validate: depth must be 1..=5, error `InvalidDepth` otherwise

  - [ ] 5.2 Implement `expand_from_chunks(chunk_ids, depth)`:
    - Expand from multiple seed chunk IDs
    - Merge results (deduplicate by node_id)
    - Return combined `Subgraph`

  - [ ] 5.3 Implement `find_path(from_id, to_id, max_depth)`:
    - BFS shortest path algorithm
    - Track parent map for path reconstruction
    - Return `Option<Vec<Edge>>` — ordered list of edges forming the path
    - If no path found within max_depth, return `None`

  - [ ] 5.4 Implement `get_file_graph(file_id)`:
    - Query all nodes and edges with matching `file_id`
    - Return `Subgraph { seed_id: format!("file:{}", file_id), nodes, edges, depth: 0 }`

  _Requirements: R5, R6_

  - [ ] 5.5 Write integration tests:
    - Build a small graph → expand from a node → verify correct depth
    - Expand with depth 1 returns only direct neighbors
    - Expand with depth 2 returns neighbors-of-neighbors
    - Deduplication: expanding from two connected nodes returns unique set
    - Cycle safety: graph with cycles doesn't cause infinite loop
    - `find_path` returns correct path between connected nodes
    - `find_path` returns None for disconnected nodes
    - `get_file_graph` returns complete subgraph for a file

### Sub-Phase F: CLI Integration

- [ ] 6. Add graph CLI commands

  _Requirements: R7_

  - [ ] 6.1 Add `GraphArgs` struct to `ocean_cli::args.rs`:
    - `Commands::Graph(GraphArgs)` variant
    - `GraphCommands` enum: `Info`, `Expand`, `Path`, `Stats`
    - `Info`: `file <path>`
    - `Expand`: `node_id <id> [--depth N] [--direction forward|backward|both]`
    - `Path`: `from <from-id> <to-id> [--max-depth N]`
    - `Stats`: no args

  - [ ] 6.2 Implement `cmd_graph_info` in `ocean_cli::run.rs`:
    - Initialize GraphStore (persistent at default path or `--db-path`)
    - Find file node by scanning or using `get_node_by_ref`
    - Get file subgraph via `get_file_graph`
    - Display: node count, edge count, breakdown by node type

  - [ ] 6.3 Implement `cmd_graph_expand`:
    - Initialize GraphStore + ExpansionEngine
    - Call `expand(node_id, depth, direction)`
    - Display: seed node info, then each reachable node with its edges
    - Format: `[Node] <type> <id> <label>` then indented edges `─<relation>→ <target> (weight: <w>)`

  - [ ] 6.4 Implement `cmd_graph_path`:
    - Initialize GraphStore + ExpansionEngine
    - Call `find_path(from_id, to_id, max_depth)`
    - Display path as ordered edge list or "No path found within N hops"

  - [ ] 6.5 Implement `cmd_graph_stats`:
    - Initialize GraphStore
    - Query node count, edge count, group-by-type counts
    - Display as a table

  - [ ] 6.6 Register `Graph` command in `ocean_cli::args.rs` `Commands` enum
  - [ ] 6.7 Add dispatch case in `ocean_cli::run.rs` `run()` function

  _Requirements: R7_

### Sub-Phase G: Index Pipeline Integration

- [ ] 7. Integrate graph building into index pipeline

  _Requirements: R8_

  - [ ] 7.1 Extend `IndexReport` in `ocean_vector::pipeline.rs` with fields:
    - `graph_nodes: usize`
    - `graph_edges: usize`

  - [ ] 7.2 In `cmd_index` (or the pipeline orchestrator):
    - After vector indexing completes, initialize `GraphBuilder` and `GraphStore`
    - Call `GraphBuilder::from_chunks(chunks, file_id, &config)` with a `GraphConfig`
    - Call `GraphStore::insert_nodes_batch(nodes)` and `GraphStore::insert_edges_batch(edges)`
    - On `--reindex`: call `delete_nodes_by_file` and `delete_edges_by_file` before rebuilding

  - [ ] 7.3 Add graph CLI args to the `index` command:
    - `--no-graph` — skip graph building (default: graph is built)
    - `--no-references` — disable reference extraction
    - `--no-entities` — disable entity extraction

  - [ ] 7.4 Update display: `cmd_index` output shows graph counts alongside vector counts

  _Requirements: R8_

### Sub-Phase H: Graph + Vector Integration

- [ ] 8. Implement context expansion for search results

  _Requirements: R9_

  - [ ] 8.1 Add `expand_results` method to `SearchEngine` (or a new `GraphSearchEnricher`):
    - Take vector search results (containing chunk_ids)
    - For each chunk_id, call `ExpansionEngine::expand(chunk_node_id, depth=1, Both)`
    - Collect all chunk nodes from expanded subgraphs
    - Merge with original results (dedup by chunk_id)
    - Compute combined score: `0.7 * vector_score + 0.3 * (1.0 / (1.0 + hop_distance))`

  - [ ] 8.2 Add `--expand` flag to the `ocean search` CLI command:
    - `--expand-depth N` (default 0 = no expansion)
    - When set, runs graph expansion after vector search and shows enriched results

  - [ ] 8.3 Display: expanded results include a `graph_score` column and indicate which results came from graph expansion vs. vector only

  _Requirements: R9_

### Sub-Phase I: Integration Tests

- [ ] 9. End-to-end integration tests

  _Requirements: R1–R9_

  - [ ] 9.1 Full pipeline test: scan dir → parse files → chunk → index vectors → build graph → search → expand
  - [ ] 9.2 Use in-memory SurrealDB and mock embedder (from ocean-vector tests)
  - [ ] 9.3 Verify: graph contains correct nodes and edges for each test file
  - [ ] 9.4 Verify: `graph expand` returns expected neighbors
  - [ ] 9.5 Verify: `graph path` returns correct path
  - [ ] 9.6 Verify: reindex produces identical graph state
  - [ ] 9.7 Verify: deleting a file removes its graph nodes/edges

### Sub-Phase J: Registration and Documentation

- [ ] 10. Register module and update docs

  - [ ] 10.1 Add `pub mod ocean_graph;` to `src/lib.rs`
  - [ ] 10.2 Register test files in `src/tests.rs`:
    ```rust
    #[path = "ocean_graph/store_test.rs"]
    mod graph_store_spec;
    #[path = "ocean_graph/builder_test.rs"]
    mod graph_builder_spec;
    #[path = "ocean_graph/entity_test.rs"]
    mod graph_entity_spec;
    #[path = "ocean_graph/expansion_test.rs"]
    mod graph_expansion_spec;
    ```
  - [ ] 10.3 Update `AGENTS.md` with ocean-graph module overview and commands
  - [ ] 10.4 Update `cli-docs.md` with `graph` and `search --expand` commands

## Notes

- The `GraphStore` must share the same SurrealDB instance as `VectorStore` when used in the same pipeline. Use a shared `Surreal<Db>` connection or construct both from the same path.
- Node IDs must be deterministic: same chunks → same graph state. Use `sha256` for heading/entity ID generation.
- The BFS expansion must handle cycles gracefully via a `visited` set.
- Entity extraction is heuristic and best-effort. The `EntityExtractor` module is designed for future replacement with NLP.
- Reference extraction uses simple regex patterns. These may produce false positives; the `metadata` field on edges can store the matched text for debugging.
- Edge weights default to 1.0. References edges get 0.7, Mentions edges get 0.5.
- The `--db-path` flag for graph CLI commands should default to `ocean.db` (same default as ocean-vector).
- For the `graph info` command, file lookup uses `get_node_by_ref(file_id)` — the scanner from `ocean_fs` provides file_id → path mapping.

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Entity extraction produces too many low-quality entities | Configurable `entity_min_frequency` (default 3); entities are cheap noise but can be filtered |
| Graph expansion depth causes performance issues | Hard limit max_depth=5; BFS is bounded by visited set; indexes on from_id/to_id |
| Reference regex matches unrelated text | Conservative patterns; metadata field captures matched text for audit |
| SurrealDB insert performance with many edges | Batch inserts; indexes on from_id/to_id enable fast neighbor lookup |
| Circular references cause infinite expansion | `visited` set in BFS; max_depth hard cap; `CycleDetected` error as safety net |
| Memory with large graphs (>100k nodes) | Only materialize subgraph for expansion result; no full-graph loads |
