use crate::ocean_vector::embedder::*;
use crate::ocean_vector::embedder;
use crate::ocean_vector::pipeline::IndexConfig;
use crate::ocean_vector::search::{SearchFilter, SearchResult};

#[test]
fn test_embedder_error_display() {
    let e = EmbedderError::ConnectionFailed("refused".into());
    assert_eq!(format!("{}", e), "connection failed: refused");

    let e = EmbedderError::AuthenticationFailed("bad key".into());
    assert_eq!(format!("{}", e), "authentication failed: bad key");

    let e = EmbedderError::RateLimited("too fast".into());
    assert_eq!(format!("{}", e), "rate limited: too fast");

    let e = EmbedderError::ModelReturnedError("bad dim".into());
    assert_eq!(format!("{}", e), "model error: bad dim");

    let e = EmbedderError::Timeout("timed out".into());
    assert_eq!(format!("{}", e), "timeout: timed out");

    let e = EmbedderError::Unexpected("weird".into());
    assert_eq!(format!("{}", e), "unexpected error: weird");
}

#[test]
fn test_normalize_zero_vector() {
    let mut v = vec![0.0f32; 10];
    embedder::normalize(&mut v);
    assert_eq!(v, vec![0.0f32; 10]);
}

#[test]
fn test_normalize_unit_vector() {
    let mut v = vec![1.0f32, 0.0, 0.0, 0.0];
    embedder::normalize(&mut v);
    assert!((v[0] - 1.0).abs() < 1e-6);
}

#[test]
fn test_normalize_arbitrary() {
    let mut v = vec![3.0f32, 4.0];
    embedder::normalize(&mut v);
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-6);
}

#[test]
fn test_ollama_embedder_new() {
    let e = OllamaEmbedder::new("test-model", "http://localhost:11434").unwrap();
    assert_eq!(e.model_name(), "test-model");
    assert_eq!(e.dimension(), 768);
}

#[test]
fn test_ollama_embedder_with_dimension() {
    let e = OllamaEmbedder::with_dimension("test-model", "http://localhost:11434", 512).unwrap();
    assert_eq!(e.dimension(), 512);
}

#[test]
fn test_openai_embedder_new_default_dimension() {
    let e = OpenAIEmbedder::new(
        "text-embedding-3-small",
        "https://api.openai.com/v1",
        "sk-test",
    )
    .unwrap();
    assert_eq!(e.dimension(), 1536);
    assert_eq!(e.model_name(), "text-embedding-3-small");
}

#[test]
fn test_openai_embedder_with_dimension() {
    let e = OpenAIEmbedder::with_dimension(
        "text-embedding-3-small",
        "https://api.openai.com/v1",
        "sk-test",
        512,
    )
    .unwrap();
    assert_eq!(e.dimension(), 512);
}

#[test]
fn test_openai_embedder_ada_dimension() {
    let e = OpenAIEmbedder::new("text-embedding-ada-002", "https://api.openai.com/v1", "sk-test").unwrap();
    assert_eq!(e.dimension(), 1536);
}

#[test]
fn test_search_filter_builder() {
    let filter = SearchFilter::new()
        .with_file_id("abc-123")
        .with_heading("Introduction")
        .with_block_type("Text");

    assert_eq!(filter.file_id, Some("abc-123".into()));
    assert_eq!(filter.heading_prefix, Some("Introduction".into()));
    assert_eq!(filter.block_type, Some("Text".into()));
}

#[test]
fn test_search_filter_where_clause_all() {
    let filter = SearchFilter::new()
        .with_file_id("abc")
        .with_heading("Intro")
        .with_block_type("Text");
    let clause = filter.build_where_clause();
    assert!(clause.is_some());
    let clause = clause.unwrap();
    assert!(clause.contains("file_id = 'abc'"));
    assert!(clause.contains("heading STARTSWITH 'Intro'"));
    assert!(clause.contains("block_type = 'Text'"));
}

#[test]
fn test_search_filter_where_clause_empty() {
    let filter = SearchFilter::new();
    assert!(filter.build_where_clause().is_none());
}

#[test]
fn test_search_filter_where_clause_single() {
    let filter = SearchFilter::new().with_file_id("xyz");
    let clause = filter.build_where_clause().unwrap();
    assert_eq!(clause, "file_id = 'xyz'");
}

#[test]
fn test_index_config_defaults() {
    let cfg = IndexConfig::default();
    assert_eq!(cfg.batch_size, 10);
    assert_eq!(cfg.model, "nomic-embed-text");
    assert_eq!(cfg.dimension, 768);
    assert_eq!(cfg.db_path, "ocean.db");
    assert!(!cfg.reindex);
}

#[test]
fn test_search_result_creation() {
    let r = SearchResult {
        chunk_id: "c1".into(),
        file_id: "f1".into(),
        content: "hello world".into(),
        heading: Some("Intro".into()),
        score: 0.95,
        block_type: "Text".into(),
        vector_score: Some(0.95),
        fts_score: None,
    };
    assert_eq!(r.chunk_id, "c1");
    assert_eq!(r.score, 0.95);
}

pub struct MockEmbedder {
    dim: usize,
    model: String,
}

impl MockEmbedder {
    pub fn new(dim: usize, model: &str) -> Self {
        Self { dim, model: model.into() }
    }
}

impl Embedder for MockEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        let mut v = vec![0.0f32; self.dim];
        let h: u64 = text.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let val = (h as f32) / (u64::MAX as f32);
        v[0] = val;
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
        Ok(v)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedderError> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
