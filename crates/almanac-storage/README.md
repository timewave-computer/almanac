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
- Tuple-based key encoding for efficient querying
- Transaction support with ACID guarantees

### Storage Traits
- `StorageWriter` - Trait for write operations (e.g., storing events, updating block status)
- `StorageReader` - Trait for read operations (e.g., querying events, retrieving latest blocks)
- `Storage` - Convenience trait for types implementing both `StorageWriter` and `StorageReader`
- Async trait implementations for all operations

## Usage

### Basic Storage Operations

```rust
use almanac_storage::{create_storage, StorageBackend, FdbConfig, MemoryStorage};
use almanac_core::{Event, EventFilter, BlockStatus, UnifiedEvent, ChainId};
use std::time::SystemTime;

// Helper to create a test event
fn create_test_event() -> UnifiedEvent {
    UnifiedEvent {
        id: "test_event_1".to_string(),
        chain: ChainId::Ethereum.to_string(),
        block_number: 1000,
        block_hash: "0xabc".to_string(),
        tx_hash: "0xdef".to_string(),
        timestamp: SystemTime::now(),
        event_type: "Transfer".to_string(),
        contract_address: None,
        raw_data: vec![1, 2, 3],
        attributes: std::collections::HashMap::new(),
        source: almanac_core::EventSource::Chain,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create FoundationDB storage
    let fdb_config = FdbConfig {
        cluster_file: None, // Use default cluster file
        namespace: "almanac_example".to_string(),
        connection_timeout: 30,
        max_retries: 3,
    };

    let storage = create_storage(StorageBackend::Fdb(fdb_config)).await?;

    // Store events
    let event = create_test_event();
    storage.store_event(Box::new(event)).await?;

    // Query events
    let filter = EventFilter {
        chain: Some(ChainId::Ethereum.to_string()),
        block_range: Some((1000, 2000)),
        limit: Some(100),
        offset: Some(0),
        ..Default::default()
    };

    let events = storage.get_events(vec![filter]).await?;

    // Get latest block
    let latest_block = storage.get_latest_block(ChainId::Ethereum.as_str()).await?;

    // Update block status
    storage.update_block_status(ChainId::Ethereum.as_str(), 1000, BlockStatus::Finalized).await?;
    let latest_finalized_block = storage.get_latest_block_with_status(ChainId::Ethereum.as_str(), BlockStatus::Finalized).await?;

    // Memory Storage for Testing
    let memory_storage = MemoryStorage::new();
    let event = create_test_event();
    memory_storage.store_event(Box::new(event)).await?;

    let filter = EventFilter::default();
    let events = memory_storage.get_events(vec![filter]).await?;

    Ok(())
}
```

### Configuration

```rust
// FoundationDB configuration
let fdb_config = FdbConfig {
    cluster_file: Some("/etc/foundationdb/fdb.cluster".to_string()),
    namespace: "almanac".to_string(),
    connection_timeout: 30,
    max_retries: 3,
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

### Keyspaces
- `ContractState` (1): Contract storage slots
- `BlockMeta` (2): Block headers and consensus data
- `StorageProofs` (3): Merkle proofs for verification
- `SemanticMeta` (4): Trust-scoped semantic interpretations
- `TrustGraph` (5): Developer trust relationships
- `ContractLayouts` (6): Layout commitments from Traverse
- `ViewRegistry` (7): Custom view definitions
- `LightwaveState` (8): Recursive proof coordination
- `TrustedCheckpoints` (9): Network checkpoints
- `SolanaAccounts` (10): Solana account data
- `NetworkConfig` (11): Blockchain network configuration
- `CircuitKeys` (12): SP1 verification keys
- `ChangeTracking` (13): State change logs
- `ValidatorSignatures` (14): Proposer validator signatures
- `TrustPolicies` (15): Developer trust policies

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
let filter = EventFilter {
    chain: Some(ChainId::Ethereum.to_string()),          // Indexed
    event_type: Some("Transfer".to_string()),               // Indexed
    block_range: Some((1000, 2000)),                     // Range query
    limit: Some(100),                           // Limit results
    offset: Some(0),
    ..Default::default()
};

// Avoid broad queries without filters
let broad_filter = EventFilter::default(); // Less efficient
```

### Batch Operations
```rust
// Process events in batches for better performance
let batch_size = 1000;
let mut offset = 0;

loop {
    let filter = EventFilter {
        chain: Some(ChainId::Ethereum.to_string()),
        limit: Some(batch_size),
        offset: Some(offset),
        ..Default::default()
    };
    
    let events = storage.get_events(vec![filter]).await?;
    if events.is_empty() {
        break;
    }
    
    // Process batch
    // process_events(events).await?;
    offset += batch_size;
}
```

## Error Handling

```rust
use almanac_core::Result;

// Storage operations return Result<T>
async fn store_event_safely(
    storage: &dyn almanac_core::StorageWriter,
    event: Box<dyn Event>,
) -> Result<()> {
    storage.store_event(event).await.map_err(|e| {
        eprintln!("Failed to store event: {}", e);
        e
    })
}

// Handle specific storage errors
match storage.get_events(vec![filter]).await {
    Ok(events) => { /* process events */ },
    Err(e) => {
        eprintln!("Query failed: {}", e);
        // Handle error or retry
    }
}
```

## Testing

Run tests with:
```bash
nix develop -c cargo test -p almanac-storage
```

For FoundationDB tests:
```bash
# Start FoundationDB locally
nix run .#fdb-manage -- start

# Run tests with FDB feature
nix develop -c cargo test -p almanac-storage --features fdb
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
- `namespace`: Namespace for key isolation
- `connection_timeout`: Connection timeout in seconds
- `max_retries`: Number of retry attempts

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