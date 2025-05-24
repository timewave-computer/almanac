/// Event aggregation functionality for time-based analytics
use std::collections::HashMap;
use std::time::SystemTime;
use async_trait::async_trait;

use crate::event::Event;
use crate::types::{
    AggregationConfig, AggregationResult, AggregationFunction, AggregationValue, TimePeriod
};
use crate::Result;

/// Trait for event aggregation implementations
#[async_trait]
pub trait Aggregator: Send + Sync {
    /// Aggregate events according to the configuration
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
    
    /// Convert time period to seconds
    fn time_period_to_seconds(&self, period: &TimePeriod) -> u64 {
        match period {
            TimePeriod::Hour => 3600,
            TimePeriod::Day => 86400,
            TimePeriod::Week => 604800,
            TimePeriod::Month => 2629746, // Average month
            TimePeriod::Year => 31556952, // Average year
            TimePeriod::Custom { seconds } => *seconds,
        }
    }
    
    /// Create time bucket for an event
    fn get_time_bucket(&self, timestamp: SystemTime, period: &TimePeriod) -> SystemTime {
        let period_seconds = self.time_period_to_seconds(period);
        
        let epoch_seconds = timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Round down to the nearest period boundary
        let bucket_seconds = (epoch_seconds / period_seconds) * period_seconds;
        
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(bucket_seconds)
    }
    
    /// Extract numeric value from event data
    fn extract_numeric_value(&self, event: &dyn Event, field: &str) -> Option<f64> {
        // Try to parse from raw data as JSON
        if let Ok(raw_str) = String::from_utf8(event.raw_data().to_vec()) {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&raw_str) {
                return self.extract_numeric_from_json(&json_value, field);
            }
        }
        
        // Fallback to built-in fields
        match field {
            "block_number" => Some(event.block_number() as f64),
            "timestamp" => {
                let timestamp = event.timestamp()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                Some(timestamp as f64)
            }
            _ => None,
        }
    }
    
    /// Extract numeric value from JSON
    fn extract_numeric_from_json(&self, json: &serde_json::Value, field: &str) -> Option<f64> {
        match json {
            serde_json::Value::Object(obj) => {
                obj.get(field).and_then(|v| match v {
                    serde_json::Value::Number(n) => n.as_f64(),
                    serde_json::Value::String(s) => s.parse::<f64>().ok(),
                    _ => None,
                })
            },
            _ => None,
        }
    }
    
    /// Extract grouping value from event
    fn extract_grouping_value(&self, event: &dyn Event, field: &str) -> String {
        match field {
            "chain" => event.chain().to_string(),
            "event_type" => event.event_type().to_string(),
            "block_hash" => event.block_hash().to_string(),
            "tx_hash" => event.tx_hash().to_string(),
            _ => {
                // Try to parse from raw data as JSON
                if let Ok(raw_str) = String::from_utf8(event.raw_data().to_vec()) {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&raw_str) {
                        return self.extract_string_from_json(&json_value, field)
                            .unwrap_or_else(|| "unknown".to_string());
                    }
                }
                "unknown".to_string()
            }
        }
    }
    
    /// Extract string value from JSON
    fn extract_string_from_json(&self, json: &serde_json::Value, field: &str) -> Option<String> {
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
    
    /// Create group key for an event
    fn create_group_key(&self, event: &dyn Event, time_bucket: SystemTime, group_by: &[String]) -> String {
        let mut key_parts = vec![
            time_bucket
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        ];
        
        for field in group_by {
            key_parts.push(self.extract_grouping_value(event, field));
        }
        
        key_parts.join("::")
    }
}

#[async_trait]
impl Aggregator for DefaultAggregator {
    async fn aggregate(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &AggregationConfig,
    ) -> Result<Vec<AggregationResult>> {
        let mut buckets: HashMap<String, Vec<&dyn Event>> = HashMap::new();
        let mut bucket_metadata: HashMap<String, (SystemTime, HashMap<String, String>)> = HashMap::new();
        
        // Filter events by time range if specified
        let filtered_events: Vec<&Box<dyn Event>> = if let Some((start_time, end_time)) = &config.time_range {
            events.iter()
                .filter(|event| {
                    let timestamp = event.timestamp();
                    timestamp >= *start_time && timestamp <= *end_time
                })
                .collect()
        } else {
            events.iter().collect()
        };
        
        // Group events into time buckets
        for event in filtered_events {
            let time_bucket = self.get_time_bucket(event.timestamp(), &config.time_period);
            let group_by_fields = config.group_by.as_deref().unwrap_or(&[]);
            let group_key = self.create_group_key(event.as_ref(), time_bucket, group_by_fields);
            
            buckets.entry(group_key.clone()).or_insert_with(Vec::new).push(event.as_ref());
            
            // Store bucket metadata
            if !bucket_metadata.contains_key(&group_key) {
                let mut group_values = HashMap::new();
                for field in group_by_fields {
                    group_values.insert(field.clone(), self.extract_grouping_value(event.as_ref(), field));
                }
                bucket_metadata.insert(group_key.clone(), (time_bucket, group_values));
            }
        }
        
        // Apply max_buckets limit if specified
        let mut bucket_keys: Vec<String> = buckets.keys().cloned().collect();
        bucket_keys.sort();
        
        if let Some(max_buckets) = config.max_buckets {
            bucket_keys.truncate(max_buckets);
        }
        
        // Calculate aggregations for each bucket
        let mut results = Vec::new();
        
        for bucket_key in bucket_keys {
            if let (Some(bucket_events), Some((time_bucket, group_values))) = 
                (buckets.get(&bucket_key), bucket_metadata.get(&bucket_key)) {
                
                let mut aggregations = HashMap::new();
                
                for function in &config.functions {
                    let (agg_name, agg_value) = match function {
                        AggregationFunction::Count => {
                            ("count".to_string(), AggregationValue::Count(bucket_events.len() as u64))
                        }
                        AggregationFunction::Sum { field } => {
                            let sum: f64 = bucket_events.iter()
                                .filter_map(|event| self.extract_numeric_value(*event, field))
                                .sum();
                            (format!("sum_{}", field), AggregationValue::Sum(sum))
                        }
                        AggregationFunction::Average { field } => {
                            let values: Vec<f64> = bucket_events.iter()
                                .filter_map(|event| self.extract_numeric_value(*event, field))
                                .collect();
                            let avg = if !values.is_empty() {
                                values.iter().sum::<f64>() / values.len() as f64
                            } else {
                                0.0
                            };
                            (format!("avg_{}", field), AggregationValue::Average(avg))
                        }
                        AggregationFunction::Min { field } => {
                            let min = bucket_events.iter()
                                .filter_map(|event| self.extract_numeric_value(*event, field))
                                .fold(f64::INFINITY, f64::min);
                            let min_value = if min.is_finite() { min } else { 0.0 };
                            (format!("min_{}", field), AggregationValue::Min(min_value))
                        }
                        AggregationFunction::Max { field } => {
                            let max = bucket_events.iter()
                                .filter_map(|event| self.extract_numeric_value(*event, field))
                                .fold(f64::NEG_INFINITY, f64::max);
                            let max_value = if max.is_finite() { max } else { 0.0 };
                            (format!("max_{}", field), AggregationValue::Max(max_value))
                        }
                        AggregationFunction::Distinct { field } => {
                            let distinct_values: std::collections::HashSet<String> = bucket_events.iter()
                                .map(|event| self.extract_grouping_value(*event, field))
                                .collect();
                            (format!("distinct_{}", field), AggregationValue::Distinct(distinct_values.len() as u64))
                        }
                    };
                    
                    aggregations.insert(agg_name, agg_value);
                }
                
                results.push(AggregationResult {
                    time_bucket: *time_bucket,
                    group_values: group_values.clone(),
                    aggregations,
                });
            }
        }
        
        // Sort results by time bucket
        results.sort_by_key(|result| result.time_bucket);
        
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
    
    fn create_test_event(id: &str, timestamp_offset: u64, amount: u64) -> Box<dyn Event> {
        Box::new(TestEvent {
            id: id.to_string(),
            chain: "ethereum".to_string(),
            block_number: 100 + timestamp_offset,
            block_hash: format!("hash_{}", timestamp_offset),
            tx_hash: format!("tx_{}", timestamp_offset),
            timestamp: UNIX_EPOCH + Duration::from_secs(timestamp_offset),
            event_type: "transfer".to_string(),
            raw_data: format!(r#"{{"amount": {}}}"#, amount).as_bytes().to_vec(),
        })
    }
    
    #[tokio::test]
    async fn test_count_aggregation() {
        let aggregator = DefaultAggregator::new();
        
        let events = vec![
            create_test_event("1", 1000, 10),
            create_test_event("2", 1001, 20),
            create_test_event("3", 1002, 30),
        ];
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Count],
            group_by: None,
            time_range: None,
            max_buckets: Some(10),
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 1); // All events in same hour bucket
        assert_eq!(results[0].aggregations.get("count"), Some(&AggregationValue::Count(3)));
    }
    
    #[tokio::test]
    async fn test_sum_aggregation() {
        let aggregator = DefaultAggregator::new();
        
        let events = vec![
            create_test_event("1", 1000, 10),
            create_test_event("2", 1001, 20),
            create_test_event("3", 1002, 30),
        ];
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Sum { field: "amount".to_string() }],
            group_by: None,
            time_range: None,
            max_buckets: Some(10),
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].aggregations.get("sum_amount"), Some(&AggregationValue::Sum(60.0)));
    }
    
    #[tokio::test]
    async fn test_average_aggregation() {
        let aggregator = DefaultAggregator::new();
        
        let events = vec![
            create_test_event("1", 1000, 10),
            create_test_event("2", 1001, 20),
            create_test_event("3", 1002, 30),
        ];
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Average { field: "amount".to_string() }],
            group_by: None,
            time_range: None,
            max_buckets: Some(10),
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].aggregations.get("avg_amount"), Some(&AggregationValue::Average(20.0)));
    }
    
    #[tokio::test]
    async fn test_time_bucket_grouping() {
        let aggregator = DefaultAggregator::new();
        
        let events = vec![
            create_test_event("1", 1000, 10),  // First hour
            create_test_event("2", 4600, 20),  // Second hour (1 hour + 10 min later)
            create_test_event("3", 4601, 30),  // Second hour
        ];
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Count],
            group_by: None,
            time_range: None,
            max_buckets: Some(10),
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 2); // Two different hour buckets
        assert_eq!(results[0].aggregations.get("count"), Some(&AggregationValue::Count(1)));
        assert_eq!(results[1].aggregations.get("count"), Some(&AggregationValue::Count(2)));
    }
    
    #[tokio::test]
    async fn test_grouping_by_field() {
        let aggregator = DefaultAggregator::new();
        
        let mut events = vec![
            create_test_event("1", 1000, 10),
            create_test_event("2", 1001, 20),
            create_test_event("3", 1002, 30),
        ];
        
        // Modify chain for event 3
        if let Some(test_event) = events[2].as_any().downcast_ref::<TestEvent>() {
            let mut modified_event = test_event.clone();
            modified_event.chain = "polygon".to_string();
            events[2] = Box::new(modified_event);
        }
        
        let config = AggregationConfig {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Count],
            group_by: Some(vec!["chain".to_string()]),
            time_range: None,
            max_buckets: Some(10),
        };
        
        let results = aggregator.aggregate(events, &config).await.unwrap();
        
        assert_eq!(results.len(), 2); // Two different chains
        
        // Find ethereum and polygon groups
        let ethereum_result = results.iter().find(|r| r.group_values.get("chain") == Some(&"ethereum".to_string()));
        let polygon_result = results.iter().find(|r| r.group_values.get("chain") == Some(&"polygon".to_string()));
        
        assert!(ethereum_result.is_some());
        assert!(polygon_result.is_some());
        
        assert_eq!(ethereum_result.unwrap().aggregations.get("count"), Some(&AggregationValue::Count(2)));
        assert_eq!(polygon_result.unwrap().aggregations.get("count"), Some(&AggregationValue::Count(1)));
    }
} 