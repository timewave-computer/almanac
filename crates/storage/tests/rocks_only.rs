use indexer_core::event::{Event, EventMetadata};
use indexer_pipeline::BlockStatus;
use indexer_storage::rocks::{RocksStorage, RocksConfig};
use indexer_storage::{Storage, Result, EventFilter};
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;
use std::sync::Arc;
use std::{fs, env};
use std::time::Duration;
use std::any::Any;

// MockEvent implementation for testing
#[derive(Debug)]
struct MockEvent {
    metadata: EventMetadata,
    raw_data: Vec<u8>,
}

impl Event for MockEvent {
    fn id(&self) -> &str {
        &self.metadata.id
    }
    
    fn chain(&self) -> &str {
        &self.metadata.chain
    }
    
    fn block_number(&self) -> u64 {
        self.metadata.block_number
    }
    
    fn block_hash(&self) -> &str {
        &self.metadata.block_hash
    }
    
    fn tx_hash(&self) -> &str {
        &self.metadata.tx_hash
    }
    
    fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.metadata.timestamp)
    }
    
    fn event_type(&self) -> &str {
        &self.metadata.event_type
    }
    
    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Helper function to create a test event
fn create_test_event(chain: &str, block_number: u64) -> Box<dyn Event> {
    Box::new(MockEvent {
        metadata: EventMetadata {
            id: format!("test_event_{}", block_number),
            chain: chain.to_string(),
            block_number,
            block_hash: format!("block_hash_{}", block_number),
            tx_hash: format!("tx_hash_{}", block_number),
            timestamp: 1000 + block_number,
            event_type: "TestEvent".to_string(),
        },
        raw_data: format!("test_event_{}", block_number).as_bytes().to_vec(),
    })
}

#[tokio::test]
async fn test_rocks_store_and_retrieve() -> Result<()> {
    // Create a temporary directory for RocksDB
    let temp_dir = env::temp_dir().join("rocks_test");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;
    
    println!("Using temp directory: {:?}", temp_dir);

    // Initialize RocksDB with proper config
    let config = RocksConfig {
        path: temp_dir.to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
    };
    let rocks = RocksStorage::new(config)?;
    let rocks = Arc::new(rocks);
    
    // Store test events
    let chain = "testchain";
    let event1 = create_test_event(chain, 100);
    let event2 = create_test_event(chain, 101);
    let event3 = create_test_event(chain, 102);
    
    // Store the events
    rocks.store_event(chain, event1).await?;
    rocks.store_event(chain, event2).await?;
    rocks.store_event(chain, event3).await?;
    
    // Get the latest block
    let latest_block = rocks.get_latest_block(chain).await?;
    assert_eq!(latest_block, 102, "Latest block should be 102");
    
    // Filter events
    let mut filter = EventFilter::new();
    filter.chain = Some(chain.to_string());
    filter.block_range = Some((100, 102));
    filter.event_types = Some(vec!["TestEvent".to_string()]);
    
    let events = rocks.get_events(chain, 100, 102).await?;
    assert_eq!(events.len(), 3, "Should have retrieved 3 events");
    
    // Clean up
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rocks_block_status() -> Result<()> {
    // Create a temporary directory for RocksDB
    let temp_dir = env::temp_dir().join("rocks_status_test");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;
    
    // Initialize RocksDB
    let config = RocksConfig {
        path: temp_dir.to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
    };
    let rocks = RocksStorage::new(config)?;
    let rocks = Arc::new(rocks);
    
    // Store test events
    let chain = "testchain";
    let events = (100..=105).map(|block_num| create_test_event(chain, block_num)).collect::<Vec<_>>();
    
    // Store the events
    for event in events {
        rocks.store_event(chain, event).await?;
    }
    
    // Update block status to finalized
    rocks.update_block_status(chain, 103, BlockStatus::Finalized).await?;
    
    // Check latest block with finalized status
    let final_block = rocks.get_latest_block_with_status(chain, BlockStatus::Finalized).await?;
    assert_eq!(final_block, 103, "Latest finalized block should be 103");
    
    // Check that we can still retrieve all events
    let mut filter = EventFilter::new();
    filter.chain = Some(chain.to_string());
    filter.block_range = Some((100, 105));
    filter.event_types = Some(vec!["TestEvent".to_string()]);
    
    let retrieved_events = rocks.get_events(vec![filter]).await?;
    assert_eq!(retrieved_events.len(), 6, "Should have retrieved 6 events");
    
    // Clean up
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rocks_chain_reorg() -> Result<()> {
    // Create a temporary directory for RocksDB
    let temp_dir = env::temp_dir().join("rocks_reorg_test");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;
    
    // Initialize RocksDB
    let config = RocksConfig {
        path: temp_dir.to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
    };
    let rocks = RocksStorage::new(config)?;
    let rocks = Arc::new(rocks);
    
    // Store test events
    let chain = "testchain";
    let events = (100..=105).map(|block_num| create_test_event(chain, block_num)).collect::<Vec<_>>();
    
    // Store the events
    for event in events {
        rocks.store_event(chain, event).await?;
    }
    
    // Simulate chain reorg - remove blocks from 103 onwards
    rocks.reorg_chain(chain, 103).await?;
    
    // Check latest block after reorg
    let latest_block = rocks.get_latest_block(chain).await?;
    assert_eq!(latest_block, 102, "Latest block after reorg should be 102");
    
    // Check that events from reorged blocks are gone
    let mut filter = EventFilter::new();
    filter.chain = Some(chain.to_string());
    filter.block_range = Some((100, 105));
    filter.event_types = Some(vec!["TestEvent".to_string()]);
    
    let retrieved_events = rocks.get_events(vec![filter]).await?;
    assert_eq!(retrieved_events.len(), 3, "Should have retrieved 3 events after reorg");
    
    // Add new events after reorg
    let new_events = (103..=107).map(|block_num| create_test_event(chain, block_num)).collect::<Vec<_>>();
    for event in new_events {
        rocks.store_event(chain, event).await?;
    }
    
    // Check latest block after adding new events
    let latest_block = rocks.get_latest_block(chain).await?;
    assert_eq!(latest_block, 107, "Latest block after adding new events should be 107");
    
    // Clean up
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    
    Ok(())
}

// Test that re-orgs work correctly
#[tokio::test]
async fn test_rocks_reorg_update() -> Result<()> {
    // Create a temporary directory for RocksDB
    let temp_dir = env::temp_dir().join("rocks_reorg_update_test");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;
    
    // Initialize RocksDB
    let config = RocksConfig {
        path: temp_dir.to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
    };
    let rocks = RocksStorage::new(config)?;
    let rocks = Arc::new(rocks);
    
    // Store test events
    let chain = "testchain";
    let events = (100..=105).map(|block_num| create_test_event(chain, block_num)).collect::<Vec<_>>();
    
    // Store the events
    for event in events {
        rocks.store_event(chain, event).await?;
    }
    
    // Test that re-orgs work correctly
    assert!(
        rocks.update_block_status(chain, 101, BlockStatus::Pending, None)
            .await
            .is_ok(),
        "Should be able to update block status to pending"
    );

    // Get events again, should only get events from blocks 100 and 102
    let events = rocks.get_events(chain, 100, 102).await?;
    assert_eq!(events.len(), 2, "Should have retrieved 2 events");
    
    // Test another re-org case
    // ... existing code ...
    
    // Should be able to retrieve events again
    let events = rocks.get_events(chain, 100, 102).await?;
    assert_eq!(events.len(), 3, "Should have retrieved 3 events again");
    
    Ok(())
} 