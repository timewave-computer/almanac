# Almanac: Cross-Chain Indexer

A performant indexer designed to track Valence, Causality and associated contract state across multiple chains. Almanac currently supports Ethereum and Cosmos chains using the `valence-domain-clients` library for robust blockchain connectivity, with easy extensibility to others in the future.

![](./almanac.png)


## Project Overview

Almanac enables tracking Valence programs and the state associated with related contracts across different blockchains. For non-BFT chains it handles chain finality conditions with appropriate determinism classifications. The system employs a hybrid storage architecture using RocksDB for high-performance paths and PostgreSQL for complex queries. This makes it appropriate for use with both cross-chain strategy operations and client UI development.

Almanac has been integrated with `valence-domain-clients` for robust multi-chain connectivity:

- **Multi-chain support**: 
  - **EVM chains**: Ethereum, Polygon, Base (via `EthereumClient`)
  - **Cosmos chains**: Noble, Osmosis, Neutron (via `NobleClient`)
  - Easy extensibility for additional chains through valence client implementations
- **Hybrid storage architecture**:
  - RocksDB for high-performance, real-time access paths
  - PostgreSQL for complex queries, historical data, and relationships
- **Valence contract tracking**:
  - Account creation, ownership, and library approvals
  - Processor state and cross-chain message processing
  - Authorization grants, revocations, and policy changes
  - Library deployments, upgrades, and usage
- **Advanced blockchain features**:
  - Chain reorganization handling
  - Block finality tracking: confirmed, safe, justified, finalized states
  - Determinism classification for Cosmos events
  - Program lifecycle tracking across domains
  - Cross-chain debugging with multi-chain traces
- **Dependency isolation**: All blockchain-specific dependencies managed through valence-domain-clients
- **Unified event system**: Cross-chain event normalization and processing

## Getting Started

### Installation

Clone the repository:

```bash
git clone <repository-url>
cd almanac
```

Enter the development shell:

```bash
nix develop
```

This will set up a complete development environment with all necessary dependencies including:
- Rust toolchain
- PostgreSQL database server 
- crate2nix for reproducible builds
- Foundry (anvil, forge, cast) for Ethereum development
- All required system libraries and build tools

**Available commands in the dev shell:**
- `init_databases` - Initialize and start PostgreSQL and RocksDB
- `stop_databases` - Stop PostgreSQL server  
- `generate_cargo_nix` - Generate Cargo.nix for Nix builds
- `update_cargo_nix` - Update existing Cargo.nix file
- `run_almanac_tests` - Run the Almanac test suite

### Build and Run

#### Option 1: Direct Cargo Build (Development)
Build the project:
```bash
cargo build
```

Run the indexer:
```bash
cargo run --bin almanac
```

#### Option 2: Nix Build (Production/Reproducible)
Almanac uses `crate2nix` for reproducible builds. First, generate the Nix build configuration:

```bash
generate_cargo_nix
```

Then build specific components:
```bash
# Build the main almanac binary
nix build .#indexer-api

# Build specific workspace crates
nix build .#indexer-core
nix build .#indexer-ethereum
nix build .#indexer-cosmos
nix build .#indexer-api
```

**Available crate2nix commands in the dev shell:**
- `generate_cargo_nix` - Generate Cargo.nix from Cargo.toml using crate2nix
- `update_cargo_nix` - Update existing Cargo.nix file

## Development Workflows

Almanac provides several Nix-based development environments for working with different blockchain backends.

### Running the Workflow Menu

To select which workflow to run, execute:

```bash
cd nix/environments
nix run .
```

This will display a menu where you can select which workflow to run:

1. Anvil Workflow - For Ethereum development with Anvil
2. Reth Workflow - For Ethereum development with Reth
3. CosmWasm Workflow - For CosmWasm chain development with wasmd
4. All Workflows - Run all workflows in sequence

### Running Specific Workflows Directly

You can also run specific workflows directly:

```bash
# Anvil workflow
nix run ./nix/environments#anvil-workflow

# Reth workflow
nix run ./nix/environments#reth-workflow

# CosmWasm workflow
nix run ./nix/environments#cosmwasm-workflow

# All workflows in sequence
nix run ./nix/environments#all-workflows
```

### Development Shell for Workflows

You can enter a development shell with all workflow tools available:

```bash
cd nix/environments
nix develop
```

For more details on using the workflow environments, see the [workflows documentation](nix/environments/README.md).

## Storage Benchmarks

The system includes performance benchmarks comparing RocksDB with filesystem operations:

```bash
cargo run --bin run_rocks_benchmark
```

## Testing

### Unit Tests

Run unit tests for the client integrations:

```bash
# Test Ethereum client integration
cargo test --package indexer-ethereum

# Test Cosmos client integration  
cargo test --package indexer-cosmos

# Test storage synchronization
cargo test -p indexer-storage --test storage_sync
```

### Integration Tests

Test the adapters against live blockchain nodes:

```bash
# Test Ethereum adapter against Anvil
nix run .#test-ethereum-adapter-anvil

# Test Cosmos adapter
nix run .#test-cosmos-adapter

# Run the comprehensive test suite
run_almanac_tests
```

### Chain Reorganization Tests

```bash
# Test chain reorganization handling
./scripts/test-chain-reorg.sh
```

## Available Commands

The following commands are available:

**Cosmos:**
- `nix run .#wasmd-node`: Start a local wasmd node for testing
- `nix run .#test-cosmos-adapter`: Run cosmos adapter tests against the local node

**Ethereum:**
- `nix run .#start-anvil`: Start local Ethereum test node (Anvil)
- `nix run .#start-reth`: Start Reth Ethereum node (requires config)
- `nix run .#test-ethereum-adapter-anvil`: Run tests against local Anvil node
- `nix run .#test-ethereum-adapter-reth`: Run tests against local Reth node

These commands are also available directly within the Nix development shell (`nix develop`).

## Architecture

The indexer follows a modular architecture:

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
│  │Schema       │ │Cross-Chain  │ │Sync         │ │Extension    │  │
│  │Registry     │ │Aggregator   │ │Manager      │ │Framework    │  │
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
    │  (Performance)     │   │   (Rick Queries)  │
    └────────────────────┘   └───────────────────┘
```

## Performance

The system is designed to meet the following performance targets:

- Block processing latency < 500ms for normal operations
- Event indexing latency < 1 second from block finality
- Read latency for RocksDB paths < 50ms (99th percentile)
- Complex query latency for PostgreSQL < 500ms (95th percentile)
- Support for at least 100 concurrent read queries

## Cross-Chain Testing

The project now includes full Ethereum contract implementations for testing cross-chain functionality:

1. **Simplified Contract Suite**:
   - `TestToken`: An ERC20-compatible token for cross-chain transfers
   - `BaseAccount`: An account abstraction for controlled access
   - `UniversalGateway`: A gateway for cross-chain message passing
   - `EthereumProcessor`: A contract for handling cross-chain messages

2. **End-to-End Test Script**:
   The `scripts/cross_chain_e2e_test.sh` script demonstrates the full Ethereum-side functionality:
   - Deploys all necessary contracts to an Anvil test node
   - Configures contracts and their relationships
   - Tests token transfers through the BaseAccount abstraction
   - Demonstrates cross-chain message sending and delivery
   - Verifies correct event emission for indexer integration
   - Tests sequential transfers between Ethereum and Cosmos chains

3. **Running the Test**:
   ```bash
   # Run directly (not recommended, use Nix instead)
   ./scripts/cross_chain_e2e_test.sh
   
   # Or using Nix (preferred method)
   nix run .#cross-chain-e2e-test
   ```

   The Nix run command will:
   - Compile the test WASM contracts automatically
   - Set up both Ethereum and Cosmos nodes
   - Deploy necessary contracts
   - Run the full end-to-end test suite
   - Clean up all resources after completion

This implementation provides a robust framework for testing cross-chain interactions between Ethereum and Cosmos chains.



## Troubleshooting

If a workflow doesn't complete successfully, check the respective log files:
- Anvil logs: `logs/anvil.log`
- Reth logs: `logs/reth.log`
- wasmd logs: `logs/wasmd.log`
- PostgreSQL logs: `data/postgres/postgres.log`

## Credit

- Cover image from [Gaine's New-York pocket almanack for the year 1789](https://www.loc.gov/resource/rbc0001.2022madison98629)
