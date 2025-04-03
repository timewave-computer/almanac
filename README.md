# Ethereum & UFO Node Integration

This project integrates an Ethereum node with a faucet contract and a UFO (Universal Fast Orderer) node for Cosmos SDK chains. Both nodes include faucet functionality with the same API. The entire system is packaged as a Nix flake for reproducible builds and easy development.

## Features

### Ethereum Node
- Local Ethereum node (Anvil) with preconfigured accounts
- Faucet contract for token issuance and management
- Scripts for deploying contracts and minting tokens
- End-to-end testing capabilities

### UFO Node
- Integration with Osmosis blockchain
- Multiple build modes (patched, bridged, fauxmosis)
- Faucet for token issuance (same API as Ethereum faucet)
- Benchmarking capabilities
- Customizable validators and block times

## Getting Started

### Prerequisites
- [Nix](https://nixos.org/download.html) with flakes enabled
- Git
- (Optional) Go 1.22+ for UFO development

### Installation

Clone the repository:

```bash
git clone https://github.com/your-org/your-repo.git
cd your-repo
```

Enter the development shell:

```bash
nix develop
```

### Running the Ethereum Node

Start the Ethereum node:

```bash
nix run .#start-anvil
```

Deploy the faucet contract:

```bash
nix run .#deploy-contract
```

Mint tokens to an address:

```bash
nix run .#mint-tokens 0x70997970C51812dc3A010C7d01b50e0d17dc79C8 100
```

### Running the UFO Node

Before running a patched UFO node, you need to have the Osmosis source code:

```bash
git clone https://github.com/osmosis-labs/osmosis.git /tmp/osmosis-source
```

Build Osmosis with UFO integration:

```bash
nix run .#build-osmosis-ufo
```

Start the UFO node:

```bash
nix run .#run-ufo-node
```

You can customize the UFO node with command-line arguments:

```bash
nix run .#run-ufo-node -- --build-mode bridged --validators 4 --block-time 100
```

Mint UFO tokens to an address:

```bash
nix run .#ufo-mint-tokens osmo1exampleaddress 100
```

### Running Both Nodes

Start both nodes simultaneously:

```bash
nix run .#run-all-nodes
```

## Configuration

The UFO integration is configured in the `flake.nix` file under the `ufo` section:

```nix
ufo = {
  osmosisSource = "/tmp/osmosis-source"; # Path to the Osmosis source code
  buildMode = "patched";                 # Integration mode: patched, bridged, or fauxmosis
  validators = 1;                        # Number of validators
  blockTimes = [ 1000 100 10 1 ];        # Block times in milliseconds for benchmarking
  faucet = {
    enabled = true;                      # Enable the faucet functionality
    initialSupply = 1000000;             # Initial token supply
    tokenName = "UFO";                   # Token name
    tokenSymbol = "UFO";                 # Token symbol
  };
};
```

## Integration Modes

UFO supports multiple integration modes:

1. **Patched Mode**: Direct modification of Osmosis to use UFO instead of CometBFT
2. **Bridged Mode**: UFO and Osmosis run as separate processes with adapter interfaces
3. **Fauxmosis Mode**: Lightweight mock Cosmos SDK application for testing

## Development

The project uses Nix for dependency management and reproducible builds. All scripts are in the `scripts` directory, and the main configuration is in `flake.nix`.

For development, use the provided shell scripts or the Nix commands directly. The system is designed to be modular and extensible.

## Project Structure

```
├── contracts/           # Solidity contracts
│   └── Faucet.sol       # Faucet contract
├── scripts/             # Utility scripts
│   ├── deploy-contract.sh   # Contract deployment
│   ├── e2e-test.sh          # Ethereum end-to-end test
│   ├── mint-tokens.sh       # Ethereum token minting
│   ├── run-node.sh          # Ethereum node runner
│   ├── run-ufo-node.sh      # UFO node runner
│   ├── ufo-mint-tokens.sh   # UFO token minting
│   └── ufo-e2e-test.sh      # UFO end-to-end test
├── docs/                # Documentation
│   └── architecture.md  # System architecture
├── nix/                 # Nix modules
│   ├── ethereum-module.nix  # Ethereum module
│   └── ufo-module.nix       # UFO module
├── flake.nix            # Main Nix flake
└── README.md            # This file
```

## Available Commands

### Ethereum Commands
- `

# Cross-Chain Indexer

A high-performance cross-chain indexer supporting Ethereum and Cosmos chains with future Causality integration capability.

## Quick Start

To build and check the codebase, use the provided cargo-check.sh script which ensures proper environment variables are set:

```bash
# Run with all features
./scripts/cargo-check.sh

# Run with specific features
./scripts/cargo-check.sh --features "sqlx-postgres"
```

## Development Environment

The project uses environment variables to configure the build process and connections to databases and nodes:

- `MACOSX_DEPLOYMENT_TARGET=11.0` - Required for building on macOS
- `SOURCE_DATE_EPOCH=1672531200` - Ensures reproducible builds
- `DATABASE_URL="postgresql://postgres:postgres@localhost:5432/indexer"` - PostgreSQL connection
- `ROCKSDB_PATH="./data/rocksdb"` - Path for RocksDB storage
- `ETH_RPC_URL="http://localhost:8545"` - Ethereum node connection
- `COSMOS_RPC_URL="http://localhost:26657"` - Cosmos node connection

These variables are automatically set by the cargo-check.sh script.

## Project Structure

The project is organized as a Rust workspace with multiple crates:

- **crates/core**: Core types, traits, and utilities used across the project
- **crates/ethereum**: Ethereum chain adapter for indexing Ethereum chains
- **crates/cosmos**: Cosmos chain adapter for indexing Cosmos chains
- **crates/storage**: Storage implementations for both PostgreSQL and RocksDB
- **crates/api**: API servers (GraphQL, REST) for querying indexed data

## Database Setup

The project uses PostgreSQL for relational data storage and SQLx for compile-time validated SQL:

1. Make sure PostgreSQL is running locally
2. Create a database named `indexer`:
   ```bash
   createdb indexer
   ```
3. Update the `DATABASE_URL` environment variable if necessary

## Running Tests

Run tests using the cargo-check.sh script:

```bash
./scripts/cargo-check.sh test
```

## Documentation

- [Work Plan](./work/00.md) - Detailed development plan and roadmap
- [Indexer Architecture](./docs/indexer_architecture.md) - Core requirements and architectural design
- [ADRs](./docs/adr_*.md) - Architecture Decision Records

## License

MIT OR Apache-2.0