use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;

use indexer_storage::rocks::{RocksConfig, RocksStorage, Key};

#[tokio::main]
async fn main() {
    println!("Running Storage Benchmarks");

    // Create a temporary directory for RocksDB
    let rocksdb_dir = TempDir::new().expect("Failed to create RocksDB temp dir");
    let rocks_path = rocksdb_dir.path().to_path_buf();
    
    // Create a temporary directory for file system 
    let fs_dir = TempDir::new().expect("Failed to create filesystem temp dir");
    let fs_path = fs_dir.path().to_path_buf();
    
    println!("Using RocksDB directory: {:?}", rocks_path);
    println!("Using Filesystem directory: {:?}", fs_path);
    
    // Run RocksDB performance test
    println!("\n===== ROCKSDB BENCHMARK =====");
    benchmark_rocksdb_performance(rocks_path.clone()).await;
    
    // Run filesystem performance test
    println!("\n===== FILESYSTEM BENCHMARK =====");
    benchmark_filesystem_performance(fs_path.clone()).await;
}

async fn benchmark_rocksdb_performance(path: PathBuf) {
    println!("Benchmarking RocksDB write performance...");
    
    // Test parameters
    let num_events = 10_000;
    
    // Initialize RocksDB
    let config = RocksConfig { 
        path: path.to_string_lossy().to_string(),
        create_if_missing: true
    };
    let storage = RocksStorage::new(config).expect("Failed to create RocksDB storage");
    
    // Prepare key-value pairs
    let mut kvs = Vec::new();
    for i in 0..num_events {
        let key = Key::new("benchmark", format!("key_{}", i));
        let value = format!("value_{}", i).into_bytes();
        kvs.push((key, value));
    }
    
    // Measure write time for individual writes
    println!("Writing {} key-value pairs to RocksDB...", num_events);
    let start = Instant::now();
    
    for (key, value) in kvs {
        storage.put(&key, &value).expect("Failed to write value");
    }
    
    let duration = start.elapsed();
    println!("RocksDB write completed in {:?}", duration);
    println!("Average time per write: {:?}", duration / num_events as u32);
    
    // Read keys
    println!("\nReading {} key-value pairs from RocksDB...", num_events);
    let start = Instant::now();
    
    for i in 0..num_events {
        let key = Key::new("benchmark", format!("key_{}", i));
        let _value = storage.get(&key).expect("Failed to read value");
    }
    
    let duration = start.elapsed();
    println!("RocksDB read completed in {:?}", duration);
    println!("Average time per read: {:?}", duration / num_events as u32);
}

async fn benchmark_filesystem_performance(path: PathBuf) {
    println!("Benchmarking filesystem write performance...");
    
    // Test parameters
    let num_events = 10_000;
    
    // Prepare test data
    let mut data = Vec::new();
    for i in 0..num_events {
        let value = format!("value_{}", i);
        data.push((format!("key_{}", i), value));
    }
    
    // Measure write time
    println!("Writing {} files to filesystem...", num_events);
    let start = Instant::now();
    
    for (key, value) in &data {
        let file_path = path.join(key);
        let mut file = File::create(file_path).expect("Failed to create file");
        file.write_all(value.as_bytes()).expect("Failed to write data");
    }
    
    let duration = start.elapsed();
    println!("Filesystem write completed in {:?}", duration);
    println!("Average time per write: {:?}", duration / num_events as u32);
    
    // Measure read time
    println!("\nReading {} files from filesystem...", num_events);
    let start = Instant::now();
    
    for (key, _) in &data {
        let file_path = path.join(key);
        let _content = fs::read_to_string(file_path).expect("Failed to read file");
    }
    
    let duration = start.elapsed();
    println!("Filesystem read completed in {:?}", duration);
    println!("Average time per read: {:?}", duration / num_events as u32);
} 