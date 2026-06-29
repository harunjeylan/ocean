use crate::ocean_chunk::split::{find_sentence_boundary, extract_last_sentences, split_with_overlap};
use crate::ocean_chunk::types::ChunkConfig;

#[test]
fn short_text_no_split() {
    let config = ChunkConfig::default();
    let text = "Short text.";
    let result = split_with_overlap(text, &config);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], text);
}

#[test]
fn find_boundary_at_sentence_end() {
    let text = "First sentence. Second sentence. Third sentence.";
    let boundary = find_sentence_boundary(text, 30);
    assert!(boundary.is_some());
    let b = boundary.unwrap();
    assert!(b <= 30);
    assert_eq!(&text[b..], "Second sentence. Third sentence.");
}

#[test]
fn find_boundary_at_newline() {
    let text = "First paragraph.\n\nSecond paragraph.\n\nThird.";
    let boundary = find_sentence_boundary(text, 25);
    assert!(boundary.is_some());
}

#[test]
fn returns_none_when_under_limit() {
    let text = "Short.";
    assert_eq!(find_sentence_boundary(text, 100), None);
}

#[test]
fn extract_last_n_sentences() {
    let text = "A. B. C. D.";
    assert_eq!(extract_last_sentences(text, 2), "C. D.");
}

#[test]
fn extract_last_n_zero() {
    assert_eq!(extract_last_sentences("A. B.", 0), "");
}

#[test]
fn extract_more_than_available() {
    assert_eq!(extract_last_sentences("A.", 5), "A.");
}

#[test]
fn split_with_overlap_produces_multiple_segments() {
    let config = ChunkConfig {
        max_tokens: 5,
        overlap_sentences: 1,
        ..Default::default()
    };
    let text = "One. Two. Three. Four. Five. Six.";
    let result = split_with_overlap(text, &config);
    assert!(result.len() >= 2);
    for segment in &result {
        assert!(!segment.is_empty());
    }
}

#[test]
fn hard_split_when_no_sentence_boundary() {
    let text = "a".repeat(1000);
    let boundary = find_sentence_boundary(&text, 100);
    assert!(boundary.is_some());
}

#[test]
fn overlap_appears_in_second_segment() {
    let config = ChunkConfig {
        max_tokens: 5,
        overlap_sentences: 1,
        ..Default::default()
    };
    let text = "Alpha. Beta. Gamma. Delta. Epsilon. Zeta. Eta. Theta. Iota. Kappa.";
    let result = split_with_overlap(text, &config);
    if result.len() >= 2 {
        assert!(result[1].contains("Beta."));
    }
}
