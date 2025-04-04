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

## Project Overview

This indexer is designed to track state across Ethereum and Cosmos chains, providing both high-performance access paths via RocksDB for real-time operations and rich query capabilities via PostgreSQL for complex analytics.

## Features

- Multi-chain support (Ethereum and Cosmos)
- High-performance state tracking
- RocksDB for real-time data access
- PostgreSQL for rich query capabilities
- Chain reorganization handling
- Block finality tracking for Ethereum (confirmed, safe, justified, finalized)
- Event classification for Cosmos chains
- Test node configuration for both Ethereum (Anvil) and Cosmos (UFO)
- Live node integration for real networks

## Development

This project uses Nix for development environment management. All dependencies, tools, and runtime components are defined and managed through Nix to ensure reproducibility across development, CI/CD, and production environments.

### Getting Started

1. Install Nix (if not already installed):
```
curl -L https://nixos.org/nix/install | sh
```

2. Enable Flakes (if not already enabled):
```
# Add to ~/.config/nix/nix.conf
experimental-features = nix-command flakes
```

3. Clone the repository:
```
git clone <repository-url>
cd almanac
```

4. Enter the development shell:
```
nix develop
```

This will set up a complete development environment with all necessary dependencies.

### Available Commands

The following commands are available within the Nix development shell:

- `nix run .#start-postgres` - Start PostgreSQL server
- `nix run .#start-anvil` - Start Ethereum test node (Anvil)
- `nix run .#run-ufo-node` - Start UFO node
- `nix run .#deploy-contracts` - Deploy test contracts to Anvil
- `nix run .#mint-tokens` - Mint tokens to an Ethereum address
- `nix run .#ufo-mint-tokens` - Mint tokens to a Cosmos address
- `nix run .#e2e-test` - Run Ethereum end-to-end test
- `nix run .#ufo-e2e-test` - Run UFO end-to-end test
- `nix run .#prepare-sqlx` - Prepare SQL migrations for sqlx
- `nix run .#setup-test-nodes` - Configure test nodes for development
- `nix run .#test-nodes` - Test node configuration
- `nix run .#connect-live-nodes` - Test connection to live network nodes
- `nix run .#run-all-nodes` - Start all nodes for development

### Testing

To run tests:

```
cargo test
```

For end-to-end tests with live nodes:

```
nix run .#e2e-test
nix run .#ufo-e2e-test
```

### Development Workflow

1. Enter the development environment: `nix develop`
2. Start the required services: `nix run .#run-all-nodes`
3. Run the application or tests as needed
4. Make changes to the codebase
5. Verify changes with tests

## Architecture

The indexer follows a modular architecture with these key components:

1. **Chain Adapters** - Provide a unified interface to different blockchain networks
2. **Storage Layer** - Manages data persistence across RocksDB and PostgreSQL
3. **Indexing Pipeline** - Processes blockchain data efficiently
4. **API Layer** - Exposes data through GraphQL and REST endpoints
5. **Block Finality Tracking** - Monitors and tracks different levels of block finality:
   - **Confirmed**: Recently mined blocks that may still be reorganized
   - **Safe**: Blocks with enough attestations to be considered unlikely to be reorganized
   - **Justified**: Blocks voted on by validators in the current epoch (Ethereum PoS)
   - **Finalized**: Blocks that are permanently confirmed and cannot be reorganized

See the [architecture documentation](docs/indexer_architecture.md) and [Ethereum finality documentation](docs/src/ethereum_finality.md) for more details.

## Project Status

The project is actively under development. See [work plan](work/00.md) for details on the current status and upcoming work.

## License

[Insert license information here]