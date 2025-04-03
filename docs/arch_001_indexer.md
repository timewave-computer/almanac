# Product Requirements Document: Cross-Chain Indexer

## Summary

We are building a high-performance cross-chain indexer that will initially support Ethereum and Cosmos chains, with future expansion to Solana and Move-based chains. The indexer will focus on tracking Valence protocol contracts and related blockchain state, while being architected to support future Causality integration. The system will use a hybrid RocksDB and PostgreSQL approach to satisfy both high-performance requirements for strategists/relayers and complex query capabilities for frontend applications.

## Background

The Valence protocol operates across multiple blockchain ecosystems, requiring comprehensive indexing of on-chain state for monitoring, analysis, and cross-chain interactions. Currently, there is no unified solution for tracking Valence contracts across different blockchains with the appropriate determinism classifications and performance characteristics needed for our use case.

## Objectives

1. Develop a robust cross-chain indexer capable of tracking Valence contract state across Ethereum and Cosmos chains
2. Implement a hybrid storage architecture using RocksDB for high-performance paths and PostgreSQL for complex queries
3. Classify Cosmos events by determinism level to ensure appropriate handling
4. Create an extensible architecture to support future Causality integration
5. Provide the foundation for adding additional chains including Solana and Move-based chains

## Target Users

1. **Strategists and Relayers**: Require real-time access to chain state with minimal latency
2. **Frontend Applications**: Need rich querying capabilities for historical data and cross-chain state
3. **Protocol Developers**: Need visibility into contract state and cross-chain interactions
4. **System Administrators**: Need monitoring and diagnostic information

## Key Requirements

### Blockchain Support

1. **Ethereum Indexing**
   - Contract state for Valence accounts, processor, authorization, and libraries
   - Block headers, transaction data, and event logs
   - Contract state information for 3rd party contracts programs may interact with
   - Fee market information
   - Historical information for each of these

2. **Cosmos Indexing**
   - Contract state for Valence accounts, processor, authorization, and libraries
   - Block headers, transaction data, and event logs
   - Contract state information for 3rd party contracts programs may interact with
   - Fee market information
   - Historical information for each of these
   - Determinism classification for different event types:
     - Events (non-deterministic due to node-specific view)
     - Queries (information for light clients)
     - MsgResponse (deterministic state machine composability)
   - Contract state compatible with Cosmos SDK and CosmWasm
   - Consensus and finality information

3. **Future Support**
   - State information from the SMT tree of the Causality coprocessor
   - Architecture must accommodate future addition of Solana and Move-based chains
   - Abstraction layer to handle chain-specific indexing logic

### Valence Contract Indexing

1. **Valence Accounts**
   - Track account creation, ownership, and library approvals
   - Monitor account execution history

2. **Valence Processor**
   - Track processor state and cross-chain message processing
   - Monitor processor upgrades and configuration changes

3. **Valence Authorization**
   - Track authorization grants and revocations
   - Monitor policy changes and enforcement

4. **Valence Libraries**
   - Track library deployments and upgrades
   - Monitor library usage by accounts

5. **Program Lifecycle Tracking**
   - Track program initialization across chains
   - Monitor program execution flow between domains
   - Capture state transitions throughout lifecycle 
   - Maintain causal relationships between cross-chain actions
   - Track program termination or completion events

6. **Cross-Chain Debugging Support**
   - Generate execution traces spanning multiple chains
   - Provide debuggable view of message propagation
   - Trace impact of operations across domains
   - Capture error states and failure points
   - Support time-travel debugging capabilities

### Performance Requirements

1. **Hybrid Storage Architecture**
   - RocksDB for high-performance, real-time access paths
   - PostgreSQL for complex queries, historical data, and relationships
   - Synchronization mechanism to ensure consistency between stores

2. **Indexing Latency**
   - Block processing latency < 2 seconds for normal operations
   - Event indexing latency < 3 seconds from block finality
   - Contract state extraction latency < 5 seconds

3. **Query Performance**
   - Read latency for RocksDB paths < 50ms (99th percentile)
   - Complex query latency for PostgreSQL < 500ms (95th percentile)
   - Support for at least 100 concurrent read queries

### Extensibility Requirements

1. **Schema Registry**
   - Support for registering new contract schemas
   - Support for custom event definitions
   - Framework for cross-chain program state definitions

2. **Causality Integration Preparation**
   - Extensible architecture for future effect schemas
   - Support for content-addressed resources
   - Framework for cross-chain program state representation

3. **Custom Indexers**
   - Mechanism for adding new contract-specific indexers
   - Plugin system for custom data transformations

### API Requirements

1. **Query API**
   - GraphQL API for complex relational queries
   - RESTful API for simpler data access
   - WebSocket subscription API for real-time updates

2. **Admin API**
   - Endpoints for managing schema registry
   - Endpoints for controlling indexing process
   - Health and diagnostic information

3. **Debugging API**
   - Endpoints for accessing cross-chain program traces
   - Filtering capabilities for debugging specific programs
   - Time-range queries for execution analysis
   - Visual trace representation endpoints
   - Error analysis and diagnostics

### Blockchain State Migrations

1. **Migration Framework**
   - Support for defining and applying schema migrations for both RocksDB and PostgreSQL
   - Versioning system to track migration history
   - Ability to rollback migrations in case of failures
   - Migration locking to prevent concurrent migrations

2. **Contract Evolution Support**
   - Handle contract upgrades and evolving data schemas
   - Track contract versioning across multiple chains
   - Map data between different versions of contract schemas
   - Maintain historical data through schema changes

3. **Chain Upgrade Handling**
   - Detect and adapt to network protocol upgrades
   - Handle hard forks with potential state transitions
   - Support for chain-specific migration logic
   - Backward compatibility for historical data

4. **Data Transformation Pipelines**
   - Transform data between schemas during migrations
   - Support for complex data transformations
   - Validation of data integrity during migrations
   - Performance optimization for large state migrations

5. **Migration Coordination**
   - Coordinate migrations across multiple storage backends
   - Atomicity guarantees for multi-store migrations
   - Consistency verification post-migration
   - Graceful service degradation during migrations

## Technical Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Cross-Chain Indexer                              │
│                                                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐        │
│  │Chain        │ │Indexing     │ │Storage      │ │Query        │        │
│  │Adapters     │ │Pipeline     │ │Layer        │ │Engine       │        │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘        │
│                                                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐        │
│  │Schema       │ │Cross-Chain  │ │Sync         │ │Extension    │        │
│  │Registry     │ │Aggregator   │ │Manager      │ │Framework    │        │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘        │
└─────────────────────────────────────────────────────────────────────────┘
```

### Storage Architecture

```
┌─────────────────────────────────────────────────┐
│              Hybrid Storage Layer               │
└───────────────┬─────────────────────┬───────────┘
                │                     │
    ┌───────────▼────────┐   ┌────────▼──────────┐
    │  RocksDB Layer     │   │  PostgreSQL Layer │
    │  (Performance)     │   │  (Query Richness) │
    └────────────────────┘   └───────────────────┘
```

### Key Data Models

1. **Chain-Agnostic Models**:
   - Block headers with finality information
   - Transactions with metadata
   - Events with determinism classification
   - Contract state snapshots

2. **Valence-Specific Models**:
   - Account state and authorized libraries
   - Processor configuration and message queue
   - Authorization policy state
   - Library deployment data

3. **Cross-Chain Models**:
   - Program state spanning multiple chains
   - Cross-chain message correlations
   - Aggregated contract state
   - **Program lifecycle tracking across domains**
   - **Cross-chain execution traces for debugging**

## Implementation Plan

### Phase 1: Core Infrastructure (8 weeks)

1. Chain adapter implementations for Ethereum and Cosmos
2. Hybrid storage layer with RocksDB and PostgreSQL
3. Basic indexing pipeline for blocks, transactions, and events
4. Schema registry for contract definitions
5. Basic query API

### Phase 2: Valence Contract Indexing (6 weeks)

1. Specialized indexers for Valence contracts
2. Historical state tracking
3. Event classification for Cosmos events
4. Enhanced query capabilities

### Phase 3: Performance Optimization (4 weeks)

1. Storage optimization for high-performance paths
2. Query optimization for complex data relationships
3. Synchronization manager improvements
4. Testing under production-like load

### Phase 4: Extension Framework (6 weeks)

1. Cross-chain state aggregation
2. Framework for adding custom indexers
3. Preparation for Causality integration
4. Support for new contract types

### Phase 5: Additional Chain Support (8 weeks per chain)

1. Chain adapter for Solana
2. Chain adapter for Move-based chains
3. Optimization for chain-specific characteristics

### Phase 6: Program Lifecycle and Debugging (4 weeks)

1. Cross-Chain Program Lifecycle Tracking
   - Program initialization tracking
   - Cross-chain execution flow monitoring
   - State transition capture and indexing
   - Program completion/termination detection

2. Debugging Trace Implementation
   - Cross-chain execution trace collection
   - Message propagation tracking
   - Error state capture and analysis
   - Time-travel debugging interface
   
3. Debugging API Development
   - Trace query endpoints
   - Visual trace representation
   - Error diagnostic tools
   - Debugging dashboard integration

4. Performance Optimization for Debugging
   - Trace storage optimization
   - Query performance for large traces
   - Selective trace capture configuration

## Success Metrics

1. **Performance Metrics**
   - Query latency under target thresholds
   - Indexing throughput meeting chain block production rates
   - Storage efficiency (bytes per block/event)

2. **Functionality Metrics**
   - Number of Valence contracts successfully indexed
   - Accuracy of cross-chain state representation
   - Extensibility capabilities demonstrated

3. **Developer Experience**
   - Time required to add new contract schemas
   - Effort to implement custom indexers
   - Quality of documentation and examples

## Technical Considerations

### Determinism Classification

The system must properly classify Cosmos chain events by their determinism level:

```
DeterminismLevel::Deterministic    // For Ethereum events, Cosmos MsgResponses
DeterminismLevel::NonDeterministic // For some Cosmos events
DeterminismLevel::LightClientVerifiable // For queries used by light clients
```

### Chain Reorganizations

The indexer must handle chain reorganizations gracefully across both storage layers:

1. Detect reorganizations through block header monitoring
2. Roll back affected data in both RocksDB and PostgreSQL
3. Re-index blocks from the common ancestor
4. Update cross-chain state as necessary

### Scaling Considerations

1. **Horizontal Scaling**
   - PostgreSQL read replicas for query scaling
   - RocksDB sharding for performance-critical paths
   - Separate services for different chains

2. **Vertical Scaling**
   - Memory optimization for RocksDB cache
   - Efficient disk usage patterns
   - CPU optimization for serialization/deserialization

### Security Considerations

1. **Data Integrity**
   - Cryptographic verification of block data
   - Validation of contract state transitions
   - Consistency checks for cross-chain state

2. **Access Control**
   - Authentication for write operations
   - Rate limiting for query API
   - Audit logging for administrative actions

## Future Extensions

The following items are not part of the initial scope but are planned for future development:

1. **Full Causality Integration**
   - Effect system integration
   - Resource register tracking
   - Temporal validation

2. **Advanced Analytics**
   - Time-series analysis of contract state
   - Anomaly detection for contract behavior
   - Cross-chain correlation analysis

3. **Additional Chain Support**
   - Support for L2 solutions (StarkNet, zkSync, etc.)
   - Support for parachains (Polkadot ecosystem)
   - Support for new chain launches

## Conclusion

The Cross-Chain Indexer is a critical infrastructure component for the Valence protocol ecosystem, enabling high-performance tracking of contract state across multiple blockchain networks. By implementing a hybrid storage architecture and focusing on extensibility, the system will provide both the performance needed for relayers and strategists and the rich query capabilities required by frontend applications. The design accommodates future growth with support for additional chains and integration with the Causality system.