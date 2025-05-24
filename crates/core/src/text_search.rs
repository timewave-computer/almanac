/// Text search functionality for event data
use async_trait::async_trait;
use regex::Regex;

use crate::event::Event;
use crate::types::{TextSearchConfig, TextSearchMode};
use crate::{Error, Result};

/// Result of a text search with optional score
#[derive(Debug)]
pub struct TextSearchResult {
    /// The event that matched
    pub event: Box<dyn Event>,
    
    /// Match score (0.0 to 1.0, where 1.0 is perfect match)
    pub score: f32,
    
    /// Highlighted snippets of matching text
    pub highlights: Vec<String>,
}

/// Trait for text search implementations
#[async_trait]
pub trait TextSearcher: Send + Sync {
    /// Search for events matching the given query and configuration
    async fn search(
        &self,
        query: &str,
        config: &TextSearchConfig,
        events: Vec<Box<dyn Event>>,
    ) -> Result<Vec<TextSearchResult>>;
}

/// Default text search implementation
pub struct DefaultTextSearcher;

impl DefaultTextSearcher {
    pub fn new() -> Self {
        Self
    }
    
    /// Extract searchable text from an event
    fn extract_searchable_text(&self, event: &dyn Event, fields: &Option<Vec<String>>) -> String {
        let mut text_parts = Vec::new();
        
        // Always include basic event information
        text_parts.push(event.id().to_string());
        text_parts.push(event.chain().to_string());
        text_parts.push(event.event_type().to_string());
        text_parts.push(event.tx_hash().to_string());
        text_parts.push(event.block_hash().to_string());
        
        // Convert raw data to string if possible
        if let Ok(raw_str) = String::from_utf8(event.raw_data().to_vec()) {
            text_parts.push(raw_str);
        }
        
        // If specific fields are requested, filter to those
        // For now, we include all available text since the Event trait 
        // doesn't expose structured attributes
        let _ = fields; // Suppress unused variable warning
        
        text_parts.join(" ")
    }
    
    /// Perform contains search
    fn search_contains(&self, query: &str, text: &str, case_sensitive: bool) -> Option<f32> {
        let (search_query, search_text) = if case_sensitive {
            (query.to_string(), text.to_string())
        } else {
            (query.to_lowercase(), text.to_lowercase())
        };
        
        if search_text.contains(&search_query) {
            // Calculate score based on how many times the query appears
            let count = search_text.matches(&search_query).count() as f32;
            let text_len = search_text.len() as f32;
            let query_len = search_query.len() as f32;
            
            // Score based on frequency and query coverage
            Some((count * query_len / text_len).min(1.0))
        } else {
            None
        }
    }
    
    /// Perform fuzzy search using Levenshtein distance
    fn search_fuzzy(
        &self,
        query: &str,
        text: &str,
        max_distance: u32,
        case_sensitive: bool,
    ) -> Option<f32> {
        let (search_query, search_text) = if case_sensitive {
            (query.to_string(), text.to_string())
        } else {
            (query.to_lowercase(), text.to_lowercase())
        };
        
        // Split text into words and find best match
        let words: Vec<&str> = search_text.split_whitespace().collect();
        let mut best_score = 0.0f32;
        
        for word in words {
            let distance = levenshtein_distance(word, &search_query);
            if distance <= max_distance {
                let score = 1.0 - (distance as f32 / search_query.len().max(word.len()) as f32);
                best_score = best_score.max(score);
            }
        }
        
        if best_score > 0.0 {
            Some(best_score)
        } else {
            None
        }
    }
    
    /// Perform regex search
    fn search_regex(&self, pattern: &str, text: &str) -> Result<Option<f32>> {
        let regex = Regex::new(pattern)
            .map_err(|e| Error::generic(&format!("Invalid regex pattern: {}", e)))?;
        
        if regex.is_match(text) {
            // Count matches to calculate score
            let matches: Vec<_> = regex.find_iter(text).collect();
            let coverage = matches.iter().map(|m| m.len()).sum::<usize>() as f32 / text.len() as f32;
            Ok(Some(coverage.min(1.0)))
        } else {
            Ok(None)
        }
    }
    
    /// Perform phrase search (exact phrase match)
    fn search_phrase(&self, phrase: &str, text: &str, case_sensitive: bool) -> Option<f32> {
        let (search_phrase, search_text) = if case_sensitive {
            (phrase.to_string(), text.to_string())
        } else {
            (phrase.to_lowercase(), text.to_lowercase())
        };
        
        if search_text.contains(&search_phrase) {
            // Score based on phrase coverage
            let coverage = search_phrase.len() as f32 / search_text.len() as f32;
            Some(coverage.min(1.0))
        } else {
            None
        }
    }
    
    /// Perform boolean search (AND, OR, NOT operators)
    fn search_boolean(&self, query: &str, text: &str, case_sensitive: bool) -> Result<Option<f32>> {
        // Simple boolean parser - split by AND, OR, NOT
        let search_text = if case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };
        
        let search_query = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        
        // Parse boolean expression (simplified)
        if search_query.contains(" and ") {
            let terms: Vec<&str> = search_query.split(" and ").collect();
            let all_match = terms.iter().all(|term| search_text.contains(term.trim()));
            if all_match {
                return Ok(Some(1.0));
            }
        } else if search_query.contains(" or ") {
            let terms: Vec<&str> = search_query.split(" or ").collect();
            let any_match = terms.iter().any(|term| search_text.contains(term.trim()));
            if any_match {
                return Ok(Some(0.7)); // Lower score for OR matches
            }
        } else if search_query.starts_with("not ") {
            let term = search_query.strip_prefix("not ").unwrap().trim();
            if !search_text.contains(term) {
                return Ok(Some(1.0));
            }
        } else {
            // Fallback to contains search
            return Ok(self.search_contains(&search_query, &search_text, case_sensitive));
        }
        
        Ok(None)
    }
    
    /// Generate highlights for matching text
    fn generate_highlights(&self, query: &str, text: &str, case_sensitive: bool) -> Vec<String> {
        let mut highlights = Vec::new();
        
        let (search_query, search_text) = if case_sensitive {
            (query.to_string(), text.to_string())
        } else {
            (query.to_lowercase(), text.to_lowercase())
        };
        
        // Find all occurrences of the query
        let mut start = 0;
        while let Some(pos) = search_text[start..].find(&search_query) {
            let abs_pos = start + pos;
            
            // Extract context around the match (50 characters before and after)
            let context_start = abs_pos.saturating_sub(50);
            let context_end = (abs_pos + search_query.len() + 50).min(text.len());
            
            let mut highlight = text[context_start..context_end].to_string();
            
            // Add ellipsis if truncated
            if context_start > 0 {
                highlight = format!("...{}", highlight);
            }
            if context_end < text.len() {
                highlight = format!("{}...", highlight);
            }
            
            highlights.push(highlight);
            start = abs_pos + search_query.len();
            
            // Limit to max 5 highlights
            if highlights.len() >= 5 {
                break;
            }
        }
        
        highlights
    }
}

#[async_trait]
impl TextSearcher for DefaultTextSearcher {
    async fn search(
        &self,
        query: &str,
        config: &TextSearchConfig,
        events: Vec<Box<dyn Event>>,
    ) -> Result<Vec<TextSearchResult>> {
        let mut results = Vec::new();
        
        for event in events {
            let searchable_text = self.extract_searchable_text(event.as_ref(), &config.fields);
            
            let score = match &config.mode {
                TextSearchMode::Contains => {
                    self.search_contains(query, &searchable_text, config.case_sensitive)
                }
                TextSearchMode::FullText => {
                    // For full-text search, use contains with stemming considerations
                    // In a real implementation, this would use a proper full-text search engine
                    self.search_contains(query, &searchable_text, config.case_sensitive)
                }
                TextSearchMode::Fuzzy { max_distance } => {
                    self.search_fuzzy(query, &searchable_text, *max_distance, config.case_sensitive)
                }
                TextSearchMode::Regex => {
                    self.search_regex(query, &searchable_text)?
                }
                TextSearchMode::Phrase => {
                    self.search_phrase(query, &searchable_text, config.case_sensitive)
                }
                TextSearchMode::Boolean => {
                    self.search_boolean(query, &searchable_text, config.case_sensitive)?
                }
            };
            
            if let Some(match_score) = score {
                // Apply minimum score filter
                if let Some(min_score) = config.min_score {
                    if match_score < min_score {
                        continue;
                    }
                }
                
                let highlights = self.generate_highlights(query, &searchable_text, config.case_sensitive);
                
                results.push(TextSearchResult {
                    event,
                    score: match_score,
                    highlights,
                });
            }
        }
        
        // Sort by score (highest first)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Apply max results limit
        if let Some(max_results) = config.max_results {
            results.truncate(max_results);
        }
        
        Ok(results)
    }
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> u32 {
    let len1 = s1.len();
    let len2 = s2.len();
    
    if len1 == 0 {
        return len2 as u32;
    }
    if len2 == 0 {
        return len1 as u32;
    }
    
    let mut matrix = vec![vec![0u32; len2 + 1]; len1 + 1];
    
    // Initialize first row and column
    for i in 0..=len1 {
        matrix[i][0] = i as u32;
    }
    for j in 0..=len2 {
        matrix[0][j] = j as u32;
    }
    
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    
    // Fill the matrix
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    
    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Mock event for testing
    #[derive(Debug, Clone)]
    struct TestEvent {
        id: String,
        chain: String,
        block_number: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: SystemTime,
        event_type: String,
        raw_data: Vec<u8>,
    }
    
    impl Event for TestEvent {
        fn id(&self) -> &str { &self.id }
        fn chain(&self) -> &str { &self.chain }
        fn block_number(&self) -> u64 { self.block_number }
        fn block_hash(&self) -> &str { &self.block_hash }
        fn tx_hash(&self) -> &str { &self.tx_hash }
        fn timestamp(&self) -> SystemTime { self.timestamp }
        fn event_type(&self) -> &str { &self.event_type }
        fn raw_data(&self) -> &[u8] { &self.raw_data }
        fn as_any(&self) -> &dyn std::any::Any { self }
    }
    
    fn create_test_event(id: &str, event_type: &str, raw_data: &str) -> Box<dyn Event> {
        Box::new(TestEvent {
            id: id.to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: event_type.to_string(),
            raw_data: raw_data.as_bytes().to_vec(),
        })
    }
    
    #[tokio::test]
    async fn test_contains_search() {
        let searcher = DefaultTextSearcher::new();
        
        let config = TextSearchConfig {
            mode: TextSearchMode::Contains,
            case_sensitive: false,
            ..Default::default()
        };
        
        let events1 = vec![
            create_test_event("1", "transfer", "Alice sent 100 tokens to Bob"),
            create_test_event("2", "mint", "Minted 50 tokens for Charlie"),
            create_test_event("3", "burn", "Burned 25 tokens from Dave"),
        ];
        
        let results = searcher.search("tokens", &config, events1).await.unwrap();
        assert_eq!(results.len(), 3); // All events contain "tokens"
        
        let events2 = vec![
            create_test_event("1", "transfer", "Alice sent 100 tokens to Bob"),
            create_test_event("2", "mint", "Minted 50 tokens for Charlie"),
            create_test_event("3", "burn", "Burned 25 tokens from Dave"),
        ];
        
        let results = searcher.search("Alice", &config, events2).await.unwrap();
        assert_eq!(results.len(), 1); // Only first event contains "Alice"
    }
    
    #[tokio::test]
    async fn test_fuzzy_search() {
        let searcher = DefaultTextSearcher::new();
        let events = vec![
            create_test_event("1", "transfer", "Alice sent tokens"),
            create_test_event("2", "transfer", "Alicia received tokens"),
        ];
        
        let config = TextSearchConfig {
            mode: TextSearchMode::Fuzzy { max_distance: 2 },
            case_sensitive: false,
            ..Default::default()
        };
        
        let results = searcher.search("Alice", &config, events).await.unwrap();
        assert_eq!(results.len(), 2); // Both "Alice" and "Alicia" should match
    }
    
    #[tokio::test]
    async fn test_regex_search() {
        let searcher = DefaultTextSearcher::new();
        let events = vec![
            create_test_event("1", "transfer", "Transaction hash: 0x123abc"),
            create_test_event("2", "transfer", "Transaction hash: 0x456def"),
            create_test_event("3", "transfer", "No hash here"),
        ];
        
        let config = TextSearchConfig {
            mode: TextSearchMode::Regex,
            case_sensitive: false,
            ..Default::default()
        };
        
        let results = searcher.search(r"0x[0-9a-f]+", &config, events).await.unwrap();
        assert_eq!(results.len(), 2); // Two events with hex patterns
    }
    
    #[tokio::test]
    async fn test_phrase_search() {
        let searcher = DefaultTextSearcher::new();
        let events = vec![
            create_test_event("1", "transfer", "Alice sent tokens to Bob"),
            create_test_event("2", "transfer", "Bob received tokens from Alice"), // Should not match "sent tokens"
            create_test_event("3", "transfer", "Charlie sent tokens"),
        ];
        
        let config = TextSearchConfig {
            mode: TextSearchMode::Phrase,
            case_sensitive: false,
            ..Default::default()
        };
        
        let results = searcher.search("sent tokens", &config, events).await.unwrap();
        // Only events 1 and 3 should match the exact phrase "sent tokens"
        assert_eq!(results.len(), 2, "Expected 2 events with 'sent tokens' phrase, got {}", results.len());
        
        // Check the IDs of matching events
        let matching_ids: Vec<&str> = results.iter().map(|r| r.event.id()).collect();
        assert!(matching_ids.contains(&"1"), "Event 1 should match");
        assert!(matching_ids.contains(&"3"), "Event 3 should match");
        assert!(!matching_ids.contains(&"2"), "Event 2 should not match");
    }
    
    #[tokio::test]
    async fn test_boolean_search() {
        let searcher = DefaultTextSearcher::new();
        
        let config = TextSearchConfig {
            mode: TextSearchMode::Boolean,
            case_sensitive: false,
            ..Default::default()
        };
        
        let events1 = vec![
            create_test_event("1", "transfer", "Alice sent tokens to Bob"),
            create_test_event("2", "transfer", "Alice received tokens"),
            create_test_event("3", "transfer", "Bob sent tokens"),
            create_test_event("4", "mint", "Charlie minted tokens"),
        ];
        
        let results = searcher.search("Alice and tokens", &config, events1).await.unwrap();
        assert_eq!(results.len(), 2); // Events 1 and 2 contain both "Alice" and "tokens"
        
        let events2 = vec![
            create_test_event("1", "transfer", "Alice sent tokens to Bob"),
            create_test_event("2", "transfer", "Alice received tokens"),
            create_test_event("3", "transfer", "Bob sent tokens"),
            create_test_event("4", "mint", "Charlie minted tokens"),
        ];
        
        let results = searcher.search("Alice or Charlie", &config, events2).await.unwrap();
        assert_eq!(results.len(), 3); // Events 1, 2, and 4 contain either "Alice" or "Charlie"
    }
    
    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "abcd"), 1);
        assert_eq!(levenshtein_distance("abc", "ab"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }
} 