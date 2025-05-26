use std::{env, fs};
use anyhow::Result;
use indexer_storage::{rocks::{RocksStorage, RocksConfig}, Storage};
use indexer_core::{BlockStatus, event::Event};
use std::time::SystemTime;
use std::any::Any;

// Simple test event implementation
#[derive(Debug)]
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
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn create_test_event(chain: &str, block_number: u64) -> Box<dyn Event> {
    Box::new(TestEvent {
        id: format!("{}:{}", chain, block_number),
        chain: chain.to_string(),
        block_number,
        block_hash: format!("0x{:064x}", block_number),
        tx_hash: format!("0x{:064x}", block_number + 1000),
        timestamp: SystemTime::now(),
        event_type: "TestEvent".to_string(),
        raw_data: vec![1, 2, 3, 4],
    })
}

#[tokio::test]
async fn test_rocks_basic_operations() -> Result<()> {
    // Create a temporary directory for RocksDB
    let temp_dir = env::temp_dir().join("rocks_basic_test");
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
    let chain = "testchain";
    
    // Store a test event
    let event = create_test_event(chain, 100);
    rocks.store_event(chain, event).await?;
    
    // Get the latest block
    let latest_block = rocks.get_latest_block(chain).await?;
    assert_eq!(latest_block, 100, "Latest block should be 100");
    
    // Get events
    let events = rocks.get_events(chain, 100, 100).await?;
    assert_eq!(events.len(), 1, "Should have retrieved 1 event");
    
    // Test block status operations
    rocks.update_block_status(chain, 100, BlockStatus::Finalized).await?;
    
    let latest_finalized = rocks.get_latest_block_with_status(chain, BlockStatus::Finalized).await?;
    assert_eq!(latest_finalized, 100, "Latest finalized block should be 100");
    
    // Clean up
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rocks_multiple_events() -> Result<()> {
    // Create a temporary directory for RocksDB
    let temp_dir = env::temp_dir().join("rocks_multiple_test");
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
    let chain = "testchain";
    
    // Store multiple test events
    for i in 100..105 {
        let event = create_test_event(chain, i);
        rocks.store_event(chain, event).await?;
    }
    
    // Get the latest block
    let latest_block = rocks.get_latest_block(chain).await?;
    assert_eq!(latest_block, 104, "Latest block should be 104");
    
    // Get events in range
    let events = rocks.get_events(chain, 100, 104).await?;
    assert_eq!(events.len(), 5, "Should have retrieved 5 events");
    
    // Clean up
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    
    Ok(())
} 