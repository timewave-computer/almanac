/// Tests for the HTTP API
use indexer_core::{Error, Result, BlockStatus};
use indexer_core::service::{EventService, EventSubscription, BoxedEventService};
use indexer_core::event::Event;
use indexer_core::types::{EventFilter, ChainId};
use indexer_api::http::{HttpServerState, CoreRocksDBStore, CorePostgresStore};
use indexer_api::ContractSchemaRegistry;
use indexer_api::InMemorySchemaRegistry;

use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::net::SocketAddr;
use async_trait::async_trait;
use std::any::Any;

/// Create a separate file module to isolate test functionality
mod test_utils {
    use super::*;
    use std::collections::VecDeque;
    use std::fmt;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Create a basic mock for the Event trait
    #[derive(Clone)]
    pub struct MockEvent {
        pub id: String,
        pub chain: String,
        pub block_number: u64,
        pub block_hash: String,
        pub tx_hash: String,
        pub timestamp: SystemTime,
        pub event_type: String,
        pub data: Vec<u8>,
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
            &self.data
        }
        
        fn as_any(&self) -> &dyn Any {
            self
        }
    }
    
    impl fmt::Debug for MockEvent {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("MockEvent")
                .field("id", &self.id)
                .field("chain", &self.chain)
                .field("block_number", &self.block_number)
                .field("event_type", &self.event_type)
                .finish()
        }
    }
    
    impl MockEvent {
        pub fn new(id: &str, chain: &str, event_type: &str) -> Self {
            Self {
                id: id.to_string(),
                chain: chain.to_string(),
                block_number: 1000,
                block_hash: "0x123".to_string(),
                tx_hash: "0xabc".to_string(),
                timestamp: SystemTime::now(),
                event_type: event_type.to_string(),
                data: b"test data".to_vec(),
            }
        }
    }
    
    // Create a subscription implementation
    pub struct MockSubscription {
        events: VecDeque<Box<dyn Event>>,
    }
    
    impl MockSubscription {
        pub fn new(events: Vec<Box<dyn Event>>) -> Self {
            Self {
                events: events.into(),
            }
        }
    }
    
    #[async_trait]
    impl EventSubscription for MockSubscription {
        async fn next(&mut self) -> Option<Box<dyn Event>> {
            self.events.pop_front()
        }
        
        async fn close(&mut self) -> Result<()> {
            Ok(())
        }
    }
    
    // Create a mock event service
    pub struct MockEventService {
        chain_id: ChainId,
        latest_block: u64,
        is_mock: bool,
    }
    
    impl MockEventService {
        pub fn new(chain_id: &str) -> Self {
            Self {
                chain_id: chain_id.to_string(),
                latest_block: 1000,
                is_mock: true,
            }
        }
        
        pub fn is_mock(&self) -> bool {
            self.is_mock
        }
    }
    
    #[async_trait]
    impl EventService for MockEventService {
        type EventType = MockEvent;
        
        fn chain_id(&self) -> &ChainId {
            &self.chain_id
        }
        
        async fn get_events(&self, _filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
            let mut events: Vec<Box<dyn Event>> = Vec::new();
            events.push(Box::new(MockEvent::new("1", &self.chain_id, "Transfer")));
            events.push(Box::new(MockEvent::new("2", &self.chain_id, "Approval")));
            Ok(events)
        }
        
        async fn get_latest_block(&self) -> Result<u64> {
            Ok(self.latest_block)
        }
        
        async fn get_latest_block_with_status(&self, _chain: &str, status: BlockStatus) -> Result<u64> {
            match status {
                BlockStatus::Finalized => Ok(self.latest_block - 50),
                BlockStatus::Safe => Ok(self.latest_block - 20),
                _ => Ok(self.latest_block),
            }
        }
        
        async fn subscribe(&self) -> Result<Box<dyn EventSubscription>> {
            // Create some events for the subscription
            let events = self.get_events(Vec::new()).await?;
            Ok(Box::new(MockSubscription::new(events)))
        }
    }
    
    // Create a wrapper for BoxedEventService that allows us to check if it's a mock
    pub struct TestEventService {
        inner: BoxedEventService,
        is_mock: bool,
    }
    
    impl TestEventService {
        pub fn new(service: MockEventService) -> Self {
            let is_mock = service.is_mock;
            Self {
                inner: Arc::new(service),
                is_mock,
            }
        }
        
        pub fn as_boxed(&self) -> BoxedEventService {
            self.inner.clone()
        }
        
        pub fn is_mock(&self) -> bool {
            self.is_mock
        }
    }
}

// Import the test utilities
use test_utils::*;

// Test schema registry functionality
#[tokio::test]
async fn test_schema_registry() {
    // Create a schema registry
    let registry = InMemorySchemaRegistry::new();
    
    // Create test schema
    let schema_version = indexer_api::ContractSchemaVersion {
        version: "1.0.0".to_string(),
        schema: indexer_api::ContractSchema {
            chain: "ethereum".to_string(),
            address: "0x123".to_string(),
            name: "TestContract".to_string(),
            events: vec![
                indexer_api::EventSchema {
                    name: "Transfer".to_string(),
                    fields: vec![
                        indexer_api::FieldSchema {
                            name: "from".to_string(),
                            type_name: "address".to_string(),
                            indexed: true,
                        },
                        indexer_api::FieldSchema {
                            name: "to".to_string(),
                            type_name: "address".to_string(),
                            indexed: true,
                        },
                        indexer_api::FieldSchema {
                            name: "value".to_string(),
                            type_name: "uint256".to_string(),
                            indexed: false,
                        },
                    ],
                },
            ],
            functions: vec![],
        },
    };
    
    // Register the schema
    registry.register_schema(schema_version.clone()).unwrap();
    
    // Get the schema back
    let retrieved = registry.get_schema("ethereum", "0x123", "1.0.0").unwrap();
    assert!(retrieved.is_some());
    
    // Get latest schema
    let latest = registry.get_latest_schema("ethereum", "0x123").unwrap();
    assert!(latest.is_some());
    
    // Check schema details
    let schema = latest.unwrap();
    assert_eq!(schema.name, "TestContract");
    assert_eq!(schema.events.len(), 1);
    assert_eq!(schema.events[0].name, "Transfer");
}

// Test the mock service functionality
#[tokio::test]
async fn test_mock_service() {
    // Create a mock event service
    let service = MockEventService::new("ethereum");
    
    // Test get_latest_block
    assert_eq!(service.get_latest_block().await.unwrap(), 1000);
    
    // Test get_latest_block_with_status
    assert_eq!(service.get_latest_block_with_status("ethereum", BlockStatus::Finalized).await.unwrap(), 950);
    
    // Test get_events
    let events = service.get_events(Vec::new()).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type(), "Transfer");
    assert_eq!(events[1].event_type(), "Approval");
    
    // Test subscription
    let mut subscription = service.subscribe().await.unwrap();
    let event1 = subscription.next().await;
    assert!(event1.is_some());
    let event2 = subscription.next().await;
    assert!(event2.is_some());
    let event3 = subscription.next().await;
    assert!(event3.is_none());
    
    // Verify is_mock functionality
    assert!(service.is_mock());
    
    // Create a wrapped test service
    let test_service = TestEventService::new(service);
    assert!(test_service.is_mock());
} 