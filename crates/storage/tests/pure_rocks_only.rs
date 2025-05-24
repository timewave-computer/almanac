// Purpose: RocksDB-only tests that don't require PostgreSQL

use indexer_common::prelude::*;
use indexer_core::types::{BlockStatus, Event, EventType};

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::tempdir;
use tokio::test;

// Import only RocksDB related code
use indexer_storage::rocks::RocksDB;

// Helper function to create a test event
fn create_test_event(block: u64, tx_index: u64, log_index: u64, event_type: &str) -> Event {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    Event::new(
        format!("tx_hash_{}", tx_index),
        "test_chain",
        block,
        format!("block_hash_{}", block),
        tx_index,
        log_index,
        event_type.to_string(),
        vec![],
        timestamp,
    )
}

#[test]
async fn test_rocks_store_and_retrieve() {
    // Create a temporary directory for RocksDB
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Initialize RocksDB
    let rocks = RocksDB::new(db_path).expect("Failed to create RocksDB instance");
    
    // Store some test events
    let chain = "test_chain";
    let events = vec![
        create_test_event(1, 0, 0, "Transfer"),
        create_test_event(1, 0, 1, "Approval"),
        create_test_event(2, 0, 0, "Transfer"),
    ];
    
    // Store block 1 with its events
    rocks.store_events(chain, &events[0..2]).expect("Failed to store events for block 1");
    
    // Store block 2 with its event
    rocks.store_events(chain, &events[2..3]).expect("Failed to store events for block 2");
    
    // Mark blocks as final
    rocks.update_block_status(chain, 1, BlockStatus::Final).expect("Failed to update block 1 status");
    rocks.update_block_status(chain, 2, BlockStatus::Final).expect("Failed to update block 2 status");
    
    // Get latest block
    let latest_block = rocks.get_latest_block(chain).expect("Failed to get latest block");
    assert_eq!(latest_block, 2, "Latest block should be 2");
    
    // Filter events
    let filter = EventFilter {
        event_type: Some("Transfer".to_string()),
        from_block: Some(1),
        to_block: Some(2),
        ..Default::default()
    };
    
    let filtered_events = rocks.filter_events(chain, &filter).expect("Failed to filter events");
    assert_eq!(filtered_events.len(), 2, "Should find 2 Transfer events");
    
    // Clean up (this is done automatically when temp_dir goes out of scope)
}

#[test]
async fn test_rocks_block_finality() {
    // Create a temporary directory for RocksDB
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Initialize RocksDB
    let rocks = RocksDB::new(db_path).expect("Failed to create RocksDB instance");
    
    // Store some test events with different block statuses
    let chain = "test_chain";
    let events = vec![
        create_test_event(1, 0, 0, "Transfer"),
        create_test_event(2, 0, 0, "Transfer"),
        create_test_event(3, 0, 0, "Transfer"),
    ];
    
    // Store all events
    rocks.store_events(chain, &events).expect("Failed to store events");
    
    // Set different statuses
    rocks.update_block_status(chain, 1, BlockStatus::Final).expect("Failed to update block 1 status");
    rocks.update_block_status(chain, 2, BlockStatus::Pending).expect("Failed to update block 2 status");
    rocks.update_block_status(chain, 3, BlockStatus::Pending).expect("Failed to update block 3 status");
    
    // Get latest finalized block
    let latest_final = rocks.get_latest_block_with_status(chain, BlockStatus::Final)
        .expect("Failed to get latest finalized block");
    assert_eq!(latest_final, 1, "Latest finalized block should be 1");
    
    // Update block 2 to final
    rocks.update_block_status(chain, 2, BlockStatus::Final).expect("Failed to update block 2 status");
    
    // Get latest finalized block again
    let new_latest_final = rocks.get_latest_block_with_status(chain, BlockStatus::Final)
        .expect("Failed to get latest finalized block");
    assert_eq!(new_latest_final, 2, "Latest finalized block should now be 2");
    
    // Clean up (this is done automatically when temp_dir goes out of scope)
}

#[test]
async fn test_rocks_chain_reorg() {
    // Create a temporary directory for RocksDB
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Initialize RocksDB
    let rocks = RocksDB::new(db_path).expect("Failed to create RocksDB instance");
    
    // Store events for blocks 1-5
    let chain = "test_chain";
    for block in 1..=5 {
        let event = create_test_event(block, 0, 0, "Transfer");
        rocks.store_events(chain, &[event]).expect("Failed to store event");
        rocks.update_block_status(chain, block, BlockStatus::Pending).expect("Failed to update block status");
    }
    
    // Finalize blocks 1-3
    for block in 1..=3 {
        rocks.update_block_status(chain, block, BlockStatus::Final).expect("Failed to update block status");
    }
    
    // Simulate chain reorganization starting from block 4
    rocks.delete_blocks_from(chain, 4).expect("Failed to delete blocks");
    
    // Check that block 3 is the latest block
    let latest_block = rocks.get_latest_block(chain).expect("Failed to get latest block");
    assert_eq!(latest_block, 3, "After reorg, latest block should be 3");
    
    // Add new blocks 4-6 after reorg
    for block in 4..=6 {
        let event = create_test_event(block, 1, 0, "NewTransfer");  // different tx_index to simulate different chain
        rocks.store_events(chain, &[event]).expect("Failed to store event");
        rocks.update_block_status(chain, block, BlockStatus::Pending).expect("Failed to update block status");
    }
    
    // Verify total number of blocks
    let latest_block = rocks.get_latest_block(chain).expect("Failed to get latest block");
    assert_eq!(latest_block, 6, "Latest block should be 6 after adding new blocks");
    
    // Clean up (this is done automatically when temp_dir goes out of scope)
} 