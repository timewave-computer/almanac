//! Test for database migrations
use std::time::{SystemTime, UNIX_EPOCH};

use indexer_core::event::{Event, EventContainer, EventMetadata};
use indexer_core::Result;
use indexer_storage::postgres::{PostgresConfig, PostgresStorage};
use indexer_storage::Storage;

struct TestEvent {
    metadata: EventMetadata,
    raw_data: Vec<u8>,
}

impl Event for TestEvent {
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
        UNIX_EPOCH + std::time::Duration::from_secs(self.metadata.timestamp)
    }
    
    fn event_type(&self) -> &str {
        &self.metadata.event_type
    }
    
    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }
}

#[tokio::test]
async fn test_postgres_migrations() -> Result<()> {
    // Skip the test if DATABASE_URL is not set
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/indexer_test".to_string());
    
    // Create a config that points to a test database
    let config = PostgresConfig {
        connection_string: database_url,
        max_connections: 5,
        migrate: true,
        migrations_path: "./migrations".to_string(),
    };
    
    // Create storage instance - this will run migrations
    let storage = PostgresStorage::new(config).await?;
    
    // Now test that we can store and retrieve events
    let event = TestEvent {
        metadata: EventMetadata {
            id: "test-event-1".to_string(),
            chain: "ethereum".to_string(),
            block_number: 12345,
            block_hash: "0xabcdef".to_string(),
            tx_hash: "0x123456".to_string(),
            timestamp: 1617235200, // April 1, 2021
            event_type: "test-event".to_string(),
        },
        raw_data: b"test data".to_vec(),
    };
    
    // Store the event
    storage.store_event(Box::new(event)).await?;
    
    // Retrieve the latest block
    let latest_block = storage.get_latest_block("ethereum").await?;
    assert_eq!(latest_block, 12345);
    
    // Retrieve events
    let events = storage.get_events(vec![]).await?;
    assert!(!events.is_empty());
    
    Ok(())
}

// We don't run this test by default because it requires a database
// To run it, use: cargo test -- --ignored
#[tokio::test]
#[ignore]
async fn test_postgres_storage_full() -> Result<()> {
    // Create a random test database name
    let random_suffix = rand::random::<u32>();
    let database_url = format!("postgres://postgres:postgres@localhost/indexer_test_{}", random_suffix);
    
    // Create a config
    let config = PostgresConfig {
        connection_string: database_url,
        max_connections: 5,
        migrate: true,
        migrations_path: "./migrations".to_string(),
    };
    
    // Create storage instance
    let storage = PostgresStorage::new(config).await?;
    
    // Create 100 test events
    for i in 0..100 {
        let block_number = 1000 + i;
        let event = TestEvent {
            metadata: EventMetadata {
                id: format!("test-event-{}", i),
                chain: "ethereum".to_string(),
                block_number,
                block_hash: format!("0xblock{}", block_number),
                tx_hash: format!("0xtx{}", i),
                timestamp: 1617235200 + i as u64 * 12, // 12 seconds per block
                event_type: "test-event".to_string(),
            },
            raw_data: format!("test data for event {}", i).into_bytes(),
        };
        
        storage.store_event(Box::new(event)).await?;
    }
    
    // Check the latest block
    let latest_block = storage.get_latest_block("ethereum").await?;
    assert_eq!(latest_block, 1099); // 1000 + 99
    
    // Retrieve all events
    let events = storage.get_events(vec![]).await?;
    assert_eq!(events.len(), 100);
    
    Ok(())
} 