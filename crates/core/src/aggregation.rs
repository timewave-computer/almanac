/// Aggregation functionality for event data
use std::collections::HashMap;
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use async_trait::async_trait;

use crate::event::Event;
use crate::types::{AggregationConfig, AggregationResult, AggregationFunction, AggregationValue, TimePeriod};
use crate::Result;

/// Trait for aggregation implementations
#[async_trait]
pub trait Aggregator: Send + Sync {
    /// Perform aggregation on a set of events
    async fn aggregate(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &AggregationConfig,
    ) -> Result<Vec<AggregationResult>>;
}

/// Default aggregation implementation
pub struct DefaultAggregator;

impl DefaultAggregator {
    pub fn new() -> Self {
        Self
    }
    
    /// Calculate time bucket for an event based on the time period
    fn calculate_time_bucket(&self, timestamp: SystemTime, period: &TimePeriod) -> SystemTime {
        let duration_since_epoch = timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));
        let seconds = duration_since_epoch.as_secs();
        
        let bucket_seconds = match period {
            TimePeriod::Hour => (seconds / 3600) * 3600,
            TimePeriod::Day => (seconds / 86400) * 86400,
            TimePeriod::Week => {
                // Start of week (Monday)
                let days_since_epoch = seconds / 86400;
                let days_since_monday = (days_since_epoch + 3) % 7; // Epoch was Thursday
                (days_since_epoch - days_since_monday) * 86400
            },
            TimePeriod::Month => {
                // Approximate month buckets (30 days)
                let days_since_epoch = seconds / 86400;
                (days_since_epoch / 30) * 30 * 86400
            },
            TimePeriod::Year => {
                // Approximate year buckets (365 days)
                let days_since_epoch = seconds / 86400;
                (days_since_epoch / 365) * 365 * 86400
            },
            TimePeriod::Custom { seconds: period_seconds } => {
                (seconds / period_seconds) * period_seconds
            },
        };
        
        UNIX_EPOCH + Duration::from_secs(bucket_seconds)
    }
    
    /// Extract numeric value from event for aggregation
    fn extract_numeric_value(&self, event: &dyn Event, field: &str) -> Option<f64> {
        match field {
            "block_number" => Some(event.block_number() as f64),
            "timestamp" => {
                event.timestamp()
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs() as f64)
            },
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
    
    /// Extract numeric value from JSON object
    fn extract_from_json(&self, json: &serde_json::Value, field: &str) -> Option<f64> {
        match json {
            serde_json::Value::Object(obj) => {
                if let Some(value) = obj.get(field) {
                    match value {
                        serde_json::Value::Number(num) => num.as_f64(),
                        serde_json::Value::String(s) => s.parse::<f64>().ok(),
                        _ => None,
                    }
                } else {
                    None
                }
            },
            _ => None,
        }
    }
    
    /// Extract string value from event for grouping
    fn extract_string_value(&self, event: &dyn Event, field: &str) -> String {
        match field {
            "chain" => event.chain().to_string(),
            "event_type" => event.event_type().to_string(),
            "block_hash" => event.block_hash().to_string(),
            "tx_hash" => event.tx_hash().to_string(),
            _ => {
                // Try to parse from raw data as JSON
                if let Ok(raw_str) = String::from_utf8(event.raw_data().to_vec()) {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&raw_str) {
                        self.extract_string_from_json(&json_value, field)
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                }
            }
        }
    }
    
    /// Extract string value from JSON object
    fn extract_string_from_json(&self, json: &serde_json::Value, field: &str) -> String {
        match json {
            serde_json::Value::Object(obj) => {
                if let Some(value) = obj.get(field) {
                    match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(num) => num.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        _ => "unknown".to_string(),
                    }
                } else {
                    "unknown".to_string()
                }
            },
            _ => "unknown".to_string(),
        }
    }
    
    /// Create grouping key from event
    fn create_group_key(&self, event: &dyn Event, group_by: &[String]) -> HashMap<String, String> {
        let mut group_values = HashMap::new();
        
        for field in group_by {
            let value = self.extract_string_value(event, field);
            group_values.insert(field.clone(), value);
        }
        
        group_values
    }
    
    /// Apply aggregation functions to a group of events
    fn apply_aggregations(
        &self,
        events: &[&dyn Event],
        functions: &[AggregationFunction],
    ) -> HashMap<String, AggregationValue> {
        let mut results = HashMap::new();
        
        for function in functions {
            let (name, value) = match function {
                AggregationFunction::Count => {
                    ("count".to_string(), AggregationValue::Count(events.len() as u64))
                },
                AggregationFunction::Sum { field } => {
                    let sum: f64 = events
                        .iter()
                        .filter_map(|e| self.extract_numeric_value(*e, field))
                        .sum();
                    (format!("sum_{}", field), AggregationValue::Sum(sum))
                },
                AggregationFunction::Average { field } => {
                    let values: Vec<f64> = events
                        .iter()
                        .filter_map(|e| self.extract_numeric_value(*e, field))
                        .collect();
                    
                    let avg = if values.is_empty() {
                        0.0
                    } else {
                        values.iter().sum::<f64>() / values.len() as f64
                    };
                    
                    (format!("avg_{}", field), AggregationValue::Average(avg))
                },
                AggregationFunction::Min { field } => {
                    let min = events
                        .iter()
                        .filter_map(|e| self.extract_numeric_value(*e, field))
                        .fold(f64::INFINITY, f64::min);
                    
                    let min_val = if min.is_infinite() { 0.0 } else { min };
                    (format!("min_{}", field), AggregationValue::Min(min_val))
                },
                AggregationFunction::Max { field } => {
                    let max = events
                        .iter()
                        .filter_map(|e| self.extract_numeric_value(*e, field))
                        .fold(f64::NEG_INFINITY, f64::max);
                    
                    let max_val = if max.is_infinite() { 0.0 } else { max };
                    (format!("max_{}", field), AggregationValue::Max(max_val))
                },
                AggregationFunction::Distinct { field } => {
                    let mut unique_values = std::collections::HashSet::new();
                    for event in events {
                        let value = self.extract_string_value(*event, field);
                        unique_values.insert(value);
                    }
                    (format!("distinct_{}", field), AggregationValue::Distinct(unique_values.len() as u64))
                },
            };
            
            results.insert(name, value);
        }
        
        results
    }
}

#[async_trait]
impl Aggregator for DefaultAggregator {
    async fn aggregate(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &AggregationConfig,
    ) -> Result<Vec<AggregationResult>> {
        // Filter events by time range if specified
        let filtered_events: Vec<Box<dyn Event>> = if let Some((start, end)) = config.time_range {
            events
                .into_iter()
                .filter(|event| {
                    let timestamp = event.timestamp();
                    timestamp >= start && timestamp <= end
                })
                .collect()
        } else {
            events
        };
        
        // Group events by time bucket and additional group_by fields
        let mut buckets: HashMap<(SystemTime, Vec<(String, String)>), Vec<&dyn Event>> = HashMap::new();
        
        for event in &filtered_events {
            let time_bucket = self.calculate_time_bucket(event.timestamp(), &config.time_period);
            
            let group_values = if let Some(ref group_by) = config.group_by {
                self.create_group_key(event.as_ref(), group_by)
            } else {
                HashMap::new()
            };
            
            // Convert group_values to sorted vector for consistent keys
            let mut group_vec: Vec<(String, String)> = group_values.into_iter().collect();
            group_vec.sort();
            
            let key = (time_bucket, group_vec);
            buckets.entry(key).or_insert_with(Vec::new).push(event.as_ref());
        }
        
        // Apply aggregations to each bucket
        let mut results = Vec::new();
        
        for ((time_bucket, group_vec), bucket_events) in buckets {
            let group_values: HashMap<String, String> = group_vec.into_iter().collect();
            let aggregations = self.apply_aggregations(&bucket_events, &config.functions);
            
            results.push(AggregationResult {
                time_bucket,
                group_values,
                aggregations,
            });
        }
        
        // Sort results by time bucket
        results.sort_by_key(|r| r.time_bucket);
        
        // Apply max_buckets limit
        if let Some(max_buckets) = config.max_buckets {
            results.truncate(max_buckets);
        }
        
        Ok(results)
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
    
    fn create_test_event(id: &str, chain: &str, block_number: u64, timestamp_offset: u64) -> Box<dyn Event> {
        Box::new(TestEvent {
            id: id.to_string(),
            chain: chain.to_string(),
            block_number,
            block_hash: format!("hash_{}", block_number),
            tx_hash: format!("tx_{}", id),
            timestamp: UNIX_EPOCH + Duration::from_secs(timestamp_offset),
            event_type: "transfer".to_string(),
            raw_data: format!(r#"{{"amount": {}}}"#, block_number * 10).as_bytes().to_vec(),
        })
    }
    
    #[tokio::test]
    async fn test_count_aggregation() {
        let aggregator = DefaultAggregator::new();
        
        let events = vec![
            create_test_event("1", "ethereum", 100, 3600),  // Hour 1
            create_test_event("2", "ethereum", 101, 3660),  // Hour 1
            create_test_event("3", "ethereum", 102, 7200),  // Hour 2
        ];
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Count],
            ..Default::default()
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 2);
        
        // Check first bucket (hour 1)
        assert_eq!(results[0].aggregations.get("count"), Some(&AggregationValue::Count(2)));
        
        // Check second bucket (hour 2)
        assert_eq!(results[1].aggregations.get("count"), Some(&AggregationValue::Count(1)));
    }
    
    #[tokio::test]
    async fn test_sum_aggregation() {
        let aggregator = DefaultAggregator::new();
        
        let events = vec![
            create_test_event("1", "ethereum", 100, 3600),  // Amount: 1000
            create_test_event("2", "ethereum", 200, 3660),  // Amount: 2000
            create_test_event("3", "ethereum", 300, 7200),  // Amount: 3000
        ];
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Sum { field: "amount".to_string() }],
            ..Default::default()
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 2);
        
        // Check first bucket (sum of 1000 + 2000)
        assert_eq!(results[0].aggregations.get("sum_amount"), Some(&AggregationValue::Sum(3000.0)));
        
        // Check second bucket (sum of 3000)
        assert_eq!(results[1].aggregations.get("sum_amount"), Some(&AggregationValue::Sum(3000.0)));
    }
    
    #[tokio::test]
    async fn test_group_by_aggregation() {
        let aggregator = DefaultAggregator::new();
        
        let events = vec![
            create_test_event("1", "ethereum", 100, 3600),
            create_test_event("2", "polygon", 101, 3660),
            create_test_event("3", "ethereum", 102, 3720),
        ];
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Count],
            group_by: Some(vec!["chain".to_string()]),
            ..Default::default()
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 2); // One for ethereum, one for polygon
        
        // Find ethereum group
        let ethereum_result = results.iter()
            .find(|r| r.group_values.get("chain") == Some(&"ethereum".to_string()))
            .unwrap();
        assert_eq!(ethereum_result.aggregations.get("count"), Some(&AggregationValue::Count(2)));
        
        // Find polygon group
        let polygon_result = results.iter()
            .find(|r| r.group_values.get("chain") == Some(&"polygon".to_string()))
            .unwrap();
        assert_eq!(polygon_result.aggregations.get("count"), Some(&AggregationValue::Count(1)));
    }
    
    #[tokio::test]
    async fn test_average_aggregation() {
        let aggregator = DefaultAggregator::new();
        
        let events = vec![
            create_test_event("1", "ethereum", 100, 3600),  // block_number: 100
            create_test_event("2", "ethereum", 200, 3660),  // block_number: 200
        ];
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Average { field: "block_number".to_string() }],
            ..Default::default()
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].aggregations.get("avg_block_number"), Some(&AggregationValue::Average(150.0)));
    }
    
    #[tokio::test]
    async fn test_multiple_aggregations() {
        let aggregator = DefaultAggregator::new();
        
        let events = vec![
            create_test_event("1", "ethereum", 100, 3600),
            create_test_event("2", "ethereum", 200, 3660),
        ];
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![
                AggregationFunction::Count,
                AggregationFunction::Sum { field: "block_number".to_string() },
                AggregationFunction::Average { field: "block_number".to_string() },
                AggregationFunction::Min { field: "block_number".to_string() },
                AggregationFunction::Max { field: "block_number".to_string() },
            ],
            ..Default::default()
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 1);
        let aggregations = &results[0].aggregations;
        
        assert_eq!(aggregations.get("count"), Some(&AggregationValue::Count(2)));
        assert_eq!(aggregations.get("sum_block_number"), Some(&AggregationValue::Sum(300.0)));
        assert_eq!(aggregations.get("avg_block_number"), Some(&AggregationValue::Average(150.0)));
        assert_eq!(aggregations.get("min_block_number"), Some(&AggregationValue::Min(100.0)));
        assert_eq!(aggregations.get("max_block_number"), Some(&AggregationValue::Max(200.0)));
    }
} 