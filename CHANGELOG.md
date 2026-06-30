# Changelog

## Unreleased

### Added

- Initial open-source release preparation
- MIT license, contributing guidelines, code of conduct, security policy
- CI quality gates (clippy, fmt, audit)
- Community governance and issue templates

## 0.1.0 - 2025-01-15

### Added

- 7 format backends: PDF (lopdf), DOCX/PPTX (zip + quick-xml), XLSX (calamine), TXT/MD/HTML
- Semantic chunking with heading detection, sentence-boundary split, configurable overlap, 9 chunk types
- 4 embedding providers: Ollama, OpenAI, Anthropic, Gemini — auto-normalized, configurable
- HNSW vector index: SurrealDB-backed, hybrid search (KNN + FTS + RRF fusion)
- Knowledge graph: structural, reference, and entity extraction with BFS expansion
- Unified query engine: auto/vector/hybrid/expand modes with context windows and reranking
- CLI with 13 commands: info, metadata, outline, read, search, grep, scan, hash, verify, watch, chunk, index, query
- Filesystem sandbox: path validation, symlink protection, read-only mode
- Config layering: CLI flags > config file > .env > defaults
- SurrealDB persistence with embedded RocksDB and in-memory modes
- Skip/take slicing across all format backends
