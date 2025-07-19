# Development Workflows

This guide covers development workflows for the Almanac multi-chain indexing system, including the Nix-based simulation environment, testing workflows, and operational scripts.

## Quick Start

### Prerequisites

- **Nix**: Package manager with flakes enabled
- **Git**: Version control
- **Hardware**: 8GB+ RAM recommended for full simulation

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/timewave-computer/almanac
cd almanac

# Enter development environment
nix develop

# Set up FoundationDB headers (macOS)
./scripts/dev/setup-fdb-headers.sh
source .env.fdb
```

## Nix-Based Simulation Environment

### Overview

The simulation environment provides a complete multi-chain development cluster:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Simulation Environment                       │
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
│  │               Almanac Services                             │ │
│  │  • almanac-api (:3000)      • almanac-ethereum (:9091)    │ │
│  │  • almanac-cosmos (:9092)   • almanac-solana (:9093)      │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Basic Simulation Commands

```bash
# Initialize simulation with all chains
nix run .#sim-cluster -- init ethereum,cosmos,solana,fdb

# Start all services
nix run .#sim-cluster -- start

# Check status of all services
nix run .#sim-cluster -- status

# View logs for specific services
nix run .#sim-cluster -- logs ethereum
nix run .#sim-cluster -- logs api
nix run .#sim-cluster -- logs all

# Stop all services
nix run .#sim-cluster -- stop
```

### Individual Service Management

#### Blockchain Nodes

```bash
# Ethereum (Anvil)
nix run .#sim-ethereum -- start
nix run .#sim-ethereum -- stop
nix run .#sim-ethereum -- status

# Cosmos (Neutron)
nix run .#sim-cosmos -- start
nix run .#sim-cosmos -- stop
nix run .#sim-cosmos -- status

# Solana (Test Validator) - not available on aarch64-linux
nix run .#sim-solana -- start
nix run .#sim-solana -- stop
nix run .#sim-solana -- status
```

#### Almanac Services

```bash
# API Server
nix run .#sim-almanac-api -- start
nix run .#sim-almanac-api -- stop
nix run .#sim-almanac-api -- status

# Chain Indexers
nix run .#sim-almanac-indexer -- start
nix run .#sim-almanac-indexer -- stop
nix run .#sim-almanac-indexer -- status
```

### Configuration

The simulation automatically generates configuration files:

```
./
├── config/simulation/
│   ├── simulation.json          # Main simulation configuration
│   └── almanac-indexer.toml     # Almanac indexer configuration
├── data/
│   ├── fdb.cluster              # FoundationDB cluster file
│   ├── foundationdb/            # FoundationDB data
│   ├── ethereum/                # Ethereum/Anvil data
│   ├── cosmos/                  # Cosmos/Neutron data
│   └── solana/                  # Solana test validator data
└── logs/
    ├── foundationdb/            # FoundationDB logs
    ├── anvil.log                # Ethereum node logs
    ├── neutron.log              # Cosmos node logs
    ├── solana.log               # Solana node logs
    ├── almanac-api.log          # Almanac API logs
    └── almanac-indexer.log      # Almanac indexer logs
```

## Development Workflows

### Typical Development Session

1. **Start Development Environment**:
   ```bash
   nix develop
   ```

2. **Initialize Simulation**:
   ```bash
   nix run .#sim-cluster -- init ethereum,cosmos,fdb
   ```

3. **Start Services**:
   ```bash
   nix run .#sim-cluster -- start
   ```

4. **Verify Everything is Running**:
   ```bash
   nix run .#sim-cluster -- status
   ```

5. **Access Services**:
   - **Ethereum RPC**: http://localhost:8545
   - **Cosmos RPC**: http://localhost:26657
   - **Almanac API**: http://localhost:3000
   - **API Health**: http://localhost:3000/health
   - **Swagger UI**: http://localhost:3000/swagger-ui/

6. **Develop and Test**:
   ```bash
   # Run unit tests
   nix develop --command cargo test
   
   # Run API tests
   ./scripts/testing/api-tests.sh
   
   # Run specific chain tests
   nix develop --command cargo test -p almanac-chains --features ethereum
   ```

7. **View Logs During Development**:
   ```bash
   # Watch API logs
   nix run .#sim-cluster -- logs api
   
   # Watch Ethereum indexer logs
   nix run .#sim-cluster -- logs ethereum
   
   # Watch all logs
   nix run .#sim-cluster -- logs all
   ```

8. **Stop When Done**:
   ```bash
   nix run .#sim-cluster -- stop
   ```

### Testing Different Chain Combinations

```bash
# Ethereum + FoundationDB only
nix run .#sim-cluster -- init ethereum,fdb
nix run .#sim-cluster -- start

# Cosmos + Solana + FoundationDB  
nix run .#sim-cluster -- init cosmos,solana,fdb
nix run .#sim-cluster -- start

# All chains
nix run .#sim-cluster -- init ethereum,cosmos,solana,fdb
nix run .#sim-cluster -- start
```

## Testing Workflows

### Test Categories

#### 1. Unit Tests
```bash
# Run all unit tests
nix develop --command cargo test

# Test specific crate
nix develop --command cargo test -p almanac-core
nix develop --command cargo test -p almanac-storage

# Test with features
nix develop --command cargo test -p almanac-chains --features ethereum,cosmos
```

#### 2. Integration Tests
```bash
# Run all integration tests
./scripts/testing/run-all-tests.sh

# Run specific test categories
./scripts/testing/api-tests.sh              # API tests only
./scripts/testing/fdb-tests.sh              # FoundationDB tests only
./scripts/testing/e2e-tests.sh              # E2E tests only
```

#### 3. End-to-End Tests
```bash
# Run complete E2E test suite (requires FoundationDB)
nix run .#run-e2e-tests

# Run specific E2E test module
nix develop .#fdb --command cargo test -p e2e fdb_data_validation_e2e -- --ignored --nocapture

# Run in FDB environment
nix develop .#fdb --command cargo test -p e2e
```

### Test Script Options

```bash
# Quick tests during development
./scripts/testing/run-all-tests.sh --quick

# Verbose output for debugging
./scripts/testing/run-all-tests.sh --verbose

# Dry run to see what would be executed
./scripts/testing/run-all-tests.sh --dry-run

# Test specific backend
./scripts/testing/api-tests.sh --backend memory
./scripts/testing/api-tests.sh --backend fdb

# Simulation tests only
./scripts/testing/e2e-tests.sh --simulation-only

# Components tests only
./scripts/testing/e2e-tests.sh --components-only
```

## Service Management

### Local Development Services

#### API Service Management
```bash
# Start API service
./scripts/services/manage-api-service.sh start

# Check status
./scripts/services/manage-api-service.sh status

# View logs
./scripts/services/manage-api-service.sh logs

# Stop service
./scripts/services/manage-api-service.sh stop

# Restart service
./scripts/services/manage-api-service.sh restart
```

#### Health Monitoring
```bash
# Single health check
./scripts/services/monitor-api-health.sh --once

# Continuous monitoring
./scripts/services/monitor-api-health.sh --continuous

# Monitor with custom interval
./scripts/services/monitor-api-health.sh --interval 30
```

#### Indexer Services
```bash
# Run Ethereum indexer
./scripts/services/run-almanac-indexer.sh --config config/ethereum.toml

# Run Cosmos indexer
./scripts/services/run-almanac-indexer.sh --config config/cosmos.toml

# Run with specific chain
./scripts/services/run-almanac-indexer.sh --chain ethereum --start-block 1000
```

### FoundationDB Management

```bash
# Start FoundationDB
nix run .#fdb-manage -- start

# Check status
nix run .#fdb-manage -- status

# Stop FoundationDB
nix run .#fdb-manage -- stop

# Restart FoundationDB
nix run .#fdb-manage -- restart

# Access FDB CLI
nix run .#fdb-manage -- cli

# Clear test data
nix run .#fdb-manage -- clear-test-data
```

## Development Environment Configuration

### Environment Variables

Common environment variables for development:

```bash
# Storage configuration
export STORAGE_BACKEND=foundationdb  # or "memory"
export FDB_CLUSTER_FILE=./data/fdb.cluster
export FDB_NAMESPACE=almanac

# API configuration
export API_HOST=0.0.0.0
export API_PORT=8080
export METRICS_PORT=9090
export ENABLE_CORS=true
export CORS_ORIGINS=*

# Logging
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Test configuration
export TEST_TIMEOUT=600
```

### Configuration Files

#### Development Configuration
```toml
# config/dev.toml
[api]
host = "127.0.0.1"
port = 8080
cors_enabled = true

[storage]
backend = "foundationdb"

[storage.fdb]
cluster_file = "./data/fdb.cluster"
namespace = "almanac_dev"

[[chains]]
chain_id = "ethereum"
rpc_url = "http://localhost:8545"
confirmations = 6
```

#### Chain-Specific Configuration
```toml
# config/ethereum.toml
[chain]
id = "ethereum"
confirmations = 12

[rpc]
url = "http://localhost:8545"
timeout_secs = 30
max_retries = 3

[indexing]
start_block = 0
batch_size = 100
interval_ms = 1000
```
