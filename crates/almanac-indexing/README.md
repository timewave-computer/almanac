# almanac-indexing

Event indexing service for the Almanac multi-chain blockchain indexing system.

## Overview

This crate provides the core event indexing functionality, processing blockchain events from multiple chains and storing them in a normalized format. It implements a configurable pipeline for batch processing, event ordering, validation, and finality tracking.

## Key Components

### Indexing Service
- `IndexingService` - Main indexing orchestrator
- `IndexingConfig` - Configuration for batch sizes, intervals, and chain-specific settings
- Configurable processing intervals and batch sizes
- Chain-specific optimization parameters

### Batch Processing
- `BatchProcessor` - Processes events in configurable batches
- Efficient batching for high-throughput chains
- Configurable batch sizes and processing intervals
- Error handling and retry logic

### Event Ordering
- `EventOrderer` - Ensures correct event ordering across chains
- Block-based ordering for consistency
- Cross-chain event correlation
- Timestamp-based ordering fallback

### Validation
- `EventValidator` - Validates event data integrity
- Schema validation for event structures
- Chain-specific validation rules
- Data consistency checks

### Finality Processing
- `FinalityProcessor` - Tracks event finality across chains
- Chain-specific finality rules
- Confirmation-based finality tracking
- Reorg-safe event processing

### Ingestion Queue
- `IngestionQueue` - Manages event ingestion from multiple sources
- Backpressure handling for high-volume chains
- Priority-based event processing
- Rate limiting and throttling

## Usage

```rust
use almanac_indexing::{IndexingService, IndexingConfig};
use almanac_core::Storage;
use almanac_chains::ChainAdapter;
use almanac_consensus::{create_fork_choice, ForkChoiceRule};
use std::sync::Arc;

// Assume you have a ChainAdapter and Storage implementation
// let chain_adapter: Arc<dyn ChainAdapter> = ...;
// let storage: Arc<dyn Storage> = ...;

// Configure the indexing service
let config = IndexingConfig {
    chain_id: "ethereum".to_string(),
    batch_size: 100,
    max_concurrent_batches: 10,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    finality_confirmations: 12,
    enable_validation: true,
    enable_ordering: true,
    checkpoint_interval: 1000,
    start_block: 0,
};

// Create a fork choice rule (e.g., LongestChain)
let fork_choice = create_fork_choice(ForkChoiceRule::LongestChain, /* initial_block_info */);

// Create and run the indexing service
let indexer = IndexingService::builder()
    .chain_adapter(chain_adapter)
    .storage(storage)
    .fork_choice(fork_choice)
    .config(config)
    .build();

indexer.run().await?;

// The service will automatically:
// 1. Ingest events from blockchain adapters
// 2. Validate event data
// 3. Order events correctly
// 4. Process events in batches
// 5. Track finality status
// 6. Handle reorganizations
```

## Configuration

### Chain-Specific Settings

```rust
// Ethereum configuration (higher latency, more confirmations)
let eth_config = IndexingConfig {
    chain_id: "ethereum".to_string(),
    batch_size: 100,
    max_concurrent_batches: 10,
    retry_attempts: 3,
    retry_delay_ms: 1000,
    finality_confirmations: 12,
    enable_validation: true,
    enable_ordering: true,
    checkpoint_interval: 1000,
    start_block: 0,
};

// Cosmos configuration (instant finality, higher throughput)
let cosmos_config = IndexingConfig {
    chain_id: "cosmos".to_string(),
    batch_size: 200,
    max_concurrent_batches: 20,
    retry_attempts: 5,
    retry_delay_ms: 500,
    finality_confirmations: 1,
    enable_validation: true,
    enable_ordering: true,
    checkpoint_interval: 100,
    start_block: 0,
};

// Solana configuration (high throughput, slot-based finality)
let solana_config = IndexingConfig {
    chain_id: "solana".to_string(),
    batch_size: 1000,
    max_concurrent_batches: 50,
    retry_attempts: 2,
    retry_delay_ms: 200,
    finality_confirmations: 32,
    enable_validation: true,
    enable_ordering: true,
    checkpoint_interval: 2000,
    start_block: 0,
};
```

### Performance Tuning

```rust
// High-performance configuration
let high_perf_config = IndexingConfig {
    chain_id: "ethereum".to_string(),
    batch_size: 1000,
    max_concurrent_batches: 100,
    retry_attempts: 2,
    retry_delay_ms: 100,
    finality_confirmations: 6,
    enable_validation: false, // Disable for performance
    enable_ordering: false,   // Disable for performance
    checkpoint_interval: 10000,
    start_block: 0,
};

// Low-latency configuration
let low_latency_config = IndexingConfig {
    chain_id: "ethereum".to_string(),
    batch_size: 10,
    max_concurrent_batches: 2,
    retry_attempts: 1,
    retry_delay_ms: 50,
    finality_confirmations: 1,
    enable_validation: true,
    enable_ordering: true,
    checkpoint_interval: 10,
    start_block: 0,
};
```

## Features

- **Multi-chain support**: Configurable for different blockchain networks
- **Batch processing**: Efficient batch processing with configurable sizes
- **Event ordering**: Maintains correct event ordering across chains
- **Validation**: Comprehensive event validation and integrity checking
- **Finality tracking**: Chain-specific finality detection and handling
- **Error recovery**: Robust error handling and retry mechanisms
- **Backpressure handling**: Manages high-volume event streams
- **Checkpointing**: Regular state checkpointing for recovery

## Pipeline Stages

### 1. Ingestion
- Receives events from blockchain adapters
- Manages ingestion queues with backpressure
- Applies rate limiting and throttling

### 2. Validation
- Validates event schema and data integrity
- Applies chain-specific validation rules
- Filters out invalid or duplicate events

### 3. Ordering
- Ensures correct event ordering by block number
- Handles cross-chain event correlation
- Maintains temporal consistency

### 4. Batch Processing
- Groups events into configurable batches
- Processes batches concurrently
- Handles batch failures and retries

### 5. Finality Tracking
- Tracks event finality status
- Handles chain reorganizations
- Updates finality levels based on confirmations

### 6. Storage
- Persists processed events to storage
- Maintains event indexes
- Handles storage failures and recovery

## Error Handling

The pipeline provides comprehensive error handling:

```rust
// Indexer errors are logged and handled gracefully
indexer.run().await.map_err(|e| {
    eprintln!("Indexer error: {}", e);
    e
})?;

// Individual event errors don't stop the the indexer
// Failed events are retried with exponential backoff
// Persistent failures are logged and skipped
```

## Testing

Run tests with:
```bash
nix develop -c cargo test -p almanac-indexing
```

The test suite covers:
- Pipeline configuration and creation
- Batch processing logic
- Event ordering algorithms
- Validation rules and error handling
- Finality tracking mechanisms
- Performance characteristics

## Performance Considerations

- **Configurable batch sizes**: Optimize for throughput vs latency
- **Concurrent processing**: Multiple batches processed in parallel
- **Memory efficiency**: Streaming processing to minimize memory usage
- **Checkpoint frequency**: Balance between performance and recovery time
- **Validation overhead**: Can be disabled for performance-critical scenarios

## Monitoring

The pipeline provides metrics for monitoring:

- Events processed per second
- Batch processing latency
- Validation success/failure rates
- Finality confirmation times
- Error rates and retry statistics
- Queue depths and backpressure indicators