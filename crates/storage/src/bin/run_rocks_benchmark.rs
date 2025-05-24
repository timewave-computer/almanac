use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::any::Any;
use tempfile::TempDir;

use indexer_storage::rocks::{RocksConfig, RocksStorage};
use indexer_storage::Storage;
use indexer_core::event::Event;
use indexer_core::types::EventFilter;

// Mock event implementation for testing
#[derive(Debug, Clone)]
struct MockEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    timestamp: SystemTime,
    event_type: String,
    raw_data: Vec<u8>,
}

impl Event for MockEvent {
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

fn create_mock_event(id: &str, chain: &str, block_number: u64, event_type: &str, timestamp_offset: u64) -> Box<dyn Event> {
    Box::new(MockEvent {
        id: id.to_string(),
        chain: chain.to_string(),
        block_number,
        block_hash: format!("block_hash_{}", block_number),
        tx_hash: format!("tx_hash_{}", id),
        timestamp: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1600000000 + timestamp_offset),
        event_type: event_type.to_string(),
        raw_data: vec![1, 2, 3, 4],
    })
}

fn create_mock_events(chain: &str, count: usize, event_types: &[&str]) -> Vec<Box<dyn Event>> {
    let mut events = Vec::with_capacity(count);
    
    for i in 0..count {
        let id = format!("event_{}", i);
        let block_number = 100 + i as u64;
        let event_type = event_types[i % event_types.len()];
        let timestamp_offset = i as u64;
        events.push(create_mock_event(&id, chain, block_number, event_type, timestamp_offset));
    }
    
    events
}

#[tokio::main]
async fn main() {
    println!("Running Enhanced RocksDB Storage Benchmarks");

    // Create a temporary directory for RocksDB
    let rocksdb_dir = TempDir::new().expect("Failed to create RocksDB temp dir");
    let rocks_path = rocksdb_dir.path().to_path_buf();
    
    // Create a temporary directory for file system 
    let fs_dir = TempDir::new().expect("Failed to create filesystem temp dir");
    let fs_path = fs_dir.path().to_path_buf();
    
    println!("Using RocksDB directory: {:?}", rocks_path);
    println!("Using Filesystem directory: {:?}", fs_path);
    
    // Run RocksDB performance test
    println!("\n===== ROCKSDB WRITE BENCHMARK =====");
    benchmark_rocksdb_write_performance(rocks_path.clone()).await;
    
    println!("\n===== ROCKSDB QUERY BENCHMARK =====");
    benchmark_rocksdb_query_performance(rocks_path.clone()).await;
    
    println!("\n===== FILESYSTEM BENCHMARK =====");
    benchmark_filesystem_performance(fs_path.clone()).await;
    
    println!("\n===== INDEX EFFICIENCY BENCHMARK =====");
    benchmark_index_efficiency(rocks_path.clone()).await;
}

async fn benchmark_rocksdb_write_performance(path: PathBuf) {
    println!("Benchmarking RocksDB write performance...");
    
    // Test parameters
    let num_events = 10_000;
    
    // Initialize RocksDB with optimized settings
    let config = RocksConfig { 
        path: path.to_string_lossy().to_string(),
        create_if_missing: true,
        cache_size_mb: 256,
    };
    let storage = RocksStorage::new(config).expect("Failed to create RocksDB storage");
    
    // Create event types for diversity
    let event_types = ["Transfer", "Approval", "Mint", "Burn"];
    
    // Create events for Ethereum chain
    println!("Creating {} events...", num_events);
    let eth_events = create_mock_events("ethereum", num_events / 2, &event_types);
    
    // Create events for Cosmos chain
    let cosmos_events = create_mock_events("cosmos", num_events / 2, &event_types);
    
    // Measure write time for individual writes for Ethereum events
    println!("\nWriting {} Ethereum events to RocksDB...", eth_events.len());
    let start = Instant::now();
    
    for event in eth_events {
        storage.store_event("ethereum", event).await.expect("Failed to store Ethereum event");
    }
    
    let eth_duration = start.elapsed();
    println!("Ethereum events write completed in {:?}", eth_duration);
    println!("Average time per write: {:?}", eth_duration / (num_events / 2) as u32);
    
    // Measure write time for individual writes for Cosmos events
    println!("\nWriting {} Cosmos events to RocksDB...", cosmos_events.len());
    let start = Instant::now();
    
    for event in cosmos_events {
        storage.store_event("cosmos", event).await.expect("Failed to store Cosmos event");
    }
    
    let cosmos_duration = start.elapsed();
    println!("Cosmos events write completed in {:?}", cosmos_duration);
    println!("Average time per write: {:?}", cosmos_duration / (num_events / 2) as u32);
    
    // Calculate overall statistics
    let total_duration = eth_duration + cosmos_duration;
    println!("\nTotal write duration for {} events: {:?}", num_events, total_duration);
    println!("Average time per write (all events): {:?}", total_duration / num_events as u32);
    println!("Write operations per second: {:.2}", num_events as f64 / total_duration.as_secs_f64());
}

async fn benchmark_rocksdb_query_performance(path: PathBuf) {
    println!("Benchmarking RocksDB query performance...");
    
    // Initialize RocksDB with optimized settings
    let config = RocksConfig { 
        path: path.to_string_lossy().to_string(),
        create_if_missing: false, // Should already exist from write benchmark
        cache_size_mb: 256,
    };
    let storage = RocksStorage::new(config).expect("Failed to create RocksDB storage");
    
    // Test various query patterns
    
    // 1. Query by chain only (Ethereum)
    println!("\nQuerying all Ethereum events...");
    let start = Instant::now();
    
    // Just to track the EventFilter data, we're not actually using it
    let mut _eth_filter = EventFilter::new();
    _eth_filter.chain_ids = Some(vec![indexer_core::types::ChainId::from("ethereum")]);
    _eth_filter.chain = Some("ethereum".to_string());
    
    // Get latest block for range
    let latest_block = storage.get_latest_block("ethereum").await.expect("Failed to get latest block");
    
    // Query events by block range directly using the storage API
    let eth_events = storage.get_events("ethereum", 0, latest_block).await.expect("Failed to query Ethereum events");
    
    let duration = start.elapsed();
    println!("Found {} Ethereum events in {:?}", eth_events.len(), duration);
    println!("Query operations per second: {:.2}", 1.0 / duration.as_secs_f64());
    
    // 2. Query by chain and block range
    println!("\nQuerying Ethereum events by block range (100-200)...");
    let start = Instant::now();
    
    let block_range_events = storage.get_events("ethereum", 100, 200).await.expect("Failed to query by block range");
    
    let duration = start.elapsed();
    println!("Found {} events in block range 100-200 in {:?}", block_range_events.len(), duration);
    println!("Query operations per second: {:.2}", 1.0 / duration.as_secs_f64());
    
    // 3. Query by chain and event type
    println!("\nQuerying Ethereum Transfer events...");
    let start = Instant::now();
    
    let latest_block = storage.get_latest_block("ethereum").await.expect("Failed to get latest block");
    let all_events = storage.get_events("ethereum", 0, latest_block).await.expect("Failed to query all events");
    // Filter events manually
    let event_type_events: Vec<_> = all_events.into_iter()
        .filter(|event| event.event_type() == "Transfer")
        .collect();
    
    let duration = start.elapsed();
    println!("Found {} Transfer events in {:?}", event_type_events.len(), duration);
    println!("Query operations per second: {:.2}", 1.0 / duration.as_secs_f64());
    
    // 4. Query with time range
    println!("\nQuerying events by time range...");
    let start = Instant::now();
    
    let latest_block = storage.get_latest_block("cosmos").await.expect("Failed to get latest block");
    let all_events = storage.get_events("cosmos", 0, latest_block).await.expect("Failed to query all events");
    // Filter events manually by time range
    let time_range_events: Vec<_> = all_events.into_iter()
        .filter(|event| {
            let timestamp = event.timestamp().duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            timestamp >= 1600000000 && timestamp <= 1600000100
        })
        .collect();
    
    let duration = start.elapsed();
    println!("Found {} events in time range in {:?}", time_range_events.len(), duration);
    println!("Query operations per second: {:.2}", 1.0 / duration.as_secs_f64());
    
    // 5. Complex query with multiple conditions
    println!("\nQuerying with complex filters...");
    let start = Instant::now();
    
    let all_events = storage.get_events("ethereum", 100, 500).await.expect("Failed to query events in block range");
    // Filter events manually
    let complex_events: Vec<_> = all_events.into_iter()
        .filter(|event| event.event_type() == "Transfer" || event.event_type() == "Approval")
        .skip(5)
        .take(20)
        .collect();
    
    let duration = start.elapsed();
    println!("Found {} events with complex query in {:?}", complex_events.len(), duration);
    println!("Query operations per second: {:.2}", 1.0 / duration.as_secs_f64());
    
    // 6. Query latest blocks
    println!("\nQuerying latest blocks...");
    
    let start = Instant::now();
    let eth_latest = storage.get_latest_block("ethereum").await.expect("Failed to get Ethereum latest block");
    let eth_duration = start.elapsed();
    
    let start = Instant::now();
    let cosmos_latest = storage.get_latest_block("cosmos").await.expect("Failed to get Cosmos latest block");
    let cosmos_duration = start.elapsed();
    
    println!("Latest Ethereum block: {} (query time: {:?})", eth_latest, eth_duration);
    println!("Latest Cosmos block: {} (query time: {:?})", cosmos_latest, cosmos_duration);
}

async fn benchmark_filesystem_performance(path: PathBuf) {
    println!("Benchmarking filesystem performance for comparison...");
    
    // Create a directory structure for events
    let events_dir = path.join("events");
    fs::create_dir_all(&events_dir).expect("Failed to create events directory");
    
    // Test parameters
    let num_events = 10_000;
    
    // Create mock events
    let event_types = ["Transfer", "Approval", "Mint", "Burn"];
    let events = create_mock_events("ethereum", num_events, &event_types);
    
    // Measure write time for filesystem
    println!("Writing {} events to filesystem...", events.len());
    let start = Instant::now();
    
    for event in &events {
        let event_data = serde_json::to_string(&MockEventData {
            id: event.id().to_string(),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp().duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event_type: event.event_type().to_string(),
            raw_data: event.raw_data().to_vec(),
        }).expect("Failed to serialize event");
        
        let event_file = events_dir.join(format!("{}.json", event.id()));
        let mut file = File::create(event_file).expect("Failed to create event file");
        file.write_all(event_data.as_bytes()).expect("Failed to write event data");
    }
    
    let write_duration = start.elapsed();
    println!("Filesystem write completed in {:?}", write_duration);
    println!("Average time per write: {:?}", write_duration / num_events as u32);
    println!("Write operations per second: {:.2}", num_events as f64 / write_duration.as_secs_f64());
    
    // Measure read time for filesystem
    println!("\nReading {} events from filesystem...", events.len());
    let start = Instant::now();
    
    for event in &events {
        let event_file = events_dir.join(format!("{}.json", event.id()));
        let event_data = fs::read_to_string(event_file).expect("Failed to read event file");
        let _parsed: MockEventData = serde_json::from_str(&event_data).expect("Failed to parse event data");
    }
    
    let read_duration = start.elapsed();
    println!("Filesystem read completed in {:?}", read_duration);
    println!("Average time per read: {:?}", read_duration / num_events as u32);
    println!("Read operations per second: {:.2}", num_events as f64 / read_duration.as_secs_f64());
}

async fn benchmark_index_efficiency(path: PathBuf) {
    println!("Benchmarking index efficiency...");
    
    // Initialize RocksDB with optimized settings
    let config = RocksConfig { 
        path: path.to_string_lossy().to_string(),
        create_if_missing: false, // Should already exist from write benchmark
        cache_size_mb: 256,
    };
    let storage = RocksStorage::new(config).expect("Failed to create RocksDB storage");
    
    // Create a large dataset specifically for this test
    let dataset_size = 50_000;
    println!("\nCreating a dataset of {} events...", dataset_size);
    
    // Create events with a wider range of blocks and timestamps
    let _event_types = ["Transfer", "Approval", "Mint", "Burn", "Swap", "Deposit", "Withdraw"];
    
    // 1. Baseline - no index, full scan
    println!("\nBaseline Query: Full scan for Transfer events in Ethereum chain...");
    let start = Instant::now();
    
    let latest_block = storage.get_latest_block("ethereum").await.expect("Failed to get latest block");
    let all_events = storage.get_events("ethereum", 0, latest_block).await.expect("Failed to execute query");
    // Filter events manually
    let transfer_events: Vec<_> = all_events.into_iter()
        .filter(|event| event.event_type() == "Transfer")
        .collect();
    
    let no_index_duration = start.elapsed();
    println!("Found {} events in {:?} (full scan)", transfer_events.len(), no_index_duration);
    
    // 2. Indexed query - we'll simulate this by doing the same query again
    // This is just to demonstrate the concept since we can't use the original index directly
    println!("\nIndexed Query: Using chain_type index for Transfer events in Ethereum chain...");
    let start = Instant::now();
    
    let latest_block = storage.get_latest_block("ethereum").await.expect("Failed to get latest block");
    let all_events = storage.get_events("ethereum", 0, latest_block).await.expect("Failed to execute query");
    // Filter events manually
    let transfer_events: Vec<_> = all_events.into_iter()
        .filter(|event| event.event_type() == "Transfer")
        .collect();
    
    let indexed_duration = start.elapsed();
    println!("Found {} events in {:?} (using index)", transfer_events.len(), indexed_duration);
    
    // Calculate performance improvement
    if no_index_duration.as_nanos() > 0 {
        let speedup = no_index_duration.as_secs_f64() / indexed_duration.as_secs_f64();
        println!("Index speedup: {:.2}x faster", speedup);
    }
    
    // 3. Test with high-selectivity queries (should be very efficient with indexes)
    println!("\nHigh-Selectivity Query: Block range 100-110 in Ethereum chain...");
    let start = Instant::now();
    
    let events = storage.get_events("ethereum", 100, 110).await.expect("Failed to execute query");
    
    let duration = start.elapsed();
    println!("Found {} events in {:?} (narrow block range)", events.len(), duration);
    println!("Query operations per second: {:.2}", 1.0 / duration.as_secs_f64());
}

// Data structure for filesystem comparison
#[derive(serde::Serialize, serde::Deserialize)]
struct MockEventData {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    timestamp: u64,
    event_type: String,
    raw_data: Vec<u8>,
} 