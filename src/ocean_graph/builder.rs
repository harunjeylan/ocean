use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use sha2::{Digest, Sha256};

use crate::ocean_chunk::Chunk;
use crate::ocean_graph::entity::EntityExtractor;
use crate::ocean_graph::types::{Edge, GraphConfig, Node, NodeType, RelationType};

pub struct GraphBuilder;

impl GraphBuilder {
    pub fn from_chunks(
        chunks: &[Chunk],
        file_id: &str,
        config: &GraphConfig,
    ) -> (Vec<Node>, Vec<Edge>) {
        let (mut nodes, mut edges) = Self::structural(chunks, file_id);

        if config.extract_references {
            let ref_edges = Self::extract_references(chunks, &nodes);
            edges.extend(ref_edges);
        }

        if config.extract_entities {
            let content_by_chunk: Vec<(String, &str)> = chunks
                .iter()
                .map(|c| (c.id.clone(), c.content.as_str()))
                .collect();

            let mut entity_names: Vec<String> = EntityExtractor::extract_repeated(&content_by_chunk, config.entity_min_frequency);

            let all_text: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
            for text in &all_text {
                let caps = EntityExtractor::extract_capitalized(text);
                for cap in caps {
                    let cap_lower = cap.to_lowercase();
                    if !entity_names.iter().any(|e| e == &cap_lower) {
                        entity_names.push(cap_lower);
                    }
                }
            }

            let mut entity_names_dedup = entity_names;
            entity_names_dedup.sort();
            entity_names_dedup.dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());

            for name in &entity_names_dedup {
                let entity_id = Self::entity_id(file_id, name);
                nodes.push(Node {
                    id: entity_id.clone(),
                    node_type: NodeType::Entity,
                    ref_id: entity_id,
                    label: Some(name.clone()),
                });
            }

            for chunk in chunks {
                let chunk_node_id = format!("chunk:{}", chunk.id);
                for name in &entity_names_dedup {
                    if chunk.content.to_lowercase().contains(&name.to_lowercase()) {
                        edges.push(Edge {
                            from: chunk_node_id.clone(),
                            to: Self::entity_id(file_id, name),
                            relation: RelationType::Mentions,
                            weight: 0.5,
                            metadata: None,
                        });
                    }
                }
            }
        }

        (nodes, edges)
    }

    pub fn structural(chunks: &[Chunk], file_id: &str) -> (Vec<Node>, Vec<Edge>) {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let file_node_id = format!("file:{}", file_id);
        nodes.push(Node {
            id: file_node_id.clone(),
            node_type: NodeType::File,
            ref_id: file_id.to_string(),
            label: None,
        });

        let mut heading_nodes: HashMap<String, Node> = HashMap::new();

        for chunk in chunks {
            let chunk_node_id = format!("chunk:{}", chunk.id);
            nodes.push(Node {
                id: chunk_node_id.clone(),
                node_type: NodeType::Chunk,
                ref_id: chunk.id.clone(),
                label: None,
            });

            edges.push(Edge {
                from: file_node_id.clone(),
                to: chunk_node_id.clone(),
                relation: RelationType::Contains,
                weight: 1.0,
                metadata: None,
            });

            edges.push(Edge {
                from: chunk_node_id.clone(),
                to: file_node_id.clone(),
                relation: RelationType::BelongsTo,
                weight: 1.0,
                metadata: None,
            });

            if let Some(ref heading) = chunk.heading {
                let heading_id = Self::heading_id(file_id, heading);
                if !heading_nodes.contains_key(&heading_id) {
                    heading_nodes.insert(
                        heading_id.clone(),
                        Node {
                            id: heading_id.clone(),
                            node_type: NodeType::Heading,
                            ref_id: heading_id.clone(),
                            label: Some(heading.clone()),
                        },
                    );
                }
                edges.push(Edge {
                    from: chunk_node_id,
                    to: heading_id,
                    relation: RelationType::BelongsTo,
                    weight: 1.0,
                    metadata: None,
                });
            }
        }

        for (_id, node) in heading_nodes {
            nodes.push(node);
        }

        (nodes, edges)
    }

    fn see_regex() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"(?i)see\s+['\x{201C}]?([A-Z][A-Za-z0-9 ]{2,50})").unwrap())
    }

    fn refer_regex() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"(?i)refer\s+to\s+['\x{201C}]?([A-Z][A-Za-z0-9 ]{2,50})").unwrap())
    }

    fn as_per_regex() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"(?i)as\s+per\s+['\x{201C}]?([A-Z][A-Za-z0-9 ]{2,50})").unwrap())
    }

    fn per_regex() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"(?i)per\s+['\x{201C}]?([A-Z][A-Za-z0-9 ]{2,50})").unwrap())
    }

    fn quoted_regex() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r#"['\u{201C}]([A-Z][A-Za-z0-9 ]{3,60})['\u{201D}]"#).unwrap())
    }

    pub fn extract_references(chunks: &[Chunk], nodes: &[Node]) -> Vec<Edge> {
        let mut edges = Vec::new();

        let patterns: [&Regex; 5] = [Self::see_regex(), Self::refer_regex(), Self::as_per_regex(), Self::per_regex(), Self::quoted_regex()];

        let known_targets: Vec<String> = nodes
            .iter()
            .map(|n| format!("{} {} {}", n.id, n.label.as_deref().unwrap_or(""), n.ref_id))
            .collect();

        for chunk in chunks {
            let chunk_node_id = format!("chunk:{}", chunk.id);
            for pat in &patterns {
                for cap in pat.captures_iter(&chunk.content) {
                    let matched = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                    if matched.is_empty() {
                        continue;
                    }

                    let best_target = known_targets
                        .iter()
                        .find(|t| {
                            t.to_lowercase().contains(&matched.to_lowercase())
                        })
                        .map(|t| {
                            t.split_whitespace().next().unwrap_or("").to_string()
                        });

                    edges.push(Edge {
                        from: chunk_node_id.clone(),
                        to: best_target.unwrap_or_else(|| format!("unknown:{}", matched)),
                        relation: RelationType::References,
                        weight: 0.7,
                        metadata: Some(format!("matched: {}", matched)),
                    });
                }
            }
        }

        edges
    }

    fn heading_id(file_id: &str, heading_text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(heading_text.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        format!("heading:{}:{}", file_id, &hash[..16])
    }

    fn entity_id(file_id: &str, name: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(file_id.as_bytes());
        hasher.update(name.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        format!("entity:{}", &hash[..16])
    }
}
