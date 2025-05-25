/// Event correlation and pattern matching functionality
use std::collections::HashMap;
use std::time::SystemTime;
use async_trait::async_trait;

use crate::event::Event;
use crate::types::{
    CorrelationConfig, CorrelationResult, EventPattern, EventPatternStep, 
    PatternMatchResult
};
use crate::Result;

/// Trait for event correlation implementations
#[async_trait]
pub trait EventCorrelator: Send + Sync {
    /// Find correlated events based on configuration
    async fn correlate_events(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &CorrelationConfig,
    ) -> Result<Vec<CorrelationResult>>;
}

/// Trait for pattern matching implementations
#[async_trait]
pub trait PatternMatcher: Send + Sync {
    /// Find pattern matches in event sequences
    async fn match_patterns(
        &self,
        events: Vec<Box<dyn Event>>,
        patterns: &[EventPattern],
    ) -> Result<Vec<PatternMatchResult>>;
}

/// Default correlation implementation
pub struct DefaultEventCorrelator;

impl DefaultEventCorrelator {
    pub fn new() -> Self {
        Self
    }
    
    /// Extract correlation value from event
    fn extract_correlation_value(&self, event: &dyn Event, field: &str) -> Option<String> {
        match field {
            "tx_hash" => Some(event.tx_hash().to_string()),
            "block_hash" => Some(event.block_hash().to_string()),
            "chain" => Some(event.chain().to_string()),
            "event_type" => Some(event.event_type().to_string()),
            _ => {
                // Try to parse from raw data as JSON
                if let Ok(raw_str) = String::from_utf8(event.raw_data().to_vec()) {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&raw_str) {
                        self.extract_from_json(&json_value, field)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
    
    /// Extract value from JSON object
    fn extract_from_json(&self, json: &serde_json::Value, field: &str) -> Option<String> {
        match json {
            serde_json::Value::Object(obj) => {
                obj.get(field).and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                })
            },
            _ => None,
        }
    }
    
    /// Check if two events are within correlation constraints
    fn events_correlatable(
        &self,
        event1: &dyn Event,
        event2: &dyn Event,
        config: &CorrelationConfig,
    ) -> bool {
        // Check time window
        if let Some(time_window) = config.time_window {
            let time_diff = if event1.timestamp() > event2.timestamp() {
                event1.timestamp().duration_since(event2.timestamp())
            } else {
                event2.timestamp().duration_since(event1.timestamp())
            };
            
            if let Ok(duration) = time_diff {
                if duration.as_secs() > time_window {
                    return false;
                }
            }
        }
        
        // Check block distance
        if let Some(max_distance) = config.max_block_distance {
            let block_diff = if event1.block_number() > event2.block_number() {
                event1.block_number() - event2.block_number()
            } else {
                event2.block_number() - event1.block_number()
            };
            
            if block_diff > max_distance {
                return false;
            }
        }
        
        // Check chains if specified
        if let Some(ref chains) = config.chains {
            if !chains.contains(&event1.chain().to_string()) || 
               !chains.contains(&event2.chain().to_string()) {
                return false;
            }
        }
        
        true
    }
}

#[async_trait]
impl EventCorrelator for DefaultEventCorrelator {
    async fn correlate_events(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &CorrelationConfig,
    ) -> Result<Vec<CorrelationResult>> {
        let mut correlations = Vec::new();
        let mut processed_events = std::collections::HashSet::new();
        
        // Group events by correlation field values
        let mut correlation_groups: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
        
        for (idx, event) in events.iter().enumerate() {
            let mut correlation_key = Vec::new();
            let mut skip_event = false;
            
            // Extract values for all correlation fields
            for field in &config.correlation_fields {
                if let Some(value) = self.extract_correlation_value(event.as_ref(), field) {
                    correlation_key.push(value);
                } else {
                    skip_event = true;
                    break;
                }
            }
            
            if !skip_event && !correlation_key.is_empty() {
                correlation_groups.entry(correlation_key).or_default().push(idx);
            }
        }
        
        // Process each correlation group
        for (correlation_values, event_indices) in correlation_groups {
            if event_indices.len() < config.min_events.unwrap_or(2) {
                continue;
            }
            
            // Verify events are within correlation constraints
            let mut valid_indices = Vec::new();
            for &idx in &event_indices {
                if processed_events.contains(&idx) {
                    continue;
                }
                
                let event = events[idx].as_ref();
                
                // Check if this event correlates with at least one other in the group
                for &other_idx in &event_indices {
                    if idx != other_idx && !processed_events.contains(&other_idx) {
                        let other_event = events[other_idx].as_ref();
                        if self.events_correlatable(event, other_event, config) {
                            valid_indices.push(idx);
                            break;
                        }
                    }
                }
            }
            
            if valid_indices.len() >= config.min_events.unwrap_or(2) {
                // Create correlation result
                let correlation_id = format!("corr_{}", uuid::Uuid::new_v4());
                let event_ids: Vec<String> = valid_indices.iter()
                    .map(|&idx| events[idx].id().to_string())
                    .collect();
                
                // Calculate time and block spans
                let timestamps: Vec<SystemTime> = valid_indices.iter()
                    .map(|&idx| events[idx].timestamp())
                    .collect();
                let blocks: Vec<u64> = valid_indices.iter()
                    .map(|&idx| events[idx].block_number())
                    .collect();
                
                let time_span = if timestamps.len() > 1 {
                    Some((*timestamps.iter().min().unwrap(), *timestamps.iter().max().unwrap()))
                } else {
                    None
                };
                
                let block_span = if blocks.len() > 1 {
                    Some((*blocks.iter().min().unwrap(), *blocks.iter().max().unwrap()))
                } else {
                    None
                };
                
                // Get unique chains
                let chains: std::collections::HashSet<String> = valid_indices.iter()
                    .map(|&idx| events[idx].chain().to_string())
                    .collect();
                
                // Create correlation values map
                let mut correlation_values_map = HashMap::new();
                for (field, value) in config.correlation_fields.iter().zip(correlation_values.iter()) {
                    correlation_values_map.insert(field.clone(), value.clone());
                }
                
                correlations.push(CorrelationResult {
                    correlation_id,
                    events: event_ids,
                    correlation_values: correlation_values_map,
                    time_span,
                    block_span,
                    chains: chains.into_iter().collect(),
                });
                
                // Mark events as processed
                for &idx in &valid_indices {
                    processed_events.insert(idx);
                }
            }
        }
        
        Ok(correlations)
    }
}

/// Default pattern matcher implementation
pub struct DefaultPatternMatcher;

impl DefaultPatternMatcher {
    pub fn new() -> Self {
        Self
    }
    
    /// Check if an event matches a pattern step
    fn matches_pattern_step(&self, event: &dyn Event, step: &EventPatternStep) -> bool {
        // Check event type
        if let Some(ref event_type) = step.event_type {
            if event.event_type() != event_type {
                return false;
            }
        }
        
        // Check chain
        if let Some(ref chain) = step.chain {
            if event.chain() != chain {
                return false;
            }
        }
        
        // Check required attributes
        if let Some(ref required_attrs) = step.required_attributes {
            // Try to parse raw data as JSON
            if let Ok(raw_str) = String::from_utf8(event.raw_data().to_vec()) {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&raw_str) {
                    for (key, expected_value) in required_attrs {
                        if let Some(actual_value) = self.extract_from_json(&json_value, key) {
                            if &actual_value != expected_value {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                }
            }
        }
        
        true
    }
    
    /// Extract value from JSON (same as in correlator)
    fn extract_from_json(&self, json: &serde_json::Value, field: &str) -> Option<String> {
        match json {
            serde_json::Value::Object(obj) => {
                obj.get(field).and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                })
            },
            _ => None,
        }
    }
    
    /// Find pattern matches in sorted events
    fn find_pattern_matches(
        &self,
        events: &[Box<dyn Event>],
        pattern: &EventPattern,
    ) -> Vec<PatternMatchResult> {
        let mut matches = Vec::new();
        
        if pattern.sequence.is_empty() {
            return matches;
        }
        
        // Try to match pattern starting from each event
        for start_idx in 0..events.len() {
            if let Some(pattern_match) = self.try_match_pattern_from(events, pattern, start_idx) {
                matches.push(pattern_match);
            }
        }
        
        matches
    }
    
    /// Try to match pattern starting from a specific event index
    fn try_match_pattern_from(
        &self,
        events: &[Box<dyn Event>],
        pattern: &EventPattern,
        start_idx: usize,
    ) -> Option<PatternMatchResult> {
        let mut matched_events: Vec<usize> = Vec::new();
        let mut current_idx = start_idx;
        let mut step_idx = 0;
        
        let start_time = events[start_idx].timestamp();
        let mut end_time = start_time;
        
        while step_idx < pattern.sequence.len() && current_idx < events.len() {
            let step = &pattern.sequence[step_idx];
            let event = events[current_idx].as_ref();
            
            // Check time window constraint
            if let Some(time_window) = pattern.time_window {
                if let Ok(duration) = event.timestamp().duration_since(start_time) {
                    if duration.as_secs() > time_window {
                        break;
                    }
                }
            }
            
            // Check block gap constraint
            if let Some(max_gap) = pattern.max_gap {
                if !matched_events.is_empty() {
                    let last_idx = *matched_events.last().unwrap();
                    let last_block = events[last_idx].block_number();
                    if event.block_number() > last_block + max_gap {
                        break;
                    }
                }
            }
            
            if self.matches_pattern_step(event, step) {
                matched_events.push(current_idx);
                end_time = event.timestamp();
                step_idx += 1;
                
                // Handle repeats
                if let Some((min_repeat, max_repeat)) = step.repeat {
                    let mut repeat_count = 1;
                    let mut next_idx = current_idx + 1;
                    
                    // Try to match more instances of this step
                    while next_idx < events.len() && repeat_count < max_repeat {
                        if self.matches_pattern_step(events[next_idx].as_ref(), step) {
                            matched_events.push(next_idx);
                            end_time = events[next_idx].timestamp();
                            repeat_count += 1;
                            next_idx += 1;
                        } else {
                            break;
                        }
                    }
                    
                    // Check if we met minimum repeat requirement
                    if repeat_count < min_repeat {
                        return None;
                    }
                    
                    current_idx = next_idx;
                } else {
                    current_idx += 1;
                }
            } else if step.optional {
                // Skip optional step
                step_idx += 1;
                continue;
            } else if pattern.strict_order {
                // Pattern failed in strict mode
                return None;
            } else {
                // Continue searching for this step
                current_idx += 1;
            }
        }
        
        // Check if we matched all required steps
        let required_steps = pattern.sequence.iter()
            .filter(|step| !step.optional)
            .count();
        
        if step_idx >= required_steps {
            // Calculate confidence based on how many steps matched
            let confidence = matched_events.len() as f32 / pattern.sequence.len() as f32;
            
            Some(PatternMatchResult {
                pattern_name: pattern.name.clone(),
                matched_events: matched_events.iter()
                    .map(|&idx| events[idx].id().to_string())
                    .collect(),
                start_time,
                end_time,
                confidence,
                metadata: HashMap::new(),
            })
        } else {
            None
        }
    }
}

#[async_trait]
impl PatternMatcher for DefaultPatternMatcher {
    async fn match_patterns(
        &self,
        mut events: Vec<Box<dyn Event>>,
        patterns: &[EventPattern],
    ) -> Result<Vec<PatternMatchResult>> {
        // Sort events by timestamp and block number
        events.sort_by(|a, b| {
            a.timestamp().cmp(&b.timestamp())
                .then_with(|| a.block_number().cmp(&b.block_number()))
        });
        
        let mut all_matches = Vec::new();
        
        for pattern in patterns {
            let pattern_matches = self.find_pattern_matches(&events, pattern);
            all_matches.extend(pattern_matches);
        }
        
        // Sort matches by start time
        all_matches.sort_by_key(|m| m.start_time);
        
        Ok(all_matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{UNIX_EPOCH, Duration};
    
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
    
    fn create_test_event(
        id: &str, 
        chain: &str, 
        event_type: &str,
        tx_hash: &str,
        block_number: u64, 
        timestamp_offset: u64
    ) -> Box<dyn Event> {
        Box::new(TestEvent {
            id: id.to_string(),
            chain: chain.to_string(),
            block_number,
            block_hash: format!("hash_{}", block_number),
            tx_hash: tx_hash.to_string(),
            timestamp: UNIX_EPOCH + Duration::from_secs(timestamp_offset),
            event_type: event_type.to_string(),
            raw_data: format!(r#"{{"amount": {}}}"#, block_number * 10).as_bytes().to_vec(),
        })
    }
    
    #[tokio::test]
    async fn test_event_correlation_by_tx_hash() {
        let correlator = DefaultEventCorrelator::new();
        
        let events = vec![
            create_test_event("1", "ethereum", "transfer", "tx_abc", 100, 1000),
            create_test_event("2", "ethereum", "approval", "tx_abc", 101, 1010),
            create_test_event("3", "polygon", "mint", "tx_xyz", 102, 1020),
            create_test_event("4", "ethereum", "burn", "tx_abc", 103, 1030),
        ];
        
        let config = CorrelationConfig {
            correlation_fields: vec!["tx_hash".to_string()],
            time_window: Some(60), // 1 minute
            max_block_distance: Some(10),
            min_events: Some(2),
            chains: None,
        };
        
        let results = correlator.correlate_events(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].events.len(), 3); // Events 1, 2, and 4 should be correlated
        assert_eq!(results[0].correlation_values.get("tx_hash"), Some(&"tx_abc".to_string()));
    }
    
    #[tokio::test]
    async fn test_pattern_matching() {
        let matcher = DefaultPatternMatcher::new();
        
        let events = vec![
            create_test_event("1", "ethereum", "approve", "tx_1", 100, 1000),
            create_test_event("2", "ethereum", "transfer", "tx_2", 101, 1010),
            create_test_event("3", "ethereum", "mint", "tx_3", 102, 1020),
            create_test_event("4", "ethereum", "approve", "tx_4", 103, 1030),
            create_test_event("5", "ethereum", "transfer", "tx_5", 104, 1040),
        ];
        
        let pattern = EventPattern {
            name: "approve_then_transfer".to_string(),
            sequence: vec![
                EventPatternStep {
                    event_type: Some("approve".to_string()),
                    chain: Some("ethereum".to_string()),
                    address: None,
                    required_attributes: None,
                    optional: false,
                    repeat: None,
                },
                EventPatternStep {
                    event_type: Some("transfer".to_string()),
                    chain: Some("ethereum".to_string()),
                    address: None,
                    required_attributes: None,
                    optional: false,
                    repeat: None,
                },
            ],
            time_window: Some(60),
            strict_order: true,
            max_gap: Some(5),
        };
        
        let results = matcher.match_patterns(events, &[pattern]).await.unwrap();
        
        assert_eq!(results.len(), 2); // Two approve->transfer patterns
        
        // Check first pattern match (events 1 and 2)
        assert_eq!(results[0].matched_events, vec!["1".to_string(), "2".to_string()]);
        assert_eq!(results[0].pattern_name, "approve_then_transfer");
        
        // Check second pattern match (events 4 and 5)
        assert_eq!(results[1].matched_events, vec!["4".to_string(), "5".to_string()]);
    }
    
    #[tokio::test]
    async fn test_correlation_with_time_constraint() {
        let correlator = DefaultEventCorrelator::new();
        
        let events = vec![
            create_test_event("1", "ethereum", "transfer", "tx_abc", 100, 1000),
            create_test_event("2", "ethereum", "approval", "tx_abc", 101, 1010),
            create_test_event("3", "ethereum", "burn", "tx_abc", 102, 2000), // 1000s later
        ];
        
        let config = CorrelationConfig {
            correlation_fields: vec!["tx_hash".to_string()],
            time_window: Some(60), // 1 minute window
            max_block_distance: Some(100),
            min_events: Some(2),
            chains: None,
        };
        
        let results = correlator.correlate_events(events, &config).await.unwrap();
        
        // Only events 1 and 2 should be correlated (within time window)
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].events.len(), 2);
    }
} 