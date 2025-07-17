// Test fixtures and mock data generation for comprehensive testing
//
// This module provides test data, mock objects, and utilities for all testing scenarios
// including events, contracts, API responses, and performance test data

use std::any::Any;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use almanac_infrastructure_core::types::Event;

/// Mock event implementation for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEvent {
    pub id: String,
    pub chain_id: String,
    pub block_number: u64,
    pub block_hash: String,
    pub transaction_hash: String,
    pub transaction_index: Option<u32>,
    pub event_type: String,
    pub data: Vec<u8>,
    pub attributes: HashMap<String, Value>,
    pub timestamp: SystemTime,
}

impl Event for MockEvent {
    fn id(&self) -> &str {
        &self.id
    }

    fn chain(&self) -> &str {
        &self.chain_id
    }

    fn block_number(&self) -> u64 {
        self.block_number
    }

    fn block_hash(&self) -> &str {
        &self.block_hash
    }

    fn tx_hash(&self) -> &str {
        &self.transaction_hash
    }

    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    fn event_type(&self) -> &str {
        &self.event_type
    }

    fn raw_data(&self) -> &[u8] {
        &self.data
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MockEvent {
    /// Helper method to access chain_id field for backward compatibility
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    /// Helper method to access transaction_hash field for backward compatibility
    pub fn transaction_hash(&self) -> &str {
        &self.transaction_hash
    }

    /// Helper method to access transaction_index field
    pub fn transaction_index(&self) -> Option<u32> {
        self.transaction_index
    }

    /// Helper method to access data field
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Helper method to access attributes field
    pub fn attributes(&self) -> &HashMap<String, Value> {
        &self.attributes
    }

    /// Helper method to get timestamp as DateTime<Utc>
    pub fn timestamp_utc(&self) -> DateTime<Utc> {
        DateTime::from(self.timestamp)
    }
}

/// Create test events with default data
pub fn create_test_events(count: usize) -> Vec<Box<dyn Event>> {
    let mut events = Vec::new();
    
    for i in 0..count {
        let chain_id = match i % 3 {
            0 => "ethereum",
            1 => "osmosis", 
            _ => "cosmoshub",
        };
        
        let event_type = match i % 4 {
            0 => "Transfer",
            1 => "Swap",
            2 => "Mint",
            _ => "Burn",
        };
        
        let event = create_test_event_with_params(
            &format!("event_{}", i),
            chain_id,
            (i as u64) + 1,
            &format!("0x{:064x}", i),
            &format!("0x{:064x}", i + 1000),
            event_type,
            &format!("test_data_{}", i).as_bytes(),
        );
        
        events.push(Box::new(event) as Box<dyn Event>);
    }
    
    events
}

/// Create test events with a specific prefix
pub fn create_test_events_with_prefix(count: usize, prefix: &str) -> Vec<Box<dyn Event>> {
    let mut events = Vec::new();
    
    for i in 0..count {
        let event = create_test_event_with_params(
            &format!("{}_event_{}", prefix, i),
            "ethereum",
            (i as u64) + 1,
            &format!("0x{:064x}", i),
            &format!("0x{:064x}", i + 1000),
            "Transfer",
            &format!("{}_data_{}", prefix, i).as_bytes(),
        );
        
        events.push(Box::new(event) as Box<dyn Event>);
    }
    
    events
}

/// Create a test event with specific parameters
pub fn create_test_event_with_params(
    id: &str,
    chain_id: &str,
    block_number: u64,
    block_hash: &str,
    transaction_hash: &str,
    event_type: &str,
    data: &[u8],
) -> MockEvent {
    let mut attributes = HashMap::new();
    attributes.insert("from".to_string(), json!("0x1234567890123456789012345678901234567890"));
    attributes.insert("to".to_string(), json!("0x0987654321098765432109876543210987654321"));
    attributes.insert("amount".to_string(), json!("1000000000000000000"));
    
    MockEvent {
        id: id.to_string(),
        chain_id: chain_id.to_string(),
        block_number,
        block_hash: block_hash.to_string(),
        transaction_hash: transaction_hash.to_string(),
        transaction_index: Some(0),
        event_type: event_type.to_string(),
        data: data.to_vec(),
        attributes,
        timestamp: SystemTime::now(),
    }
}

/// Create test event with custom data
pub fn create_test_event_with_data(
    id: &str,
    chain_id: &str,
    block_number: u64,
    block_hash: &str,
    transaction_hash: &str,
    event_type: &str,
    data: &[u8],
) -> MockEvent {
    create_test_event_with_params(id, chain_id, block_number, block_hash, transaction_hash, event_type, data)
}

/// Create a simple test event with minimal parameters
pub fn create_simple_test_event(id: &str, chain_id: &str, block_number: u64) -> MockEvent {
    create_test_event_with_params(
        id,
        chain_id,
        block_number,
        &format!("0x{:064x}", block_number),
        &format!("0x{:064x}", block_number + 1000),
        "Transfer",
        b"simple_test_data",
    )
}

/// Create test event with transaction index
pub fn create_test_event_with_index(
    id: &str,
    chain_id: &str,
    block_number: u64,
    transaction_index: u32,
) -> MockEvent {
    let mut event = create_simple_test_event(id, chain_id, block_number);
    event.transaction_index = Some(transaction_index);
    event
}

/// Create a valid test event for validation testing
pub fn create_valid_test_event(
    id: &str,
    chain_id: &str,
    block_number: u64,
    block_hash: &str,
    transaction_hash: &str,
    event_type: &str,
) -> MockEvent {
    create_test_event_with_params(
        id,
        chain_id,
        block_number,
        block_hash,
        transaction_hash,
        event_type,
        b"valid_test_data",
    )
}

/// Validate a test event (simplified validation)
pub fn validate_test_event(id: &str, chain_id: &str, block_number: u64) -> bool {
    !id.is_empty() && !chain_id.is_empty() && block_number > 0
}

/// Mock storage operation result
#[derive(Debug, Clone)]
pub struct MockStorageResult {
    pub id: String,
    pub chain_id: String,
    pub block_number: u64,
    pub success: bool,
}

/// Simulate storage operation
pub fn simulate_storage_operation(event: &dyn Event) -> MockStorageResult {
    MockStorageResult {
        id: event.id().to_string(),
        chain_id: event.chain().to_string(),
        block_number: event.block_number(),
        success: true,
    }
}

/// Simulate concurrent storage operation
pub fn simulate_concurrent_storage(id: &str) -> String {
    // Simulate some processing delay and return the same ID
    std::thread::sleep(Duration::from_millis(1));
    id.to_string()
}

/// Mock performance result
#[derive(Debug, Clone)]
pub struct MockPerformanceResult {
    pub duration_ms: u64,
    pub operations_count: usize,
    pub success_rate: f64,
}

/// Simulate storage performance test
pub fn simulate_storage_performance(events: &[Box<dyn Event>]) -> MockPerformanceResult {
    let operations_count = events.len();
    let duration_ms = operations_count as u64 * 2; // 2ms per operation
    let success_rate = if operations_count > 0 { 0.99 } else { 1.0 };
    
    MockPerformanceResult {
        duration_ms,
        operations_count,
        success_rate,
    }
}

/// Mock failure recovery result
#[derive(Debug, Clone)]
pub struct MockRecoveryResult {
    pub recovered_count: usize,
    pub corruption_detected: bool,
    pub recovery_time_ms: u64,
}

/// Simulate failure recovery
pub fn simulate_failure_recovery(events: &[Box<dyn Event>], scenario: u8) -> MockRecoveryResult {
    match scenario {
        0 => MockRecoveryResult {
            recovered_count: events.len(),
            corruption_detected: false,
            recovery_time_ms: 100,
        },
        1 => MockRecoveryResult {
            recovered_count: events.len() / 2,
            corruption_detected: true,
            recovery_time_ms: 500,
        },
        _ => MockRecoveryResult {
            recovered_count: 0,
            corruption_detected: true,
            recovery_time_ms: 1000,
        },
    }
}

/// Mock API response
#[derive(Debug, Clone)]
pub struct MockApiResponse {
    pub status_code: u16,
    pub body: String,
    pub content_type: String,
    pub error_message: String,
}

/// Simulate API request
pub fn simulate_api_request(query_params: &std::collections::BTreeMap<String, String>) -> MockApiResponse {
    if query_params.is_empty() {
        MockApiResponse {
            status_code: 200,
            body: json!({
                "status": "ok",
                "data": [],
                "pagination": {
                    "total": 0,
                    "limit": 10,
                    "offset": 0
                }
            }).to_string(),
            content_type: "application/json".to_string(),
            error_message: String::new(),
        }
    } else if query_params.keys().any(|k| k.contains(char::is_control)) {
        MockApiResponse {
            status_code: 400,
            body: json!({"error": "Invalid parameters"}).to_string(),
            content_type: "application/json".to_string(),
            error_message: "Invalid parameters".to_string(),
        }
    } else {
        MockApiResponse {
            status_code: 200,
            body: json!({
                "status": "ok",
                "data": query_params,
                "pagination": {
                    "total": query_params.len(),
                    "limit": 10,
                    "offset": 0
                }
            }).to_string(),
            content_type: "application/json".to_string(),
            error_message: String::new(),
        }
    }
}

/// Simulate simple API request
pub fn simulate_api_request_simple(endpoint: &str, query: &str) -> MockApiResponse {
    let success = !endpoint.is_empty() && !endpoint.contains("error");
    
    if success {
        MockApiResponse {
            status_code: 200,
            body: json!({
                "endpoint": endpoint,
                "query": query,
                "timestamp": Utc::now().timestamp()
            }).to_string(),
            content_type: "application/json".to_string(),
            error_message: String::new(),
        }
    } else {
        MockApiResponse {
            status_code: 404,
            body: json!({"error": "Not found"}).to_string(),
            content_type: "application/json".to_string(),
            error_message: "Not found".to_string(),
        }
    }
}

/// Mock paginated response
#[derive(Debug, Clone)]
pub struct MockPaginatedResponse {
    pub items: Vec<Value>,
    pub total_items: usize,
    pub page_size: usize,
    pub current_page: usize,
    pub total_pages: usize,
}

/// Simulate paginated API request
pub fn simulate_paginated_request(page: usize, page_size: usize, total_items: usize) -> MockPaginatedResponse {
    let total_pages = (total_items + page_size - 1) / page_size;
    let start_index = page * page_size;
    let end_index = std::cmp::min(start_index + page_size, total_items);
    
    let items: Vec<Value> = (start_index..end_index)
        .map(|i| json!({
            "id": format!("item_{}", i),
            "index": i,
            "data": format!("test_data_{}", i)
        }))
        .collect();
    
    MockPaginatedResponse {
        items,
        total_items,
        page_size,
        current_page: page,
        total_pages,
    }
}

/// Simulate API request with invalid parameters
pub fn simulate_api_request_with_invalid_params(key: &str, value: &str) -> MockApiResponse {
    let is_invalid = key.is_empty() || 
                    key.len() > 100 || 
                    key.contains(char::is_control) ||
                    value.len() > 1000;
    
    if is_invalid {
        MockApiResponse {
            status_code: 400,
            body: json!({
                "error": "Invalid parameter",
                "details": {
                    "key": key,
                    "value_length": value.len()
                }
            }).to_string(),
            content_type: "application/json".to_string(),
            error_message: "Invalid parameter".to_string(),
        }
    } else {
        MockApiResponse {
            status_code: 200,
            body: json!({
                "status": "ok",
                "params": {
                    key: value
                }
            }).to_string(),
            content_type: "application/json".to_string(),
            error_message: String::new(),
        }
    }
}

/// Mock blockchain data
#[derive(Debug, Clone)]
pub struct MockBlockchainData {
    pub chain_id: String,
    pub latest_block: u64,
    pub block_time: Duration,
    pub events_per_block: usize,
}

/// Create mock Ethereum data
pub fn create_mock_ethereum_data() -> MockBlockchainData {
    MockBlockchainData {
        chain_id: "ethereum".to_string(),
        latest_block: 18_500_000,
        block_time: Duration::from_secs(12),
        events_per_block: 50,
    }
}

/// Create mock Cosmos data
pub fn create_mock_cosmos_data() -> MockBlockchainData {
    MockBlockchainData {
        chain_id: "osmosis-1".to_string(),
        latest_block: 12_000_000,
        block_time: Duration::from_secs(6),
        events_per_block: 25,
    }
}

/// Mock contract data
#[derive(Debug, Clone)]
pub struct MockContractData {
    pub address: String,
    pub name: String,
    pub events: Vec<String>,
    pub functions: Vec<String>,
}

/// Create mock ERC20 contract
pub fn create_mock_erc20_contract() -> MockContractData {
    MockContractData {
        address: "0xA0b86a33E6411183EB78B0B35a6F4bB0B3B8b100".to_string(),
        name: "USDC".to_string(),
        events: vec![
            "Transfer".to_string(),
            "Approval".to_string(),
            "Mint".to_string(),
            "Burn".to_string(),
        ],
        functions: vec![
            "transfer".to_string(),
            "approve".to_string(),
            "mint".to_string(),
            "burn".to_string(),
            "balanceOf".to_string(),
            "totalSupply".to_string(),
        ],
    }
}

/// Create mock CosmWasm contract
pub fn create_mock_cosmwasm_contract() -> MockContractData {
    MockContractData {
        address: "osmo1234567890123456789012345678901234567890".to_string(),
        name: "OsmoSwap".to_string(),
        events: vec![
            "instantiate".to_string(),
            "execute".to_string(),
            "swap".to_string(),
            "provide_liquidity".to_string(),
        ],
        functions: vec![
            "swap".to_string(),
            "provide_liquidity".to_string(),
            "withdraw_liquidity".to_string(),
            "query_pool".to_string(),
        ],
    }
}

/// Test configuration builder
#[derive(Debug, Clone)]
pub struct TestConfigBuilder {
    pub chains: Vec<MockBlockchainData>,
    pub contracts: Vec<MockContractData>,
    pub api_endpoints: Vec<String>,
    pub database_urls: Vec<String>,
}

impl TestConfigBuilder {
    pub fn new() -> Self {
        Self {
            chains: Vec::new(),
            contracts: Vec::new(),
            api_endpoints: Vec::new(),
            database_urls: Vec::new(),
        }
    }
    
    pub fn with_ethereum(mut self) -> Self {
        self.chains.push(create_mock_ethereum_data());
        self.contracts.push(create_mock_erc20_contract());
        self
    }
    
    pub fn with_cosmos(mut self) -> Self {
        self.chains.push(create_mock_cosmos_data());
        self.contracts.push(create_mock_cosmwasm_contract());
        self
    }
    
    pub fn with_api_endpoints(mut self, endpoints: Vec<String>) -> Self {
        self.api_endpoints = endpoints;
        self
    }
    
    pub fn with_databases(mut self, urls: Vec<String>) -> Self {
        self.database_urls = urls;
        self
    }
    
    pub fn build(self) -> TestConfiguration {
        TestConfiguration {
            chains: self.chains,
            contracts: self.contracts,
            api_endpoints: self.api_endpoints,
            database_urls: self.database_urls,
        }
    }
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete test configuration
#[derive(Debug, Clone)]
pub struct TestConfiguration {
    pub chains: Vec<MockBlockchainData>,
    pub contracts: Vec<MockContractData>,
    pub api_endpoints: Vec<String>,
    pub database_urls: Vec<String>,
}

/// Create comprehensive test scenario
pub fn create_comprehensive_test_scenario() -> TestConfiguration {
    TestConfigBuilder::new()
        .with_ethereum()
        .with_cosmos()
        .with_api_endpoints(vec![
            "/api/v1/health".to_string(),
            "/api/v1/events/chain/ethereum".to_string(),
            "/api/v1/events/chain/osmosis".to_string(),
            "/api/v1/blocks/ethereum/latest".to_string(),
            "/graphql".to_string(),
        ])
        .with_databases(vec![
            "postgres://test:test@localhost:5433/indexer_test".to_string(),
            "./test_data/rocks".to_string(),
        ])
        .build()
}

/// Mock chaos engineering scenarios
#[derive(Debug, Clone)]
pub enum ChaosScenario {
    NetworkPartition { duration: Duration, affected_percentage: f64 },
    DatabaseFailure { duration: Duration, error_rate: f64 },
    MemoryPressure { duration: Duration, pressure_level: f64 },
    CpuStarvation { duration: Duration, cpu_limit: f64 },
    DiskSpaceExhaustion { duration: Duration, remaining_percentage: f64 },
}

/// Create chaos testing scenarios
pub fn create_chaos_scenarios() -> Vec<ChaosScenario> {
    vec![
        ChaosScenario::NetworkPartition { 
            duration: Duration::from_secs(30), 
            affected_percentage: 0.2 
        },
        ChaosScenario::DatabaseFailure { 
            duration: Duration::from_secs(10), 
            error_rate: 0.1 
        },
        ChaosScenario::MemoryPressure { 
            duration: Duration::from_secs(60), 
            pressure_level: 0.8 
        },
        ChaosScenario::CpuStarvation { 
            duration: Duration::from_secs(45), 
            cpu_limit: 0.1 
        },
        ChaosScenario::DiskSpaceExhaustion { 
            duration: Duration::from_secs(20), 
            remaining_percentage: 0.05 
        },
    ]
}

/// Generate large test dataset
pub fn generate_large_test_dataset(size: usize) -> Vec<Box<dyn Event>> {
    let mut events = Vec::with_capacity(size);
    
    for i in 0..size {
        let chain_idx = i % 5;
        let chain_id = match chain_idx {
            0 => "ethereum",
            1 => "osmosis",
            2 => "cosmoshub",
            3 => "juno",
            _ => "stargaze",
        };
        
        let event = create_test_event_with_params(
            &format!("large_dataset_event_{}", i),
            chain_id,
            (i as u64 / 100) + 1, // Group events into blocks
            &format!("0x{:064x}", i / 100),
            &format!("0x{:064x}", i),
            "Transfer",
            &format!("large_dataset_data_{}", i).as_bytes(),
        );
        
        events.push(Box::new(event) as Box<dyn Event>);
    }
    
    events
}

/// Create time-series test data
pub fn create_time_series_test_data(
    start_time: DateTime<Utc>,
    interval: Duration,
    count: usize,
) -> Vec<Box<dyn Event>> {
    let mut events = Vec::new();
    let mut current_time = start_time;
    
    for i in 0..count {
        let mut event = create_simple_test_event(
            &format!("timeseries_event_{}", i),
            "ethereum",
            i as u64 + 1,
        );
        event.timestamp = current_time.into();
        
        events.push(Box::new(event) as Box<dyn Event>);
        current_time += chrono::Duration::from_std(interval).unwrap();
    }
    
    // Verify time ordering
    for i in 1..events.len() {
        let prev_time = events[i-1].timestamp();
        let curr_time = events[i].timestamp();
        
        if curr_time < prev_time {
            panic!("Time series not properly ordered at index {}", i);
        }
        
        // Verify the expected interval
        if let Ok(diff) = curr_time.duration_since(prev_time) {
            let expected_interval = chrono::Duration::from_std(interval).unwrap();
            let actual_interval = chrono::Duration::from_std(diff).unwrap();
            assert_eq!(actual_interval, expected_interval);
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_events() {
        let events = create_test_events(5);
        assert_eq!(events.len(), 5);
        
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.id(), format!("event_{}", i));
            assert!(event.block_number() > 0);
            assert!(!event.chain().is_empty());
        }
    }

    #[test]
    fn test_mock_api_response() {
        let empty_params = std::collections::BTreeMap::new();
        let response = simulate_api_request(&empty_params);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"status\":\"ok\""));
    }

    #[test]
    fn test_paginated_response() {
        let response = simulate_paginated_request(0, 5, 20);
        assert_eq!(response.items.len(), 5);
        assert_eq!(response.total_items, 20);
        assert_eq!(response.current_page, 0);
        assert_eq!(response.total_pages, 4);
    }

    #[test]
    fn test_test_config_builder() {
        let config = TestConfigBuilder::new()
            .with_ethereum()
            .with_cosmos()
            .build();
        
        assert_eq!(config.chains.len(), 2);
        assert_eq!(config.contracts.len(), 2);
        assert_eq!(config.chains[0].chain_id, "ethereum");
        assert_eq!(config.chains[1].chain_id, "osmosis-1");
    }

    #[test]
    fn test_large_dataset_generation() {
        let events = generate_large_test_dataset(1000);
        assert_eq!(events.len(), 1000);
        
        // Check that we have events from different chains
        let chains: std::collections::HashSet<_> = events.iter()
            .map(|e| e.chain())
            .collect();
        assert!(chains.len() > 1);
    }

    #[test]
    fn test_time_series_data() {
        let start = Utc::now();
        let interval = Duration::from_secs(60);
        let events = create_time_series_test_data(start, interval, 10);
        
        assert_eq!(events.len(), 10);
        
        // Check time progression
        for i in 1..events.len() {
            let prev_time = events[i-1].timestamp();
            let curr_time = events[i].timestamp();
            
            if curr_time < prev_time {
                panic!("Time series not properly ordered at index {}", i);
            }
            
            // Verify the expected interval
            if let Ok(diff) = curr_time.duration_since(prev_time) {
                let expected_interval = chrono::Duration::from_std(interval).unwrap();
                let actual_interval = chrono::Duration::from_std(diff).unwrap();
                assert_eq!(actual_interval, expected_interval);
            }
        }
    }
} 