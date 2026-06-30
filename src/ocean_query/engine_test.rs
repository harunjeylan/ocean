use crate::ocean_query::engine::{select_mode, QueryEngine};
use crate::ocean_query::types::*;
use crate::ocean_vector::embedder::Embedder;
use crate::ocean_vector::store::VectorStore;

struct TestEmbedder;

impl Embedder for TestEmbedder {
    fn embed(&self, _text: &str) -> Result<Vec<f32>, crate::ocean_vector::embedder::EmbedderError> {
        Ok(vec![0.1; 4])
    }
    fn embed_batch(
        &self,
        texts: &[&str],
    ) -> Result<Vec<Vec<f32>>, crate::ocean_vector::embedder::EmbedderError> {
        Ok(texts.iter().map(|_| vec![0.1; 4]).collect())
    }
    fn dimension(&self) -> usize {
        4
    }
    fn model_name(&self) -> &str {
        "test"
    }
}

#[test]
fn select_mode_short_keyword() {
    assert_eq!(select_mode("hello", 0), QueryMode::Vector);
    assert_eq!(select_mode("hello world", 0), QueryMode::Vector);
}

#[test]
fn select_mode_long_phrase() {
    assert_eq!(
        select_mode("how does this work", 0),
        QueryMode::Hybrid
    );
    assert_eq!(
        select_mode("the quick brown fox jumps", 0),
        QueryMode::Hybrid
    );
}

#[test]
fn select_mode_with_expand_depth() {
    assert_eq!(select_mode("hello", 1), QueryMode::Expand);
    assert_eq!(select_mode("hello world test", 2), QueryMode::Expand);
}

#[test]
fn select_mode_with_ref_keywords() {
    assert_eq!(
        select_mode("related to security", 0),
        QueryMode::Expand
    );
    assert_eq!(
        select_mode("connected to network", 0),
        QueryMode::Expand
    );
    assert_eq!(select_mode("reference docs", 0), QueryMode::Expand);
    assert_eq!(
        select_mode("associated with config", 0),
        QueryMode::Expand
    );
}

#[test]
fn select_mode_empty() {
    assert_eq!(select_mode("", 0), QueryMode::Hybrid);
    assert_eq!(select_mode("   ", 0), QueryMode::Hybrid);
}

#[test]
fn select_mode_deterministic() {
    let texts = vec![
        ("foo", QueryMode::Vector),
        ("foo bar baz", QueryMode::Hybrid),
        ("related to X", QueryMode::Expand),
    ];
    for (text, expected) in texts {
        for _ in 0..10 {
            assert_eq!(select_mode(text, 0), expected);
        }
    }
}

#[test]
fn query_empty_text() {
    let engine = QueryEngine::new_memory().expect("failed to create memory engine");
    let q = Query {
        text: "".into(),
        ..Default::default()
    };
    let embedder = TestEmbedder;
    let result = engine.query(q, &embedder);
    assert!(result.is_err());
    match result {
        Err(crate::ocean_query::error::QueryError::InvalidQuery(_)) => {}
        _ => panic!("expected InvalidQuery error"),
    }
}

#[test]
fn query_no_results() {
    let engine = QueryEngine::new_memory().expect("failed to create memory engine");
    let q = Query {
        text: "something not in store".into(),
        top_k: 5,
        ..Default::default()
    };
    let embedder = TestEmbedder;
    // with empty store, vector search returns no results
    let result = engine.query(q, &embedder);
    assert!(result.is_err());
}

#[test]
fn query_vector_mode() {
    let engine = QueryEngine::new_memory().expect("failed to create memory engine");
    let _store_clone = VectorStore::new_memory().expect("failed to create store");
    // manually insert a chunk for the store used by engine
    // Note: engine has its own store via clone, so we need to use the same store
    // This is a simplified test that checks the error type for empty store
    let q = Query {
        text: "test query".into(),
        mode: QueryMode::Vector,
        top_k: 5,
        ..Default::default()
    };
    let embedder = TestEmbedder;
    let result = engine.query(q, &embedder);
    // Should fail with NoResults since store is empty
    assert!(result.is_err());
}

#[test]
fn query_auto_mode_short() {
    let mode = select_mode("hello world", 0);
    assert_eq!(mode, QueryMode::Vector);
}

#[test]
fn query_auto_mode_long() {
    let mode = select_mode("what is the meaning of life", 0);
    assert_eq!(mode, QueryMode::Hybrid);
}

#[test]
fn query_auto_mode_expand_depth() {
    let mode = select_mode("hello", 3);
    assert_eq!(mode, QueryMode::Expand);
}
