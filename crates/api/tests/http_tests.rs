/// HTTP API integration tests
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::time::Duration;
use async_trait::async_trait;

use indexer_api::{
    ContractSchemaRegistry, InMemorySchemaRegistry, ContractSchema, ContractSchemaVersion, EventSchema, FieldSchema,
    auth::{AuthState, UserRole},
    http::{HttpState, EventFilterRequest, EventsQuery, AggregationRequest},
};
use indexer_core::{
    Result,
    event::Event,
    service::{BoxedEventService, EventService, EventSubscription, EventServiceWrapper},
    security::RateLimiter,
    types::{EventFilter, ChainId},
};

// Mock event implementation for testing
#[derive(Debug, Clone)]
pub struct MockEvent {
    pub id: String,
    pub chain: String,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_hash: String,
    pub timestamp: SystemTime,
    pub event_type: String,
    pub raw_data: Vec<u8>,
}

impl Event for MockEvent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn chain(&self) -> &str {
        &self.chain
    }
    
    fn block_number(&self) -> u64 {
        self.block_number
    }
    
    fn block_hash(&self) -> &str {
        &self.block_hash
    }
    
    fn tx_hash(&self) -> &str {
        &self.tx_hash
    }
    
    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    
    fn event_type(&self) -> &str {
        &self.event_type
    }
    
    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Mock event service for testing
#[derive(Debug)]
struct MockEventService {
    events: Vec<MockEvent>,
    chain_id: ChainId,
}

impl MockEventService {
    fn new() -> Self {
        let events = vec![
            MockEvent {
                id: "event-1".to_string(),
                chain: "ethereum".to_string(),
                block_number: 1000,
                block_hash: "0xabc123".to_string(),
                tx_hash: "0xdef456".to_string(),
                timestamp: UNIX_EPOCH + Duration::from_secs(1640000000),
                event_type: "Transfer".to_string(),
                raw_data: b"transfer data".to_vec(),
            },
            MockEvent {
                id: "event-2".to_string(),
                chain: "ethereum".to_string(),
                block_number: 1001,
                block_hash: "0xabc124".to_string(),
                tx_hash: "0xdef457".to_string(),
                timestamp: UNIX_EPOCH + Duration::from_secs(1640000060),
                event_type: "Approval".to_string(),
                raw_data: b"approval data".to_vec(),
            },
        ];
        
        Self { 
            events,
            chain_id: ChainId::from("ethereum".to_string()),
        }
    }
}

#[async_trait]
impl EventService for MockEventService {
    type EventType = MockEvent;

    fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        let mut results: Vec<Box<dyn Event>> = Vec::new();
        
        for filter in filters {
            for event in &self.events {
                let mut matches = true;
                
                // Check chain filter
                if let Some(ref chains) = filter.chains {
                    if !chains.contains(&event.chain) {
                        matches = false;
                    }
                }
                
                // Check block range filter
                if let Some((start, end)) = filter.block_range {
                    if event.block_number < start || event.block_number > end {
                        matches = false;
                    }
                }
                
                // Check event type filter
                if let Some(ref event_types) = filter.event_types {
                    if !event_types.contains(&event.event_type) {
                        matches = false;
                    }
                }
                
                if matches {
                    results.push(Box::new(event.clone()));
                }
            }
        }
        
        Ok(results)
    }

    async fn get_latest_block(&self) -> Result<u64> {
        Ok(1010)
    }

    async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> {
        // Mock subscription - not implemented for tests
        todo!("Mock subscription not implemented")
    }
}

fn create_test_http_state() -> HttpState {
    let mock_service = Arc::new(MockEventService::new());
    let event_service: BoxedEventService = Arc::new(EventServiceWrapper::new(mock_service));
    let schema_registry = Arc::new(InMemorySchemaRegistry::new());
    let jwt_secret = b"test-secret-key-for-testing-only-32-bytes";
    let auth_state = AuthState::new(jwt_secret);
    let rate_limiter = Arc::new(RateLimiter::new(100, Duration::from_secs(60)));
    
    HttpState {
        event_service,
        schema_registry,
        auth_state,
        rate_limiter,
        start_time: SystemTime::now(),
    }
}

#[tokio::test]
async fn test_http_state_creation() {
    let state = create_test_http_state();
    
    let latest = state.event_service.get_latest_block().await;
    assert!(latest.is_ok());
    assert_eq!(latest.unwrap(), 1010);
    
    let filter = EventFilter {
        chains: Some(vec!["ethereum".to_string()]),
        block_range: Some((1000, 1010)),
        event_types: Some(vec!["Transfer".to_string()]),
        ..Default::default()
    };
    
    let events = state.event_service.get_events(vec![filter]).await;
    assert!(events.is_ok());
    let events = events.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id(), "event-1");
}

#[test]
fn test_event_filter_conversion() {
    let api_filter = EventFilterRequest {
        chain_id: Some("ethereum".to_string()),
        address: Some("0x123".to_string()),
        event_type: Some("Transfer".to_string()),
        attributes: Some(HashMap::new()),
        from_height: Some(100),
        to_height: Some(200),
        limit: Some(10),
        offset: Some(0),
        text_query: None,
        text_search_mode: None,
        case_sensitive: None,
        max_search_results: None,
    };
    
    let core_filter: EventFilter = api_filter.into();
    
    // Check that the conversion sets the correct fields
    assert_eq!(core_filter.limit, Some(10));
    assert_eq!(core_filter.offset, Some(0));
    
    // Check that chain_ids is set correctly
    if let Some(chain_ids) = &core_filter.chain_ids {
        assert_eq!(chain_ids.len(), 1);
        assert_eq!(chain_ids[0].0, "ethereum");
    } else {
        panic!("chain_ids should be set");
    }
    
    // Check block range
    assert_eq!(core_filter.block_range, Some((100, 200)));
    
    // Check event types
    if let Some(event_types) = &core_filter.event_types {
        assert_eq!(event_types.len(), 1);
        assert_eq!(event_types[0], "Transfer");
    } else {
        panic!("event_types should be set");
    }
    
    // Check custom filters (address should be added as a custom filter)
    assert_eq!(core_filter.custom_filters.get("address"), Some(&"0x123".to_string()));
}

#[tokio::test]
async fn test_mock_event_service() {
    let service = MockEventService::new();
    
    // Test getting all events
    let filter = EventFilter {
        chains: Some(vec!["ethereum".to_string()]),
        ..Default::default()
    };
    let events = service.get_events(vec![filter]).await.unwrap();
    assert_eq!(events.len(), 2);
    
    // Test filtering by event type
    let filter = EventFilter {
        chains: Some(vec!["ethereum".to_string()]),
        event_types: Some(vec!["Transfer".to_string()]),
        ..Default::default()
    };
    let events = service.get_events(vec![filter]).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type(), "Transfer");
    
    // Test filtering by block range
    let filter = EventFilter {
        chains: Some(vec!["ethereum".to_string()]),
        block_range: Some((1001, 1001)),
        ..Default::default()
    };
    let events = service.get_events(vec![filter]).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].block_number(), 1001);
    
    // Test filtering with no matches
    let filter = EventFilter {
        chains: Some(vec!["ethereum".to_string()]),
        block_range: Some((2000, 3000)),
        ..Default::default()
    };
    let events = service.get_events(vec![filter]).await.unwrap();
    assert_eq!(events.len(), 0);
}

#[test]
fn test_events_query_basic() {
    let query = EventsQuery {
        limit: Some(50),
        offset: Some(10),
        ascending: Some(true),
        from_height: Some(100),
        to_height: Some(200),
        event_type: Some("Transfer".to_string()),
        text_query: Some("test".to_string()),
        text_search_mode: Some("contains".to_string()),
        case_sensitive: Some(false),
    };
    
    // Test that all fields are properly set
    assert_eq!(query.limit, Some(50));
    assert_eq!(query.offset, Some(10));
    assert_eq!(query.ascending, Some(true));
    assert_eq!(query.from_height, Some(100));
    assert_eq!(query.to_height, Some(200));
    assert_eq!(query.event_type, Some("Transfer".to_string()));
    assert_eq!(query.text_query, Some("test".to_string()));
    assert_eq!(query.text_search_mode, Some("contains".to_string()));
    assert_eq!(query.case_sensitive, Some(false));
}

#[test]
fn test_aggregation_request_basic() {
    let request = AggregationRequest {
        time_period: "hour".to_string(),
        functions: vec!["count".to_string(), "sum".to_string()],
        group_by: Some(vec!["event_type".to_string(), "chain".to_string()]),
        start_time: Some("2023-01-01T00:00:00Z".to_string()),
        end_time: Some("2023-01-02T00:00:00Z".to_string()),
        max_buckets: Some(100),
    };
    
    // Test that all fields are properly set
    assert_eq!(request.time_period, "hour");
    assert_eq!(request.functions.len(), 2);
    assert!(request.functions.contains(&"count".to_string()));
    assert!(request.functions.contains(&"sum".to_string()));
    
    if let Some(group_by) = &request.group_by {
        assert_eq!(group_by.len(), 2);
        assert!(group_by.contains(&"event_type".to_string()));
        assert!(group_by.contains(&"chain".to_string()));
    }
    
    assert!(request.start_time.is_some());
    assert!(request.end_time.is_some());
    assert_eq!(request.max_buckets, Some(100));
}

#[test]
fn test_contract_schema_operations() {
    let registry = InMemorySchemaRegistry::new();
    
    let schema = ContractSchema {
        chain: "ethereum".to_string(),
        address: "0x123".to_string(),
        name: "TestContract".to_string(),
        events: vec![
            EventSchema {
                name: "Transfer".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "from".to_string(),
                        type_name: "address".to_string(),
                        indexed: true,
                    },
                    FieldSchema {
                        name: "to".to_string(),
                        type_name: "address".to_string(),
                        indexed: true,
                    },
                    FieldSchema {
                        name: "value".to_string(),
                        type_name: "uint256".to_string(),
                        indexed: false,
                    },
                ],
            },
        ],
        functions: vec![],
    };
    
    let schema_version = ContractSchemaVersion {
        version: "1.0.0".to_string(),
        schema,
    };
    
    // Test schema registration
    let result = registry.register_schema(schema_version);
    assert!(result.is_ok());
    
    // Test schema retrieval
    let retrieved = registry.get_schema("ethereum", "0x123", "1.0.0");
    assert!(retrieved.is_ok());
    let retrieved = retrieved.unwrap();
    assert!(retrieved.is_some());
    let retrieved_schema = retrieved.unwrap();
    assert_eq!(retrieved_schema.name, "TestContract");
    assert_eq!(retrieved_schema.events.len(), 1);
    assert_eq!(retrieved_schema.events[0].name, "Transfer");
    assert_eq!(retrieved_schema.events[0].fields.len(), 3);
    
    // Test latest schema retrieval
    let latest = registry.get_latest_schema("ethereum", "0x123");
    assert!(latest.is_ok());
    let latest = latest.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().name, "TestContract");
}

#[tokio::test]
async fn test_rate_limiter_basic() {
    let rate_limiter = RateLimiter::new(5, Duration::from_secs(60));
    
    // Test that we can make requests up to the limit
    for _ in 0..5 {
        assert!(rate_limiter.is_allowed("127.0.0.1").await);
    }
    
    // Test that we're rate limited after exceeding the limit
    assert!(!rate_limiter.is_allowed("127.0.0.1").await);
    
    // Test that different IPs have separate limits
    assert!(rate_limiter.is_allowed("192.168.1.1").await);
}

#[test]
fn test_mock_event_properties() {
    let event = MockEvent {
        id: "test-event".to_string(),
        chain: "ethereum".to_string(),
        block_number: 12345,
        block_hash: "0xabcdef".to_string(),
        tx_hash: "0x123456".to_string(),
        timestamp: UNIX_EPOCH + Duration::from_secs(1640000000),
        event_type: "TestEvent".to_string(),
        raw_data: b"test data".to_vec(),
    };
    
    assert_eq!(event.id(), "test-event");
    assert_eq!(event.chain(), "ethereum");
    assert_eq!(event.block_number(), 12345);
    assert_eq!(event.block_hash(), "0xabcdef");
    assert_eq!(event.tx_hash(), "0x123456");
    assert_eq!(event.event_type(), "TestEvent");
    assert_eq!(event.raw_data(), b"test data");
    
    // Test timestamp
    let expected_timestamp = UNIX_EPOCH + Duration::from_secs(1640000000);
    assert_eq!(event.timestamp(), expected_timestamp);
}

#[tokio::test]
async fn test_auth_state_creation() {
    let jwt_secret = b"test-secret-key-for-testing-only-32-bytes";
    let auth_state = AuthState::new(jwt_secret);
    
    // Wait a bit for the admin user to be inserted
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    // Test that we can create users
    let user_result = auth_state.user_store.create_user("testuser".to_string(), UserRole::Read).await;
    assert!(user_result.is_ok());
    
    let user = user_result.unwrap();
    assert_eq!(user.username, "testuser");
    assert_eq!(user.role, UserRole::Read);
    
    // Test that we can list users (admin + testuser)
    let users = auth_state.user_store.list_users().await;
    assert_eq!(users.len(), 2); // admin + testuser
    let usernames: Vec<&str> = users.iter().map(|u| u.username.as_str()).collect();
    assert!(usernames.contains(&"testuser"));
    assert!(usernames.contains(&"admin"));
}

#[tokio::test]
async fn test_http_state_components() {
    let state = create_test_http_state();
    
    // Test that all components are properly initialized
    assert_eq!(state.event_service.chain_id().0, "ethereum");
    
    // Test schema registry
    let schema = ContractSchema {
        chain: "ethereum".to_string(),
        address: "0x123".to_string(),
        name: "TestContract".to_string(),
        events: vec![],
        functions: vec![],
    };
    
    let schema_version = ContractSchemaVersion {
        version: "1.0.0".to_string(),
        schema,
    };
    
    let result = state.schema_registry.register_schema(schema_version);
    assert!(result.is_ok());
    
    let retrieved = state.schema_registry.get_schema("ethereum", "0x123", "1.0.0");
    assert!(retrieved.is_ok());
    assert!(retrieved.unwrap().is_some());
    
    // Test rate limiter
    assert!(state.rate_limiter.is_allowed("127.0.0.1").await);
} 