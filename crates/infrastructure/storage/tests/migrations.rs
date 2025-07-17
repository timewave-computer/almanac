//! Test for database migrations
use almanac_infrastructure_storage::postgres::{PostgresConfig, PostgresStorage};
use almanac_infrastructure_storage::Storage;
use almanac_infrastructure_core::{Result, BlockStatus};
use almanac_infrastructure_core::types::{Event, EventFilter};
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

/// Test migrations for PostgreSQL storage
#[cfg(feature = "postgres")]
#[tokio::test]
async fn test_postgres_migrations() -> Result<()> {
    // Skip if no database URL provided
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("Skipping PostgreSQL migration test (no DATABASE_URL)");
            return Ok(());
        }
    };
    
    let config = PostgresConfig {
        url: database_url,
        max_connections: 5,
        connection_timeout: 30,
    };
    
    // Test that PostgresStorage can be created (which runs migrations)
    let storage = PostgresStorage::new(config).await?;
    
    // Test basic operations
    let event = TestEvent {
        id: "migration-test-1".to_string(),
        chain: "ethereum".to_string(),
        block_number: 100,
        block_hash: "0xabc123".to_string(),
        tx_hash: "0xdef456".to_string(),
        timestamp: UNIX_EPOCH + std::time::Duration::from_secs(1650000000),
        event_type: "test".to_string(),
        raw_data: b"test data".to_vec(),
    };
    
    // Store an event
    storage.store_event(Box::new(event)).await?;
    
    // Get latest block
    let latest_block = storage.get_latest_block("ethereum").await?;
    assert_eq!(latest_block, 100);
    
    // Get events using EventFilter
    let filter = EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((0, latest_block)),
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
        ..Default::default()
    };
    let events = storage.get_events(vec![filter]).await?;
    assert!(!events.is_empty());
    assert_eq!(events[0].id(), "migration-test-1");
    
    // Update block status  
    storage.update_block_status("ethereum", 100, BlockStatus::Confirmed).await?;
    
    println!("PostgreSQL migrations test passed successfully");
    Ok(())
}

/// Test storage with multiple chains and blocks
#[cfg(feature = "postgres")]
#[tokio::test]
async fn test_multi_chain_storage() -> Result<()> {
    // Skip if no database URL provided
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("Skipping multi-chain test (no DATABASE_URL)");
            return Ok(());
        }
    };
    
    let config = PostgresConfig {
        url: database_url,
        max_connections: 5,
        connection_timeout: 30,
    };
    
    let storage = PostgresStorage::new(config).await?;
    
    // Store events for multiple chains and blocks
    for chain in &["ethereum", "polygon", "base"] {
        for block_num in 100..105 {
            let event = TestEvent {
                id: format!("event-{}-{}", chain, block_num),
                chain: chain.to_string(),
                block_number: block_num,
                block_hash: format!("0x{:x}", block_num),
                tx_hash: format!("0x{:x}", block_num + 1000),
                timestamp: UNIX_EPOCH + std::time::Duration::from_secs(1650000000 + block_num),
                event_type: "multi-chain-test".to_string(),
                raw_data: format!("data-{}-{}", chain, block_num).into_bytes(),
            };
            
            storage.store_event(Box::new(event)).await?;
        }
    }
    
    // Test getting events per chain
    for chain in &["ethereum", "polygon", "base"] {
        let latest_block = storage.get_latest_block(chain).await?;
        assert_eq!(latest_block, 104);
        
        let filter = EventFilter {
            chain: Some(chain.to_string()),
            block_range: Some((100, latest_block)),
            time_range: None,
            event_types: None,
            limit: None,
            offset: None,
            ..Default::default()
        };
        let events = storage.get_events(vec![filter]).await?;
        assert_eq!(events.len(), 5); // blocks 100-104
        
        // Verify all events are for the correct chain
        for event in &events {
            assert_eq!(event.chain(), *chain);
        }
    }
    
    println!("Multi-chain storage test passed successfully");
    Ok(())
} 