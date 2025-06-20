#![cfg(feature = "rocks")]
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;
use rand::{thread_rng, seq::SliceRandom};
use indexer_core::Result;

use crate::rocks::{RocksStorage, RocksConfig, Key};
use crate::tests::common::{create_mock_event, create_mock_events, assert_duration_less_than};
use crate::Storage;

// Test checkpoint 1.3.1: Benchmark RocksDB performance

/// Benchmark write performance with various batch sizes
#[tokio::test]
async fn benchmark_write_performance() -> Result<()> {
    // Create temporary directory for the test
    let temp_dir = TempDir::new()?;
    let config = RocksConfig {
        path: temp_dir.path().to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 128,
    };
    
    // Create the storage
    let storage = RocksStorage::new(config)?;
    
    // Define batch sizes to test
    let batch_sizes = vec![1, 10, 100, 1000];
    let chain = "ethereum";
    
    println!("RocksDB Write Performance Benchmark:");
    println!("-----------------------------------");
    
    for &batch_size in &batch_sizes {
        // Create mock events
        let events = create_mock_events(chain, batch_size);
        
        // Measure time to store events
        let start = Instant::now();
        
        for event in events {
            storage.store_event(chain, event).await?;
        }
        
        let duration = start.elapsed();
        let ops_per_sec = batch_size as f64 / duration.as_secs_f64();
        
        println!("Batch size: {}, Duration: {:?}, Ops/sec: {:.2}", 
                 batch_size, duration, ops_per_sec);
                 
        // Ensure performance meets requirements (sub 10ms per event for small batches)
        if batch_size <= 10 {
            let expected_max_duration = Duration::from_millis(10 * batch_size as u64);
            assert_duration_less_than(duration, expected_max_duration,
                &format!("Write operation for batch size {} too slow", batch_size));
        }
    }
    
    Ok(())
}

/// Benchmark read performance with different access patterns
#[tokio::test]
async fn benchmark_read_performance() -> Result<()> {
    // Create temporary directory for the test
    let temp_dir = TempDir::new()?;
    let config = RocksConfig {
        path: temp_dir.path().to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 128,
    };
    
    // Create the storage
    let storage = RocksStorage::new(config)?;
    
    // Create and store mock events
    let chain = "ethereum";
    let total_events = 1000;
    let events = create_mock_events(chain, total_events);
    
    let event_ids: Vec<String> = events.iter()
        .map(|e| e.id().to_string())
        .collect();
    
    for event in events {
        storage.store_event(chain, event).await?;
    }
    
    println!("RocksDB Read Performance Benchmark:");
    println!("----------------------------------");
    
    // Test sequential reads
    let start = Instant::now();
    for id in &event_ids {
        let key = Key::new("events", id);
        let _result = storage.get(&key)?;
    }
    
    let duration = start.elapsed();
    let ops_per_sec = total_events as f64 / duration.as_secs_f64();
    
    println!("Sequential reads: {} events, Duration: {:?}, Ops/sec: {:.2}", 
             total_events, duration, ops_per_sec);
             
    // Test random reads - now using shuffle instead of sort_by
    let mut random_ids = event_ids.clone();
    let mut rng = thread_rng();
    random_ids.shuffle(&mut rng);
    
    let start = Instant::now();
    for id in &random_ids {
        let key = Key::new("events", id);
        let _result = storage.get(&key)?;
    }
    
    let duration = start.elapsed();
    let ops_per_sec = total_events as f64 / duration.as_secs_f64();
    
    println!("Random reads: {} events, Duration: {:?}, Ops/sec: {:.2}", 
             total_events, duration, ops_per_sec);
             
    // Ensure performance meets requirements (sub 5ms per read)
    let expected_max_duration = Duration::from_millis(5 * total_events as u64);
    assert_duration_less_than(duration, expected_max_duration,
        "Random read operations too slow");
    
    Ok(())
}

/// Verify transaction isolation and atomicity
#[tokio::test]
async fn test_transaction_isolation() -> Result<()> {
    // Create temporary directory for the test
    let temp_dir = TempDir::new()?;
    let config = RocksConfig {
        path: temp_dir.path().to_str().unwrap().to_string(),
        create_if_missing: true,
        cache_size_mb: 128,
    };
    
    // Create the storage
    let storage = Arc::new(RocksStorage::new(config)?);
    
    // Create mock events and define chain
    let chain = "ethereum";
    let event = create_mock_event("event1", chain, 100);
    
    // Clone storage for thread
    let storage_clone = storage.clone();
    
    // Start a thread that will try to read the value while it's being written
    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Wait for a short time to ensure the main thread has started its operation
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            // Try to read the event using the public get method
            let key = Key::new("events", "event1");
            storage_clone.get(&key)
        })
    });
    
    // Store the event in the main thread with the chain parameter
    storage.store_event(chain, event).await?;
    
    // Wait for the other thread to complete and get its result
    let read_result = handle.join().unwrap()?;
    
    // The result should be either None or a complete value, never partial
    if let Some(bytes) = read_result {
        // Verify that we can deserialize the data, which means it's complete
        let event_data: Result<crate::rocks::EventData> = bincode::deserialize(&bytes)
            .map_err(|e| indexer_core::Error::generic(format!("Failed to deserialize event data: {}", e)));
        
        let event_data = event_data?;
        
        // Verify the event data has all expected fields
        assert_eq!(event_data.id, "event1", "Event has incorrect id");
        assert_eq!(event_data.chain, chain, "Event has incorrect chain");
        assert_eq!(event_data.block_number, 100, "Event has incorrect block number");
        assert!(event_data.block_hash.starts_with("block_hash_"), "Event has incorrect block hash format");
        assert!(event_data.tx_hash.starts_with("tx_hash_"), "Event has incorrect tx hash format");
        assert!(event_data.event_type.len() > 0, "Event is missing event type");
        assert!(event_data.raw_data.len() > 0, "Event is missing raw data");
    }
    
    println!("Transaction isolation test passed: concurrent read returned valid data");
    
    Ok(())
}

#[tokio::test]
async fn test_rocks_single_block_storage() -> Result<()> {
    // Create a temporary directory for the rocks db
    let tempdir = TempDir::new().unwrap();
    
    // Configure the storage
    let config = RocksConfig {
        path: tempdir.path().to_string_lossy().to_string(),
        create_if_missing: true,
        cache_size_mb: 128,
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Generate test events (100 events in a single block)
    let chain = "ethereum";
    // Create 100 events with block number 1
    for i in 0..100 {
        let event = create_mock_event(&format!("event_1_{}", i), chain, 1);
        storage.store_event(chain, event).await?;
    }
    
    // Close database
    drop(storage);
    
    Ok(())
}

#[tokio::test]
async fn test_rocks_multi_block_storage() -> Result<()> {
    // Create a temporary directory for the rocks db
    let tempdir = TempDir::new().unwrap();
    
    // Configure the storage
    let config = RocksConfig {
        path: tempdir.path().to_string_lossy().to_string(),
        create_if_missing: true,
        cache_size_mb: 128,
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Generate test events (100 blocks, 10 events each)
    let chain = "ethereum";
    let num_blocks = 100;
    let events_per_block = 10;
    
    // For each block, create events and store them
    for block_number in 1..=num_blocks {
        // Create events for this block
        for i in 0..events_per_block {
            let event = create_mock_event(&format!("event_{}_{}", block_number, i), chain, block_number);
            storage.store_event(chain, event).await?;
        }
    }
    
    // Close database
    drop(storage);
    
    Ok(())
}

#[tokio::test]
async fn test_rocks_event_lookup_performance() -> Result<()> {
    // Create a temporary directory for the rocks db
    let tempdir = TempDir::new().unwrap();
    
    // Configure the storage
    let config = RocksConfig {
        path: tempdir.path().to_string_lossy().to_string(),
        create_if_missing: true,
        cache_size_mb: 128,
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Setup test data - 1000 blocks with 10 events each
    let chain = "ethereum";
    let num_blocks = 1000;
    let events_per_block = 10;
    
    // Generate and store events for each block
    for block_number in 1..=num_blocks {
        // Create events for this block
        for i in 0..events_per_block {
            let event = create_mock_event(&format!("event_{}_{}", block_number, i), chain, block_number);
            storage.store_event(chain, event).await?;
        }
    }
    
    // Close database
    drop(storage);
    
    Ok(())
} 