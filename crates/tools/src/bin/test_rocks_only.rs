// Purpose: A standalone script to test RocksDB functionality without PostgreSQL dependencies

use std::path::{Path, PathBuf};
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rocksdb::{Options, DB, ColumnFamilyDescriptor, BlockBasedOptions};
use tempfile::tempdir;
use serde::{Serialize, Deserialize};

// Simple Event struct for testing
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Event {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    tx_index: u64,
    log_index: u64,
    event_type: String,
    raw_data: Vec<u8>,
    timestamp: u64,
}

// Block status for testing
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
enum BlockStatus {
    Pending,
    Final,
}

// Basic error type
type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, Error>;

// Column family names
const CF_EVENTS: &str = "events";
const CF_BLOCKS: &str = "blocks";
const CF_META: &str = "meta";

// Helper function to create a test event
fn create_test_event(block: u64, tx_index: u64, log_index: u64, event_type: &str) -> Event {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    Event {
        id: format!("{}_{}_{}_{}", "test_chain", block, tx_index, log_index),
        chain: "test_chain".to_string(),
        block_number: block,
        block_hash: format!("block_hash_{}", block),
        tx_hash: format!("tx_hash_{}", tx_index),
        tx_index,
        log_index,
        event_type: event_type.to_string(),
        raw_data: vec![],
        timestamp,
    }
}

// Simple RocksDB wrapper
struct RocksDBStore {
    db: DB,
}

impl RocksDBStore {
    // Create a new RocksDB instance with column families
    fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        // Create directory if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        
        // Configure options
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);
        
        // Configure block based options
        let mut block_opts = BlockBasedOptions::default();
        block_opts.set_block_size(16 * 1024); // 16KB
        options.set_block_based_table_factory(&block_opts);
        
        // Prepare column family descriptors
        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new(CF_EVENTS, options.clone()),
            ColumnFamilyDescriptor::new(CF_BLOCKS, options.clone()),
            ColumnFamilyDescriptor::new(CF_META, options.clone()),
        ];
        
        // Open DB with column families
        let db = DB::open_cf_descriptors(&options, path, cf_descriptors)?;
        
        Ok(Self { db })
    }
    
    // Store events
    fn store_events(&self, chain: &str, events: &[Event]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        
        // Get column families
        let events_cf = self.db.cf_handle(CF_EVENTS).ok_or("Events CF not found")?;
        let blocks_cf = self.db.cf_handle(CF_BLOCKS).ok_or("Blocks CF not found")?;
        
        // Use a batch for atomic writes
        let mut batch = rocksdb::WriteBatch::default();
        
        for event in events {
            // Store event in events_cf
            let event_key = format!("{}:{}:{}:{}", chain, event.block_number, event.tx_index, event.log_index);
            let event_value = serde_json::to_vec(event)?;
            batch.put_cf(&events_cf, event_key.as_bytes(), event_value);
            
            // Store block info in blocks_cf
            let block_key = format!("{}:{}", chain, event.block_number);
            let block_info = serde_json::json!({
                "hash": event.block_hash,
                "timestamp": event.timestamp,
                "status": "Pending"
            });
            batch.put_cf(&blocks_cf, block_key.as_bytes(), serde_json::to_vec(&block_info)?);
        }
        
        // Write the batch
        self.db.write(batch)?;
        
        Ok(())
    }
    
    // Update block status
    fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        let blocks_cf = self.db.cf_handle(CF_BLOCKS).ok_or("Blocks CF not found")?;
        
        // Block key
        let block_key = format!("{}:{}", chain, block_number);
        
        // Get existing block info
        let block_value = self.db.get_cf(&blocks_cf, block_key.as_bytes())?;
        
        if let Some(value) = block_value {
            let mut block_info: serde_json::Value = serde_json::from_slice(&value)?;
            
            // Update status
            let status_str = match status {
                BlockStatus::Pending => "Pending",
                BlockStatus::Final => "Final",
            };
            block_info["status"] = serde_json::Value::String(status_str.to_string());
            
            // Store updated block info
            self.db.put_cf(&blocks_cf, block_key.as_bytes(), serde_json::to_vec(&block_info)?)?;
        }
        
        Ok(())
    }
    
    // Get latest block
    fn get_latest_block(&self, chain: &str) -> Result<u64> {
        let blocks_cf = self.db.cf_handle(CF_BLOCKS).ok_or("Blocks CF not found")?;
        
        // Iterate over blocks
        let mut iter = self.db.iterator_cf(&blocks_cf, rocksdb::IteratorMode::End);
        let mut latest_block = 0;
        
        while let Some(result) = iter.next() {
            let (key, _value) = result?;
            let key = key.to_vec();
            let key_str = String::from_utf8_lossy(&key);
            let parts: Vec<&str> = key_str.split(':').collect();
            
            if parts.len() == 2 && parts[0] == chain {
                if let Ok(block_number) = parts[1].parse::<u64>() {
                    latest_block = std::cmp::max(latest_block, block_number);
                }
            }
        }
        
        Ok(latest_block)
    }
    
    // Get latest block with specific status
    fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        let blocks_cf = self.db.cf_handle(CF_BLOCKS).ok_or("Blocks CF not found")?;
        
        // Status string
        let status_str = match status {
            BlockStatus::Pending => "Pending",
            BlockStatus::Final => "Final",
        };
        
        // Iterate over blocks
        let mut iter = self.db.iterator_cf(&blocks_cf, rocksdb::IteratorMode::End);
        let mut latest_block = 0;
        
        while let Some(result) = iter.next() {
            let (key, value) = result?;
            let key = key.to_vec();
            let value = value.to_vec();
            let key_str = String::from_utf8_lossy(&key);
            let parts: Vec<&str> = key_str.split(':').collect();
            
            if parts.len() == 2 && parts[0] == chain {
                if let Ok(block_number) = parts[1].parse::<u64>() {
                    // Check block status
                    let block_info: serde_json::Value = serde_json::from_slice(&value)?;
                    if let Some(block_status) = block_info["status"].as_str() {
                        if block_status == status_str {
                            latest_block = std::cmp::max(latest_block, block_number);
                        }
                    }
                }
            }
        }
        
        Ok(latest_block)
    }
    
    // Delete blocks from a specific block number
    fn delete_blocks_from(&self, chain: &str, from_block: u64) -> Result<()> {
        let events_cf = self.db.cf_handle(CF_EVENTS).ok_or("Events CF not found")?;
        let blocks_cf = self.db.cf_handle(CF_BLOCKS).ok_or("Blocks CF not found")?;
        
        // Use a batch for atomic writes
        let mut batch = rocksdb::WriteBatch::default();
        
        // Delete events
        let event_prefix = format!("{}:{}:", chain, from_block);
        let mut iter = self.db.iterator_cf(&events_cf, rocksdb::IteratorMode::From(
            event_prefix.as_bytes(),
            rocksdb::Direction::Forward,
        ));
        
        while let Some(result) = iter.next() {
            let (key, _) = result?;
            let key_vec = key.to_vec();
            let key_str = String::from_utf8_lossy(&key_vec);
            let parts: Vec<&str> = key_str.split(':').collect();
            
            if parts.len() >= 2 && parts[0] == chain {
                if let Ok(block_number) = parts[1].parse::<u64>() {
                    if block_number >= from_block {
                        batch.delete_cf(&events_cf, &key_vec);
                    }
                }
            }
        }
        
        // Delete blocks
        let block_prefix = format!("{}:", chain);
        let mut iter = self.db.iterator_cf(&blocks_cf, rocksdb::IteratorMode::From(
            block_prefix.as_bytes(),
            rocksdb::Direction::Forward,
        ));
        
        while let Some(result) = iter.next() {
            let (key, _) = result?;
            let key_vec = key.to_vec();
            let key_str = String::from_utf8_lossy(&key_vec);
            let parts: Vec<&str> = key_str.split(':').collect();
            
            if parts.len() == 2 && parts[0] == chain {
                if let Ok(block_number) = parts[1].parse::<u64>() {
                    if block_number >= from_block {
                        batch.delete_cf(&blocks_cf, &key_vec);
                    }
                }
            }
        }
        
        // Write the batch
        self.db.write(batch)?;
        
        Ok(())
    }
}

// Run tests
fn main() -> Result<()> {
    println!("Starting RocksDB tests...");
    
    // Test 1: Store and retrieve
    println!("\n--- Test 1: Store and Retrieve ---");
    test_store_and_retrieve()?;
    
    // Test 2: Block finality
    println!("\n--- Test 2: Block Finality ---");
    test_block_finality()?;
    
    // Test 3: Chain reorg
    println!("\n--- Test 3: Chain Reorganization ---");
    test_chain_reorg()?;
    
    println!("\nAll tests passed!");
    
    Ok(())
}

// Test 1: Store and retrieve
fn test_store_and_retrieve() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path();
    
    println!("Using temporary directory: {}", db_path.display());
    
    // Initialize RocksDB
    let rocks = RocksDBStore::new(db_path)?;
    
    // Store some test events
    let chain = "test_chain";
    let events = vec![
        create_test_event(1, 0, 0, "Transfer"),
        create_test_event(1, 0, 1, "Approval"),
        create_test_event(2, 0, 0, "Transfer"),
    ];
    
    println!("Storing events...");
    
    // Store block 1 with its events
    rocks.store_events(chain, &events[0..2])?;
    
    // Store block 2 with its event
    rocks.store_events(chain, &events[2..3])?;
    
    // Mark blocks as final
    rocks.update_block_status(chain, 1, BlockStatus::Final)?;
    rocks.update_block_status(chain, 2, BlockStatus::Final)?;
    
    // Get latest block
    let latest_block = rocks.get_latest_block(chain)?;
    println!("Latest block: {}", latest_block);
    assert_eq!(latest_block, 2, "Latest block should be 2");
    
    // Get latest finalized block
    let latest_final = rocks.get_latest_block_with_status(chain, BlockStatus::Final)?;
    println!("Latest finalized block: {}", latest_final);
    assert_eq!(latest_final, 2, "Latest finalized block should be 2");
    
    println!("Test passed!");
    
    Ok(())
}

// Test 2: Block finality
fn test_block_finality() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path();
    
    println!("Using temporary directory: {}", db_path.display());
    
    // Initialize RocksDB
    let rocks = RocksDBStore::new(db_path)?;
    
    // Store some test events with different block statuses
    let chain = "test_chain";
    let events = vec![
        create_test_event(1, 0, 0, "Transfer"),
        create_test_event(2, 0, 0, "Transfer"),
        create_test_event(3, 0, 0, "Transfer"),
    ];
    
    println!("Storing events...");
    
    // Store all events
    rocks.store_events(chain, &events)?;
    
    // Set different statuses
    rocks.update_block_status(chain, 1, BlockStatus::Final)?;
    rocks.update_block_status(chain, 2, BlockStatus::Pending)?;
    rocks.update_block_status(chain, 3, BlockStatus::Pending)?;
    
    // Get latest finalized block
    let latest_final = rocks.get_latest_block_with_status(chain, BlockStatus::Final)?;
    println!("Latest finalized block: {}", latest_final);
    assert_eq!(latest_final, 1, "Latest finalized block should be 1");
    
    // Update block 2 to final
    rocks.update_block_status(chain, 2, BlockStatus::Final)?;
    
    // Get latest finalized block again
    let new_latest_final = rocks.get_latest_block_with_status(chain, BlockStatus::Final)?;
    println!("Latest finalized block after update: {}", new_latest_final);
    assert_eq!(new_latest_final, 2, "Latest finalized block should now be 2");
    
    println!("Test passed!");
    
    Ok(())
}

// Test 3: Chain reorg
fn test_chain_reorg() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path();
    
    println!("Using temporary directory: {}", db_path.display());
    
    // Initialize RocksDB
    let rocks = RocksDBStore::new(db_path)?;
    
    // Store events for blocks 1-5
    let chain = "test_chain";
    println!("Storing initial blocks 1-5...");
    
    for block in 1..=5 {
        let event = create_test_event(block, 0, 0, "Transfer");
        rocks.store_events(chain, &[event])?;
        rocks.update_block_status(chain, block, BlockStatus::Pending)?;
    }
    
    // Finalize blocks 1-3
    println!("Finalizing blocks 1-3...");
    for block in 1..=3 {
        rocks.update_block_status(chain, block, BlockStatus::Final)?;
    }
    
    // Simulate chain reorganization starting from block 4
    println!("Simulating chain reorganization from block 4...");
    rocks.delete_blocks_from(chain, 4)?;
    
    // Check that block 3 is the latest block
    let latest_block = rocks.get_latest_block(chain)?;
    println!("Latest block after reorg: {}", latest_block);
    assert_eq!(latest_block, 3, "After reorg, latest block should be 3");
    
    // Add new blocks 4-6 after reorg
    println!("Adding new blocks 4-6 after reorg...");
    for block in 4..=6 {
        let event = create_test_event(block, 1, 0, "NewTransfer");  // different tx_index to simulate different chain
        rocks.store_events(chain, &[event])?;
        rocks.update_block_status(chain, block, BlockStatus::Pending)?;
    }
    
    // Verify total number of blocks
    let latest_block = rocks.get_latest_block(chain)?;
    println!("Latest block after adding new blocks: {}", latest_block);
    assert_eq!(latest_block, 6, "Latest block should be 6 after adding new blocks");
    
    println!("Test passed!");
    
    Ok(())
} 