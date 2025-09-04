# Almanac Testing Guide

This guide covers all testing approaches for the Almanac multi-chain indexing system, from unit tests to full end-to-end validation.

## Test Organization

### Unit Tests
Located alongside source code in each crate:
```
crates/almanac-core/
├── src/
│   ├── lib.rs
│   └── error.rs
└── tests/
    └── unit_tests.rs
```

### Integration Tests
Located in crate-level `tests/` directories:
```
crates/almanac-api/tests/
├── api_integration.rs
└── websocket_tests.rs
```

### End-to-End Tests
Located in the `e2e/` directory with comprehensive system validation:
```
e2e/src/tests/
├── fdb_data_validation_e2e.rs
├── ethereum_fdb_integration_e2e.rs
├── cosmos_fdb_integration_e2e.rs
├── fdb_tuple_space_coverage_e2e.rs
└── cross_chain_fdb_consistency_e2e.rs
```

## Running Tests

### Basic Commands
```bash
# Run all tests using Just (recommended)
just test

# Run with output
just test -- --nocapture

# Run all tests manually
nix develop --command cargo test

# Test specific crate
nix develop --command cargo test -p almanac-core

# Test with features
nix develop --command cargo test -p almanac-chains --features ethereum
```

### End-to-End Tests
```bash
# Run complete E2E test suite (requires FoundationDB)
nix run .#run-e2e-tests

# Run specific E2E test module
nix develop .#fdb --command cargo test -p almanac-tests fdb_data_validation_e2e -- --ignored --nocapture

# Run in FDB environment
nix develop .#fdb --command cargo test -p almanac-tests
```

## Writing Tests

### Unit Test Example
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use almanac_core::UnifiedEvent;
    use std::time::SystemTime;
    
    #[test]
    fn test_event_creation() {
        let event = UnifiedEvent {
            id: "test_id".to_string(),
            chain: "ethereum".to_string(),
            block_number: 12345,
            block_hash: "0xabc".to_string(),
            tx_hash: "0xdef".to_string(),
            timestamp: SystemTime::now(),
            event_type: "TestEvent".to_string(),
            contract_address: None,
            raw_data: vec![],
            attributes: std::collections::HashMap::new(),
            source: almanac_core::EventSource::Internal,
        };
        
        assert_eq!(event.chain, "ethereum");
        assert_eq!(event.block_number, 12345);
    }
    
    #[tokio::test]
    async fn test_async_operation() {
        // Placeholder for an async operation test
        let result: Result<(), anyhow::Error> = Ok(());
        assert!(result.is_ok());
    }
}
```

### Integration Test Example
```rust
// tests/storage_integration.rs
use almanac_storage::{MemoryStorage, StorageWriter, StorageReader};
use almanac_core::UnifiedEvent;
use std::time::SystemTime;

#[tokio::test]
async fn test_storage_roundtrip() {
    let storage = MemoryStorage::new();
    
    let event = UnifiedEvent {
        id: "test_event".to_string(),
        chain: "ethereum".to_string(),
        block_number: 100,
        block_hash: "0xabc".to_string(),
        tx_hash: "0xdef".to_string(),
        timestamp: SystemTime::now(),
        event_type: "TestEvent".to_string(),
        contract_address: None,
        raw_data: vec![1, 2, 3],
        attributes: std::collections::HashMap::new(),
        source: almanac_core::EventSource::Internal,
    };
    
    // Store and retrieve event
    storage.store_event(Box::new(event.clone())).await.unwrap();
    let retrieved = storage.get_events(vec![]).await.unwrap(); // Simplified filter
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].id(), "test_event");
}
```

### Using Test Utilities
```rust
use almanac_testing::fixtures;
use almanac_testing::mocks::{MockChainAdapter, MockStorage};
use almanac_indexing::IndexingService;
use almanac_core::{UnifiedEvent, BlockStatus};
use std::sync::Arc;

#[tokio::test]
async fn test_indexing_with_mocks() {
    let chain_adapter = MockChainAdapter::new()
        .with_latest_block(100)
        .with_block_data(almanac_chains::adapters::types::ChainDataIR::default());
    
    let storage = MockStorage::new();
    let indexer = IndexingService::builder()
        .chain_adapter(Arc::new(chain_adapter))
        .storage(Arc::new(storage.clone()))
        .node_name("test-indexer".to_string())
        .build();
    
    // Assuming IndexingService has a method like process_block
    // indexer.process_block(100).await.unwrap();
    
    // assert_eq!(storage.event_count(), 1); // MockStorage doesn't have event_count
}
```

## End-to-End Test Coverage

### 1. FoundationDB Data Validation
Tests core FDB operations and data integrity:
- Basic CRUD operations
- Transaction atomicity  
- Range queries with filtering
- Concurrent access validation
- Data type validation (numbers, strings, JSON, Unicode)
- Namespace isolation

### 2. Blockchain Integration Tests
Tests blockchain-specific indexing to FDB storage:

**Ethereum Integration:**
- Contract deployment indexing
- ERC20 transfer indexing with proper decoding
- Complex DeFi contract interactions
- Block reorganization handling
- High volume transaction processing

**Cosmos Integration:**
- Bank transfer indexing
- WASM contract deployment
- IBC transfer indexing
- Staking operations (delegation, undelegation)
- Governance proposals

### 3. FoundationDB Tuple Space Coverage
Comprehensive testing of all FDB keyspaces:
1. **ContractState** (1): Contract storage slots
2. **BlockMeta** (2): Block headers and consensus data
3. **StorageProofs** (3): Merkle proofs for verification
4. **SemanticMeta** (4): Trust-scoped semantic interpretations
5. **TrustGraph** (5): Developer trust relationships
6. **ContractLayouts** (6): Layout commitments from Traverse
7. **ViewRegistry** (7): Custom view definitions
8. **LightwaveState** (8): Recursive proof coordination
9. **TrustedCheckpoints** (9): Network checkpoints
10. **SolanaAccounts** (10): Solana account data
11. **NetworkConfig** (11): Blockchain network configuration
12. **CircuitKeys** (12): SP1 verification keys
13. **ChangeTracking** (13): State change logs
14. **ValidatorSignatures** (14): Proposer validator signatures
15. **TrustPolicies** (15): Developer trust policies

### 4. Cross-Chain Consistency Tests
Tests data consistency across multiple blockchains:
- Multi-chain event ordering
- Cross-chain transaction correlation
- Unified block height tracking
- Cross-chain asset tracking
- Concurrent chain indexing
- Unified queries across all chains

## Multi-Chain Simulation Testing

### Test Architecture
The full system test validates complete multi-chain environment:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Multi-Chain Simulation                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │   Solana    │  │  Ethereum   │  │   Cosmos    │              │
│  │Test Validator  │    Anvil    │  │  Neutron    │              │
│  │ :8899       │  │ :8545       │  │ :26657      │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                 FoundationDB Cluster                       │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                 Almanac API Server                         │ │
│  │                    REST :3000                              │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Running Multi-Chain Tests
```bash
# Run complete multi-chain simulation
./scripts/testing/e2e-tests.sh --simulation-only

# Test specific components
./scripts/testing/e2e-tests.sh --components-only      # Component tests only
./scripts/testing/run-all-tests.sh --verbose          # All tests with verbose output
./scripts/testing/run-all-tests.sh --quick            # Fast tests only

# Manual testing
nix run .#sim-cluster -- init ethereum,cosmos,solana,fdb
nix run .#sim-cluster -- start
nix run .#sim-cluster -- status
```

## Test Configuration

### Environment Variables
```bash
# FoundationDB configuration
export FDB_CLUSTER_FILE=/path/to/fdb.cluster
export FDB_NAMESPACE=almanac_test

# Test settings
export TEST_TIMEOUT=600
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Storage backend selection
export STORAGE_BACKEND=foundationdb  # or "memory"
```

### Performance Expectations
- **Basic Operations**: < 10ms per operation
- **Range Queries**: < 100ms for 1000 records
- **Concurrent Writes**: > 1000 ops/second
- **Cross-Chain Queries**: < 200ms for complex aggregations
- **API Response**: < 100ms
- **Event Indexing**: < 5 seconds latency

## Debugging and Troubleshooting

### Enable Debug Logging
```bash
# Set log level for tests
RUST_LOG=debug nix develop --command cargo test -- --nocapture

# Specific module logging
RUST_LOG=almanac_storage=debug,almanac_e2e=debug nix develop --command cargo test
```

### Common Issues

**Build Errors:**
```bash
nix develop --command bash -c "cargo clean && cargo update"
nix develop --command cargo check -p almanac-api
```

**FoundationDB Connection Issues:**
```bash
# Check FDB status
nix run .#fdb-manage -- status

# Verify cluster file
cat ./data/fdb.cluster

# Clear test data
nix run .#fdb-manage -- cli --exec "clearrange \x00 \xFF"
```

**Test Timeouts:**
```bash
export TEST_TIMEOUT=600
nix develop --command cargo test -- --test-threads=1
```

**Port Conflicts:**
```bash
# Check for conflicting services
lsof -i :8545
lsof -i :3000
```

## Best Practices

### Test Design
1. **Test Independence**: Each test should be completely independent
2. **Unique Namespaces**: Always use unique namespaces for isolation
3. **Cleanup**: Always clean up test data, even on failure
4. **Descriptive Names**: Use clear, descriptive test names
5. **Performance Monitoring**: Track test execution time

### Test Data Management
```rust
// Create unique namespace per test
let namespace = format!("almanac_test_{}", uuid::Uuid::new_v4());

// Always cleanup
#[tokio::test]
async fn test_example() -> Result<()> {
    let storage = helpers::create_memory_storage().await;
    
    // Test logic here
    
    // Cleanup
    storage.clear_all_data().await?;
    Ok(())
}
```

### Continuous Integration
```yaml
name: Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: cachix/install-nix-action@v22
    
    - name: Run unit tests
      run: nix develop --command cargo test --workspace --all-targets
      
    - name: Run E2E tests  
      run: nix run .#run-e2e-tests
```

## Test Coverage

### Generate Coverage Reports
```bash
# Install cargo-tarpaulin
nix develop -c cargo install cargo-tarpaulin

# Generate coverage
nix develop -c cargo tarpaulin --out Html --output-dir coverage

# With specific features
nix develop -c cargo tarpaulin --features ethereum,cosmos --out Lcov
```

### Coverage Requirements
- Aim for >80% code coverage
- Critical paths should have 100% coverage
- Focus on business logic coverage
- Don't test external dependencies

For more detailed information, see:
- `docs/003_foundationdb_setup_and_usage.md` - FoundationDB setup and usage
- `docs/002_running_and_configuration.md` - Running and configuration guide
- `docs/104_api_integration_examples.md` - API integration examples