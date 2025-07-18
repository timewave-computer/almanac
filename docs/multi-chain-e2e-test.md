# Multi-Chain Simulation E2E Test

## Overview

The Multi-Chain Simulation E2E test is a comprehensive end-to-end validation of the Almanac indexing system. It spins up a complete multi-chain environment with all the necessary blockchain nodes, storage backends, and Almanac services to validate the entire system architecture.

## Test Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Multi-Chain Simulation                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────┐ │
│  │   Solana    │  │  Ethereum   │  │   Cosmos    │  │ Beacon  │ │
│  │Test Validator│  │    Anvil    │  │  Neutron    │  │ Chain   │ │
│  │             │  │             │  │             │  │         │ │
│  │ :8899       │  │ :8545       │  │ :26657      │  │ :5052   │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────┘ │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                 FoundationDB Cluster                       │ │
│  │              (Unified Storage Layer)                       │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                 Almanac Network                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │   Indexer   │  │  API Server │  │  Principle Service  │ │ │
│  │  │             │  │             │  │                     │ │ │
│  │  │ Multi-Chain │  │    REST     │  │  Authentication     │ │ │
│  │  │   Syncing   │  │   :3000     │  │    & Authorization  │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Components

### Blockchain Nodes

1. **Solana Test Validator** (`solana-test-validator`)
   - Port: 8899 (RPC)
   - Provides Solana blockchain simulation
   - Supports transaction generation and querying

2. **Ethereum Anvil** (`anvil`)
   - Port: 8545 (RPC), 8546 (WebSocket)
   - Chain ID: 31337
   - Pre-funded development accounts
   - 1-second block time

3. **Cosmos Neutron** (`neutron`)
   - Port: 26657 (RPC), 9090 (gRPC), 1317 (API)
   - Chain ID: test-1
   - Cosmos SDK-based blockchain

4. **Lighthouse Beacon Chain** (Optional)
   - Port: 5052 (HTTP API)
   - Ethereum 2.0 consensus layer
   - Provides finality information

### Storage Backend

**FoundationDB Cluster**
- Distributed, ACID-compliant database
- Unified storage for all blockchain data
- Namespace: `almanac_e2e_test`
- Provides transaction semantics across chains

### Almanac Services

1. **Almanac Indexer**
   - Multi-chain event indexing
   - Real-time synchronization
   - Event correlation across chains

2. **Almanac API Server**
   - REST API endpoints
   - OpenAPI documentation
   - Port: 3000

3. **Principle Service**
   - Authentication and authorization
   - Role-based access control
   - API key management

## Test Scenarios

### 1. Infrastructure Setup
- Initialize simulation configuration
- Start all blockchain nodes
- Start FoundationDB cluster
- Start Almanac services
- Validate service health

### 2. Chain Data Loading
- Generate test transactions on each chain
- Verify transaction propagation
- Monitor block production
- Validate chain-specific data

### 3. Cross-Chain Indexing
- Index events from all chains
- Store events in FoundationDB
- Verify event correlation
- Test event ordering

### 4. FoundationDB Operations
- Write operations via indexer
- Read operations via API
- Transaction consistency
- Cross-chain queries

### 5. API Endpoint Testing
- REST API endpoints
- OpenAPI documentation
- Authentication flows

### 6. Principle Service Integration
- API key authentication
- Role-based access control
- Permission validation
- Audit logging

## Running the Test

### Prerequisites

1. **Nix Environment**
   ```bash
   nix develop --impure
   ```

2. **Simulation Environment**
   ```bash
   # Test environment setup
   ./scripts/test-simulation-env.sh
   ```

### Running the Full Test

```bash
# Run the complete E2E test
./scripts/run-multi-chain-e2e.sh

# Run specific test components
./scripts/run-multi-chain-e2e.sh test        # Full test suite
./scripts/run-multi-chain-e2e.sh standalone  # Standalone test
./scripts/run-multi-chain-e2e.sh report      # Generate report
./scripts/run-multi-chain-e2e.sh clean       # Cleanup
```

### Running via Test Runner

```bash
# Run via E2E test runner
cd e2e
cargo run --bin test-runner -- --filter=multi-chain-simulation

# Run specific test
cargo test --release multi_chain_simulation_e2e::tests::test_multi_chain_simulation_e2e -- --nocapture
```

### Manual Testing

```bash
# Initialize simulation
nix run .#simulation.sim-cluster -- init ethereum,cosmos,solana,fdb

# Start services
nix run .#simulation.sim-cluster -- start

# Check status
nix run .#simulation.sim-cluster -- status

# View logs
nix run .#simulation.sim-cluster -- logs api

# Stop services
nix run .#simulation.sim-cluster -- stop
```

## Test Configuration

### Default Configuration
```rust
MultiChainSimulationConfig {
    enabled_chains: ["ethereum", "cosmos", "solana", "fdb"],
    test_duration: 60, // seconds
    test_transactions: 5, // per chain
    // ... other settings
}
```

### Service Endpoints
- **Ethereum**: `http://localhost:8545`
- **Cosmos**: `http://localhost:26657`
- **Solana**: `http://localhost:8899`
- **Almanac API**: `http://localhost:3000`
- **Swagger UI**: `http://localhost:3000/api/swagger-ui`

## Validation Checks

### Health Checks
```bash
# Ethereum
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' \
  http://localhost:8545

# Cosmos
curl -s http://localhost:26657/status

# Solana
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"getHealth","id":1}' \
  http://localhost:8899

# Almanac API
curl -s http://localhost:3000/health
```

### Data Validation
- Event count consistency
- Block height progression
- Cross-chain event correlation
- FoundationDB transaction integrity
- API response validation

## Troubleshooting

### Common Issues

1. **Port Conflicts**
   - Check for existing services on required ports
   - Use `lsof -i :8545` to identify conflicts

2. **FoundationDB Issues**
   - Ensure FDB cluster is running: `fdb-manage status`
   - Check cluster file: `cat data/fdb.cluster`

3. **Node Startup Failures**
   - Check logs in `./logs/` directory
   - Verify data directories exist and are writable

4. **Test Timeouts**
   - Increase test duration in configuration
   - Check system resources (CPU, memory)

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG="debug"
export RUST_BACKTRACE="1"

# Run with increased verbosity
./scripts/run-multi-chain-e2e.sh test --verbose
```

## Test Reports

The test generates comprehensive reports in `./logs/`:
- `multi-chain-e2e-report.txt` - Test execution summary
- `simulation-env-report.txt` - Environment validation
- Individual service logs (`anvil.log`, `neutron.log`, etc.)

## Performance Metrics

### Expected Performance
- **Startup Time**: < 2 minutes
- **Transaction Processing**: < 1 second per chain
- **Event Indexing**: < 5 seconds latency
- **API Response**: < 100ms

### Monitoring
- Service health endpoints
- Block production rates
- Event processing throughput
- FoundationDB performance metrics

## Security Considerations

### Test Environment
- Uses development keys and accounts
- Isolated from production networks
- Temporary data directories
- No real cryptocurrency involved

### Production Differences
- Production uses secure key management
- Real blockchain networks (mainnet/testnet)
- Persistent data storage
- Enhanced authentication

## Integration with CI/CD

### GitHub Actions
```yaml
name: Multi-Chain E2E Test

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  multi-chain-e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: cachix/install-nix-action@v22
      - name: Run E2E Test
        run: |
          nix develop --impure --command \
            ./scripts/run-multi-chain-e2e.sh test
```

### Test Artifacts
- Service logs
- Configuration files
- Performance metrics
- Test reports
- Screenshots (if applicable)

## Future Enhancements

### Planned Features
- [ ] Lighthouse beacon chain integration
- [ ] Cross-chain message passing tests
- [ ] Performance benchmarking
- [ ] Chaos engineering tests
- [ ] Load testing scenarios

### Monitoring Improvements
- [ ] Real-time metrics dashboard
- [ ] Alerting for test failures
- [ ] Performance regression detection
- [ ] Resource usage tracking

## References

- [Almanac Architecture Documentation](./architecture.md)
- [FoundationDB Setup Guide](./foundationdb-setup-guide.md)
- [Chain Adapter Guide](./chain-adapter-guide.md)
- [API Development Guide](./api-development.md)
- [Nix Development Setup](./nix-development-setup.md)