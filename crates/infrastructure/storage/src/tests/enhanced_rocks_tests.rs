//! Enhanced RocksDB testing suite
#![cfg(feature = "rocks")]

use std::sync::Arc;
use std::time::SystemTime;
use crate::rocks::{RocksStorage, RocksConfig};
use crate::Storage;
use almanac_infrastructure_core::Result;
use almanac_infrastructure_core::types::{Event, EventFilter};
use std::any::Any;

/// Mock event for testing
#[derive(Clone, Debug)]
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
    fn id(&self) -> &str { &self.id }
    fn chain(&self) -> &str { &self.chain }
    fn block_number(&self) -> u64 { self.block_number }
    fn block_hash(&self) -> &str { &self.block_hash }
    fn tx_hash(&self) -> &str { &self.tx_hash }
    fn timestamp(&self) -> SystemTime { self.timestamp }
    fn event_type(&self) -> &str { &self.event_type }
    fn raw_data(&self) -> &[u8] { &self.raw_data }
    fn as_any(&self) -> &dyn Any { self }
}

/// Test column family functionality
#[tokio::test]
async fn test_column_families() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = RocksConfig {
        path: temp_dir.path().join("test_cf_db").to_string_lossy().to_string(),
        use_column_families: true,
        ..Default::default()
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Test storing events in different column families
    let event1 = TestEvent {
        id: "test_event_1".to_string(),
        chain: "ethereum".to_string(),
        block_number: 100,
        block_hash: "0x123".to_string(),
        tx_hash: "0x456".to_string(),
        timestamp: SystemTime::now(),
        event_type: "Transfer".to_string(),
        raw_data: vec![1, 2, 3, 4],
    };
    
    let event2 = TestEvent {
        id: "test_event_2".to_string(),
        chain: "polygon".to_string(),
        block_number: 200,
        block_hash: "0x789".to_string(),
        tx_hash: "0xabc".to_string(),
        timestamp: SystemTime::now(),
        event_type: "Approval".to_string(),
        raw_data: vec![5, 6, 7, 8],
    };
    
    // Store events
    storage.store_event(Box::new(event1.clone())).await?;
    storage.store_event(Box::new(event2.clone())).await?;
    
    // Retrieve events
    let ethereum_events = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((50, 150)),
        ..Default::default()
    }]).await?;
    let polygon_events = storage.get_events(vec![EventFilter {
        chain: Some("polygon".to_string()),
        block_range: Some((150, 250)),
        ..Default::default()
    }]).await?;
    
    assert_eq!(ethereum_events.len(), 1);
    assert_eq!(ethereum_events[0].id(), "test_event_1");
    assert_eq!(ethereum_events[0].chain(), "ethereum");
    
    assert_eq!(polygon_events.len(), 1);
    assert_eq!(polygon_events[0].id(), "test_event_2");
    assert_eq!(polygon_events[0].chain(), "polygon");
    
    Ok(())
}

/// Test write batch functionality
#[tokio::test]
async fn test_write_batches() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = RocksConfig {
        path: temp_dir.path().join("test_batch_db").to_string_lossy().to_string(),
        use_column_families: true,
        ..Default::default()
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Create multiple events to test batch writing
    let events: Vec<TestEvent> = (0..100).map(|i| TestEvent {
        id: format!("batch_event_{}", i),
        chain: "ethereum".to_string(),
        block_number: 1000 + i,
        block_hash: format!("0x{:x}", i),
        tx_hash: format!("0x{:x}", i + 1000),
        timestamp: SystemTime::now(),
        event_type: "BatchTest".to_string(),
        raw_data: vec![i as u8; 10],
    }).collect();
    
    // Store all events (should use batch writing internally)
    for event in &events {
        storage.store_event(Box::new(event.clone())).await?;
    }
    
    // Retrieve all events
    let retrieved_events = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((1000, 1099)),
        ..Default::default()
    }]).await?;
    
    assert_eq!(retrieved_events.len(), 100);
    
    // Verify all events are correctly stored
    for (i, event) in retrieved_events.iter().enumerate() {
        assert_eq!(event.id(), format!("batch_event_{}", i));
        assert_eq!(event.block_number(), 1000 + i as u64);
        assert_eq!(event.event_type(), "BatchTest");
    }
    
    Ok(())
}

/// Test iterator functionality and range queries
#[tokio::test]
async fn test_iterator_range_queries() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = RocksConfig {
        path: temp_dir.path().join("test_iterator_db").to_string_lossy().to_string(),
        use_column_families: true,
        ..Default::default()
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Create events with different block numbers
    let events: Vec<TestEvent> = vec![
        TestEvent {
            id: "event_1".to_string(),
            chain: "ethereum".to_string(),
            block_number: 1000,
            block_hash: "0x1000".to_string(),
            tx_hash: "0x1001".to_string(),
            timestamp: SystemTime::now(),
            event_type: "Transfer".to_string(),
            raw_data: vec![1, 2, 3],
        },
        TestEvent {
            id: "event_2".to_string(),
            chain: "ethereum".to_string(),
            block_number: 1005,
            block_hash: "0x1005".to_string(),
            tx_hash: "0x1006".to_string(),
            timestamp: SystemTime::now(),
            event_type: "Approval".to_string(),
            raw_data: vec![4, 5, 6],
        },
        TestEvent {
            id: "event_3".to_string(),
            chain: "ethereum".to_string(),
            block_number: 1010,
            block_hash: "0x1010".to_string(),
            tx_hash: "0x1011".to_string(),
            timestamp: SystemTime::now(),
            event_type: "Transfer".to_string(),
            raw_data: vec![7, 8, 9],
        },
        TestEvent {
            id: "event_4".to_string(),
            chain: "polygon".to_string(),
            block_number: 1007,
            block_hash: "0x1007".to_string(),
            tx_hash: "0x1008".to_string(),
            timestamp: SystemTime::now(),
            event_type: "Swap".to_string(),
            raw_data: vec![10, 11, 12],
        },
    ];
    
    // Store all events
    for event in &events {
        storage.store_event(Box::new(event.clone())).await?;
    }
    
    // Test range queries
    let ethereum_range = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((1000, 1007)),
        ..Default::default()
    }]).await?;
    assert_eq!(ethereum_range.len(), 2); // Should get events 1 and 2
    
    let ethereum_all = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((1000, 1020)),
        ..Default::default()
    }]).await?;
    assert_eq!(ethereum_all.len(), 3); // Should get events 1, 2, and 3
    
    let polygon_range = storage.get_events(vec![EventFilter {
        chain: Some("polygon".to_string()),
        block_range: Some((1000, 1010)),
        ..Default::default()
    }]).await?;
    assert_eq!(polygon_range.len(), 1); // Should get event 4
    
    let no_events = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((2000, 2100)),
        ..Default::default()
    }]).await?;
    assert_eq!(no_events.len(), 0); // Should get no events
    
    Ok(())
}

/// Test compression and advanced features
#[tokio::test]
async fn test_compression_features() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = RocksConfig {
        path: temp_dir.path().join("test_compression_db").to_string_lossy().to_string(),
        use_column_families: true,
        ..Default::default()
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Create a large event with compressible data
    let large_data = vec![42u8; 10000]; // 10KB of repeating data
    let large_event = TestEvent {
        id: "large_event".to_string(),
        chain: "ethereum".to_string(),
        block_number: 1000,
        block_hash: "0x1000".to_string(),
        tx_hash: "0x1001".to_string(),
        timestamp: SystemTime::now(),
        event_type: "LargeTransfer".to_string(),
        raw_data: large_data.clone(),
    };
    
    // Store the large event
    storage.store_event(Box::new(large_event.clone())).await?;
    
    // Retrieve the event and verify data integrity
    let retrieved_events = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((1000, 1000)),
        ..Default::default()
    }]).await?;
    assert_eq!(retrieved_events.len(), 1);
    
    let retrieved_event = &retrieved_events[0];
    assert_eq!(retrieved_event.id(), "large_event");
    assert_eq!(retrieved_event.raw_data(), &large_data);
    
    Ok(())
}

/// Test maintenance operations
#[tokio::test]
async fn test_maintenance_operations() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = RocksConfig {
        path: temp_dir.path().join("test_maintenance_db").to_string_lossy().to_string(),
        use_column_families: true,
        ..Default::default()
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Store some events
    let events: Vec<TestEvent> = (0..50).map(|i| TestEvent {
        id: format!("maintenance_event_{}", i),
        chain: "ethereum".to_string(),
        block_number: 1000 + i,
        block_hash: format!("0x{:x}", i),
        tx_hash: format!("0x{:x}", i + 1000),
        timestamp: SystemTime::now(),
        event_type: "MaintenanceTest".to_string(),
        raw_data: vec![i as u8; 100],
    }).collect();
    
    for event in &events {
        storage.store_event(Box::new(event.clone())).await?;
    }
    
    // Perform maintenance (should not fail)
    storage.maintenance()?;
    
    // Verify data is still accessible after maintenance
    let retrieved_events = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((1000, 1049)),
        ..Default::default()
    }]).await?;
    assert_eq!(retrieved_events.len(), 50);
    
    Ok(())
}

/// Test backwards compatibility with simple mode
#[tokio::test]
async fn test_backwards_compatibility() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = RocksConfig {
        path: temp_dir.path().join("test_simple_db").to_string_lossy().to_string(),
        use_column_families: false, // Simple mode
        ..Default::default()
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Test that basic functionality still works in simple mode
    let event = TestEvent {
        id: "simple_event".to_string(),
        chain: "ethereum".to_string(),
        block_number: 1000,
        block_hash: "0x1000".to_string(),
        tx_hash: "0x1001".to_string(),
        timestamp: SystemTime::now(),
        event_type: "SimpleTest".to_string(),
        raw_data: vec![1, 2, 3, 4],
    };
    
    storage.store_event(Box::new(event.clone())).await?;
    
    let retrieved_events = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((1000, 1000)),
        ..Default::default()
    }]).await?;
    assert_eq!(retrieved_events.len(), 1);
    assert_eq!(retrieved_events[0].id(), "simple_event");
    
    Ok(())
}

/// Test performance with large datasets
#[tokio::test]
async fn test_large_dataset_performance() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = RocksConfig {
        path: temp_dir.path().join("test_performance_db").to_string_lossy().to_string(),
        use_column_families: true,
        write_buffer_size_mb: 128,
        ..Default::default()
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Create a large number of events
    let num_events = 1000;
    let events: Vec<TestEvent> = (0..num_events).map(|i| TestEvent {
        id: format!("perf_event_{}", i),
        chain: "ethereum".to_string(),
        block_number: 1000 + i,
        block_hash: format!("0x{:x}", i),
        tx_hash: format!("0x{:x}", i + 1000),
        timestamp: SystemTime::now(),
        event_type: "PerformanceTest".to_string(),
        raw_data: vec![i as u8; 1000], // 1KB per event
    }).collect();
    
    // Time the storage operations
    let start = std::time::Instant::now();
    for event in &events {
        storage.store_event(Box::new(event.clone())).await?;
    }
    let store_duration = start.elapsed();
    
    // Time the retrieval operations
    let start = std::time::Instant::now();
    let retrieved_events = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((1000, 1000 + num_events - 1)),
        ..Default::default()
    }]).await?;
    let retrieve_duration = start.elapsed();
    
    assert_eq!(retrieved_events.len(), num_events as usize);
    
    // Basic performance assertions (should be reasonably fast)
    assert!(store_duration.as_secs() < 10, "Storage took too long: {:?}", store_duration);
    assert!(retrieve_duration.as_secs() < 2, "Retrieval took too long: {:?}", retrieve_duration);
    
    println!("Performance test results:");
    println!("  Stored {} events in {:?}", num_events, store_duration);
    println!("  Retrieved {} events in {:?}", num_events, retrieve_duration);
    
    Ok(())
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = RocksConfig {
        path: temp_dir.path().join("test_error_db").to_string_lossy().to_string(),
        use_column_families: true,
        ..Default::default()
    };
    
    let storage = RocksStorage::new(config)?;
    
    // Test querying non-existent chain
    let events = storage.get_events(vec![EventFilter {
        chain: Some("nonexistent".to_string()),
        block_range: Some((1000, 2000)),
        ..Default::default()
    }]).await?;
    assert_eq!(events.len(), 0);
    
    // Test querying with invalid block range
    let events = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((2000, 1000)),
        ..Default::default()
    }]).await?;
    assert_eq!(events.len(), 0);
    
    // Test getting latest block for non-existent chain
    let latest_block = storage.get_latest_block("nonexistent").await?;
    assert_eq!(latest_block, 0);
    
    Ok(())
}

/// Test concurrent access
#[tokio::test]
async fn test_concurrent_access() -> Result<()> {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = RocksConfig {
        path: temp_dir.path().join("test_concurrent_db").to_string_lossy().to_string(),
        use_column_families: true,
        ..Default::default()
    };
    
    let storage = Arc::new(RocksStorage::new(config)?);
    
    // Create multiple tasks that write events concurrently
    let mut tasks = Vec::new();
    for task_id in 0..10 {
        let storage_clone = storage.clone();
        let task = tokio::spawn(async move {
            for i in 0..50 {
                let event = TestEvent {
                    id: format!("concurrent_event_{}_{}", task_id, i),
                    chain: "ethereum".to_string(),
                    block_number: 1000 + (task_id * 50) + i,
                    block_hash: format!("0x{:x}", task_id * 50 + i),
                    tx_hash: format!("0x{:x}", task_id * 50 + i + 1000),
                    timestamp: SystemTime::now(),
                    event_type: "ConcurrentTest".to_string(),
                    raw_data: vec![task_id as u8, i as u8],
                };
                storage_clone.store_event(Box::new(event)).await.unwrap();
            }
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    for task in tasks {
        task.await.unwrap();
    }
    
    // Verify all events were stored
    let events = storage.get_events(vec![EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((1000, 1499)),
        ..Default::default()
    }]).await?;
    assert_eq!(events.len(), 500); // 10 tasks * 50 events each
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Helper to create a test storage instance
    fn create_test_storage() -> Result<RocksStorage> {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = RocksConfig {
            path: temp_dir.path().join("test_db").to_string_lossy().to_string(),
            use_column_families: true,
            ..Default::default()
        };
        RocksStorage::new(config)
    }
    
    #[test]
    fn test_config_defaults() {
        let config = RocksConfig::default();
        assert_eq!(config.path, "data/rocksdb");
        assert_eq!(config.create_if_missing, true);
        assert_eq!(config.cache_size_mb, 128);
        assert_eq!(config.write_buffer_size_mb, 64);
        assert_eq!(config.max_write_buffer_number, 3);
        assert_eq!(config.block_size_kb, 4);
        assert_eq!(config.use_column_families, true);
    }
    
    #[test]
    fn test_storage_creation() -> Result<()> {
        let storage = create_test_storage()?;
        assert!(storage.maintenance().is_ok());
        Ok(())
    }
} 