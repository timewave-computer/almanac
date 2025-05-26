// Purpose: Basic example to test PostgreSQL storage functionality

use indexer_storage::postgres::{PostgresConfig, PostgresStorage};
use indexer_storage::Storage;
use indexer_core::{BlockStatus, Result};
use indexer_core::event::Event;
use std::any::Any;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct SimpleEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    timestamp: u64,
    event_type: String,
    data: Vec<u8>,
}

impl Event for SimpleEvent {
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
        UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp)
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

#[tokio::main]
async fn main() -> Result<()> {
    // Configure PostgreSQL
    let config = PostgresConfig {
        url: "postgres://postgres:postgres@localhost:5432/indexer_test".into(),
        max_connections: 5,
        connection_timeout: 30,
    };
    
    println!("Connecting to PostgreSQL...");
    let storage = Arc::new(PostgresStorage::new(config).await?);
    println!("✓ Connected to PostgreSQL");
    
    // Create a test event
    let event = SimpleEvent {
        id: "test-event-1".into(),
        chain: "ethereum".into(),
        block_number: 12345,
        block_hash: "0xabcdef123456".into(),
        tx_hash: "0x123456abcdef".into(),
        timestamp: 1650000000,
        event_type: "log".into(),
        data: b"sample data".to_vec(),
    };
    
    // Store the event
    println!("Storing test event...");
    let chain = event.chain.clone();
    storage.store_event(&chain, Box::new(event)).await?;
    println!("✓ Event stored successfully");
    
    // Update block status
    println!("Updating block status...");
    storage.mark_block_processed("ethereum", 12345, "0xabcdef123456", BlockStatus::Confirmed).await?;
    println!("✓ Block status updated successfully");
    
    // Get latest block
    println!("Getting latest block...");
    let latest_block = storage.get_latest_block("ethereum").await?;
    println!("✓ Latest block: {}", latest_block);
    
    // Get events
    println!("Getting events...");
    let events = storage.get_events("ethereum", 12300, 12400).await?;
    println!("✓ Retrieved {} events", events.len());
    
    println!("All tests passed! PostgreSQL storage is working correctly.");
    Ok(())
} 