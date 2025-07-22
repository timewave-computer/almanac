use almanac_core::{BlockStatus, Event, EventFilter, StorageReader, StorageWriter};
use almanac_storage::MemoryStorage;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestEvent {
    id: String,
    chain_id: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    event_type: String,
    data: String,
}

impl Event for TestEvent {
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
        &self.tx_hash
    }

    fn timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    fn event_type(&self) -> &str {
        &self.event_type
    }

    fn raw_data(&self) -> &[u8] {
        self.data.as_bytes()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn create_test_event(
    id: &str,
    chain: &str,
    block_number: u64,
    event_type: &str,
    data: &str,
) -> TestEvent {
    TestEvent {
        id: id.to_string(),
        chain_id: chain.to_string(),
        block_number,
        block_hash: format!("0x{:x}", block_number),
        tx_hash: format!("0x{:x}", block_number + 1000),
        event_type: event_type.to_string(),
        data: data.to_string(),
    }
}

#[tokio::test]
async fn test_memory_storage_creation() {
    let _storage = MemoryStorage::new();

    // Test that we can create a memory storage instance
    // Basic creation test - storage should be created successfully
}

#[tokio::test]
async fn test_store_event() {
    let storage = MemoryStorage::new();

    let event = create_test_event("test_1", "ethereum", 1000, "Transfer", "test data");

    let result = storage.store_event(Box::new(event)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_store_multiple_events() {
    let storage = MemoryStorage::new();

    let event1 = create_test_event("test_1", "ethereum", 1000, "Transfer", "transfer data");
    let event2 = create_test_event("test_2", "ethereum", 1001, "Approval", "approval data");
    let event3 = create_test_event("test_3", "cosmos", 500, "Send", "send data");

    let result1 = storage.store_event(Box::new(event1)).await;
    let result2 = storage.store_event(Box::new(event2)).await;
    let result3 = storage.store_event(Box::new(event3)).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result3.is_ok());
}

#[tokio::test]
async fn test_get_events_no_filter() {
    let storage = MemoryStorage::new();

    // Store some events
    let event1 = create_test_event("test_1", "ethereum", 1000, "Transfer", "data1");
    let event2 = create_test_event("test_2", "ethereum", 1001, "Approval", "data2");

    let _ = storage.store_event(Box::new(event1)).await;
    let _ = storage.store_event(Box::new(event2)).await;

    // Get all events
    let filter = EventFilter::new();
    let events = storage.get_events(vec![filter]).await.unwrap();

    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn test_get_events_with_chain_filter() {
    let storage = MemoryStorage::new();

    // Store events from different chains
    let eth_event = create_test_event("eth_1", "ethereum", 1000, "Transfer", "eth data");
    let cosmos_event = create_test_event("cosmos_1", "cosmos", 500, "Send", "cosmos data");

    let _ = storage.store_event(Box::new(eth_event)).await;
    let _ = storage.store_event(Box::new(cosmos_event)).await;

    // Filter by chain
    let filter = EventFilter::new().with_chain("ethereum");
    let events = storage.get_events(vec![filter]).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].chain(), "ethereum");
}

#[tokio::test]
async fn test_get_events_with_block_range() {
    let storage = MemoryStorage::new();

    // Store events at different block numbers
    let event1 = create_test_event("test_1", "ethereum", 1000, "Transfer", "data1");
    let event2 = create_test_event("test_2", "ethereum", 1500, "Transfer", "data2");
    let event3 = create_test_event("test_3", "ethereum", 2000, "Transfer", "data3");

    let _ = storage.store_event(Box::new(event1)).await;
    let _ = storage.store_event(Box::new(event2)).await;
    let _ = storage.store_event(Box::new(event3)).await;

    // Filter by block range
    let filter = EventFilter::new().with_block_range(1200, 1800);
    let filtered_events = storage.get_events(vec![filter]).await.unwrap();

    assert_eq!(filtered_events.len(), 1);
    assert_eq!(filtered_events[0].block_number(), 1500);
}

#[tokio::test]
async fn test_get_events_with_event_type_filter() {
    let storage = MemoryStorage::new();

    // Store events of different types
    let event1 = create_test_event("test_1", "ethereum", 1000, "Transfer", "transfer data");
    let event2 = create_test_event("test_2", "ethereum", 1001, "Approval", "approval data");
    let event3 = create_test_event("test_3", "ethereum", 1002, "Transfer", "another transfer");

    let _ = storage.store_event(Box::new(event1)).await;
    let _ = storage.store_event(Box::new(event2)).await;
    let _ = storage.store_event(Box::new(event3)).await;

    // Filter by event type
    let filter = EventFilter::new().with_event_types(vec!["Transfer".to_string()]);
    let filtered_events = storage.get_events(vec![filter]).await.unwrap();

    assert_eq!(filtered_events.len(), 2);
    for event in filtered_events {
        assert_eq!(event.event_type(), "Transfer");
    }
}

#[tokio::test]
async fn test_get_events_with_limit() {
    let storage = MemoryStorage::new();

    // Store multiple events
    for i in 0..10 {
        let event = create_test_event(
            &format!("test_{}", i),
            "ethereum",
            1000 + i,
            "Transfer",
            &format!("data_{}", i),
        );
        let _ = storage.store_event(Box::new(event)).await;
    }

    // Filter with limit
    let filter = EventFilter::new().with_limit(5);
    let limited_events = storage.get_events(vec![filter]).await.unwrap();

    assert_eq!(limited_events.len(), 5);
}

#[tokio::test]
async fn test_get_latest_block() {
    let storage = MemoryStorage::new();

    // Initially no blocks - this should return an error or 0
    let latest_result = storage.get_latest_block("ethereum").await;
    // Handle both cases: error or 0
    match latest_result {
        Ok(latest) => assert_eq!(latest, 0),
        Err(_) => {
            // This is also acceptable - no blocks found
        }
    }

    // Store some events
    let event1 = create_test_event("test_1", "ethereum", 1000, "Transfer", "data1");
    let event2 = create_test_event("test_2", "ethereum", 1500, "Transfer", "data2");

    let _ = storage.store_event(Box::new(event1)).await;
    let _ = storage.store_event(Box::new(event2)).await;

    // Should return the highest block number
    let latest = storage.get_latest_block("ethereum").await.unwrap();
    assert_eq!(latest, 1500);
}

#[tokio::test]
async fn test_update_block_status() {
    let storage = MemoryStorage::new();

    // Update block status
    let result = storage
        .update_block_status("ethereum", 1000, BlockStatus::Finalized)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_latest_block_with_status() {
    let storage = MemoryStorage::new();

    // Update block status
    let _ = storage
        .update_block_status("ethereum", 1000, BlockStatus::Finalized)
        .await;

    // Get latest block with status
    let latest = storage
        .get_latest_block_with_status("ethereum", BlockStatus::Finalized)
        .await
        .unwrap();
    assert_eq!(latest, 1000);
}

#[tokio::test]
async fn test_complex_filter_combination() {
    let storage = MemoryStorage::new();

    // Store various events
    let event1 = create_test_event("test_1", "ethereum", 1000, "Transfer", "data1");
    let event2 = create_test_event("test_2", "ethereum", 1500, "Transfer", "data2");
    let event3 = create_test_event("test_3", "cosmos", 1200, "Transfer", "data3");
    let event4 = create_test_event("test_4", "ethereum", 1200, "Approval", "data4");

    let _ = storage.store_event(Box::new(event1)).await;
    let _ = storage.store_event(Box::new(event2)).await;
    let _ = storage.store_event(Box::new(event3)).await;
    let _ = storage.store_event(Box::new(event4)).await;

    // Complex filter: Ethereum chain, Transfer events, in block range
    let filter = EventFilter::new()
        .with_chain("ethereum")
        .with_event_types(vec!["Transfer".to_string()])
        .with_block_range(1200, 2000);

    let filtered_events = storage.get_events(vec![filter]).await.unwrap();

    assert_eq!(filtered_events.len(), 1);
    assert_eq!(filtered_events[0].chain(), "ethereum");
    assert_eq!(filtered_events[0].event_type(), "Transfer");
    assert_eq!(filtered_events[0].block_number(), 1500);
}

#[tokio::test]
async fn test_reorg_chain() {
    let storage = MemoryStorage::new();

    // Store some events
    let event1 = create_test_event("test_1", "ethereum", 1000, "Transfer", "data1");
    let event2 = create_test_event("test_2", "ethereum", 1001, "Transfer", "data2");

    let _ = storage.store_event(Box::new(event1)).await;
    let _ = storage.store_event(Box::new(event2)).await;

    // Trigger reorg
    let result = storage.reorg_chain("ethereum", 1000).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_events_with_status() {
    let storage = MemoryStorage::new();

    // Store an event
    let event = create_test_event("test_1", "ethereum", 1000, "Transfer", "data");
    let _ = storage.store_event(Box::new(event)).await;

    // Update block status
    let _ = storage
        .update_block_status("ethereum", 1000, BlockStatus::Finalized)
        .await;

    // Get events with status
    let filter = EventFilter::new().with_chain("ethereum");
    let events = storage
        .get_events_with_status(vec![filter], BlockStatus::Finalized)
        .await
        .unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].block_number(), 1000);
}
