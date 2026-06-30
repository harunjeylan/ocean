# Roadmap

Ocean's development is guided by the [project plan](project-plan.md).

## v0.1.0 — Initial Release

- [x] 7 format backends: PDF, DOCX, PPTX, XLSX, TXT, MD, HTML
- [x] Semantic chunking with heading detection and sentence-boundary split
- [x] 4 embedding providers: Ollama, OpenAI, Anthropic, Gemini
- [x] HNSW vector index with hybrid search (KNN + FTS + RRF fusion)
- [x] Knowledge graph with structural, reference, and entity extraction
- [x] Unified query engine with auto/vector/hybrid/expand modes
- [x] CLI with 13 commands
- [x] Config layering: CLI flags > config file > .env > defaults

## v0.2.0 — Upcoming

- [ ] MCP server (Model Context Protocol) — expose document search, read, and metadata as tools/resources for AI agents
- [ ] Watcher-based auto-indexing
- [ ] Incremental index updates
- [ ] Multi-query batching
- [ ] Export/import index snapshots

## v0.3.0 — On Horizon

- [ ] REST API server mode
- [ ] MCP server enhancements — dynamic tool registration, resource subscriptions, prompt templates
- [ ] Plugin system for custom format backends
- [ ] Document comparison and diffing
- [ ] Web UI (optional companion)
- [ ] Performance benchmarks and optimization

See [project-plan.md](project-plan.md) for the full architecture and component design.
