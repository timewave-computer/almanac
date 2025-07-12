use indexer_storage::rocks::{RocksStorage, RocksConfig, CompressionType};
use indexer_storage::Storage;
use indexer_core::{Result, BlockStatus};
use indexer_core::types::{Event, EventFilter};
use std::any::Any;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[tokio::test]
async fn test_basic_rocks_storage() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    
    let config = RocksConfig {
        path: temp_dir.path().join("test_rocks").to_string_lossy().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
        write_buffer_size_mb: 32,
        max_write_buffer_number: 2,
        block_size_kb: 4,
        compression: CompressionType::Snappy,
        use_column_families: true,
    };
    
    let rocks = RocksStorage::new(config)?;
    
    let event = TestEvent {
        id: "test-event-1".to_string(),
        chain: "ethereum".to_string(),
        block_number: 100,
        block_hash: "0xabc123".to_string(),
        tx_hash: "0xdef456".to_string(),
        timestamp: UNIX_EPOCH + std::time::Duration::from_secs(1650000000),
        event_type: "test".to_string(),
        raw_data: b"test data".to_vec(),
    };
    
    // Store event using new API
    rocks.store_event(Box::new(event)).await?;
    
    // Get latest block
    let latest_block = rocks.get_latest_block("ethereum").await?;
    assert_eq!(latest_block, 100);
    
    // Get events using EventFilter
    let filter = EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((100, 100)),
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
        ..Default::default()
    };
    let events = rocks.get_events(vec![filter]).await?;
    
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id(), "test-event-1");
    assert_eq!(events[0].chain(), "ethereum");
    
    Ok(())
}

#[tokio::test]
async fn test_rocks_storage_multiple_events() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    
    let config = RocksConfig {
        path: temp_dir.path().join("test_rocks_multi").to_string_lossy().to_string(),
        create_if_missing: true,
        cache_size_mb: 64,
        write_buffer_size_mb: 32,
        max_write_buffer_number: 2,
        block_size_kb: 4,
        compression: CompressionType::Snappy,
        use_column_families: true,
    };
    
    let rocks = RocksStorage::new(config)?;
    
    // Store multiple events
    for i in 100..105 {
        let event = TestEvent {
            id: format!("test-event-{}", i),
            chain: "ethereum".to_string(),
            block_number: i,
            block_hash: format!("0x{:x}", i),
            tx_hash: format!("0x{:x}", i + 1000),
            timestamp: UNIX_EPOCH + std::time::Duration::from_secs(1650000000 + i),
            event_type: "test".to_string(),
            raw_data: format!("test data {}", i).into_bytes(),
        };
        
        rocks.store_event(Box::new(event)).await?;
    }
    
    // Get latest block
    let latest_block = rocks.get_latest_block("ethereum").await?;
    assert_eq!(latest_block, 104);
    
    // Get events using EventFilter
    let filter = EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((100, 104)),
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
        ..Default::default()
    };
    let events = rocks.get_events(vec![filter]).await?;
    
    assert_eq!(events.len(), 5);
    
    // Verify events are in correct order
    for (i, event) in events.iter().enumerate() {
        assert_eq!(event.id(), format!("test-event-{}", 100 + i as u64));
        assert_eq!(event.block_number(), 100 + i as u64);
    }
    
    Ok(())
} 