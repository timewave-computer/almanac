# Almanac System Architecture

This document describes the overall architecture of the Almanac cross-chain indexer system.

## Overview

The Almanac system consists of several key components:
1. A cross-chain indexer service supporting Ethereum and Cosmos chains
2. A hybrid storage architecture using PostgreSQL and RocksDB
3. REST, GraphQL, and WebSocket APIs for data access
4. A comprehensive Nix-based development environment
5. Causality indexing with Sparse Merkle Tree implementation

These components work together to provide comprehensive indexing of Valence protocol contracts and cross-chain state across multiple blockchain ecosystems.

## Component Architecture

```mermaid
graph TD
    A[Almanac Indexer] --> B[Chain Adapters]
    A --> C[Storage Layer]
    A --> D[API Layer]
    A --> E[Causality Engine]
    
    B --> F[Ethereum Client]
    B --> G[Cosmos Client]
    B --> H[Future Chains]
    
    C --> I[PostgreSQL]
    C --> J[RocksDB]
    C --> K[Sync Manager]
    
    D --> L[HTTP API]
    D --> M[GraphQL API]
    D --> N[WebSocket API]
    
    E --> O[Sparse Merkle Tree]
    E --> P[Content Addressing]
    E --> Q[Proof Generation]
    
    R[Nix Environment] --> S[Development Tools]
    R --> T[Database Setup]
    R --> U[Test Infrastructure]
```

## Data Flow Diagrams

### Event Indexing Workflow

```mermaid
sequenceDiagram
    actor User
    participant ChainAdapter
    participant IndexingPipeline
    participant Storage
    participant API
    
    User->>ChainAdapter: Monitor chain
    ChainAdapter->>IndexingPipeline: New block/events
    IndexingPipeline->>Storage: Store events (PostgreSQL)
    IndexingPipeline->>Storage: Store state (RocksDB)
    IndexingPipeline->>Storage: Update causality (SMT)
    User->>API: Query events
    API->>Storage: Retrieve data
    Storage->>API: Return results
    API->>User: Event data
```

### Cross-Chain Message Tracking

```mermaid
sequenceDiagram
    actor User
    participant SourceChain
    participant Indexer
    participant TargetChain
    participant MessageTracker
    
    User->>SourceChain: Send cross-chain message
    SourceChain->>Indexer: Emit message event
    Indexer->>MessageTracker: Track message (Originated)
    MessageTracker->>MessageTracker: Update status (In Transit)
    TargetChain->>Indexer: Delivery event
    Indexer->>MessageTracker: Update status (Delivered)
    TargetChain->>Indexer: Execution event
    Indexer->>MessageTracker: Update status (Executed)
```

## Storage Architecture

```mermaid
classDiagram
    class HybridStorage {
        +PostgreSQL storage
        +RocksDB storage
        +SyncManager sync
        store_event()
        get_events()
        atomic_update()
    }
    
    class PostgreSQLStorage {
        +Complex queries
        +Historical data
        +Cross-chain relationships
        +Full-text search
        store_valence_account()
        store_processor_message()
        get_events_with_status()
    }
    
    class RocksDBStorage {
        +High-performance reads
        +Latest state lookups
        +Time-critical operations
        get_latest_block()
        set_account_state()
        get_processor_state()
    }
    
    HybridStorage --> PostgreSQLStorage
    HybridStorage --> RocksDBStorage
```

## Valence Contract Architecture

```mermaid
classDiagram
    class ValenceIndexer {
        +AccountIndexer account
        +ProcessorIndexer processor
        +AuthorizationIndexer authorization
        +LibraryIndexer library
        index_contract_event()
    }
    
    class AccountIndexer {
        +ValenceAccountInfo account_info
        +ValenceAccountExecution executions
        process_account_created()
        process_library_approved()
        process_executed()
    }
    
    class ProcessorIndexer {
        +ValenceProcessorInfo processor_info
        +ValenceProcessorMessage messages
        process_message_received()
        process_message_processed()
        correlate_cross_chain_messages()
    }
    
    class AuthorizationIndexer {
        +ValenceAuthorizationInfo auth_info
        +ValenceAuthorizationGrant grants
        process_policy_updated()
        process_grant_created()
        process_authorization_request()
    }
    
    class LibraryIndexer {
        +ValenceLibraryInfo library_info
        +ValenceLibraryVersion versions
        process_library_deployed()
        process_version_updated()
        track_library_usage()
    }
    
    ValenceIndexer --> AccountIndexer
    ValenceIndexer --> ProcessorIndexer
    ValenceIndexer --> AuthorizationIndexer
    ValenceIndexer --> LibraryIndexer
```

## Module Structure

```mermaid
classDiagram
    class AlmanacWorkspace {
        +indexer-core
        +indexer-storage
        +indexer-ethereum
        +indexer-cosmos
        +indexer-api
        +indexer-causality
        +indexer-query
        +indexer-tools
        +indexer-benchmarks
    }
    
    class CoreModule {
        +Error handling
        +Type definitions
        +Service abstractions
        +Event types
        +Configuration
    }
    
    class StorageModule {
        +PostgreSQL migrations
        +RocksDB configuration
        +Hybrid storage trait
        +Valence data models
        +Sync mechanisms
    }
    
    class ChainModules {
        +EthereumClient
        +CosmosClient
        +valence-domain-clients integration
        +Event processing
        +Block finality tracking
    }
    
    class APIModule {
        +HTTP REST endpoints
        +GraphQL schema
        +WebSocket subscriptions
        +Authentication
        +Schema registry
    }
    
    AlmanacWorkspace --> CoreModule
    AlmanacWorkspace --> StorageModule
    AlmanacWorkspace --> ChainModules
    AlmanacWorkspace --> APIModule
```

## Configuration Management

Configuration is managed through multiple mechanisms:

### Nix-based Environment
```bash
# Enter development environment
nix develop

# Initialize databases
init_databases

# Build specific crates
nix build .#indexer-api
nix build .#indexer-ethereum
```

### Runtime Configuration
```toml
# almanac-config.json
{
  "chains": {
    "ethereum": {
      "rpc_url": "http://localhost:8545",
      "chain_id": "1"
    },
    "cosmos": {
      "grpc_url": "http://localhost:9090",
      "chain_id": "cosmoshub-4"
    }
  },
  "storage": {
    "postgres_url": "postgresql://postgres:postgres@localhost:5432/indexer",
    "rocksdb_path": "./data/rocksdb"
  },
  "api": {
    "host": "127.0.0.1",
    "port": 8000,
    "enable_graphql": true,
    "enable_websocket": true
  }
}
```

## Integration Points

The main integration points include:

### Chain Connectivity
- **Ethereum**: Uses `valence-domain-clients` EVM integration for robust Ethereum, Polygon, and Base support
- **Cosmos**: Uses `valence-domain-clients` Cosmos integration for Noble, Osmosis, and Neutron support

### Storage Synchronization
- Atomic updates across PostgreSQL and RocksDB
- Consistency verification and error recovery
- Performance-optimized data paths

### API Consistency
```bash
# REST API
curl http://localhost:8000/api/v1/chains/ethereum/status

# GraphQL API
curl -X POST http://localhost:8000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ latestFinalizedBlock(chain: \"ethereum\") }"}'

# WebSocket API
wscat -c ws://localhost:8000/api/v1/ws
```

## Performance Targets

- **Block processing latency**: < 500ms
- **Event indexing latency**: < 1 second from block finality
- **RocksDB read latency**: < 50ms (99th percentile)
- **PostgreSQL query latency**: < 500ms (95th percentile)
- **Concurrent query support**: 100+ concurrent read queries

## Future Considerations

Potential future enhancements:
- Solana and Move-based chain support
- Enhanced causality proof verification
- Cross-chain debugging visualization
- Advanced analytics and reporting
- Horizontal scaling and sharding 