# Almanac - Multi-Chain Blockchain Indexer

**Almanac** is a comprehensive blockchain indexing and data management system built in Rust, designed to provide real-time indexing, storage, and querying capabilities for multi-chain blockchain environments with a focus on Valence protocol integration.

## 📁 **Organized Crate Structure**

The project has been reorganized into cohesive submodules for better maintainability:

### **Infrastructure** (`crates/infrastructure/`)
Core system components and foundational services:
- **`core/`** - Core types, traits, error handling, and cross-chain coordination
- **`storage/`** - Dual storage backends (PostgreSQL + RocksDB) with unified trait interface  
- **`causality/`** - Event ordering and causality tracking with SMT (Sparse Merkle Tree)

### **Chains** (`crates/chains/`)
Blockchain-specific integrations:
- **`cosmos/`** - Cosmos SDK chain integration and contract codegen
- **`ethereum/`** - Ethereum/EVM chain integration and contract codegen

### **Services** (`crates/services/`)
User-facing services and APIs:
- **`api/`** - GraphQL and REST API endpoints with authentication
- **`query/`** - Query optimization and aggregation engine

### **Development** (`crates/development/`)
Development and testing tools:
- **`benchmarks/`** - Performance testing and load testing infrastructure
- **`testing/`** - Comprehensive testing framework with chaos engineering
- **`tools/`** - Development utilities and CLI tools

### **E2E Tests** (`e2e/`)
End-to-end integration tests and test runners

## 🏗️ **Core Architecture**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Cross-Chain Indexer                               │
│                                                                            │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │Chain        │ │Indexing     │ │Storage      │ │Query        │           │
│  │Adapters     │ │Pipeline     │ │Layer        │ │Engine       │           │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘           │
│                                                                            │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │Code         │ │Cross-Chain  │ │Causality    │ │Extension    │           │
│  │Generation   │ │Aggregator   │ │Indexer      │ │Framework    │           │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘           │
│                                                                            │
│  Contract Code Generation System:                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │ABI/Schema   │ │Type-Safe    │ │Database     │ │API          │           │
│  │Parser       │ │Client Gen   │ │Schema Gen   │ │Generation   │           │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘           │
└─────────────────────────────────────────────────────────────────────────────┘
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

For contract code generation issues, see the troubleshooting guides:
- [Cosmos Troubleshooting](docs/cosmos_troubleshooting.md)
- [Ethereum Troubleshooting](docs/ethereum_troubleshooting.md)

## Credit

Cover image from [Gaine's New-York pocket almanack for the year 1789](https://www.loc.gov/resource/rbc0001.2022madison98629)