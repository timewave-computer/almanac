# almanac-config

Configuration management for the Almanac multi-chain blockchain indexing system.

## Overview

This crate provides comprehensive configuration management with TOML file support, environment variable integration, and file watching capabilities. It handles configuration for the API server, storage backends, and blockchain connections.

## Key Components

- `Config` - Main configuration container with nested sections for API, Storage, Chains, and Services.

### Features
- **TOML file parsing** with validation
- **Environment variable support** for sensitive data
- **File watching** for hot configuration reloading
- **Default values** for all configuration options
- **Chain-specific settings** for multiple blockchains

## Configuration Structure

```toml
[almanac]
name = "almanac-dev"
api_port = 8080
api_log_file = "./logs/almanac-api.log"

[storage]
backend = "memory" # "memory" or "foundationdb"

[storage.fdb]
cluster_file = "/etc/foundationdb/fdb.cluster"
namespace = "almanac_dev"

[[chains]]
chain_id = "ethereum"
rpc_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
confirmations = 12
max_retries = 3
timeout_secs = 30

[[chains]]
chain_id = "cosmos"
rpc_url = "https://rpc.cosmos.network"
confirmations = 1
max_retries = 5
timeout_secs = 10

[services.foundationdb]
enabled = true
cluster_file = "/etc/foundationdb/fdb.cluster"

[services.ethereum]
enabled = true
node_type = "anvil"
rpc_port = 8545
```

## Usage

```rust
use almanac_config::Config;

// Load configuration from a file (e.g., config/dev.toml)
let config = Config::load("config/dev.toml")?;

// Access configuration values
println!("Almanac instance name: {}", config.almanac.name);
println!("API listening on port: {}", config.almanac.api_port);
println!("Storage backend: {}", config.storage.backend);

// Iterate through chain configurations
for chain in &config.chains {
    println!("Chain: {} -> {}", chain.chain_id, chain.rpc_url);
}
```

## Environment Variables

Configuration values can be overridden with environment variables (prefixed with `ALMANAC_` or `FDB_`):

- `ALMANAC_ALMANAC_NAME` - Almanac instance name
- `ALMANAC_ALMANAC_API_PORT` - API server port
- `ALMANAC_STORAGE_BACKEND` - Storage backend type
- `FDB_CLUSTER_FILE` - FoundationDB cluster file path
- `ALMANAC_CHAINS_ETHEREUM_RPC_URL` - Ethereum RPC endpoint
- `ALMANAC_CHAINS_COSMOS_RPC_URL` - Cosmos RPC endpoint

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