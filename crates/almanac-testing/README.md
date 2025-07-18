# almanac-testing

Testing utilities and fixtures for the Almanac multi-chain blockchain indexing system.

## Overview

This crate provides testing utilities, fixtures, and helpers for testing Almanac components. It includes mock implementations, test data generators, and common testing patterns to facilitate comprehensive testing across the entire system.

## Key Components

### Test Fixtures
- `fixtures` - Common test data and mock objects
- Pre-built test events, blocks, and chain data
- Configurable test scenarios
- Realistic blockchain data for testing

### Test Helpers
- `helpers` - Utility functions for testing
- Async testing helpers
- Setup and teardown utilities
- Assertion helpers for complex data structures

## Usage

```rust
use almanac_testing::{fixtures, helpers};
use almanac_core::{Event, ChainId, BlockStatus};

#[tokio::test]
async fn test_event_processing() {
    // Create test fixtures
    let test_events = fixtures::create_test_events(10);
    let test_blocks = fixtures::create_test_blocks(5);
    
    // Use helpers for setup
    let storage = helpers::create_test_storage().await;
    let pipeline = helpers::create_test_pipeline().await;
    
    // Test your code
    for event in test_events {
        storage.store_event(Box::new(event)).await.unwrap();
    }
    
    // Use helpers for assertions
    helpers::assert_event_count(&storage, 10).await;
    helpers::assert_block_status(&storage, "ethereum", 1000, BlockStatus::Finalized).await;
}
```

## Test Fixtures

### Event Fixtures
```rust
use almanac_testing::fixtures;

// Create test events
let events = fixtures::create_test_events(100);
let ethereum_events = fixtures::create_ethereum_events(50);
let cosmos_events = fixtures::create_cosmos_events(30);

// Create specific event types
let transfer_event = fixtures::create_transfer_event(
    "ethereum",
    1000,
    "0x1234...5678",
    "0x8765...4321",
    "1000000000000000000"
);

let contract_call_event = fixtures::create_contract_call_event(
    "ethereum",
    1001,
    "0xcontract",
    "transfer",
    serde_json::json!({"to": "0x1234", "value": "1000"})
);
```

### Block Fixtures
```rust
// Create test blocks
let blocks = fixtures::create_test_blocks(10);
let ethereum_blocks = fixtures::create_ethereum_blocks(5);

// Create blocks with specific properties
let finalized_block = fixtures::create_finalized_block("ethereum", 1000);
let pending_block = fixtures::create_pending_block("ethereum", 1001);
```

### Chain Data Fixtures
```rust
// Create chain-specific test data
let eth_chain_data = fixtures::create_ethereum_chain_data();
let cosmos_chain_data = fixtures::create_cosmos_chain_data();
let solana_chain_data = fixtures::create_solana_chain_data();
```

## Test Helpers

### Storage Helpers
```rust
use almanac_testing::helpers;

// Create test storage instances
let memory_storage = helpers::create_memory_storage().await;
let fdb_storage = helpers::create_test_fdb_storage().await;

// Storage assertions
helpers::assert_event_count(&storage, 100).await;
helpers::assert_latest_block(&storage, "ethereum", 1000).await;
helpers::assert_block_status(&storage, "ethereum", 1000, BlockStatus::Safe).await;
```

### Pipeline Helpers
```rust
// Create test pipelines
let pipeline = helpers::create_test_pipeline().await;
let ethereum_pipeline = helpers::create_ethereum_pipeline().await;

// Pipeline testing
helpers::start_test_pipeline(&pipeline).await;
helpers::stop_test_pipeline(&pipeline).await;
helpers::assert_pipeline_processed(&pipeline, 100).await;
```

### Event Bus Helpers
```rust
// Create test event bus
let event_bus = helpers::create_test_event_bus().await;

// Event bus testing
helpers::publish_test_event(&event_bus, test_event).await;
helpers::assert_event_received(&event_bus, "BlockProcessed").await;
```

### Consensus Helpers
```rust
// Create test consensus components
let reorg_detector = helpers::create_test_reorg_detector().await;
let finality_processor = helpers::create_test_finality_processor().await;

// Consensus testing
helpers::trigger_test_reorg(&reorg_detector, 3).await;
helpers::assert_finality_level(&finality_processor, 1000, FinalityLevel::Finalized).await;
```

## Mock Implementations

### Mock Chain Adapter
```rust
use almanac_testing::mocks::MockChainAdapter;

let mock_adapter = MockChainAdapter::new()
    .with_chain_id("test-chain")
    .with_latest_block(1000)
    .with_block_data(block_data)
    .with_finality_confirmations(12);

// Use in tests
let latest = mock_adapter.get_latest_block().await?;
assert_eq!(latest, 1000);
```

### Mock Storage
```rust
use almanac_testing::mocks::MockStorage;

let mock_storage = MockStorage::new()
    .with_events(test_events)
    .with_latest_block("ethereum", 1000)
    .with_block_status("ethereum", 1000, BlockStatus::Finalized);

// Use in tests
let events = mock_storage.get_events(&filter).await?;
assert_eq!(events.len(), 10);
```

## Testing Patterns

### Integration Testing
```rust
#[tokio::test]
async fn test_full_indexing_pipeline() {
    // Setup
    let storage = helpers::create_test_storage().await;
    let event_bus = helpers::create_test_event_bus().await;
    let pipeline = helpers::create_test_pipeline().await;
    
    // Test data
    let test_events = fixtures::create_test_events(100);
    
    // Execute
    for event in test_events {
        pipeline.process_event(event).await.unwrap();
    }
    
    // Verify
    helpers::assert_event_count(&storage, 100).await;
    helpers::assert_pipeline_processed(&pipeline, 100).await;
}
```

### Property-Based Testing
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_event_storage_properties(
        events in prop::collection::vec(fixtures::arbitrary_event(), 1..100)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = helpers::create_test_storage().await;
            
            for event in &events {
                storage.store_event(Box::new(event.clone())).await.unwrap();
            }
            
            let filter = EventFilter::default();
            let stored_events = storage.get_events(&filter).await.unwrap();
            assert_eq!(stored_events.len(), events.len());
        });
    }
}
```

### Performance Testing
```rust
#[tokio::test]
async fn test_storage_performance() {
    let storage = helpers::create_test_storage().await;
    let events = fixtures::create_test_events(10000);
    
    let start = std::time::Instant::now();
    
    for event in events {
        storage.store_event(Box::new(event)).await.unwrap();
    }
    
    let duration = start.elapsed();
    assert!(duration.as_secs() < 10, "Storage took too long: {:?}", duration);
}
```

## Features

- **Comprehensive fixtures**: Pre-built test data for all components
- **Mock implementations**: Lightweight mocks for testing
- **Async helpers**: Utilities for async testing
- **Property-based testing**: Support for property-based test generation
- **Performance testing**: Utilities for performance benchmarking
- **Integration helpers**: End-to-end testing utilities

## Testing Best Practices

### Test Organization
- Use fixtures for consistent test data
- Group related tests in modules
- Use helpers for common setup/teardown
- Separate unit tests from integration tests

### Mock Usage
- Use mocks for external dependencies
- Keep mocks simple and focused
- Verify mock interactions
- Reset mocks between tests

### Async Testing
- Use `tokio::test` for async tests
- Handle async errors properly
- Use timeouts for long-running tests
- Clean up resources after tests

## Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p almanac-testing

# Run tests with specific features
cargo test --features fdb

# Run ignored tests (integration tests)
cargo test -- --ignored

# Run tests with output
cargo test -- --nocapture
```

## Test Coverage

The testing utilities support comprehensive test coverage:

- Unit tests for individual components
- Integration tests for component interactions
- End-to-end tests for full workflows
- Performance tests for benchmarking
- Property-based tests for edge cases