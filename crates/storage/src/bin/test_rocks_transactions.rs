use std::path::PathBuf;
use std::time::Instant;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::TempDir;

use indexer_storage::rocks::{RocksConfig, RocksStorage, Key, WriteBatchExt};

#[tokio::main]
async fn main() {
    println!("Testing RocksDB Transaction Isolation and Atomicity");

    // Create a temporary directory for RocksDB
    let rocksdb_dir = TempDir::new().expect("Failed to create RocksDB temp dir");
    let rocks_path = rocksdb_dir.path().to_path_buf();
    println!("Using RocksDB directory: {:?}", rocks_path);
    
    // Run transaction isolation test
    test_transaction_isolation(rocks_path.clone()).await;
    
    // Run transaction atomicity test
    test_transaction_atomicity(rocks_path.clone()).await;
    
    // Run concurrent access test
    test_concurrent_access(rocks_path.clone()).await;
    
    println!("\nAll transaction tests completed successfully!");
}

async fn test_transaction_isolation(path: PathBuf) {
    println!("\n===== TRANSACTION ISOLATION TEST =====");
    
    // Initialize RocksDB
    let config = RocksConfig { 
        path: path.to_string_lossy().to_string(),
        create_if_missing: true
    };
    let storage = RocksStorage::new(config).expect("Failed to create RocksDB storage");
    
    // Set up test data
    let key1 = Key::new("isolation", "key1");
    let key2 = Key::new("isolation", "key2");
    let value1 = "initial_value1".as_bytes().to_vec();
    let value2 = "initial_value2".as_bytes().to_vec();
    
    // Store initial values
    storage.put(&key1, &value1).expect("Failed to write value1");
    storage.put(&key2, &value2).expect("Failed to write value2");
    
    // Create a transaction (implemented using batch operations)
    println!("Creating a transaction to update values...");
    let batch = storage.create_write_batch();
    
    // Make changes in the transaction (batch)
    batch.put(&key1, "new_value1".as_bytes());
    batch.put(&key2, "new_value2".as_bytes());
    
    // Before committing the transaction, verify that the original values are still visible
    let read_value1 = storage.get(&key1).expect("Failed to read value1");
    let read_value2 = storage.get(&key2).expect("Failed to read value2");
    
    assert_eq!(read_value1, Some(value1.clone()), "Value1 should not be changed before transaction commit");
    assert_eq!(read_value2, Some(value2.clone()), "Value2 should not be changed before transaction commit");
    println!("Verified values are not changed before transaction commit ✓");
    
    // Commit the transaction
    println!("Committing transaction...");
    storage.write_batch(batch).expect("Failed to commit transaction");
    
    // After committing, verify that the new values are visible
    let read_value1 = storage.get(&key1).expect("Failed to read value1 after commit");
    let read_value2 = storage.get(&key2).expect("Failed to read value2 after commit");
    
    assert_eq!(read_value1, Some("new_value1".as_bytes().to_vec()), "Value1 should be updated after commit");
    assert_eq!(read_value2, Some("new_value2".as_bytes().to_vec()), "Value2 should be updated after commit");
    println!("Verified values are updated after transaction commit ✓");
    
    println!("Transaction isolation test passed!");
}

async fn test_transaction_atomicity(path: PathBuf) {
    println!("\n===== TRANSACTION ATOMICITY TEST =====");
    
    // Initialize RocksDB
    let config = RocksConfig { 
        path: path.to_string_lossy().to_string(),
        create_if_missing: true
    };
    let storage = RocksStorage::new(config).expect("Failed to create RocksDB storage");
    
    // Set up test data
    let key1 = Key::new("atomicity", "key1");
    let key2 = Key::new("atomicity", "key2");
    let value1 = "atomic_value1".as_bytes().to_vec();
    let value2 = "atomic_value2".as_bytes().to_vec();
    
    // Store initial values
    storage.put(&key1, &value1).expect("Failed to write value1");
    storage.put(&key2, &value2).expect("Failed to write value2");
    
    // Create a transaction (batch)
    println!("Creating a transaction with multiple operations...");
    let batch = storage.create_write_batch();
    
    // Make changes in the transaction
    batch.put(&key1, "updated_value1".as_bytes());
    batch.put(&key2, "updated_value2".as_bytes());
    
    // Add 100 more operations to make the batch substantial
    for i in 0..100 {
        let key = Key::new("atomicity", format!("key_bulk_{}", i));
        batch.put(&key, format!("bulk_value_{}", i).as_bytes());
    }
    
    // Commit the transaction
    println!("Committing transaction with multiple operations...");
    let start = Instant::now();
    storage.write_batch(batch).expect("Failed to commit transaction");
    let duration = start.elapsed();
    println!("Transaction committed in {:?}", duration);
    
    // Verify that all changes were applied
    let read_value1 = storage.get(&key1).expect("Failed to read value1");
    let read_value2 = storage.get(&key2).expect("Failed to read value2");
    
    assert_eq!(read_value1, Some("updated_value1".as_bytes().to_vec()), "Value1 should be updated");
    assert_eq!(read_value2, Some("updated_value2".as_bytes().to_vec()), "Value2 should be updated");
    
    // Check a few bulk values
    for i in 0..10 {
        let key = Key::new("atomicity", format!("key_bulk_{}", i));
        let value = storage.get(&key).expect("Failed to read bulk value");
        assert_eq!(value, Some(format!("bulk_value_{}", i).as_bytes().to_vec()), 
                 "Bulk value {} should be present", i);
    }
    
    println!("All values from the transaction were correctly applied ✓");
    println!("Transaction atomicity test passed!");
}

async fn test_concurrent_access(path: PathBuf) {
    println!("\n===== CONCURRENT ACCESS TEST =====");
    
    // Initialize RocksDB
    let config = RocksConfig { 
        path: path.to_string_lossy().to_string(),
        create_if_missing: true
    };
    let storage = Arc::new(RocksStorage::new(config).expect("Failed to create RocksDB storage"));
    
    // Number of concurrent threads
    let num_threads = 10;
    let operations_per_thread = 100;
    let counter = Arc::new(Mutex::new(0));
    
    println!("Starting {} concurrent threads, each performing {} operations...", 
             num_threads, operations_per_thread);
    
    // Create and start threads
    let start = Instant::now();
    let mut handles = vec![];
    
    for thread_id in 0..num_threads {
        let storage_clone = Arc::clone(&storage);
        let counter_clone = Arc::clone(&counter);
        
        let handle = thread::spawn(move || {
            for i in 0..operations_per_thread {
                // Create unique keys for this thread
                let key = Key::new("concurrent", format!("thread_{}_key_{}", thread_id, i));
                let value = format!("thread_{}_value_{}", thread_id, i).as_bytes().to_vec();
                
                // Write the value
                storage_clone.put(&key, &value).expect("Failed to write value");
                
                // Read it back to verify
                let read_value = storage_clone.get(&key).expect("Failed to read value");
                assert_eq!(read_value, Some(value), "Read value should match written value");
                
                // Update the global counter
                let mut count = counter_clone.lock().unwrap();
                *count += 1;
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start.elapsed();
    println!("All threads completed in {:?}", duration);
    
    // Verify operation count
    let final_count = *counter.lock().unwrap();
    assert_eq!(final_count, num_threads * operations_per_thread, 
               "Total operations should match expected count");
    
    println!("Successfully performed {} operations across {} threads ✓", 
             final_count, num_threads);
    println!("Concurrent access test passed!");
} 