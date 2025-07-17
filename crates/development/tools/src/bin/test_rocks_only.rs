// Purpose: A standalone script to test RocksDB functionality without PostgreSQL dependencies

use std::path::Path;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use rocksdb::{Options, DB, ColumnFamilyDescriptor, BlockBasedOptions};
use tempfile::tempdir;
use serde::{Serialize, Deserialize};
use anyhow::Result;

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

// Column families
const EVENTS_CF: &str = "events";
const BLOCKS_CF: &str = "blocks";

// Simple RocksDB storage implementation
struct RocksStorage {
    db: DB,
    path: String,
}

impl RocksStorage {
    fn new(path: &str) -> Result<Self> {
        // Create options for blocks column family
        let mut blocks_opts = Options::default();
        blocks_opts.create_if_missing(true);
        blocks_opts.set_max_write_buffer_number(3);
        
        // Create options for events column family  
        let mut events_opts = Options::default();
        events_opts.create_if_missing(true);
        events_opts.set_max_write_buffer_number(5);
        
        // Block-based table options for better performance
        let mut block_opts = BlockBasedOptions::default();
        block_opts.set_block_size(16 * 1024);
        block_opts.set_cache_index_and_filter_blocks(true);
        events_opts.set_block_based_table_factory(&block_opts);
        
        // Column family descriptors
        let cfs = vec![
            ColumnFamilyDescriptor::new(BLOCKS_CF, blocks_opts),
            ColumnFamilyDescriptor::new(EVENTS_CF, events_opts),
        ];
        
        // DB options
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        
        let db = DB::open_cf_descriptors(&db_opts, path, cfs)?;
        
        Ok(Self {
            db,
            path: path.to_string(),
        })
    }
    
    fn store_event(&self, event: &Event) -> Result<()> {
        let cf = self.db.cf_handle(EVENTS_CF).unwrap();
        
        // Key: chain_id:block_number:tx_index:log_index
        let key = format!("{}:{}:{}:{}", 
            event.chain, 
            event.block_number,
            event.tx_index,
            event.log_index
        );
        
        let value = bincode::serialize(event)?;
        self.db.put_cf(&cf, key.as_bytes(), &value)?;
        
        Ok(())
    }
    
    fn get_event(&self, chain: &str, block: u64, tx_index: u64, log_index: u64) -> Result<Option<Event>> {
        let cf = self.db.cf_handle(EVENTS_CF).unwrap();
        let key = format!("{}:{}:{}:{}", chain, block, tx_index, log_index);
        
        match self.db.get_cf(&cf, key.as_bytes())? {
            Some(value) => {
                let event = bincode::deserialize(&value)?;
                Ok(Some(event))
            }
            None => Ok(None),
        }
    }
    
    fn store_block(&self, chain: &str, block_number: u64, block_hash: &str) -> Result<()> {
        let cf = self.db.cf_handle(BLOCKS_CF).unwrap();
        let key = format!("{}:{}", chain, block_number);
        self.db.put_cf(&cf, key.as_bytes(), block_hash.as_bytes())?;
        Ok(())
    }
    
    fn get_block(&self, chain: &str, block_number: u64) -> Result<Option<String>> {
        let cf = self.db.cf_handle(BLOCKS_CF).unwrap();
        let key = format!("{}:{}", chain, block_number);
        
        match self.db.get_cf(&cf, key.as_bytes())? {
            Some(value) => Ok(Some(String::from_utf8(value)?)),
            None => Ok(None),
        }
    }
    
    fn delete_events_from_block(&self, chain: &str, block_number: u64) -> Result<u64> {
        let cf = self.db.cf_handle(EVENTS_CF).unwrap();
        let mut batch = rocksdb::WriteBatch::default();
        let mut count = 0;
        
        // Scan for all events in this block
        let prefix = format!("{}:{}:", chain, block_number);
        let iter = self.db.prefix_iterator_cf(&cf, prefix.as_bytes());
        
        for item in iter {
            if let Ok((key, _)) = item {
                if let Ok(key_str) = String::from_utf8(key.to_vec()) {
                    if key_str.starts_with(&prefix) {
                        batch.delete_cf(&cf, &key);
                        count += 1;
                    }
                }
            }
        }
        
        self.db.write(batch)?;
        Ok(count)
    }
    
    fn count_events(&self) -> Result<usize> {
        let cf = self.db.cf_handle(EVENTS_CF).unwrap();
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        Ok(iter.count())
    }
}

impl Drop for RocksStorage {
    fn drop(&mut self) {
        // RocksDB will be closed automatically
        println!("Closing RocksDB at {}", self.path);
    }
}

// Test functions
fn test_store_and_retrieve() -> Result<()> {
    let dir = tempdir()?;
    let storage = RocksStorage::new(dir.path().to_str().unwrap())?;
    
    // Create test event
    let event = Event {
        id: "evt_1".to_string(),
        chain: "ethereum".to_string(),
        block_number: 12345,
        block_hash: "0xabc123".to_string(),
        tx_hash: "0xdef456".to_string(),
        tx_index: 0,
        log_index: 1,
        event_type: "Transfer".to_string(),
        raw_data: vec![1, 2, 3, 4],
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    };
    
    // Store event
    storage.store_event(&event)?;
    println!("✓ Event stored successfully");
    
    // Retrieve event
    let retrieved = storage.get_event("ethereum", 12345, 0, 1)?;
    assert!(retrieved.is_some(), "Event should be found");
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, event.id);
    assert_eq!(retrieved.chain, event.chain);
    println!("✓ Event retrieved successfully");
    
    // Try non-existent event
    let not_found = storage.get_event("ethereum", 99999, 0, 0)?;
    assert!(not_found.is_none(), "Non-existent event should not be found");
    println!("✓ Non-existent event returns None");
    
    Ok(())
}

fn test_block_storage() -> Result<()> {
    let dir = tempdir()?;
    let storage = RocksStorage::new(dir.path().to_str().unwrap())?;
    
    // Store blocks
    storage.store_block("ethereum", 100, "0xblock100")?;
    storage.store_block("ethereum", 101, "0xblock101")?;
    storage.store_block("cosmos", 200, "cosmos_block_200")?;
    println!("✓ Blocks stored successfully");
    
    // Retrieve blocks
    let block100 = storage.get_block("ethereum", 100)?;
    assert_eq!(block100, Some("0xblock100".to_string()));
    
    let block200 = storage.get_block("cosmos", 200)?;
    assert_eq!(block200, Some("cosmos_block_200".to_string()));
    
    let not_found = storage.get_block("ethereum", 999)?;
    assert!(not_found.is_none());
    println!("✓ Blocks retrieved successfully");
    
    Ok(())
}

fn test_batch_operations() -> Result<()> {
    let dir = tempdir()?;
    let storage = RocksStorage::new(dir.path().to_str().unwrap())?;
    
    // Store multiple events in a block
    for i in 0..10 {
        let event = Event {
            id: format!("evt_{}", i),
            chain: "ethereum".to_string(),
            block_number: 1000,
            block_hash: "0xblock1000".to_string(),
            tx_hash: format!("0xtx{}", i / 3), // 3 events per tx
            tx_index: (i / 3) as u64,
            log_index: (i % 3) as u64,
            event_type: "Transfer".to_string(),
            raw_data: vec![i as u8],
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };
        storage.store_event(&event)?;
    }
    
    let count = storage.count_events()?;
    assert_eq!(count, 10, "Should have 10 events");
    println!("✓ Batch insert: {} events stored", count);
    
    // Delete all events from block 1000
    let deleted = storage.delete_events_from_block("ethereum", 1000)?;
    assert_eq!(deleted, 10, "Should delete 10 events");
    println!("✓ Batch delete: {} events deleted", deleted);
    
    let count = storage.count_events()?;
    assert_eq!(count, 0, "Should have 0 events after deletion");
    
    Ok(())
}

fn test_performance() -> Result<()> {
    let dir = tempdir()?;
    let storage = RocksStorage::new(dir.path().to_str().unwrap())?;
    
    let start = SystemTime::now();
    let num_events = 10000;
    
    // Insert many events
    for i in 0..num_events {
        let event = Event {
            id: format!("evt_{}", i),
            chain: "ethereum".to_string(),
            block_number: i / 100, // 100 events per block
            block_hash: format!("0xblock{}", i / 100),
            tx_hash: format!("0xtx{}", i / 10), // 10 events per tx
            tx_index: (i / 10) as u64,
            log_index: (i % 10) as u64,
            event_type: if i % 2 == 0 { "Transfer" } else { "Approval" }.to_string(),
            raw_data: vec![(i % 256) as u8; 100], // 100 bytes of data
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };
        storage.store_event(&event)?;
    }
    
    let elapsed = SystemTime::now().duration_since(start)?;
    let rate = num_events as f64 / elapsed.as_secs_f64();
    println!("✓ Performance test: Inserted {} events in {:.2}s ({:.0} events/sec)", 
        num_events, elapsed.as_secs_f64(), rate);
    
    // Random reads
    let start = SystemTime::now();
    let num_reads = 1000;
    for i in 0..num_reads {
        let block = (i * 7) % 100; // Random blocks
        let tx = (i * 3) % 10;
        let log = i % 10;
        let _ = storage.get_event("ethereum", block, tx, log)?;
    }
    
    let elapsed = SystemTime::now().duration_since(start)?;
    let rate = num_reads as f64 / elapsed.as_secs_f64();
    println!("✓ Random reads: {} reads in {:.2}s ({:.0} reads/sec)", 
        num_reads, elapsed.as_secs_f64(), rate);
    
    Ok(())
}

fn create_test_event(block: u64, tx_index: u64, log_index: u64, event_type: &str) -> Event {
    Event {
        id: format!("evt_{}_{}", block, log_index),
        chain: "ethereum".to_string(),
        block_number: block,
        block_hash: format!("0xblock{}", block),
        tx_hash: format!("0xtx{}_{}", block, tx_index),
        tx_index,
        log_index,
        event_type: event_type.to_string(),
        raw_data: vec![1, 2, 3],
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
    }
}

// Run tests
fn main() -> Result<()> {
    println!("Starting RocksDB tests...");
    
    // Test 1: Store and retrieve
    println!("\n--- Test 1: Store and Retrieve ---");
    test_store_and_retrieve()?;
    
    // Test 2: Block storage
    println!("\n--- Test 2: Block Storage ---");
    test_block_storage()?;
    
    // Test 3: Batch operations
    println!("\n--- Test 3: Batch Operations ---");
    test_batch_operations()?;
    
    // Test 4: Performance
    println!("\n--- Test 4: Performance ---");
    test_performance()?;
    
    println!("\n✅ All tests passed!");
    Ok(())
}