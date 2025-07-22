# Configuration Files

This directory contains the configuration files for different environments and chains:

**Environment Configurations:**
- `dev.toml`: Development environment configuration
- `test.toml`: Test environment configuration
- `prod.toml`: Production environment configuration

**Chain-Specific Configurations:**
- `ethereum.toml`: Ethereum chain indexing configuration
- `cosmos.toml`: Cosmos chain indexing configuration
- `solana.toml`: Solana chain indexing configuration
- `simulation.toml`: Multi-chain simulation environment configuration

## Usage

Pass the configuration file to the indexer using the `--config` parameter:

```bash
# Development API server
nix run .#almanac-api -- --config config/dev.toml

# Chain-specific indexing nodes
nix run .#almanac-node-ethereum -- --config config/ethereum.toml
nix run .#almanac-node-cosmos -- --config config/cosmos.toml
nix run .#almanac-node-solana -- --config config/solana.toml

# Multi-chain simulation
nix run .#sim-cluster -- --config config/simulation.toml
```

## Environment Variables

The production configuration file uses environment variables for sensitive data:

- `ETH_RPC_URL`: Ethereum RPC endpoint URL
- `COSMOS_RPC_URL`: Cosmos RPC endpoint URL

Make sure these environment variables are set before running the indexer with the production configuration.

## Configuration Sections

### API
Controls the HTTP API endpoints.

### Storage
Configures FoundationDB storage backend.

### Ethereum
Ethereum chain adapter configuration.

### Cosmos
Cosmos chain adapter configuration.

### Logging
Logging configuration for console and file output.

### Metrics
Prometheus metrics configuration. 