# Almanac Simulation Environment

This directory contains the Nix flake for running a complete Almanac multi-chain simulation environment. It provides a local blockchain cluster with FoundationDB storage for development and testing.

## Quick Start

### Initialize and Start the Simulation

```bash
# Initialize simulation with default chains (ethereum, cosmos, fdb)
nix run .#sim-cluster -- init

# Start all services
nix run .#sim-cluster -- start

# Check status of all services
nix run .#sim-cluster -- status
```

### Stop the Simulation

```bash
# Stop all services
nix run .#sim-cluster -- stop
```

## Available Commands

### Main Orchestrator Commands

```bash
# Show help
nix run .#sim-cluster -- help

# Initialize with specific chains
nix run .#sim-cluster -- init ethereum,cosmos,fdb
nix run .#sim-cluster -- init ethereum,solana,fdb
nix run .#sim-cluster -- init cosmos,solana,fdb

# Start all configured services
nix run .#sim-cluster -- start

# Stop all services  
nix run .#sim-cluster -- stop

# Check status of all services
nix run .#sim-cluster -- status

# View logs for specific services
nix run .#sim-cluster -- logs ethereum
nix run .#sim-cluster -- logs cosmos
nix run .#sim-cluster -- logs solana
nix run .#sim-cluster -- logs api
nix run .#sim-cluster -- logs indexer
nix run .#sim-cluster -- logs all
```

### Individual Service Commands

#### Configuration Generator
```bash
# Generate configuration for specific chains (default output: ./config/)
nix run .#sim-config -- ethereum,cosmos,fdb
```

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
# Almanac API Server
nix run .#sim-almanac-api -- start
nix run .#sim-almanac-api -- stop
nix run .#sim-almanac-api -- status

# Almanac Indexer Service
nix run .#sim-almanac-indexer -- start
nix run .#sim-almanac-indexer -- stop
nix run .#sim-almanac-indexer -- status
```

## Supported Chains

| Chain | Description | RPC Port | Additional Ports |
|-------|-------------|----------|------------------|
| ethereum | Anvil Ethereum node | 8545 | WS: 8546 |
| cosmos | Neutron Cosmos node | 26657 | gRPC: 9090, API: 1317 |
| solana | Solana test validator | 8899 | WS: 8900 |
| fdb | FoundationDB cluster | - | - |

## Service Endpoints

After starting the simulation, the following endpoints will be available:

- **Ethereum RPC**: http://localhost:8545
- **Cosmos RPC**: http://localhost:26657
- **Cosmos gRPC**: http://localhost:9090
- **Cosmos REST API**: http://localhost:1317
- **Solana RPC**: http://localhost:8899 (where supported)
- **Almanac API**: http://localhost:3000
  - Health check: http://localhost:3000/health
  - OpenAPI docs: http://localhost:3000/swagger-ui

## Directory Structure

The simulation creates the following directory structure:

```
./
├── config/
│   └── simulation.toml          # Main simulation configuration
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

## Platform Support

| Platform | Ethereum | Cosmos | Solana | FoundationDB |
|----------|----------|--------|--------|--------------|
| x86_64-linux | YES | YES | YES | YES |
| aarch64-linux | YES | YES | NO* | YES |
| x86_64-darwin | YES | YES | YES | YES |
| aarch64-darwin | YES | YES | YES | YES |

*Solana test validator is not available on aarch64-linux, but Solana development tools (anchor) are available.

## Environment Variables

The simulation sets up the following environment variables for Almanac services:

### API Server
- `API_HOST=0.0.0.0`
- `API_PORT=3000`
- `STORAGE_BACKEND=foundationdb`
- `FDB_CLUSTER_FILE=$PWD/data/fdb.cluster`
- `FDB_NAMESPACE=almanac`
- `ENABLE_CORS=true`
- `CORS_ORIGINS=*`
- `RUST_LOG=almanac_api=info,tower_http=debug`

### Indexer Service
- `RUST_LOG=almanac_indexing=info,almanac_core=debug`
- `ALMANAC_CONFIG=$PWD/config/simulation.toml`

## Troubleshooting

### Common Issues

1. **Services fail to start**: Check if ports are already in use
   ```bash
   # Check port usage
   lsof -i :8545  # Ethereum
   lsof -i :26657 # Cosmos
   lsof -i :8899  # Solana
   lsof -i :3000  # Almanac API
   ```

2. **FoundationDB not starting**: Ensure you have proper permissions and FoundationDB is properly installed
   ```bash
   # Check FoundationDB status
   nix run ..fdb#fdb-manage -- status
   ```

3. **Configuration not found**: Run the init command first
   ```bash
   nix run .#sim-cluster -- init
   ```

### Viewing Logs

Use the logs command to debug issues:
```bash
# View real-time logs for specific services
nix run .#sim-cluster -- logs ethereum
nix run .#sim-cluster -- logs cosmos
nix run .#sim-cluster -- logs api
nix run .#sim-cluster -- logs indexer
```

### Clean Restart

To completely reset the simulation:
```bash
# Stop all services
nix run .#sim-cluster -- stop

# Remove data and logs (optional)
rm -rf data/ logs/

# Reinitialize
nix run .#sim-cluster -- init
nix run .#sim-cluster -- start
```

## Development Workflow

### Typical Development Session

1. **Initialize the simulation**:
   ```bash
   nix run .#sim-cluster -- init ethereum,cosmos,fdb
   ```

2. **Start all services**:
   ```bash
   nix run .#sim-cluster -- start
   ```

3. **Check that everything is running**:
   ```bash
   nix run .#sim-cluster -- status
   ```

4. **Develop and test your application** using the available endpoints

5. **View logs as needed**:
   ```bash
   nix run .#sim-cluster -- logs api
   ```

6. **Stop when done**:
   ```bash
   nix run .#sim-cluster -- stop
   ```

### Testing Different Chain Combinations

```bash
# Ethereum + FoundationDB only
nix run .#sim-cluster -- init ethereum,fdb

# Cosmos + Solana + FoundationDB  
nix run .#sim-cluster -- init cosmos,solana,fdb

# All chains
nix run .#sim-cluster -- init ethereum,cosmos,solana,fdb
```

## Integration with Almanac

The simulation environment is designed to work seamlessly with Almanac's multi-chain indexing capabilities. The indexer service will automatically:

1. Connect to all configured blockchain nodes
2. Store indexed data in FoundationDB
3. Provide data through the API server
4. Handle chain reorganizations and finality

The API server provides REST endpoints for querying indexed blockchain data across all configured chains.