# Almanac Cross-Chain Indexer Architecture

## Overview

The Almanac Cross-Chain Indexer is a production-ready system for tracking contract state and events across multiple blockchain networks. It supports Ethereum and Cosmos chains with a triple storage approach optimized for both real-time performance, complex querying, and distributed coordination.

Key components include a cross-chain indexer service, triple storage architecture, comprehensive APIs (REST and WebSocket), and Nix-based development environment.

## System Requirements

### Target Users

- **Strategists and Relayers**: Real-time chain state access with minimal latency
- **Frontend Applications**: Rich querying for historical data and cross-chain state  
- **Protocol Developers**: Contract state visibility and cross-chain interaction monitoring
- **System Administrators**: Monitoring and diagnostic capabilities

### Performance Targets

- Block processing latency: < 500ms
- Event indexing latency: < 1 second from block finality
- Query performance: < 50ms (RocksDB), < 500ms (PostgreSQL), < 100ms (FoundationDB)
- Concurrent operations: 100+ concurrent read queries
- Distributed coordination: Sub-second leader election and work assignment

## Component Architecture

```mermaid
graph TD
    A[Almanac Indexer] --> B[Chain Adapters]
    A --> C[Storage Layer]
    A --> D[API Layer]
    
    A --> F[Coordination Layer]
    
    B --> G[Ethereum Client]
    B --> H[Cosmos Client]
    B --> I[Future Chains]
    
    C --> J[PostgreSQL]
    C --> K[RocksDB]
    C --> L[FoundationDB]
    C --> M[Sync Manager]
    
    D --> N[HTTP REST API]

    D --> P[WebSocket API]
    
    E --> Q[Sparse Merkle Tree]
    E --> R[Content Addressing]
    E --> S[Proof Generation]
    
    F --> T[Distributed Coordination]
    F --> U[Event Correlation]
    F --> V[Monitoring Service]
    
    L --> T
    L --> U
    L --> V
```

### Chain Adapters

Blockchain-specific adapters handle network communication through specialized clients. Ethereum clients connect to RPC endpoints for Ethereum, Polygon, and Base networks. Cosmos clients integrate with SDK chains and CosmWasm contracts for Noble, Osmosis, and Neutron. Valence domain clients provide optimized adapters for Valence protocol contracts.

### Event Processing Pipeline

The processing pipeline ensures reliable data ingestion through event capture from blockchain nodes, batch processing with retry mechanisms, validation of data integrity and state transitions, and atomic storage operations across both storage systems.

### API Layer

Multiple interfaces serve different use cases: HTTP REST API for standard RESTful operations and WebSocket for real-time event subscriptions and streaming updates.

## Data Flow

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

The triple storage architecture combines PostgreSQL, RocksDB, and FoundationDB for optimal performance across different query patterns, real-time access, and distributed coordination while maintaining data consistency.

### Triple Storage Design

```mermaid
classDiagram
    class TripleStorage {
        +PostgreSQL storage
        +RocksDB storage  
        +FoundationDB storage
        +SyncManager sync
        store_event()
        get_events()
        atomic_update()
        coordinate_distributed_ops()
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
    
    class FoundationDBStorage {
        +Distributed coordination
        +Event correlation
        +Cluster monitoring
        +ACID transactions
        register_node()
        assign_work()
        correlate_events()
    }
    
    TripleStorage --> PostgreSQLStorage
    TripleStorage --> RocksDBStorage
    TripleStorage --> FoundationDBStorage
```

### RocksDB Layer

Handles performance-optimized operations including latest state lookups, real-time event streams, and time-critical queries. Uses LSM-Tree optimization for write-heavy workloads with 64MB write buffers, direct I/O, LZ4 compression, and bloom filters for optimal performance.

### PostgreSQL Layer

Provides complex relational queries, historical data analysis, and cross-chain relationships through normalized table structures with strategic indexing. Supports full-text search and JSON columns for flexible schema-less data.

### FoundationDB Layer

Provides distributed coordination and ACID transactions across the cluster with features including:

**Distributed Coordination**: Node registration, leader election, and work assignment across multiple indexer instances
**Event Correlation**: Cross-chain event correlation and relationship tracking with cryptographic proofs
**Cluster Monitoring**: Distributed metrics collection, health monitoring, and alerting
**ACID Transactions**: Globally consistent transactions for critical operations requiring strong consistency

Uses namespace-based key organization with prefixes for different data types:
- `/events/` - Cross-chain event storage with correlation metadata
- `/coord/nodes/` - Node registration and heartbeat data
- `/coord/assignments/` - Work assignment and load balancing
- `/monitoring/` - Cluster metrics and health status

### Synchronization Mechanism

A transaction coordinator ensures atomic consistency between all three storage systems, validating operations succeed across PostgreSQL, RocksDB, and FoundationDB before committing changes. Includes rollback capabilities and automatic recovery mechanisms with distributed consensus.

## Block Finality Tracking

The indexer tracks multiple finality levels for appropriate security guarantees:

**Ethereum**: Confirmed (may be reorganized), Safe (unlikely to be orphaned), Justified (validator voted), Finalized (irreversible)

**Cosmos**: Committed (consensus validated), Final (chain-specific finality rules)

The system polls blockchain nodes for finality status and updates the database so applications can choose appropriate security levels.

## Valence Protocol Integration

### Contract Architecture

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

### Contract Types

**Valence Accounts**: Track creation, ownership transitions, library approvals, and execution history with comprehensive audit trails.

**Valence Processor**: Monitor processor state, cross-chain message processing, upgrades, and configuration changes with full lifecycle tracking.

**Valence Authorization**: Manage authorization grants, revocations, policy changes, and enforcement with audit trails for security analysis.

**Valence Libraries**: Track deployments, version upgrades, and usage patterns with dependency analysis for security implications.

## Security and Performance

### Security Measures

- Cryptographic verification of block headers and transaction data
- State validation ensuring contract transitions follow protocol rules
- JWT-based authentication with rate limiting and input validation
- Chain reorganization handling with atomic rollback and re-indexing

### Performance Characteristics

Performance benchmarks demonstrate triple storage approach advantages:

| Operation Type | RocksDB (ms) | PostgreSQL (ms) | FoundationDB (ms) | Used Backend |
|----------------|--------------|-----------------|-------------------|--------------|
| Latest Block Query | 1.2 | 8.7 | 2.1 | RocksDB |
| Account State Lookup | 0.9 | 6.4 | 1.8 | RocksDB |
| Cross-Chain Analysis | 15.3 | 3.8 | 4.2 | PostgreSQL |
| Event Correlation | 22.1 | 12.3 | 3.1 | FoundationDB |
| Distributed Coordination | N/A | N/A | 0.8 | FoundationDB |
| Event Filtering (simple) | 2.5 | 11.2 | 3.7 | RocksDB |
| Event Filtering (complex) | 36.8 | 7.5 | 8.9 | PostgreSQL |

### Scaling Capabilities

Supports 100+ concurrent queries with efficient storage utilization and optimized patterns. Architecture supports horizontal scaling through:
- PostgreSQL read replicas and partitioning
- RocksDB sharding across multiple instances  
- FoundationDB automatic sharding and cluster scaling
- Distributed query processing with cross-storage coordination

## Development and Operations

### Configuration Management

Uses Nix-based environment setup, TOML configuration files with hot-reload support, and environment variable overrides for deployment flexibility. FoundationDB cluster configuration supports both single-node development and multi-node production deployments.

### Module Structure

The project is organized into a hierarchical modular architecture:

```mermaid
classDiagram
    class AlmanacWorkspace {
        +Infrastructure Layer
        +Services Layer
        +Chains Layer
        +Development Layer
        +API Layer
    }
    
    class InfrastructureLayer {
        +almanac-infrastructure-core
        +almanac-infrastructure-storage
        
        +almanac-infrastructure-coordination
        +almanac-infrastructure-query
        +almanac-infrastructure-monitoring
        +almanac-infrastructure-caching
        +almanac-infrastructure-chains
    }
    
    class ServicesLayer {
        +indexer-api
        +almanac-services-query
        +almanac-services-indexing
    }
    
    class ChainsLayer {
        +indexer-ethereum
        +indexer-cosmos
    }
    
    class DevelopmentLayer {
        +indexer-benchmarks
        +indexer-testing
        +indexer-tools
    }
    
    class APILayer {
        +almanac-api-core
        +almanac-api-http
    }
    
    AlmanacWorkspace --> InfrastructureLayer
    AlmanacWorkspace --> ServicesLayer
    AlmanacWorkspace --> ChainsLayer
    AlmanacWorkspace --> DevelopmentLayer
    AlmanacWorkspace --> APILayer
```

### Core Modules

- **almanac-infrastructure-core**: Essential types, traits, error handling, and cross-chain coordination
- **almanac-infrastructure-storage**: Triple storage backends (PostgreSQL + RocksDB + FoundationDB) with unified trait interface

- **almanac-infrastructure-coordination**: Distributed coordination services using FoundationDB
- **indexer-ethereum**: Ethereum/EVM chain integration with valence-domain-clients
- **indexer-cosmos**: Cosmos SDK chain integration with valence-domain-clients
- **indexer-api**: HTTP REST and WebSocket APIs with authentication
- **almanac-services-query**: Query optimization and aggregation engine

### Monitoring and Testing

Includes component-level health monitoring, Prometheus-compatible metrics, structured logging with correlation IDs, and comprehensive testing framework with integration tests, property-based tests, performance tests, and chaos engineering.

**FoundationDB Monitoring**: Dedicated monitoring infrastructure including:
- FDB-specific metrics (read/write operations, latency, storage usage, conflict retries)
- Cluster health monitoring (machine count, process status, network throughput)
- Automated alerting for high conflict rates, storage capacity, and process failures
- Grafana dashboards for real-time FDB cluster visibility

## Current Status and Future Extensions

### Implementation Status

Production-ready with triple storage architecture (PostgreSQL + RocksDB + FoundationDB), robust Ethereum and Cosmos integration, complete Valence protocol support, full API coverage (REST, GraphQL, WebSocket), distributed coordination capabilities, and comprehensive Nix-based development environment. All performance targets are met with distributed scaling support.

### Future Enhancements

Planned expansions include Layer 2 solutions, Solana integration, Move-based chains, advanced analytics, and additional horizontal scaling capabilities through FoundationDB cluster optimization.

The Almanac Cross-Chain Indexer provides high-performance tracking of contract state across multiple blockchain networks with distributed coordination and a robust foundation for future growth and ecosystem support. 