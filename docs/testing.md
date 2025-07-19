# Testing Guide

This guide covers testing strategies, tools, and best practices for the Almanac system.

## Testing Philosophy

Almanac follows a comprehensive testing approach:
- **Unit tests** for individual components
- **Integration tests** for cross-crate functionality
- **End-to-end tests** for full system validation
- **Property-based tests** for complex algorithms
- **Performance tests** for benchmarking

## Test Organization

### Unit Tests
Located alongside source code in each crate:
```
crates/almanac-core/
├── src/
│   ├── lib.rs
│   ├── error.rs
│   └── event.rs
└── tests/
    └── unit_tests.rs
```

### Integration Tests
Located in crate-level `tests/` directories:
```
crates/almanac-api/tests/
├── api_integration.rs
├── auth_tests.rs
└── websocket_tests.rs
```

### End-to-End Tests
Located in the `e2e/` directory:
```
e2e/
├── Cargo.toml
└── src/
    └── tests/
        ├── ethereum_indexing.rs
        └── cosmos_indexing.rs
```

## Running Tests

### All Tests
```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run in parallel
cargo test -- --test-threads=4
```

### Crate-Specific Tests
```bash
# Test specific crate
cargo test -p almanac-core
cargo test -p almanac-consensus

# Test with features
cargo test -p almanac-chains --features ethereum
```

### Integration Tests Only
```bash
# Run integration tests
cargo test --test '*'

# Specific integration test
cargo test --test api_integration
```

### Documentation Tests
```bash
# Run doc tests
cargo test --doc

# Specific crate doc tests
cargo test --doc -p almanac-core
```

## Writing Tests

### Unit Test Example
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_creation() {
        let event = Event {
            id: "test_id".to_string(),
            chain_id: "ethereum".to_string(),
            block_number: 12345,
            data: vec![],
        };
        
        assert_eq!(event.chain_id, "ethereum");
        assert_eq!(event.block_number, 12345);
    }
    
    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Test Example
```rust
// tests/storage_integration.rs
use almanac_storage::{Storage, FoundationDBBackend};
use almanac_core::Event;

#[tokio::test]
async fn test_storage_roundtrip() {
    let storage = FoundationDBBackend::new_test().await.unwrap();
    
    let event = Event {
        id: "test_event".to_string(),
        chain_id: "ethereum".to_string(),
        block_number: 100,
        data: vec![1, 2, 3],
    };
    
    // Store event
    storage.store_event(&event).await.unwrap();
    
    // Retrieve event
    let retrieved = storage.get_event("test_event").await.unwrap();
    assert_eq!(retrieved.unwrap(), event);
}
```

### Mock Testing
```rust
use almanac_testing::mocks::{MockChainAdapter, MockStorage};

#[tokio::test]
async fn test_indexing_with_mocks() {
    let chain = MockChainAdapter::new()
        .with_block(100, vec![test_event()])
        .with_finality(100, true);
    
    let storage = MockStorage::new();
    
    let indexer = Indexer::new(chain, storage);
    indexer.process_block(100).await.unwrap();
    
    assert_eq!(storage.event_count(), 1);
}
```

## Test Fixtures

### Using almanac-testing
```rust
use almanac_testing::fixtures::*;

#[test]
fn test_with_fixtures() {
    let block = create_test_block(100);
    let event = create_test_event("transfer");
    let proof = create_test_proof();
    
    // Use fixtures in tests
    assert_eq!(block.number, 100);
}
```

### Custom Fixtures
```rust
pub mod fixtures {
    use almanac_chains::ChainDataIR;
    
    pub fn ethereum_block(number: u64) -> ChainDataIR {
        ChainDataIR {
            block_number: number,
            block_hash: format!("0x{:064x}", number),
            parent_hash: format!("0x{:064x}", number - 1),
            timestamp: 1234567890,
            events: vec![],
            state_changes: vec![],
            metadata: Default::default(),
        }
    }
}
```

## Property-Based Testing

Using `proptest` for complex scenarios:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_reorg_detection(
        blocks in prop::collection::vec(any::<u64>(), 1..100)
    ) {
        let detector = ReorgDetector::new();
        
        for block in blocks {
            let result = detector.process_block(block);
            assert!(result.is_ok());
        }
        
        // Property: detector should maintain consistency
        assert!(detector.is_consistent());
    }
}
```

## Performance Testing

### Benchmarks
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_event_processing(c: &mut Criterion) {
    c.bench_function("process 1000 events", |b| {
        let events = create_test_events(1000);
        let processor = EventProcessor::new();
        
        b.iter(|| {
            for event in &events {
                black_box(processor.process(event));
            }
        });
    });
}

criterion_group!(benches, benchmark_event_processing);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench
cargo bench -- --save-baseline master
cargo bench -- --baseline master
```

### Load Testing
```rust
#[tokio::test]
async fn test_high_load() {
    let api = create_test_api().await;
    
    // Spawn multiple concurrent requests
    let handles: Vec<_> = (0..1000)
        .map(|i| {
            let api = api.clone();
            tokio::spawn(async move {
                api.query_events(i).await
            })
        })
        .collect();
    
    // Wait for all requests
    let results = futures::future::join_all(handles).await;
    
    // Verify success rate
    let success_count = results.iter()
        .filter(|r| r.is_ok())
        .count();
    
    assert!(success_count > 950); // 95% success rate
}
```

## Test Utilities

### Test Database Setup
```rust
use almanac_testing::TestDatabase;

#[tokio::test]
async fn test_with_database() {
    let db = TestDatabase::new().await;
    
    // Use test database
    let storage = FoundationDBBackend::new(db.connection_string()).await.unwrap();
    
    // Test operations...
    
    // Cleanup happens automatically on drop
}
```

### Time Control
```rust
use almanac_testing::time::MockClock;

#[test]
fn test_time_dependent_logic() {
    let clock = MockClock::new();
    clock.set_time(1234567890);
    
    let processor = FinalityProcessor::with_clock(clock.clone());
    
    // Advance time
    clock.advance(Duration::from_secs(60));
    
    // Test time-dependent behavior
    assert!(processor.is_finalized());
}
```

## Testing Best Practices

### 1. Test Naming
```rust
#[test]
fn test_event_creation_with_valid_data_succeeds() { }

#[test]
fn test_event_creation_with_empty_id_fails() { }

#[test]
fn test_reorg_detection_identifies_simple_fork() { }
```

### 2. Test Organization
- Group related tests in modules
- Use descriptive test names
- Keep tests focused and small
- Test both success and failure cases

### 3. Test Independence
```rust
// Bad: Tests depend on shared state
static mut COUNTER: u32 = 0;

// Good: Each test is independent
#[test]
fn test_independent() {
    let state = TestState::new();
    // ...
}
```

### 4. Async Testing
```rust
#[tokio::test]
async fn test_async_timeout() {
    let result = timeout(Duration::from_secs(5), async {
        slow_operation().await
    }).await;
    
    assert!(result.is_ok());
}
```

### 5. Error Testing
```rust
#[test]
fn test_error_handling() {
    let result = risky_operation();
    
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Expected error message"
    );
}

#[test]
#[should_panic(expected = "assertion failed")]
fn test_panic() {
    panic_inducing_function();
}
```

## Continuous Integration

### GitHub Actions Example
```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        
    - name: Install FoundationDB
      run: |
        wget https://www.foundationdb.org/downloads/7.1.0/ubuntu/installers/foundationdb-clients_7.1.0-1_amd64.deb
        sudo dpkg -i foundationdb-clients_7.1.0-1_amd64.deb
        
    - name: Run tests
      run: cargo test --all-features
      
    - name: Run benchmarks
      run: cargo bench --no-run
```

## Test Coverage

### Generate Coverage Report
```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage

# With specific features
cargo tarpaulin --features ethereum,cosmos --out Lcov
```

### Coverage Requirements
- Aim for >80% code coverage
- Critical paths should have 100% coverage
- Focus on business logic coverage
- Don't test external dependencies

## Debugging Tests

### Enable Logging
```bash
# Set log level for tests
RUST_LOG=debug cargo test

# Specific module
RUST_LOG=almanac_consensus=trace cargo test

# With test output
cargo test -- --nocapture
```

### Using Debugger
```bash
# Run test in debugger
rust-gdb ./target/debug/deps/almanac_core-xxxxx
```

### Test Isolation
```rust
// Run single test
cargo test test_specific_function

// Run tests matching pattern
cargo test reorg

// Exclude tests
cargo test -- --skip slow
```

## Performance Profiling

### CPU Profiling
```bash
# Install perf
sudo apt-get install linux-tools-common

# Profile tests
cargo test --release -- --test-threads=1
perf record -g ./target/release/deps/test_binary
perf report
```

### Memory Profiling
```bash
# Using valgrind
valgrind --leak-check=full ./target/debug/deps/test_binary

# Using heaptrack
heaptrack ./target/debug/deps/test_binary
```
