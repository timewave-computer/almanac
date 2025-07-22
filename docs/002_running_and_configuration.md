# Running Almanac

This guide covers how to build, configure, and run the Almanac multi-chain indexing system.

## Architecture Overview

Almanac uses a distributed microservices architecture where chain-specific indexing nodes and an API server connect to a shared FoundationDB data layer:

```
┌─────────────────────────────────────────────────────────────────┐
│                         External Clients                        │
│         • Web Apps  • CLI Tools  • Dashboards                   │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼ HTTP/WebSocket
┌─────────────────────────────────────────────────────────────────┐
│                     almanac-api                                 │
│  • HTTP API Server        • WebSocket Support                   │
│  • Authentication         • Rate Limiting                       │
│  • OpenAPI Documentation  • Response Caching                    │
│  • Read-only operations   • Query optimization                  │
└─────────────────────────┬───────────────────────────────────────┘
                          │ Reads
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                  FoundationDB Cluster                           │
│  • Distributed ACID transactions                                │
│  • Automatic replication and sharding                           │
│  • Multi-dimensional tuple indexing                             │
│  • 13 keyspaces for multi-chain data                            │
└─────────────────────────▲───────────────────────────────────────┘
                          │ Writes
       ┌──────────────────┼───────────────────┐
       │                  │                   │
┌──────▼──────────┐ ┌─────▼──────────┐ ┌──────▼──────────┐
│ almanac-ethereum│ │almanac-cosmos  │ │almanac-solana   │
│     :9091       │ │     :9092      │ │    :9093        │
├─────────────────┤ ├────────────────┤ ├─────────────────┤
│ • Event indexing│ │• Event indexing│ │• Account data   │
│ • Reorg handling│ │• BFT consensus │ │• PoH validation │
│ • State proofs  │ │• IBC tracking  │ │• Program logs   │
│ • EVM traces    │ │• Module events │ │• Slot tracking  │
│ • Health :9091  │ │• Health :9092  │ │• Health :9093   │
│ • Metrics       │ │• Metrics       │ │• Metrics        │
└─────────────────┘ └────────────────┘ └─────────────────┘
        │                    │                  │
        ▼ RPC                ▼ RPC              ▼ RPC
┌─────────────────────────────────────────────────────────────────┐
│                      Blockchain Networks                        │
│  • Ethereum/Anvil       • Cosmos/Neutron      • Solana          │
│  • :8545                • :26657               • :8899          │
└─────────────────────────────────────────────────────────────────┘
```

## Prerequisites

### System Requirements
- **Operating System**: Linux, macOS, or Windows with WSL2
- **Memory**: Minimum 16GB RAM for full cluster, 32GB recommended
- **Storage**: 100GB+ available disk space (SSD recommended)
- **CPU**: 8+ cores recommended for production
- **Network**: Low-latency network between nodes

### Software Dependencies
- **Rust**: 1.70+ (install via [rustup](https://rustup.rs/))
- **Nix**: (recommended) for development environment
- **FoundationDB**: 7.1+ (optional, for persistent storage)

### Development Setup
Using Nix (recommended):

```bash
# Clone repository
git clone https://github.com/timewave-computer/almanac
cd almanac

# Enter development shell
nix develop

# For FoundationDB development
nix develop .#fdb
```

Manual setup:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install FoundationDB (optional)
# See docs/003_foundationdb_setup_and_usage.md for detailed instructions
```

## Building Almanac

### Build Commands

```bash
# Build all crates (use nix develop)
nix develop --command cargo build --all

# Build with specific features
nix develop --command cargo build --all --features ethereum,cosmos

# Build with FoundationDB support
nix develop .#fdb --command cargo build --all --features fdb

# Build API server only
nix develop --command cargo build --bin almanac-api

# Release build
nix develop --command cargo build --all --release
```

### Run Tests

```bash
# Run all tests
nix develop --command cargo test

# Run e2e tests (requires FoundationDB)
nix run .#run-e2e-tests

# Run specific crate tests
nix develop --command cargo test -p almanac-core
```

## Configuration

Almanac uses TOML configuration files in the `config/` directory:

### Main Configuration Structure
```toml
[simulation]
name = "almanac-dev-cluster"
chains = ["ethereum", "cosmos", "fdb"]

[almanac]
name = "development"
api_port = 8080
api_log_file = "./logs/almanac-api.log"

[storage]
backend = "foundationdb"  # or "memory"

[storage.fdb]
cluster_file = "/usr/local/etc/foundationdb/fdb.cluster"
namespace = "almanac_dev"

# Service configurations (optional)
[services.foundationdb]
enabled = true
cluster_file = "/usr/local/etc/foundationdb/fdb.cluster"

[services.ethereum]
enabled = true
node_type = "anvil"
rpc_port = 8545

# Chain-specific configurations
[[chains.ethereum]]
name = "ethereum-dev"
rpc_url = "http://localhost:8545"
chain_id = 1337
starting_block = 0
batch_size = 50

[[chains.cosmos]]
name = "cosmos-dev"
rpc_url = "http://localhost:26657"
chain_id = "ufo-local-1"
starting_block = 0
batch_size = 50
```

### Available Configuration Files
- `config/dev.toml` - Development environment
- `config/test.toml` - Test environment  
- `config/prod.toml` - Production environment
- `config/simulation.toml` - Local simulation

## Running Almanac

### Development Environment

#### Using Nix (Recommended)
```bash
# Start FoundationDB and run API
nix develop .#fdb
cargo run --bin almanac-api --features fdb

# Or run tests
nix run .#run-e2e-tests
```

#### Manual Setup
```bash
# Start FoundationDB (if using persistent storage)
nix run .#fdb-manage -- start

# Set storage backend via environment
export STORAGE_BACKEND=foundationdb
export FDB_CLUSTER_FILE=./data/fdb.cluster
export FDB_NAMESPACE=almanac

# Run API server
cargo run --bin almanac-api --features fdb

# Or use memory storage for development
export STORAGE_BACKEND=memory
cargo run --bin almanac-api
```

### Using Configuration Files
```bash
# Run with specific config
nix develop --command cargo run --bin almanac-api -- --config config/dev.toml

# Override specific settings
ALMANAC_API_PORT=8081 nix develop --command cargo run --bin almanac-api
```

### Health Checks
```bash
# Check API health
curl http://localhost:8080/health

# Check API liveness
curl http://localhost:8080/liveness

# Check API readiness
curl http://localhost:8080/readiness

# Check specific endpoints
curl http://localhost:8080/api/v1/events
curl http://localhost:8080/swagger-ui
curl http://localhost:8080/openapi.json
```

## API Endpoints

### Core Endpoints
- `GET /health` - Service health check
- `GET /liveness` - Liveness probe
- `GET /readiness` - Readiness probe
- `GET /swagger-ui` - Interactive API documentation
- `GET /openapi.json` - OpenAPI specification
- `GET /api/v1/events` - Query blockchain events
- `GET /api/v1/events/{id}` - Get specific event
- `GET /api/v1/chains/{chain}/events` - Chain-specific events
- `GET /api/v1/chains/{chain}/latest-block` - Latest block info

### Authentication
Some endpoints may require API key authentication. Include your API key in the `X-API-Key` header:

```bash
export API_KEY="your-api-key"
curl -H "X-API-Key: $API_KEY" "http://localhost:8080/api/v1/events"
```

## Troubleshooting

### Common Issues

**Build Errors**
```bash
# Clean build cache
nix develop --command cargo clean

# Update dependencies
nix develop --command cargo update

# Check specific crate
nix develop --command cargo check -p almanac-api
```

**FoundationDB Connection Issues**
```bash
# Check FDB status
nix run .#fdb-manage -- status

# Restart FDB
nix run .#fdb-manage -- restart

# Check cluster file
cat ./data/fdb.cluster
```

**Configuration Errors**
```bash
# Validate configuration
nix develop --command cargo run -p almanac-config --example load_config

# Check environment variables
env | grep ALMANAC
env | grep FDB
```

### Development Tips

- Use `RUST_LOG=debug` for verbose logging
- Use memory storage for faster iteration: `export STORAGE_BACKEND=memory`
- Check `docs/003_foundationdb_setup_and_usage.md` for FDB-specific setup
- Run `nix develop -c cargo clippy` and `nix develop -c cargo fmt` before committing
- Use `nix develop` for consistent development environment

For more detailed information, see:
- `docs/003_foundationdb_setup_and_usage.md` - FoundationDB setup and usage
- `docs/201_comprehensive_testing_guide.md` - Testing guide and examples
- `docs/104_api_integration_examples.md` - API integration examples
