# almanac-storage

Storage layer for the Almanac multi-chain blockchain indexing system with FoundationDB backend.

## Overview

This crate provides the storage implementation for Almanac, featuring a FoundationDB-based backend with tuple-based encoding for efficient multi-chain event storage. It includes both production-ready FoundationDB storage and in-memory storage for testing.

## Key Components

### Storage Backends
- `FdbStorage` - FoundationDB-based storage with tuple encoding
- `MemoryStorage` - In-memory storage for testing and development
- `StorageBackend` - Enum for selecting storage backend
- Factory function for creating storage instances

### FoundationDB Integration
- `FdbConfig` - Configuration for FoundationDB connections
- `FdbStorageExt` - Extended FoundationDB operations
- Tuple-based key encoding for efficient querying
- Transaction support with ACID guarantees

### Storage Traits
- `Storage` - Core storage trait from almanac-core
- `ExtendedStorageReader` - Extended read operations
- `StorageReader` - API adapter for read operations
- Async trait implementations for all operations

### API Adapter
- `StorageReader` - Adapter for API layer integration
- `ExtendedStorageReader` - Extended query capabilities
- Read-only operations for API consumers
- Optimized query performance

## Usage

### Basic Storage Operations

```rust
use almanac_storage::{create_storage, StorageBackend, FdbConfig};
use almanac_core::{Event, EventFilter, BlockStatus};

// Create FoundationDB storage
let fdb_config = FdbConfig {
    cluster_file: "/etc/foundationdb/fdb.cluster".to_string(),
    api_version: 720,
    transaction_timeout_ms: 5000,
    retry_limit: 3,
    max_connections: 100,
};

let storage = create_storage(StorageBackend::Fdb(fdb_config)).await?;

// Store events
let event = create_test_event();
storage.store_event(Box::new(event)).await?;

// Query events
let filter = EventFilter::builder()
    .chain_id(ChainId::Ethereum)
    .from_block(1000)
    .to_block(2000)
    .limit(100)
    .build();

let events = storage.get_events(&filter).await?;

// Get latest block
let latest_block = storage.get_latest_block("ethereum").await?;

// Track block status
storage.set_block_status("ethereum", 1000, BlockStatus::Finalized).await?;
let status = storage.get_block_status("ethereum", 1000).await?;
```

### Memory Storage for Testing

```rust
use almanac_storage::MemoryStorage;

// Create in-memory storage for testing
let storage = MemoryStorage::new();

// All storage operations work the same way
let event = create_test_event();
storage.store_event(Box::new(event)).await?;

let filter = EventFilter::default();
let events = storage.get_events(&filter).await?;
```

### Configuration

```rust
// FoundationDB configuration
let fdb_config = FdbConfig {
    cluster_file: "/etc/foundationdb/fdb.cluster".to_string(),
    api_version: 720,
    transaction_timeout_ms: 5000,
    retry_limit: 3,
    max_connections: 100,
};

// Create storage with configuration
let storage = create_storage(StorageBackend::Fdb(fdb_config)).await?;

// In-memory storage (no configuration needed)
let memory_storage = create_storage(StorageBackend::Memory).await?;
```

## Features

- **FoundationDB backend**: High-performance, distributed storage
- **Tuple encoding**: Efficient key-value encoding for range queries
- **ACID transactions**: Strong consistency guarantees
- **Multi-chain support**: Separate namespaces for different chains
- **Event filtering**: Rich query capabilities with indexes
- **Block status tracking**: Finality and confirmation tracking
- **Memory storage**: In-memory implementation for testing
- **Async operations**: Full async/await support

## FoundationDB Schema

### Event Storage
```
("events", chain_id, block_number, event_index) -> event_data
("events_by_type", chain_id, event_type, block_number, event_index) -> event_id
("events_by_tx", chain_id, tx_hash, event_index) -> event_id
```

### Block Status
```
("blocks", chain_id, block_number) -> block_status
("latest_block", chain_id) -> block_number
```

### Indexes
```
("index_chain_block", chain_id, block_number) -> [event_ids]
("index_chain_type", chain_id, event_type) -> [event_ids]
("index_chain_tx", chain_id, tx_hash) -> [event_ids]
```

## Performance Characteristics

### FoundationDB Storage
- **Throughput**: 10,000+ events/second per node
- **Latency**: <10ms for single event operations
- **Scalability**: Horizontally scalable across multiple nodes
- **Consistency**: Strong consistency with ACID guarantees
- **Durability**: Persistent storage with replication

### Memory Storage
- **Throughput**: 100,000+ events/second (in-memory)
- **Latency**: <1ms for all operations
- **Scalability**: Single-node only
- **Consistency**: Immediate consistency
- **Durability**: No persistence (lost on restart)

## Query Optimization

### Efficient Queries
```rust
// Use specific filters for better performance
let filter = EventFilter::builder()
    .chain_id(ChainId::Ethereum)          // Indexed
    .event_type("Transfer")               // Indexed
    .from_block(1000)                     // Range query
    .to_block(2000)                       // Range query
    .limit(100)                           // Limit results
    .build();

// Avoid broad queries without filters
let broad_filter = EventFilter::default(); // Less efficient
```

### Batch Operations
```rust
// Process events in batches for better performance
let batch_size = 1000;
let mut offset = 0;

loop {
    let filter = EventFilter::builder()
        .chain_id(ChainId::Ethereum)
        .limit(batch_size)
        .offset(offset)
        .build();
    
    let events = storage.get_events(&filter).await?;
    if events.is_empty() {
        break;
    }
    
    // Process batch
    process_events(events).await?;
    offset += batch_size;
}
```

## Error Handling

```rust
use almanac_core::Result;

// Storage operations return Result<T>
async fn store_event_safely(
    storage: &dyn Storage,
    event: Box<dyn Event>,
) -> Result<()> {
    storage.store_event(event).await.map_err(|e| {
        eprintln!("Failed to store event: {}", e);
        e
    })
}

// Handle specific storage errors
match storage.get_events(&filter).await {
    Ok(events) => process_events(events).await,
    Err(e) => {
        eprintln!("Query failed: {}", e);
        // Handle error or retry
    }
}
```

## Testing

Run tests with:
```bash
cargo test -p almanac-storage
```

For FoundationDB tests:
```bash
# Start FoundationDB locally
fdbserver --config-file /etc/foundationdb/fdb.cluster

# Run tests with FDB feature
cargo test -p almanac-storage --features fdb
```

The test suite covers:
- Memory storage operations
- FoundationDB storage operations (with feature flag)
- Event querying and filtering
- Block status tracking
- Error handling and edge cases
- Performance characteristics

## Configuration Options

### FoundationDB
- `cluster_file`: Path to FDB cluster file
- `api_version`: FDB API version (recommended: 720)
- `transaction_timeout_ms`: Transaction timeout
- `retry_limit`: Number of retry attempts
- `max_connections`: Connection pool size

### Memory Storage
- No configuration required
- Automatically manages memory usage
- Suitable for testing and development

## Deployment Considerations

### FoundationDB Setup
1. Install FoundationDB cluster
2. Configure cluster file
3. Set appropriate API version
4. Configure connection limits
5. Monitor transaction performance

### Memory Storage
- Only for testing/development
- Not suitable for production
- Data lost on restart