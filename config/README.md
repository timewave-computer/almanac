# Configuration Files

This directory contains the configuration files for different environments:

- `dev.toml`: Development environment configuration
- `test.toml`: Test environment configuration
- `prod.toml`: Production environment configuration

## Usage

Pass the configuration file to the indexer using the `--config` parameter:

```bash
# Development
cargo run --bin almanac -- run --config config/dev.toml

# Test
cargo run --bin almanac -- run --config config/test.toml

# Production
cargo run --bin almanac -- run --config config/prod.toml
```

## Environment Variables

The production configuration file uses environment variables for sensitive data:

- `POSTGRES_PASSWORD`: Password for PostgreSQL database
- `ETH_RPC_URL`: Ethereum RPC endpoint URL
- `COSMOS_RPC_URL`: Cosmos RPC endpoint URL

Make sure these environment variables are set before running the indexer with the production configuration.

## Configuration Sections

### API
Controls the HTTP and GraphQL API endpoints.

### Storage
Configures RocksDB and PostgreSQL storage backends.

### Ethereum
Ethereum chain adapter configuration.

### Cosmos
Cosmos chain adapter configuration.

### Logging
Logging configuration for console and file output.

### Metrics
Prometheus metrics configuration. 