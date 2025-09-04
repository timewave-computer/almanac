# Almanac

![](./almanac.png)

A high-performance, multi-chain blockchain indexing and querying system built in Rust with FoundationDB as the primary storage backend.

## Overview

Almanac is a distributed blockchain data indexing platform designed to efficiently track, store, and query events across multiple blockchain networks. It provides real-time indexing capabilities with strong consistency guarantees through FoundationDB's ACID transactions.

## Architecture

The system is built around several core components organized in a modular crate structure:

### Core Crates

- **[`almanac-core`](crates/almanac-core/)** - Core types, traits, and abstractions for events, storage, and filtering
- **[`almanac-storage`](crates/almanac-storage/)** - Storage layer with FoundationDB backend and memory implementations  
- **[`almanac-indexing`](crates/almanac-indexing/)** - Event ingestion pipeline with batch processing and validation
- **[`almanac-api`](crates/almanac-api/)** - REST API server with authentication and rate limiting
- **[`almanac-chains`](crates/almanac-chains/)** - Chain adapters for different blockchain protocols
- **[`almanac-consensus`](crates/almanac-consensus/)** - Consensus and finality handling logic
- **[`almanac-eventbus`](crates/almanac-eventbus/)** - Event bus system for internal communication
- **[`almanac-config`](crates/almanac-config/)** - Configuration management
- **[`almanac-testing`](crates/almanac-testing/)** - Testing utilities and fixtures

### Chain Support

Almanac supports multiple blockchain networks through dedicated node services:

- **[`almanac-nodes/ethereum`](crates/almanac-nodes/ethereum/)** - Ethereum indexing node
- **[`almanac-nodes/cosmos`](crates/almanac-nodes/cosmos/)** - Cosmos indexing node
- **[`almanac-nodes/solana`](crates/almanac-nodes/solana/)** - Solana indexing node

Each chain adapter implements the [`ChainAdapter`](crates/almanac-chains/src/adapters/traits.rs) trait, providing a unified interface for blockchain integration.

## Features

- **Multi-chain Support** - Index events from Ethereum, Cosmos, and Solana networks
- **FoundationDB Storage** - ACID transactions with strong consistency guarantees
- **Real-time Indexing** - Low-latency event processing with configurable batch sizes
- **REST API** - Comprehensive API for querying indexed data
- **Anchor IDL Indexing** - Automatic detection and storage of Solana Anchor program IDLs
- **Authentication** - API key-based authentication with configurable permissions
- **Rate Limiting** - Built-in rate limiting to protect against abuse
- **Reorg Handling** - Automatic blockchain reorganization detection and handling
- **Distributed Architecture** - Designed for horizontal scaling across multiple nodes
- **Nix Integration** - Complete Nix flake for reproducible builds and deployments
- **Health Monitoring** - Built-in health checks and Prometheus metrics

## Storage Backend

Almanac uses FoundationDB as its primary storage backend, providing:

- **ACID Transactions** - Strong consistency across all operations
- **Horizontal Scaling** - Automatic sharding and replication
- **Tuple Space Model** - Efficient key-value storage with complex queries
- **Change Tracking** - Built-in change streams for real-time updates
- **Storage Proofs** - Cryptographic proofs for data integrity

The storage layer also includes a memory backend for testing and development.

## Configuration

Configuration is managed through TOML files with environment-specific overrides:

- [`config/dev.toml`](config/dev.toml) - Development configuration
- [`config/prod.toml`](config/prod.toml) - Production configuration
- [`config/prod-historical.toml`](config/prod-historical.toml) - Historical data configuration
- [`config/nixos-server.toml`](config/nixos-server.toml) - NixOS server configuration  
- [`config/test.toml`](config/test.toml) - Test configuration
- [`config/cosmos.toml`](config/cosmos.toml) - Cosmos-specific settings
- [`config/ethereum.toml`](config/ethereum.toml) - Ethereum-specific settings
- [`config/solana.toml`](config/solana.toml) - Solana-specific settings
- [`config/simulation.toml`](config/simulation.toml) - Multi-chain simulation settings

## Development Setup

### Prerequisites

- Rust 1.75+
- FoundationDB 7.1+ (optional - can use memory backend for development)
- Nix with flakes enabled
- Just command runner (optional - provides convenience commands)

### Using Nix

```bash
# Enter development shell
nix develop

# Build all crates (simplified - no longer requires nix wrapper)
cargo build --workspace

# Run tests  
just test

# Check nix flake
nix flake check
```

## Running Services

### API Server

```bash
# Run API server via development shell
nix develop
cargo run --bin almanac-api

# Alternative - run directly without nix
cargo run --bin almanac-api
```

The API server supports both memory and FoundationDB storage backends, configured via environment variables:

```bash
export STORAGE_BACKEND=foundationdb  # or memory
export FDB_CLUSTER_FILE=/path/to/fdb.cluster
export FDB_NAMESPACE=almanac
export API_HOST=127.0.0.1
export API_PORT=8080
```

### Simulation Environment

```bash
# Use the integrated simulation environment (recommended)
cd nix/simulation
nix develop

# Initialize with all blockchain services
sim-cluster init

# Start services (shows platform-specific commands)
sim-cluster start

# Check service status
sim-cluster status

# Or run individual indexers via development shell
nix develop
cargo run --bin almanac-node-ethereum -- --config config/ethereum.toml
cargo run --bin almanac-node-cosmos -- --config config/cosmos.toml
```

### Using Just Commands

The project uses `just` as a command runner with various automated tasks:

```bash
# Initialize project directories
just init

# Build the project
just build

# Run tests
just test

# Start/stop blockchain services
just start-fdb          # Start FoundationDB
just start-ethereum     # Start Ethereum (Anvil)
just start-cosmos       # Start Cosmos (Neutron)
just start-solana       # Start Solana test validator

# Stop all services
just stop

# Run simulation environment  
just sim-quick-start    # Quick start integrated multi-chain simulation
```

## API Usage

The REST API provides endpoints for querying indexed blockchain data:

### Authentication

```bash
# Set API key (if authentication is enabled)
export API_KEY="your-api-key"
```

### Query Events

```bash
# Get events for a specific contract
curl -X GET "http://localhost:8080/api/v1/events?address=0x..." \
  -H "X-API-Key: $API_KEY"

# Query events with filters
curl -X GET "http://localhost:8080/api/v1/events?from_block=1000000&to_block=1000100" \
  -H "X-API-Key: $API_KEY"

# Get events by chain
curl -X GET "http://localhost:8080/api/v1/chains/ethereum/events" \
  -H "X-API-Key: $API_KEY"
```

### Storage and Semantics

```bash
# Get semantic interpretations
curl -X GET "http://localhost:8080/api/v1/chains/ethereum/contracts/0x.../semantics/field_name" 
  -H "X-API-Key: $API_KEY"

# Submit semantic interpretation
curl -X POST "http://localhost:8080/api/v1/chains/ethereum/contracts/0x.../semantics/field_name" 
  -H "X-API-Key: $API_KEY" 
  -H "Content-Type: application/json" 
  -d '{"interpretation": {"semantic_tags": ["balance"], "confidence": "high"}}'

# Generate traverse request (storage proof)
curl -X POST "http://localhost:8080/api/v1/traverse/request/ethereum/0x..." 
  -H "X-API-Key: $API_KEY"
```

### Health Monitoring

```bash
# API health check
curl http://localhost:8080/health

# Node health (varies by port)
curl http://localhost:9092/health  # Ethereum node
curl http://localhost:9093/health  # Cosmos node
```

## Deployment

### Nix Deployment

```bash
# Build release binaries
cargo build --release --bin almanac-api
cargo build --release --bin almanac-node-ethereum  
cargo build --release --bin almanac-node-cosmos
cargo build --release --bin almanac-node-solana
```

### Manual Deployment

1. Build release binaries:
   ```bash
   # Build all release binaries
   cargo build --release
   ```

2. Set up FoundationDB cluster

3. Configure environment variables

4. Run services with appropriate configurations

## Testing

The project includes comprehensive testing at multiple levels:

### Unit Tests
```bash
# Run all unit tests (using just command)
just test

# Or run directly with cargo
nix develop -c cargo test

# Test specific crates
cargo test --package almanac-storage
cargo test --package almanac-api
```

### Integration Tests
```bash
# Run integration tests with specific chains
cargo test --package almanac-chains --features ethereum,cosmos -- --ignored

# E2E tests
cargo test --package tests
```

### Environment Tests
```bash
# Test individual blockchain environments
cd tests/environment/ethereum && ./test.sh
cd tests/environment/cosmos && ./test.sh  
cd tests/environment/solana && ./test.sh

# Test simulation environment
cd nix/simulation && nix develop -c sim-cluster status
```

## Monitoring

Almanac includes built-in monitoring and observability:

- **Health Checks** - Service health endpoints at `/health` and `/ready`
- **Metrics** - Prometheus-compatible metrics on port 9090
- **Logging** - Structured logging with configurable levels

## Configuration

### Environment Variables

Key environment variables for configuration:

```bash
# Storage
STORAGE_BACKEND=foundationdb|memory
FDB_CLUSTER_FILE=/path/to/fdb.cluster
FDB_NAMESPACE=almanac

# API
API_HOST=127.0.0.1
API_PORT=8080
ADMIN_API_KEY=your-admin-key
RATE_LIMIT_PER_MINUTE=100

# Logging
RUST_LOG=info
RUST_BACKTRACE=1
```

### Chain Configuration

Each chain requires specific configuration in its TOML file:

```toml
[chain]
id = "ethereum-mainnet"
confirmations = 6

[rpc]
url = "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY"
timeout_secs = 30
max_retries = 3

[indexing]
start_block = 18000000
batch_size = 100
max_concurrent_batches = 5

[storage.fdb]
cluster_file = "/etc/foundationdb/fdb.cluster"
```

## Documentation

- [Architecture Overview](docs/001_system_architecture.md)
- [Running and Configuration](docs/002_running_and_configuration.md)
- [FoundationDB Setup](docs/003_foundationdb_setup_and_usage.md)
- [FoundationDB Implementation](docs/101_foundationdb_tuple_storage_implementation.md)
- [Chain Adapter Guide](docs/103_blockchain_chain_adapter_development.md)
- [Integration Guide](docs/104_api_integration_examples.md)
- [Testing Guide](docs/201_comprehensive_testing_guide.md)
- [API Documentation](docs/301_querying_blockchain_data_api.md)
- [Swagger UI Setup](docs/302_api_documentation_swagger_ui.md)
- [Development Workflows](docs/401_development_workflows.md)
- [Deployment and Monitoring](docs/501_deployment_and_monitoring.md)

### Environment-Specific Documentation
- [Historical State Management](docs/historical-state-management.md)
- [Swagger UI NixOS Setup](docs/swagger-ui-nixos-setup.md)
- [Solana Geyser Plugin](docs/nix-environments/solana/README-geyser.md)

---

*Cover image from [Gaine's New-York pocket almanack for the year 1789](https://www.loc.gov/resource/rbc0001.2022madison98629)*