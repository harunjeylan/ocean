# Requirements Document: ocean-graph

## Introduction

ocean-graph is the structural relationship layer of the Ocean DRT. It builds a knowledge graph connecting files, chunks, headings, and entities — enabling cross-document navigation, context expansion, and dependency discovery. The graph is built AFTER chunking and vector indexing, operating on the structured output of ocean-chunk and storing nodes/edges in SurrealDB.

SurrealDB serves as the unified graph backend using `nodes` and `edges` tables with SCHEMAFULL definitions. This keeps the architecture consistent with ocean-vector (same engine, same sync-to-async pattern) while providing indexed neighbor lookups for fast traversal.

**Scope:** This phase covers the graph store (nodes + edges in SurrealDB), graph builder (structural + reference + entity extraction), expansion engine (DFS/BFS traversal), CLI commands, and integration into the index pipeline. It does NOT cover the final query orchestrator (ocean-query) which will combine vector + graph results.

---

## Glossary

- **Node**: A vertex in the graph. Represents a File, Chunk, Heading, Entity, or Folder.
- **Edge**: A directed, typed relationship between two nodes (e.g., `contains`, `references`, `belongs_to`).
- **Structural edge**: An edge derived from chunk metadata (file membership, heading hierarchy).
- **Reference edge**: An edge derived from text content (cross-document mentions, hyperlinks, citations).
- **Entity edge**: An edge connecting a chunk to an extracted entity (capitalized phrase, repeated term).
- **Graph expansion**: Starting from a seed node, traverse edges to discover connected nodes up to a depth limit.
- **Subgraph**: A subset of the graph containing nodes and edges reachable from a set of seed nodes.
- **GraphStore**: SurrealDB-backed persistence layer for nodes and edges, following the same pattern as VectorStore.
- **GraphBuilder**: Component that analyzes chunks and produces nodes + edges.
- **ExpansionEngine**: Component that traverses the graph to find connected knowledge.

---

## Requirements

### Requirement 1: Graph Node and Edge Types

**User Story:** As the system, I need well-defined node and edge types so that the graph can represent document structure and cross-document relationships.

#### Acceptance Criteria

1. THE system SHALL define a `NodeType` enum with variants: `File`, `Chunk`, `Heading`, `Entity`, `Folder`.
2. THE system SHALL define a `RelationType` enum with variants: `Contains`, `References`, `Mentions`, `BelongsTo`, `DerivedFrom`, `SimilarTo`, `CrossReference`.
3. THE system SHALL define a `Node` struct with fields: `id: String`, `node_type: NodeType`, `ref_id: String`, `label: Option<String>`.
4. THE system SHALL define an `Edge` struct with fields: `from: String`, `to: String`, `relation: RelationType`, `weight: f32`, `metadata: Option<String>`.
5. ALL types SHALL implement `Clone`, `Debug`, `PartialEq`, `Serialize`, `Deserialize`.

---

### Requirement 2: SurrealDB Graph Store

**User Story:** As the system, I need a persistent graph store that can hold nodes and edges with fast neighbor lookup, so that graph traversal works at scale.

#### Acceptance Criteria

1. THE system SHALL provide a `GraphStore` struct backed by SurrealDB (same engine pattern as `VectorStore`).
2. IT SHALL use two SurrealDB tables: `graph_node` and `graph_edge`, both SCHEMAFULL.
3. IT SHALL define fields on `graph_node`: `id` (string), `node_type` (string), `ref_id` (string), `label` (option<string>), `file_id` (string), `created_at` (int).
4. IT SHALL define fields on `graph_edge`: `from_id` (string), `to_id` (string), `relation` (string), `weight` (float), `metadata` (option<string>).
5. IT SHALL create indexes: `idx_edge_from` on `from_id`, `idx_edge_to` on `to_id`, `idx_node_file` on `file_id`.
6. IT SHALL support both in-memory (`Mem` for tests) and persistent (`SurrealKv` for production) backends.
7. IT SHALL provide CRUD methods:
   - `insert_node(node)`, `insert_edge(edge)`
   - `insert_nodes_batch(nodes)`, `insert_edges_batch(edges)`
   - `get_node(id)`, `get_edge(id)`
   - `delete_nodes_by_file(file_id)`, `delete_edges_by_file(file_id)`
   - `get_neighbors(node_id)` — returns connected nodes with edge info
   - `get_edges(node_id, direction)` — returns edges where node is from/to
   - `count_nodes()`, `count_edges()`
   - `clear()` — remove all nodes and edges

---

### Requirement 3: Graph Builder — Structural Edges

**User Story:** As the system, I want to automatically build structural graph edges from chunk metadata so that the document hierarchy is captured in the graph.

#### Acceptance Criteria

1. THE system SHALL provide `GraphBuilder::from_chunks(chunks: &[Chunk], file_id: &str) -> (Vec<Node>, Vec<Edge>)`.
2. IT SHALL create one `Node::File` per unique `file_id`.
3. IT SHALL create one `Node::Chunk` per chunk.
4. IT SHALL create one `Node::Heading` per unique heading in the chunk set.
5. IT SHALL create edges:
   - `File` → `Contains` → `Chunk` (for each chunk in the file)
   - `Chunk` → `BelongsTo` → `Heading` (if chunk has a heading)
   - `Chunk` → `BelongsTo` → `File` (chunk belongs to its file)
6. IT SHALL generate stable node IDs: `"file:<file_id>"`, `"chunk:<chunk_id>"`, `"heading:<file_id>:<heading_text>"`.

---

### Requirement 4: Graph Builder — Reference and Entity Edges

**User Story:** As the system, I want to discover cross-document references and extract named entities from chunk content so that the graph captures semantic relationships beyond structure.

#### Acceptance Criteria

1. THE system SHALL detect references in chunk text matching patterns: `"see <text>"`, `"refer to <text>"`, `"as per <text>"`, inline URLs, and quoted document titles.
2. IT SHALL create `References` edges between the source chunk node and any matching target heading/file node.
3. THE system SHALL extract entities using a heuristic approach: capitalized phrases (3+ words), repeated nouns (appearing 3+ times across chunks), and known domain keywords.
4. IT SHALL create `Mentions` edges between chunk nodes and entity nodes.
5. IT SHALL be tolerant of missing targets — if a reference target has no matching node, the edge is still created with a best-effort label.
6. THE entity extraction logic SHALL be in a dedicated `entity.rs` module for future replacement with an NLP-based approach.

---

### Requirement 5: Graph Expansion Engine

**User Story:** As a user, I want to explore the graph starting from a seed node so that I can discover related documents, sections, and entities.

#### Acceptance Criteria

1. THE system SHALL provide `ExpansionEngine::expand(node_id, depth, direction)`.
2. EXPANSION SHALL support both BFS and DFS traversal strategies (default: BFS).
3. THE `direction` parameter SHALL support `Forward` (outgoing edges), `Backward` (incoming edges), and `Both`.
4. THE expansion SHALL stop at `depth` hops from the seed node (default: 2, max: 5).
5. THE expansion SHALL return a `Subgraph` containing `nodes: Vec<Node>` and `edges: Vec<Edge>`.
6. THE system SHALL provide `expand_from_chunks(chunk_ids, depth)` — expand from multiple seed chunks.
7. THE engine SHALL deduplicate nodes and edges (no repeats in result).
8. THE engine SHALL support edge weight filtering: `min_weight: f32` (skip edges below threshold).

---

### Requirement 6: Graph Query API

**User Story:** As a user, I want to query the graph for specific patterns so that I can answer questions about document relationships.

#### Acceptance Criteria

1. THE system SHALL provide `find_path(from_id, to_id, max_depth) -> Option<Vec<Edge>>` — shortest path between two nodes.
2. THE system SHALL provide `get_node_by_ref(ref_id) -> Option<Node>` — find graph node by its reference ID.
3. THE system SHALL provide `get_nodes_by_type(node_type) -> Vec<Node>` — list all nodes of a given type.
4. THE system SHALL provide `get_edges_by_relation(relation) -> Vec<Edge>` — list all edges with a given relation type.
5. THE system SHALL provide `get_file_graph(file_id) -> Subgraph` — get the full subgraph for a single file.

---

### Requirement 7: CLI Integration

**User Story:** As a user, I want to inspect and query the knowledge graph from the command line so that I can understand document relationships.

#### Acceptance Criteria

1. THE system SHALL add CLI command: `ocean graph info <file>` — show node count, edge count, and node-type breakdown for a file's subgraph.
2. THE system SHALL add CLI command: `ocean graph expand <node-id> [--depth N] [--direction forward|backward|both]` — expand from a node and display connected subgraph.
3. THE system SHALL add CLI command: `ocean graph path <from-id> <to-id> [--max-depth N]` — find shortest path between two nodes.
4. THE system SHALL add CLI command: `ocean graph stats` — show global graph statistics (total nodes, total edges, counts by type).
5. DISPLAY for `expand` SHALL show: node ID, node type, label, and for each edge: relation type, weight, direction indicator.

---

### Requirement 8: Integration with Index Pipeline

**User Story:** As the system, I want graph building to run automatically as part of the index pipeline so that the graph stays in sync with the vector index.

#### Acceptance Criteria

1. THE `ocean index` CLI command SHALL also run graph building after vector indexing.
2. THE index pipeline SHALL call `GraphBuilder::from_chunks` and then persist via `GraphStore`.
3. ON reindex (`--reindex`), the pipeline SHALL delete old graph nodes/edges for the file before rebuilding.
4. THE `IndexReport` SHALL be extended with graph fields: `graph_nodes`, `graph_edges`.
5. THE pipeline SHALL be idempotent — rebuilding the same file produces the same graph state.

---

### Requirement 9: Graph + Vector Integration (Context Expansion)

**User Story:** As a user, I want to enrich vector search results with graph context so that I get related chunks beyond the top-K semantic matches.

#### Acceptance Criteria

1. THE system SHALL provide `SearchEngine::expand_results(results: &[SearchResult], graph: &ExpansionEngine, depth: usize) -> Vec<SearchResult>`.
2. FOR each vector search result, IT SHALL expand the corresponding chunk node in the graph up to `depth` hops.
3. THE expanded results SHALL be merged with the original results (deduplicated by chunk_id).
4. THE expanded results SHALL include a `graph_score` field that combines the vector similarity score with graph proximity scores.

---

### Requirement 10: Configuration and Error Handling

**User Story:** As a developer, I want clear error types and configurable graph building options so that I can tune behavior and diagnose issues.

#### Acceptance Criteria

1. THE system SHALL define a public `GraphError` enum with variants: `StoreError(String)`, `NodeNotFound(String)`, `EdgeNotFound(String)`, `InvalidDepth(String)`, `CycleDetected`, `SerializationError(String)`.
2. THE system SHALL provide a `GraphConfig` struct with fields: `extract_references` (bool, default true), `extract_entities` (bool, default true), `max_expansion_depth` (usize, default 3), `entity_min_freq` (usize, default 3), `default_edge_weight` (f32, default 1.0).
3. THE system SHALL provide a default `GraphConfig`.
4. ALL public types SHALL implement `fmt::Debug` and `fmt::Display`.
