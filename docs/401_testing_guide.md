# Almanac Testing Guide

This guide provides comprehensive documentation for the Almanac testing framework, covering all types of testing including integration, property-based, performance, and chaos engineering tests.

## Table of Contents

- [Overview](#overview)
- [Test Types](#test-types)
- [Getting Started](#getting-started)
- [Configuration](#configuration)
- [Running Tests](#running-tests)
- [Test Development](#test-development)
- [Performance Testing](#performance-testing)
- [Chaos Engineering](#chaos-engineering)
- [CI/CD Integration](#cicd-integration)
- [Troubleshooting](#troubleshooting)

## Overview

The Almanac testing framework (`crates/testing/`) provides a comprehensive testing suite designed to validate the reliability, performance, and resilience of the cross-chain indexer. The framework includes:

- Integration Tests: End-to-end testing of system components
- Property-Based Tests: Verification of system invariants using randomized inputs
- Performance Tests: Load testing, stress testing, and memory leak detection
- Chaos Engineering: Fault injection to test system resilience
- Benchmarks: Performance regression detection

## Test Types

### Integration Tests

Integration tests validate the interaction between different system components:

```rust
// Example: Database integration test
#[tokio::test]
async fn test_database_integration() {
    let config = TestDatabaseConfig {
        postgres_url: Some("postgres://test:test@localhost:5433/test".to_string()),
        rocks_path: Some("./test_data/rocks".to_string()),
        use_in_memory: true,
        clean_between_tests: true,
    };
    
    let result = integration::test_database_integration(&config).await;
    assert!(result.is_ok());
}
```

Covered Areas:
- Database operations (PostgreSQL, RocksDB)
- API endpoints (REST, GraphQL, WebSocket)
- Cross-chain event correlation
- Service interactions

### Property-Based Tests

Property-based tests use randomized inputs to verify system invariants:

```rust
// Example: Event serialization property
proptest! {
    #[test]
    fn event_serialization_roundtrip(
        id in "[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}",
        chain_id in "[a-z]{3,10}",
        block_number in 1u64..1_000_000u64
    ) {
        let event = create_test_event(&id, &chain_id, block_number);
        let serialized = serde_json::to_string(&event)?;
        let deserialized: Event = serde_json::from_str(&serialized)?;
        
        prop_assert_eq!(event.id(), deserialized.id());
        prop_assert_eq!(event.chain_id(), deserialized.chain_id());
        prop_assert_eq!(event.block_number(), deserialized.block_number());
    }
}
```

Verified Properties:
- Event serialization/deserialization invariants
- Event ordering properties
- Storage consistency properties
- API request/response properties
- Filter composition properties

### Performance Tests

Performance tests measure system behavior under load:

```rust
// Example: Load test configuration
let config = PerformanceTestConfig {
    concurrent_connections: 100,
    test_duration_seconds: 60,
    target_rps: 1000.0,
    memory_limit_mb: 1024,
    cpu_limit_percent: 80.0,
};

let results = performance::run_load_test(&config).await?;
```

Test Types:
- Load Testing: Normal expected traffic patterns
- Stress Testing: Beyond normal capacity limits
- Memory Leak Testing: Long-running memory usage monitoring
- Throughput Testing: Maximum sustainable request rates

### Chaos Engineering

Chaos engineering tests system resilience through controlled fault injection:

```rust
// Example: Network partition simulation
let chaos_config = ChaosTestConfig {
    enabled: true,
    network_failure_rate: 0.1,
    database_failure_rate: 0.05,
    memory_pressure: true,
    cpu_pressure: false,
};

let results = chaos::test_network_failures(&chaos_config).await?;
```

Failure Scenarios:
- Network partitions and packet loss
- Database connection failures
- Resource exhaustion (memory, CPU, disk)
- Service unavailability

## Getting Started

### Prerequisites

1. Development Environment: Ensure nix development environment is set up
2. Test Databases: PostgreSQL and RocksDB test instances
3. Test Dependencies: Install required testing tools

```bash
# Initialize test environment
nix develop

# Set up test databases
init_databases

# Run basic test suite
cargo test --package indexer-testing
```

### Basic Test Execution

```bash
# Run all tests
cargo test --package indexer-testing

# Run specific test types
cargo test --package indexer-testing --features integration
cargo test --package indexer-testing --features property
cargo test --package indexer-testing --features performance
cargo test --package indexer-testing --features chaos

# Run with full feature set
cargo test --package indexer-testing --features full
```

## Configuration

### Test Configuration File

Create a test configuration file (`test_config.toml`):

```toml
[environment]
name = "test"

[database]
postgres_url = "postgres://test:test@localhost:5433/indexer_test"
rocks_path = "./test_data/rocks"
use_in_memory = true
clean_between_tests = true

[api]
port = 8081
test_graphql = true
test_rest = true
test_websockets = true
request_timeout_ms = 5000

[chains.ethereum]
rpc_url = "http://localhost:8545"
chain_id = 1337
test_contracts = ["0x1234567890123456789012345678901234567890"]

[chains.cosmos]
rpc_url = "http://localhost:26657"
chain_id = "test-chain-1"
test_contracts = ["cosmos1234567890123456789012345678901234567890"]

[performance]
concurrent_connections = 100
test_duration_seconds = 60
target_rps = 1000.0
memory_limit_mb = 1024
cpu_limit_percent = 80.0

[chaos]
enabled = false  # Enable with caution
network_failure_rate = 0.1
database_failure_rate = 0.05
memory_pressure = true
cpu_pressure = false
```

### Environment-Specific Configuration

Different configurations for different environments:

```bash
# Development testing
cargo test --package indexer-testing -- --test-env development

# Staging testing
cargo test --package indexer-testing -- --test-env staging

# Production testing (limited subset)
cargo test --package indexer-testing -- --test-env production
```

## Running Tests

### Local Development

```bash
# Quick smoke tests
cargo test --package indexer-testing --lib integration::basic

# Full integration suite
cargo test --package indexer-testing --features integration

# Performance benchmarks
cargo test --package indexer-testing --features performance --release
```

### Continuous Integration

```bash
# CI pipeline test command
cargo test --package indexer-testing \
    --features "integration,property" \
    --verbose \
    -- --test-threads=1 --nocapture
```

### Test Reports

Generate detailed test reports:

```rust
use indexer_testing::{TestSuite, TestConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = TestConfig::default();
    let mut suite = TestSuite::new(config);
    
    let summary = suite.run_all().await?;
    
    // Generate markdown report
    summary.save_report("test_report.md")?;
    
    // Export JSON results
    let json_results = summary.export_json()?;
    std::fs::write("test_results.json", json_results)?;
    
    Ok(())
}
```

## Test Development

### Writing Integration Tests

```rust
use indexer_testing::{fixtures, TestDatabaseConfig};

#[tokio::test]
async fn test_custom_integration() {
    // Arrange
    let config = TestDatabaseConfig::default();
    let test_events = fixtures::create_test_events(10);
    
    // Act
    let result = your_function_under_test(&test_events).await;
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 10);
}
```

### Writing Property Tests

```rust
use proptest::prelude::*;
use indexer_testing::fixtures;

proptest! {
    #[test]
    fn your_property_test(
        input1 in any::<String>(),
        input2 in 1u64..1000u64
    ) {
        // Your property test logic
        let result = your_function(input1, input2);
        prop_assert!(your_invariant_check(result));
    }
}
```

### Custom Test Fixtures

```rust
use indexer_testing::fixtures;

// Create custom test data
pub fn create_custom_test_scenario() -> Vec<TestEvent> {
    let mut events = Vec::new();
    
    // Add specific test events
    events.push(fixtures::create_test_event_with_params(
        "custom_id",
        "ethereum",
        12345,
        "0x...",
        "0x...",
        "CustomEvent",
        b"custom_data",
    ));
    
    events
}
```

## Performance Testing

### Load Testing

Configure and run load tests:

```rust
use indexer_testing::performance::{run_load_test, PerformanceTestConfig};

#[tokio::test]
async fn load_test_api() {
    let config = PerformanceTestConfig {
        concurrent_connections: 50,
        test_duration_seconds: 30,
        target_rps: 500.0,
        memory_limit_mb: 512,
        cpu_limit_percent: 70.0,
    };
    
    let results = run_load_test(&config).await.unwrap();
    
    // Validate performance metrics
    assert!(results.operations_per_second >= 450.0);
    assert!(results.p95_response_time_ms <= 100.0);
    assert!(results.error_rate <= 0.01);
}
```

### Memory Leak Detection

```rust
use indexer_testing::performance::run_memory_leak_test;

#[tokio::test]
async fn memory_leak_test() {
    let config = PerformanceTestConfig {
        test_duration_seconds: 300, // 5 minutes
        ..Default::default()
    };
    
    let results = run_memory_leak_test(&config).await.unwrap();
    
    // Check memory growth
    let memory_growth = results.peak_memory_mb - results.memory_usage_mb;
    assert!(memory_growth <= 50.0); // Max 50MB growth
}
```

### Performance Regression Detection

Track performance over time:

```rust
// Store baseline performance metrics
let baseline_results = run_performance_baseline().await?;
store_performance_baseline("v1.0.0", &baseline_results)?;

// Compare against baseline
let current_results = run_load_test(&config).await?;
let regression = detect_performance_regression(&baseline_results, &current_results);

assert!(!regression.detected, "Performance regression detected: {:?}", regression);
```

## Chaos Engineering

### Network Failure Testing

```rust
use indexer_testing::chaos::{test_network_failures, ChaosTestConfig};

#[tokio::test]
async fn chaos_network_partition() {
    let config = ChaosTestConfig {
        enabled: true,
        network_failure_rate: 0.2, // 20% failure rate
        database_failure_rate: 0.0,
        memory_pressure: false,
        cpu_pressure: false,
    };
    
    let results = test_network_failures(&config).await.unwrap();
    
    // Verify system resilience
    assert!(results.contains("Network partition"));
    assert!(results.contains("recovery"));
}
```

### Resource Exhaustion Testing

```rust
use indexer_testing::chaos::test_resource_exhaustion;

#[tokio::test]
async fn chaos_memory_pressure() {
    let config = ChaosTestConfig {
        enabled: true,
        memory_pressure: true,
        cpu_pressure: false,
        ..Default::default()
    };
    
    let results = test_resource_exhaustion(&config).await.unwrap();
    
    // Verify graceful degradation
    assert!(results.contains("Memory pressure"));
}
```

### Safety Guidelines

Warning: Chaos Testing Safety:

1. Never run chaos tests in production
2. Use isolated test environments
3. Start with low failure rates (< 0.1)
4. Monitor system recovery time
5. Have rollback procedures ready

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Comprehensive Tests

on: [push, pull_request]

jobs:
  integration-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    
    steps:
      - uses: actions/checkout@v4
      - uses: cachix/install-nix-action@v22
      
      - name: Run Integration Tests
        run: |
          nix develop -c cargo test --package indexer-testing \
            --features integration \
            --verbose
      
      - name: Run Property Tests
        run: |
``` 