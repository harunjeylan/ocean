use std::collections::{HashMap, HashSet};

pub struct EntityExtractor;

impl EntityExtractor {
    pub fn extract(text: &str, min_frequency: usize) -> Vec<String> {
        let mut entities = HashSet::new();

        for phrase in Self::extract_capitalized(text) {
            entities.insert(phrase);
        }

        let repeated = Self::extract_repeated_from_single(text, min_frequency);
        for word in repeated {
            entities.insert(word);
        }

        let mut result: Vec<String> = entities.into_iter().collect();
        result.sort();
        result
    }

    pub fn extract_capitalized(text: &str) -> Vec<String> {
        let mut phrases = Vec::new();
        let mut current = Vec::new();

        for word in text.split_whitespace() {
            let cleaned: String = word.chars().filter(|c| c.is_alphanumeric() || *c == '-').collect();
            if cleaned.is_empty() {
                continue;
            }
            if cleaned.starts_with(|c: char| c.is_uppercase()) && cleaned.len() > 1 {
                current.push(cleaned);
            } else {
                if current.len() >= 3 {
                    phrases.push(current.join(" "));
                }
                current.clear();
            }
        }
        if current.len() >= 3 {
            phrases.push(current.join(" "));
        }

        phrases
    }

    pub fn extract_repeated(content_by_chunk: &[(String, &str)], min_freq: usize) -> Vec<String> {
        let mut freq: HashMap<String, usize> = HashMap::new();

        for (_id, text) in content_by_chunk {
            let mut seen_in_chunk = HashSet::new();
            for word in text.split_whitespace() {
                let cleaned: String = word
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .collect::<String>()
                    .to_lowercase();
                if cleaned.len() > 3 && seen_in_chunk.insert(cleaned.clone()) {
                    *freq.entry(cleaned).or_insert(0) += 1;
                }
            }
        }

        let mut result: Vec<String> = freq
            .into_iter()
            .filter(|(_, count)| *count >= min_freq)
            .map(|(word, _)| word)
            .collect();
        result.sort();
        result
    }

    fn extract_repeated_from_single(text: &str, min_freq: usize) -> Vec<String> {
        let mut freq: HashMap<String, usize> = HashMap::new();

        for word in text.split_whitespace() {
            let cleaned: String = word
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase();
            if cleaned.len() > 3 {
                *freq.entry(cleaned).or_insert(0) += 1;
            }
        }

        freq.into_iter()
            .filter(|(_, count)| *count >= min_freq)
            .map(|(word, _)| word)
            .collect()
    }
}
