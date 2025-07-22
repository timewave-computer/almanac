# Almanac End-to-End Tests

This directory contains comprehensive end-to-end tests for the Almanac multi-chain blockchain indexing system.

## Overview

The e2e test suite validates the complete Almanac system including:

- **API Integration Tests**: HTTP API endpoint functionality with real storage backends
- **Blockchain Indexing Tests**: Full chain integration with Ethereum, Cosmos, and Solana
- **Multi-Chain Tests**: Cross-chain event correlation and consistency
- **FoundationDB Integration**: Storage backend validation and data integrity
- **Configuration Tests**: Environment and TOML configuration validation

## Quick Start

### Run All Tests

```bash
# From project root
./scripts/testing/e2e-tests.sh
```

### Run Specific Test Categories

```bash
# Run blockchain indexing tests (e.g., Ethereum indexing)
./scripts/testing/e2e-tests.sh --components-only

# Run multi-chain simulation tests
./scripts/testing/e2e-tests.sh --simulation-only

# Run FoundationDB integration tests (currently simplified placeholders)
./scripts/testing/fdb-tests.sh --e2e-only
```

### Development Options

```bash
# Verbose output for debugging
./scripts/testing/e2e-tests.sh --verbose

# Run all E2E tests
./scripts/testing/run-all-tests.sh --skip-e2e false

# Run specific test components
./scripts/testing/e2e-tests.sh --filter specific_test
```

## Test Structure

### Blockchain Indexing Tests

#### Ethereum Integration (`ethereum_indexing_e2e.rs`)
- Tests Ethereum blockchain event indexing
- Validates ERC20 token event processing
- Smart contract state tracking
- Block finality handling

#### Multi-Chain Simulation (`multi_chain_simulation_e2e.rs`)
- Full simulation environment setup
- Cross-chain event correlation
- Multi-chain API queries
- End-to-end data consistency

### Storage Integration Tests

#### FoundationDB Tests
- `fdb_data_validation_e2e_simple.rs` - Data integrity validation
- `fdb_tuple_space_coverage_e2e_simple.rs` - Tuple space operations
- `cross_chain_fdb_consistency_e2e_simple.rs` - Cross-chain consistency

### API Tests

#### Basic API Tests (`basic.rs`)
- Health endpoint validation
- Event query functionality
- Authentication and rate limiting
- Error handling and responses

#### Configuration Tests (`config.rs`)
- TOML configuration file validation
- Environment variable handling
- Chain-specific configuration
- Storage backend selection

## Test Environment Setup

The tests require a complete Almanac environment:

### Required Components
- **FoundationDB**: Distributed storage backend
- **Blockchain Nodes**: Ethereum (Anvil/Geth), Cosmos (Neutron), Solana
- **Almanac Services**: API server, indexing nodes
- **Mock Data**: Test contracts and events

### Setup Commands

```bash
# Set up FoundationDB
./scripts/dev/setup-fdb-headers.sh

# Start simulation environment
nix run .#sim-cluster

# Verify environment
./scripts/testing/run-all-tests.sh --dry-run
```

## Running Tests Manually

### Using Cargo

```bash
# Run all tests
cd e2e
cargo test

# Run specific test
cargo test ethereum_indexing

# Run with verbose output
cargo test -- --nocapture
```

### Using Nix

```bash
# Build and run in nix environment
nix develop --command cargo test --manifest-path e2e/Cargo.toml
```

## Test Configuration

### Environment Variables

```bash
# API endpoint
export ALMANAC_API_URL=http://localhost:8080

# Storage backend
export STORAGE_BACKEND=foundationdb
export FDB_CLUSTER_FILE=/path/to/fdb.cluster

# Chain RPC endpoints
export ETH_RPC_URL=http://localhost:8545
export COSMOS_RPC_URL=http://localhost:26657
export SOLANA_RPC_URL=http://localhost:8899
```

### Test Data

Test fixtures and mock data:

```
e2e/fixtures/
├── contracts/
│   ├── erc20.sol           # ERC20 contract for testing
│   └── cosmwasm.wasm       # CosmWasm contract binary
├── events/
│   ├── ethereum_events.json # Sample Ethereum events
│   └── cosmos_events.json   # Sample Cosmos events
└── configs/
    ├── test.toml           # Test configuration
    └── simulation.toml     # Simulation configuration
```

## Test Output

### Success Output

```
Almanac CLI End-to-End Test Suite
==================================================
Almanac binary ready
Test fixtures ready

Running test suites...

Basic CLI Tests
  PASSED 15/15 tests passed (100.0%)

Multi-Chain Simulation E2E Test
  PASSED Multi-chain simulation test passed

All tests passed!
```

### Failure Output

```
Ethereum CLI Tests
  FAILED 4/5 tests passed, 1 failed (80.0%)
    Failed tests:
      - ethereum_indexing_test
```

## Test Categories

### Integration Tests
- **API Integration**: REST API with storage backends
- **Chain Integration**: Blockchain node connectivity
- **Storage Integration**: FoundationDB operations
- **Cross-Chain Integration**: Multi-chain consistency

### Functional Tests
- **Event Processing**: Blockchain event indexing
- **Query Functionality**: API event and block queries
- **Configuration**: Environment and file configuration
- **Error Handling**: Graceful failure scenarios

### Performance Tests
- **Throughput**: High-volume event processing
- **Latency**: API response times
- **Concurrency**: Multi-chain parallel processing
- **Scalability**: Large dataset handling

## Continuous Integration

The e2e tests are designed for CI environments:

- **Nix-based**: Reproducible builds with nix flake
- **Isolated**: Each test uses separate environments
- **Fast**: Optimized for quick feedback
- **Reliable**: Comprehensive error handling

### CI Configuration

```yaml
# Example GitHub Actions
- name: Run E2E Tests
  run: |
    nix develop --command ./scripts/testing/run-all-tests.sh --verbose
    nix develop --command ./scripts/testing/e2e-tests.sh --simulation-only
```

## Troubleshooting

### Common Issues

1. **Binary not found**: Build with `nix build .#almanac-api`
2. **FoundationDB connection**: Ensure FDB cluster is running
3. **Blockchain nodes**: Verify RPC endpoints are accessible
4. **Port conflicts**: Check for conflicting services

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug ./scripts/testing/e2e-tests.sh --verbose

# Test specific components
./scripts/testing/e2e-tests.sh --components-only --verbose

# Run specific failing test
cargo test ethereum_indexing -- --nocapture
```

### Test Isolation

Each test creates isolated environments:
```
e2e/test_outputs/
├── ethereum_indexing_test/
├── multi_chain_simulation/
├── fdb_integration_test/
└── ...
```

## Coverage

The e2e test suite aims to cover:

- **Most API endpoints** with real storage backends (ongoing)
- **Supported blockchains** (Ethereum, Cosmos, and partially Solana)
- **Complete indexing pipeline** from blockchain to API
- **Cross-chain functionality** and data consistency
- **Configuration scenarios** and error conditions
- **Performance characteristics** under load

This ensures the complete Almanac system works correctly in real-world scenarios and that documented functionality is accurate and reliable.