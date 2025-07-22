//! Storage usage example
//! 
//! This example demonstrates how to use the storage layer
//! for both in-memory and FoundationDB backends.

use almanac_core::{Event, EventFilter, StorageReader, StorageWriter, UnifiedEvent, EventSource};
use almanac_storage::{MemoryStorage, StorageBackend, create_storage};
#[cfg(feature = "fdb")]
use almanac_storage::FdbConfig;
use std::collections::HashMap;
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Almanac Storage Example");
    println!("======================");
    
    // Example 1: Using in-memory storage
    println!("\n1. In-Memory Storage");
    println!("-------------------");
    
    let memory_storage = MemoryStorage::new();
    println!("Created in-memory storage");
    
    // Create and store an event
    let event = UnifiedEvent {
        id: "eth_event_1".to_string(),
        chain: "ethereum".to_string(),
        block_number: 18000000,
        block_hash: "0x1234567890abcdef".to_string(),
        tx_hash: "0xabcdef1234567890".to_string(),
        timestamp: SystemTime::now(),
        event_type: "Transfer".to_string(),
        contract_address: Some("0x1111111111111111111111111111111111111111".to_string()),
        raw_data: vec![1, 2, 3, 4],
        attributes: HashMap::from([
            ("from".to_string(), serde_json::json!("0xAAA")),
            ("to".to_string(), serde_json::json!("0xBBB")),
            ("amount".to_string(), serde_json::json!("1000000")),
        ]),
        source: EventSource::Chain,
    };
    
    memory_storage.store_event(Box::new(event.clone())).await?;
    println!("Saved event {} for chain {}", event.id, event.chain);
    
    // Store another event
    let event2 = UnifiedEvent {
        id: "eth_event_2".to_string(),
        chain: "ethereum".to_string(),
        block_number: 18000001,
        block_hash: "0xfedcba0987654321".to_string(),
        tx_hash: "0x1234567890fedcba".to_string(),
        timestamp: SystemTime::now(),
        event_type: "Approval".to_string(),
        contract_address: Some("0x2222222222222222222222222222222222222222".to_string()),
        raw_data: vec![5, 6, 7, 8],
        attributes: HashMap::from([
            ("owner".to_string(), serde_json::json!("0xCCC")),
            ("spender".to_string(), serde_json::json!("0xDDD")),
            ("amount".to_string(), serde_json::json!("999999999")),
        ]),
        source: EventSource::Chain,
    };
    
    memory_storage.store_event(Box::new(event2)).await?;
    println!("Saved second event");
    
    // Query events
    let filter = EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((18000000, 18000001)),
        ..Default::default()
    };
    
    let events = memory_storage.get_events(vec![filter]).await?;
    println!("Retrieved {} events", events.len());
    
    for event in &events {
        println!("  Event: {} at block {}", event.id(), event.block_number());
    }
    
    // Get latest block
    let latest = memory_storage.get_latest_block("ethereum").await?;
    println!("Latest block: {}", latest);
    
    // Example 2: Using the storage factory
    println!("\n2. Storage Factory");
    println!("-----------------");
    
    let storage = create_storage(StorageBackend::Memory).await?;
    println!("Created storage using factory (Memory backend)");
    
    // Store events in batch
    let batch_events: Vec<Box<dyn Event>> = vec![
        Box::new(UnifiedEvent {
            id: "cosmos_event_1".to_string(),
            chain: "cosmos".to_string(),
            block_number: 15000000,
            block_hash: "cosmos_hash_15000000".to_string(),
            tx_hash: "cosmos_tx_1".to_string(),
            timestamp: SystemTime::now(),
            event_type: "MsgSend".to_string(),
            contract_address: None,
            raw_data: vec![10, 11, 12],
            attributes: HashMap::from([
                ("sender".to_string(), serde_json::json!("cosmos1abc...")),
                ("recipient".to_string(), serde_json::json!("cosmos1def...")),
                ("amount".to_string(), serde_json::json!("1000uatom")),
            ]),
            source: EventSource::Chain,
        }),
        Box::new(UnifiedEvent {
            id: "cosmos_event_2".to_string(),
            chain: "cosmos".to_string(),
            block_number: 15000001,
            block_hash: "cosmos_hash_15000001".to_string(),
            tx_hash: "cosmos_tx_2".to_string(),
            timestamp: SystemTime::now(),
            event_type: "MsgDelegate".to_string(),
            contract_address: None,
            raw_data: vec![13, 14, 15],
            attributes: HashMap::from([
                ("delegator".to_string(), serde_json::json!("cosmos1ghi...")),
                ("validator".to_string(), serde_json::json!("cosmosvaloper1...")),
                ("amount".to_string(), serde_json::json!("5000uatom")),
            ]),
            source: EventSource::Chain,
        }),
    ];
    
    storage.store_events_batch(batch_events).await?;
    println!("Stored batch of events using factory storage");
    
    // Query cosmos events
    let cosmos_filter = EventFilter {
        chain: Some("cosmos".to_string()),
        ..Default::default()
    };
    
    let cosmos_events = storage.get_events(vec![cosmos_filter]).await?;
    println!("Found {} Cosmos events", cosmos_events.len());
    
    // Example 3: FoundationDB storage (if feature enabled)
    #[cfg(feature = "fdb")]
    {
        println!("\n3. FoundationDB Storage");
        println!("----------------------");
        
        let fdb_config = FdbConfig {
            cluster_file: None, // Use default cluster file
            namespace: "almanac_example".to_string(),
            connection_timeout: 30,
            max_retries: 3,
        };
        
        match create_storage(StorageBackend::Fdb(fdb_config)).await {
            Ok(fdb_storage) => {
                println!("Created FoundationDB storage successfully");
                
                // Store an event in FDB
                let fdb_event = UnifiedEvent {
                    id: "solana_event_1".to_string(),
                    chain: "solana".to_string(),
                    block_number: 250000000,
                    block_hash: "solana_hash_250000000".to_string(),
                    tx_hash: "solana_sig_1".to_string(),
                    timestamp: SystemTime::now(),
                    event_type: "TokenTransfer".to_string(),
                    contract_address: Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()),
                    raw_data: vec![20, 21, 22, 23],
                    attributes: HashMap::from([
                        ("source".to_string(), serde_json::json!("9WzDXwBbmkg...")),
                        ("destination".to_string(), serde_json::json!("8Xjq7pDc3mZ...")),
                        ("amount".to_string(), serde_json::json!("1000000000")),
                        ("mint".to_string(), serde_json::json!("So11111111111111111111111111111111111111112")),
                    ]),
                    source: EventSource::Chain,
                };
                
                fdb_storage.store_event(Box::new(fdb_event)).await?;
                println!("Saved event to FoundationDB");
                
                // Query from FDB
                let fdb_filter = EventFilter {
                    chain: Some("solana".to_string()),
                    ..Default::default()
                };
                
                let fdb_events = fdb_storage.get_events(vec![fdb_filter]).await?;
                println!("Retrieved {} events from FDB", fdb_events.len());
                
                // Health check
                match almanac_core::StorageReader::health_check(fdb_storage.as_ref()).await {
                    Ok(()) => println!("FDB storage health: OK"),
                    Err(e) => println!("FDB storage health: ERROR - {}", e),
                }
            }
            Err(e) => {
                println!("Could not connect to FoundationDB: {}", e);
                println!("Make sure FoundationDB is running locally");
            }
        }
    }
    
    #[cfg(not(feature = "fdb"))]
    {
        println!("\n3. FoundationDB Storage");
        println!("----------------------");
        println!("FoundationDB feature not enabled. Compile with --features fdb to enable.");
    }
    
    // Example 4: Extended storage functionality
    println!("\n4. Extended Storage");
    println!("------------------");
    
    // The memory storage implements StorageReader
    match almanac_core::StorageReader::health_check(&memory_storage).await {
        Ok(()) => println!("Storage read health: OK"),
        Err(e) => println!("Storage read health: ERROR - {}", e),
    }
    
    // Test block status updates
    use almanac_core::types::BlockStatus;
    memory_storage.update_block_status("ethereum", 18000000, BlockStatus::Finalized).await?;
    println!("Updated block status to Finalized");
    
    let finalized_block = memory_storage.get_latest_block_with_status("ethereum", BlockStatus::Finalized).await?;
    println!("Latest finalized block: {}", finalized_block);
    
    println!("\n✓ Storage example completed successfully!");
    
    Ok(())
}

/// Example of using storage in tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryStorage::new();
        
        let event = UnifiedEvent {
            id: "test_event".to_string(),
            chain: "test".to_string(),
            block_number: 1,
            block_hash: "test_hash".to_string(),
            tx_hash: "test_tx".to_string(),
            timestamp: SystemTime::now(),
            event_type: "TestEvent".to_string(),
            contract_address: None,
            raw_data: vec![1, 2, 3],
            attributes: HashMap::new(),
            source: EventSource::Internal,
        };
        
        // Save and retrieve
        storage.store_event(Box::new(event.clone())).await.unwrap();
        
        let filter = EventFilter {
            chain: Some("test".to_string()),
            block_range: Some((1, 1)),
            ..Default::default()
        };
        
        let retrieved = storage.get_events(vec![filter]).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].id(), "test_event");
    }
}