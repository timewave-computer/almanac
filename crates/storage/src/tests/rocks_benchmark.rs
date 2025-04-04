use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;
use rand::{thread_rng, Rng};
use indexer_common::Result;

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
    
    println!("RocksDB Write Performance Benchmark:");
    println!("-----------------------------------");
    
    for &batch_size in &batch_sizes {
        // Create mock events
        let events = create_mock_events("ethereum", batch_size);
        
        // Measure time to store events
        let start = Instant::now();
        
        for event in events {
            storage.store_event(event).await?;
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
    let total_events = 1000;
    let events = create_mock_events("ethereum", total_events);
    
    let event_ids: Vec<String> = events.iter()
        .map(|e| e.id().to_string())
        .collect();
    
    for event in events {
        storage.store_event(event).await?;
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
             
    // Test random reads
    let mut random_ids = event_ids.clone();
    random_ids.sort_by(|_, _| {
        let mut rng = thread_rng();
        if rng.gen() { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }
    });
    
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
    
    // Create mock events
    let event = create_mock_event("event1", "ethereum", 100);
    
    // Clone storage for thread
    let storage_clone = storage.clone();
    
    // Start a thread that will try to read the value while it's being written
    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Wait for a short time to ensure the main thread has started its operation
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            // Try to read the event
            let key = Key::new("events", "event1");
            
            
            // Return the result - it should either be None or a complete event, never partial
            storage_clone.get(&key)
        })
    });
    
    // Store the event in the main thread
    storage.store_event(event).await?;
    
    // Wait for the other thread to complete and get its result
    let read_result = handle.join().unwrap()?;
    
    // The result should be either None or a complete value, never partial
    if let Some(value) = read_result {
        // If a value was read, it should be a complete JSON string
        let event_data: serde_json::Value = serde_json::from_slice(&value)?;
        
        // Verify the event data has all expected fields
        assert!(event_data.get("id").is_some(), "Event is missing id field");
        assert!(event_data.get("chain").is_some(), "Event is missing chain field");
        assert!(event_data.get("block_number").is_some(), "Event is missing block_number field");
        assert!(event_data.get("block_hash").is_some(), "Event is missing block_hash field");
        assert!(event_data.get("tx_hash").is_some(), "Event is missing tx_hash field");
        assert!(event_data.get("timestamp").is_some(), "Event is missing timestamp field");
        assert!(event_data.get("event_type").is_some(), "Event is missing event_type field");
        assert!(event_data.get("raw_data").is_some(), "Event is missing raw_data field");
    }
    
    println!("Transaction isolation test passed: concurrent read returned valid data");
    
    Ok(())
} 