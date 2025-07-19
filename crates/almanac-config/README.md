# almanac-config

Configuration management for the Almanac multi-chain blockchain indexing system.

## Overview

This crate provides comprehensive configuration management with TOML file support, environment variable integration, and file watching capabilities. It handles configuration for the API server, storage backends, and blockchain connections.

## Key Components

### Configuration Structures
- `Config` - Main configuration container
- `ApiConfig` - HTTP API server settings
- `StorageConfig` - Storage backend configuration
- `ChainConfig` - Blockchain-specific connection settings
- `FdbConfig` - FoundationDB-specific configuration

### Features
- **TOML file parsing** with validation
- **Environment variable support** for sensitive data
- **File watching** for hot configuration reloading
- **Default values** for all configuration options
- **Chain-specific settings** for multiple blockchains

## Configuration Structure

```toml
[api]
host = "127.0.0.1"
port = 8080
cors_enabled = true
rate_limit_requests = 1000
rate_limit_window_secs = 60

[storage]
backend = "fdb"  # "memory" or "fdb"
max_connections = 10
connection_string = "postgresql://localhost/almanac"

[storage.fdb]
cluster_file = "/etc/foundationdb/fdb.cluster"
api_version = 720
transaction_timeout_ms = 5000
retry_limit = 3

[[chains]]
chain_id = "ethereum"
rpc_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
api_key = "your_api_key"
confirmations = 12
max_retries = 3
timeout_secs = 30

[[chains]]
chain_id = "cosmos"
rpc_url = "https://rpc.cosmos.network"
confirmations = 1
max_retries = 5
timeout_secs = 10
```

## Usage

```rust
use almanac_config::Config;

// Load configuration from file
let config = Config::from_file("config/dev.toml")?;

// Access configuration values
println!("API listening on {}:{}", config.api.host, config.api.port);
println!("Storage backend: {}", config.storage.backend);

// Iterate through chain configurations
for chain in &config.chains {
    println!("Chain: {} -> {}", chain.chain_id, chain.rpc_url);
}
```

## Environment Variables

Configuration values can be overridden with environment variables:

- `ALMANAC_HOST` - API server host
- `ALMANAC_PORT` - API server port
- `STORAGE_BACKEND` - Storage backend type
- `FDB_CLUSTER_FILE` - FoundationDB cluster file path
- `ETHEREUM_RPC_URL` - Ethereum RPC endpoint
- `COSMOS_RPC_URL` - Cosmos RPC endpoint

## Testing

Run tests with:
```bash
nix develop -c cargo test -p almanac-config
```

The test suite covers:
- TOML file parsing and validation
- Default value handling
- Environment variable integration
- Multi-chain configuration
- Error handling for invalid configurations

## Chain-Specific Settings

Each blockchain can have its own configuration:

- **Ethereum**: Higher confirmation requirements, longer timeouts
- **Cosmos**: Lower confirmations (instant finality), faster processing
- **Solana**: High-throughput settings, slot-based confirmations