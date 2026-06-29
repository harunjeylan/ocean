pub const SYSTEM_RULES: &[&str] = &[
    "R9.1: Filesystem is sole source of truth — no index is authoritative.",
    "R9.2: Derivation chain: Files → Blocks → Chunks → Embeddings → Graph, never the reverse.",
    "R9.3: No format-awareness leaks outside the parser layer.",
    "R9.4: Every data unit has an id, source_file, and location for traceability.",
    "R9.5: Determinism — same input always produces same output.",
    "R9.6: Rebuildability — filesystem deletion triggers full rebuild from filesystem.",
];

pub const DERIVATION_CHAIN: &[&str] = &["Files", "Blocks", "Chunks", "Embeddings", "Graph"];
