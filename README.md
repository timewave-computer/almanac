# Almanac: Cross-Chain Indexer

A performant indexer designed to track Valence, Causality and associated contract state across multiple chains. Almanac currently supports Ethereum and Cosmos chains using Sam's branch of the `valence-domain-clients` library.

The system includes a comprehensive causality indexing module that provides Sparse Merkle Tree (SMT) based tracking of causal relationships between events, resources, and entities across blockchain domains.

![](./almanac.png)

## Project Overview

Almanac enables tracking Valence programs and the state associated with related contracts across different blockchains. The system employs a hybrid storage architecture using RocksDB for high-performance paths and PostgreSQL for complex queries.

Key features:
- Multi-chain support: EVM chains (Ethereum, Polygon, Base) and Cosmos chains (Noble, Osmosis, Neutron)
- Hybrid storage: RocksDB for performance, PostgreSQL for complex queries
- Valence contract tracking: Account creation, processor state, authorization grants, library deployments
- Causality indexing: Content-addressed entity tracking with SMT-based proof generation
- Advanced blockchain features: Chain reorganization handling, block finality tracking, determinism classification
- Cross-chain debugging: Multi-chain traces and causal relationship tracking

## Getting Started

Clone the repository and enter the development environment:

```bash
git clone <repository-url>
cd almanac
nix develop
```

The development shell provides:
- Rust toolchain and crate2nix for reproducible builds
- PostgreSQL database server and RocksDB
- Foundry (anvil, forge, cast) for Ethereum development
- All required system libraries and build tools

Available commands:
- `init_databases` - Initialize and start databases
- `stop_databases` - Stop PostgreSQL server  
- `generate_cargo_nix` - Generate Cargo.nix for Nix builds
- `run_almanac_tests` - Run the test suite

## Build and Run

Generate the Nix build configuration:
```bash
generate_cargo_nix
```

Build components using Nix:
```bash
# Build the main almanac binary
nix build .#indexer-api

# Build specific workspace crates
nix build .#indexer-core
nix build .#indexer-ethereum
nix build .#indexer-cosmos
```

## Development Workflows

Select and run development workflows:

```bash
cd nix/environments
nix run .  # Interactive menu
```

Or run specific workflows directly:
```bash
# Ethereum development workflows
nix run ./nix/environments#anvil-workflow
nix run ./nix/environments#reth-workflow

# Cosmos development workflow
nix run ./nix/environments#cosmwasm-workflow

# Run all workflows
nix run ./nix/environments#all-workflows
```

For more details, see the [workflows documentation](nix/environments/README.md).

## Testing

Run tests using Nix commands:

```bash
# Integration tests
nix run .#test-ethereum-adapter-anvil
nix run .#test-cosmos-adapter
nix run .#cross-chain-e2e-test

# Comprehensive test suite
run_almanac_tests

# Unit tests (in dev shell)
cargo test --package indexer-ethereum
cargo test --package indexer-cosmos
cargo test --package indexer-causality
```

## Available Nix Commands

Cosmos:
- `nix run .#wasmd-node` - Start local wasmd node
- `nix run .#test-cosmos-adapter` - Test cosmos adapter

Ethereum:
- `nix run .#start-anvil` - Start Anvil test node
- `nix run .#start-reth` - Start Reth node
- `nix run .#test-ethereum-adapter-anvil` - Test against Anvil
- `nix run .#test-ethereum-adapter-reth` - Test against Reth

## Architecture

```
┌───────────────────────────────────────────────────────────────────┐
│                        Cross-Chain Indexer                        │
│                                                                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐  │
│  │Chain        │ │Indexing     │ │Storage      │ │Query        │  │
│  │Adapters     │ │Pipeline     │ │Layer        │ │Engine       │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘  │
│                                                                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐  │
│  │Schema       │ │Cross-Chain  │ │Causality    │ │Extension    │  │
│  │Registry     │ │Aggregator   │ │Indexer      │ │Framework    │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘  │
└───────────────────────────────────────────────────────────────────┘
```

### Storage Design

```
┌─────────────────────────────────────────────────┐
│              Hybrid Storage Layer               │
└───────────────┬─────────────────────┬───────────┘
                │                     │
    ┌───────────▼────────┐   ┌────────▼──────────┐
    │  RocksDB Layer     │   │  PostgreSQL Layer │
    │  (Performance)     │   │   (Rich Queries)  │
    └────────────────────┘   └───────────────────┘
```

## Performance Targets

- Block processing latency < 500ms
- Event indexing latency < 1 second from block finality
- RocksDB read latency < 50ms (99th percentile)
- PostgreSQL query latency < 500ms (95th percentile)
- Support for 100+ concurrent read queries

## Troubleshooting

Check log files if workflows fail:
- Anvil logs: `logs/anvil.log`
- Reth logs: `logs/reth.log`
- wasmd logs: `logs/wasmd.log`
- PostgreSQL logs: `data/postgres/postgres.log`

## Credit

Cover image from [Gaine's New-York pocket almanack for the year 1789](https://www.loc.gov/resource/rbc0001.2022madison98629)
