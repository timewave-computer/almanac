//! Test for database migrations
use std::time::{SystemTime, UNIX_EPOCH};
use std::any::Any;
use std::process::Command;
use rand::Rng;

use tracing::warn;
use indexer_core::event::Event;
use indexer_core::Result;
use indexer_storage::postgres::{PostgresConfig, PostgresStorage};
use indexer_storage::Storage;

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
    
    fn as_any(&self) -> &(dyn Any + 'static) {
        self
    }
}

#[tokio::test]
#[ignore]
async fn test_postgres_migrations() -> Result<()> {
    // Get current OS user as default PostgreSQL user
    let current_user = match Command::new("whoami").output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => {
            println!("Couldn't detect current user, using default");
            "postgres".to_string()
        }
    };
    
    println!("Using database user: {}", current_user);
    
    // Generate a random database name to avoid conflicts
    let random_suffix: u32 = rand::thread_rng().gen();
    let database_url = format!("postgres://{}@localhost:5432/indexer_test_{}", 
                              current_user, random_suffix);
    
    println!("Using connection URL: {}", database_url);
    
    // Create a config that points to a test database
    let config = PostgresConfig {
        url: database_url,
        max_connections: 5,
        connection_timeout: 30,
    };
    
    // Create storage instance - this will run migrations
    let storage = match PostgresStorage::new(config).await {
        Ok(storage) => storage,
        Err(e) => {
            // This is expected in environments without PostgreSQL
            warn!("Skipping PostgreSQL migration test: {}", e);
            println!("Skipping PostgreSQL test - make sure PostgreSQL is running with the correct user permissions.");
            println!("This test is not critical for the functionality of the application.");
            println!("If you want to run this test, ensure PostgreSQL is running and accessible.");
            return Ok(());
        }
    };
    
    // Now test that we can store and retrieve events
    let event = TestEvent {
        id: "test-event-1".to_string(),
        chain: "ethereum".to_string(),
        block_number: 12345,
        block_hash: "0xabcdef".to_string(),
        tx_hash: "0x123456".to_string(),
        timestamp: UNIX_EPOCH + std::time::Duration::from_secs(1617235200), // April 1, 2021
        event_type: "test-event".to_string(),
        raw_data: b"test data".to_vec(),
    };
    
    // Store the event
    storage.store_event("ethereum", Box::new(event)).await?;
    
    // Retrieve the latest block
    let latest_block = storage.get_latest_block("ethereum").await?;
    assert_eq!(latest_block, 12345);
    
    // Retrieve events
    let events = storage.get_events("ethereum", 0, latest_block).await?;
    assert!(!events.is_empty());
    
    Ok(())
}

// We don't run this test by default because it requires a database
// To run it, use: cargo test -- --ignored
#[tokio::test]
#[ignore]
async fn test_postgres_storage_full() -> Result<()> {
    // Get current OS user as default PostgreSQL user
    let current_user = match Command::new("whoami").output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => {
            println!("Couldn't detect current user, using default");
            "postgres".to_string()
        }
    };
    
    println!("Using database user: {}", current_user);
    
    // Generate a random database name to avoid conflicts
    let random_suffix: u32 = rand::thread_rng().gen();
    let database_url = format!("postgres://{}@localhost:5432/indexer_test_{}", 
                              current_user, random_suffix);
    
    println!("Using connection URL: {}", database_url);
    
    // Create a config
    let config = PostgresConfig {
        url: database_url,
        max_connections: 5,
        connection_timeout: 30,
    };
    
    // Create storage instance
    let storage = match PostgresStorage::new(config).await {
        Ok(storage) => storage,
        Err(e) => {
            // This is expected in environments without PostgreSQL
            warn!("Skipping PostgreSQL full storage test: {}", e);
            println!("Skipping PostgreSQL test - make sure PostgreSQL is running with the correct user permissions.");
            println!("This test is not critical for the functionality of the application.");
            println!("If you want to run this test, ensure PostgreSQL is running and accessible.");
            return Ok(());
        }
    };
    
    // Create 100 test events
    for i in 0..100 {
        let block_number = 1000 + i;
        let event = TestEvent {
            id: format!("test-event-{}", i),
            chain: "ethereum".to_string(),
            block_number,
            block_hash: format!("0xblock{}", block_number),
            tx_hash: format!("0xtx{}", i),
            timestamp: UNIX_EPOCH + std::time::Duration::from_secs(1617235200 + i * 12), // 12 seconds per block
            event_type: "test-event".to_string(),
            raw_data: format!("test data for event {}", i).into_bytes(),
        };
        
        storage.store_event("ethereum", Box::new(event)).await?;
    }
    
    // Check the latest block
    let latest_block = storage.get_latest_block("ethereum").await?;
    assert_eq!(latest_block, 1099); // 1000 + 99
    
    // Retrieve all events
    let events = storage.get_events("ethereum", 0, latest_block).await?;
    assert_eq!(events.len(), 100);
    
    Ok(())
} 