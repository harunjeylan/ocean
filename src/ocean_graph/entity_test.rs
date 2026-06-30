use crate::ocean_graph::entity::EntityExtractor;

#[test]
fn test_extract_capitalized_basic() {
    let text = "The Human Resources Department manages all staffing needs.";
    let phrases = EntityExtractor::extract_capitalized(text);
    assert!(phrases.contains(&"The Human Resources Department".to_string()));
}

#[test]
fn test_extract_capitalized_no_match() {
    let text = "this is all lowercase text with no capitalized phrases";
    let phrases = EntityExtractor::extract_capitalized(text);
    assert!(phrases.is_empty());
}

#[test]
fn test_extract_capitalized_short_phrases() {
    let text = "Hello World is short.";
    let phrases = EntityExtractor::extract_capitalized(text);
    assert_eq!(phrases.len(), 0);
}

#[test]
fn test_extract_capitalized_multiple() {
    let text = "Alice Bob Charlie works at Data Science Team. They report to VP Engineering Group.";
    let phrases = EntityExtractor::extract_capitalized(text);
    assert!(phrases.contains(&"Alice Bob Charlie".to_string()), "expected Alice Bob Charlie, got: {:?}", phrases);
    assert!(phrases.contains(&"VP Engineering Group".to_string()), "expected VP Engineering Group, got: {:?}", phrases);
}

#[test]
fn test_extract_repeated_empty() {
    let results = EntityExtractor::extract_repeated(&[], 3);
    assert!(results.is_empty());
}

#[test]
fn test_extract_repeated_basic() {
    let chunks = vec![
        ("c1".to_string(), "document analysis system for document management"),
        ("c2".to_string(), "the document system provides analysis features"),
    ];
    let words = EntityExtractor::extract_repeated(&chunks, 2);
    assert!(words.contains(&"document".to_string()));
    assert!(words.contains(&"analysis".to_string()));
    assert!(words.contains(&"system".to_string()));
}

#[test]
fn test_extract_repeated_frequency_threshold() {
    let chunks = vec![
        ("c1".to_string(), "apple banana cherry"),
        ("c2".to_string(), "apple banana date"),
    ];
    let words = EntityExtractor::extract_repeated(&chunks, 2);
    assert!(words.contains(&"apple".to_string()));
    assert!(words.contains(&"banana".to_string()));
    assert!(!words.contains(&"cherry".to_string()));
    assert!(!words.contains(&"date".to_string()));
}

#[test]
fn test_extract_deduplication() {
    let text = "Research Development Team works with Research Development Team on projects.";
    let results = EntityExtractor::extract(text, 1);
    let count = results.iter().filter(|e| e.as_str() == "research development team").count();
    assert!(count <= 1);
}

#[test]
fn test_extract_combined() {
    let text = "Human Resources Department manages staffing. Human Resources Department handles hiring.";
    let results = EntityExtractor::extract(text, 2);
    assert!(!results.is_empty());
}

#[test]
fn test_case_insensitive_dedup() {
    let text = "HR Department handles all hiring. Hr Department manages staffing.";
    let entities = EntityExtractor::extract(text, 1);
    let lower: Vec<String> = entities.iter().map(|e| e.to_lowercase()).collect();
    let mut sorted = lower.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(lower.len(), sorted.len(), "case-insensitive dupes found");
}

#[test]
fn test_extract_repeated_single_source() {
    let words = EntityExtractor::extract_repeated(&[
        ("c1".to_string(), "apple dog house tree"),
    ], 1);
    assert!(words.contains(&"apple".to_string()), "expected apple, got: {:?}", words);
    assert!(words.contains(&"house".to_string()), "expected house, got: {:?}", words);
    assert!(!words.contains(&"dog".to_string()), "dog len <= 3, should not appear");
}
