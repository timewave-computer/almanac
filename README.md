# Almanac: Cross-Chain Indexer

A high-performance indexer designed to track Valence protocol contracts and related blockchain state across multiple chains. Almanac currently supports Ethereum and Cosmos chains, with easy extensibility to others in the future.

![](./almanac.png)

## Project Overview

Almanac enables tracking Valence programs and the state associated with related contracts across different blockchains. For non-BFT chains it handles chain finality conditions with appropriate determinism classifications. The system employs a hybrid storage architecture using RocksDB for high-performance paths and PostgreSQL for complex queries. This makes it appropriate for use with both cross-chain strategy operations and client UI development.

- **Multi-chain support**: Ethereum and Cosmos chains with plans for Solana and Move-based chains
- **Hybrid storage architecture**:
  - RocksDB for high-performance, real-time access paths
  - PostgreSQL for complex queries, historical data, and relationships
- **Valence contract tracking**:
  - Account creation, ownership, and library approvals
  - Processor state and cross-chain message processing
  - Authorization grants, revocations, and policy changes
  - Library deployments, upgrades, and usage
- **Chain reorganization handling**
- **Block finality tracking**: confirmed, safe, justified, finalized states
- **Determinism classification** for Cosmos events
- **Program lifecycle tracking** across domains
- **Cross-chain debugging** with multi-chain traces

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

This will set up a complete development environment with all necessary dependencies.

### Build and Run

Build the project:
```bash
cargo build
```

Run the indexer:
```bash
cargo run
```

## Storage Benchmarks

The system includes performance benchmarks comparing RocksDB with filesystem operations:

```bash
cargo run --bin run_rocks_benchmark
```

## Testing

Run the storage synchronization test:

```bash
cargo test -p indexer-storage --test storage_sync
```

Test the Ethereum adapter against Anvil:
```bash
nix run .#test-ethereum-adapter-anvil
```

Test the Cosmos adapter:
```bash
nix run .#test-cosmos-adapter
```

Test chain reorganization handling:
```bash
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

## Project Status

The project is actively under development.

## Credit

- Cover image from [Gaine's New-York pocket almanack for the year 1789](https://www.loc.gov/resource/rbc0001.2022madison98629)
