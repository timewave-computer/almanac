# Almanac Architecture Overview

## Introduction

Almanac is a high-performance, multi-chain blockchain indexing system built with Rust. It provides a unified interface for indexing events and state from multiple blockchains, with support for complex queries, storage proofs, and real-time updates.

## Core Design Principles

1. **Modularity**: Each component has a single, well-defined responsibility
2. **Extensibility**: Easy to add support for new blockchains and storage backends
3. **Performance**: Optimized for high-throughput event processing
4. **Reliability**: Built-in support for chain reorganizations and finality tracking
5. **Type Safety**: Leverages Rust's type system for correctness

## System Architecture

Almanac uses a distributed microservices architecture where chain-specific indexing nodes and an API server connect to a shared FoundationDB data layer:

```
┌─────────────────────────────────────────────────────────────────┐
│                         External Clients                        │
│         • Web Apps  • CLI Tools  • Dashboards                   │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼ HTTP/WebSocket
┌─────────────────────────────────────────────────────────────────┐
│                     almanac-api                                 │
│  • HTTP API Server        • WebSocket Support                   │
│  • Authentication         • Rate Limiting                       │
│  • OpenAPI Documentation  • Response Caching                    │
│  • Read-only operations   • Query optimization                  │
└─────────────────────────┬───────────────────────────────────────┘
                          │ Reads
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                  FoundationDB Cluster                           │
│  • Distributed ACID transactions                                │
│  • Automatic replication and sharding                           │
│  • Multi-dimensional tuple indexing                             │
│  • 13 keyspaces for multi-chain data                            │
└─────────────────────────▲───────────────────────────────────────┘
                          │ Writes
       ┌──────────────────┼───────────────────┐
       │                  │                   │
┌──────▼──────────┐ ┌─────▼──────────┐ ┌──────▼──────────┐
│ almanac-ethereum│ │almanac-cosmos  │ │almanac-solana   │
│     :9091       │ │     :9092      │ │    :9093        │
├─────────────────┤ ├────────────────┤ ├─────────────────┤
│ • Event indexing│ │• Event indexing│ │• Account data   │
│ • Reorg handling│ │• BFT consensus │ │• PoH validation │
│ • State proofs  │ │• IBC tracking  │ │• Program logs   │
│ • EVM traces    │ │• Module events │ │• Slot tracking  │
│ • Health :9091  │ │• Health :9092  │ │• Health :9093   │
│ • Metrics       │ │• Metrics       │ │• Metrics        │
└─────────────────┘ └────────────────┘ └─────────────────┘
        │                    │                  │
        ▼ RPC                ▼ RPC              ▼ RPC
┌─────────────────────────────────────────────────────────────────┐
│                      Blockchain Networks                        │
│  • Ethereum/Anvil       • Cosmos/Neutron      • Solana          │
│  • :8545                • :26657              • :8899           │
└─────────────────────────────────────────────────────────────────┘
```

## Crate Structure

### Core Crates

#### almanac-core
The foundational crate containing core types and traits used throughout the system:
- **Event trait**: Base trait for all blockchain events
- **Storage traits**: Separate `StorageWriter` and `StorageReader` traits for write and read operations, respectively
- **Error types**: Unified error handling
- **Common types**: BlockStatus, ChainId, EventFilter

#### almanac-eventbus
Internal event bus for decoupled communication between components:
- Async message passing
- Event handler registration
- Subscription management
- System-wide event distribution

#### almanac-config
Configuration management with hot-reloading support:
- TOML-based configuration
- Environment variable overrides
- Runtime configuration updates
- Validation and type safety

### Service Crates

#### almanac-api
HTTP API server built on Axum:
- RESTful endpoints for event queries
- WebSocket support for real-time updates
- Authentication and authorization
- Rate limiting and caching
- OpenAPI documentation

#### almanac-indexing
Event indexing engine:
- Multi-chain event processing
- Batch optimization for performance
- Event ordering and deduplication
- Pipeline management
- Integration with storage and consensus

#### almanac-storage
Persistent storage layer:
- FoundationDB primary backend
- Tuple-based encoding for efficient queries
- Storage proof generation
- In-memory backend for testing
- Extensible storage trait

### Blockchain Integration

#### Distributed Services Architecture
Almanac uses separate chain-specific indexing services that connect to shared storage:
- **almanac-ethereum**: Dedicated Ethereum indexing node (:9091)
- **almanac-cosmos**: Dedicated Cosmos indexing node (:9092)  
- **almanac-solana**: Dedicated Solana indexing node (:9093)
- **almanac-api**: Read-only API server for client queries (:8080)
- **Shared FoundationDB**: All services use the same FoundationDB cluster with namespace isolation

#### Service Responsibilities

**Chain Indexing Nodes**:
- **Event Processing**: Index blockchain events and state changes
- **Consensus Handling**: Manage chain-specific finality and reorganizations
- **Storage Writing**: Write normalized data to FoundationDB
- **Health Monitoring**: Provide health and metrics endpoints
- **Independent Deployment**: Each service can be scaled and deployed separately

**API Server**:
- **Query Processing**: Handle client queries and API requests
- **Authentication**: Manage API keys and rate limiting
- **Data Aggregation**: Combine data from multiple chains
- **WebSocket Support**: Real-time event streaming
- **Read-Only**: Does not perform indexing or write to storage

#### almanac-chains
Core blockchain abstraction layer shared across services:
- **ChainAdapter trait**: Unified interface implemented by each chain service
- **ChainDataIR**: Normalized intermediate representation for all chains
- **State extraction**: Merkle proofs and storage slot tracking
- **Event normalization**: Consistent event format across chains
- **Connection management**: RPC pooling and failover handling

#### almanac-consensus
Consensus and finality management:
- **Finality levels**: Confirmed -> Safe -> Justified -> Finalized
- **Fork choice rules**: LongestChain, GHOST, Casper, TendermintBFT
- **Reorg detection**: Automatic rollback support with configurable depth
- **Chain-specific configurations**: Per-chain finality and consensus rules

### Development Support

#### almanac-testing
Testing utilities and fixtures:
- Mock chain adapters
- Test event generators
- Storage test helpers
- Integration test framework

## Data Flow

### Event Indexing Pipeline

1. **Block Fetching**
   - Chain adapter connects to RPC endpoint
   - Fetches blocks at configured interval
   - Extracts events and state changes

2. **Normalization**
   - Convert chain-specific data to ChainDataIR
   - Extract storage proofs if needed
   - Generate event metadata

3. **Consensus Processing**
   - Check for reorganizations
   - Update finality status
   - Apply fork choice rules

4. **Storage**
   - Persist events to FoundationDB
   - Update indexes and views
   - Generate storage proofs

5. **API Response**
   - Query events by filters
   - Return paginated results
   - Include proof data if requested

### Query Processing

1. **Request Reception**
   - HTTP/WebSocket request received
   - Authentication and rate limiting
   - Request validation

2. **Query Planning**
   - Parse filters and parameters
   - Optimize query execution
   - Select appropriate indexes

3. **Storage Access**
   - Execute FoundationDB queries
   - Apply filters and pagination
   - Retrieve associated proofs

4. **Response Generation**
   - Format results
   - Include metadata
   - Cache if appropriate

## Key Design Decisions

### Distributed Services Architecture
- **Service Isolation**: Each blockchain has a dedicated indexing service
- **Independent Scaling**: Scale chain indexers independently based on throughput
- **Fault Tolerance**: Failure of one chain doesn't affect others
- **Resource Optimization**: Chain-specific resource allocation and tuning
- **Deployment Flexibility**: Deploy and update services independently

### FoundationDB as Shared Storage
- **ACID Guarantees**: Consistent state across all nodes
- **Horizontal Scaling**: Automatic sharding and replication
- **Multi-Version Concurrency**: Non-blocking reads
- **Tuple Layer**: Efficient multi-dimensional indexing
- **Namespace Isolation**: Logical separation between chains

### Modular Chain Services
- **Service-Based**: Each chain has its own binary and service
- **Trait-Based**: Uniform ChainAdapter interface across all blockchain implementations
- **Pluggable**: Easy to add new blockchain support with new service
- **Type-Safe**: Leverages Rust's type system for correctness
- **Configurable**: Runtime configuration for different networks per service

### Event-Driven Architecture
- **Decoupling**: Components communicate via events
- **Scalability**: Async processing throughout
- **Flexibility**: Easy to add new processors
- **Debugging**: Event trail for troubleshooting

### Type-Safe Configuration
- **Compile-time Validation**: Catch errors early
- **Runtime Flexibility**: Hot-reloading support
- **Environment Overrides**: 12-factor app compliance
- **Self-documenting**: Types describe configuration

## Security Considerations

### API Security
- Bearer token authentication
- Rate limiting per client
- Input validation
- CORS configuration

### Data Integrity
- Storage proofs for verification
- Cryptographic commitments
- Finality tracking
- Reorg protection

### Operational Security
- Secure RPC connections
- API key management
- Audit logging
- Health monitoring

## Performance Optimizations

### Batch Processing
- Group events by block
- Bulk storage operations
- Parallel chain processing
- Memory-efficient streaming

### Caching Strategy
- API response caching
- Block metadata caching
- Configuration caching
- Connection pooling

### Query Optimization
- Tuple encoding for range queries
- Indexed access patterns
- Pagination support
- Lazy loading

### Extensibility Points
- Custom chain adapters
- Storage backend plugins
- Authentication providers
- Query processors

### Scaling Strategy
- Horizontal scaling via partitioning
- Read replicas for queries
- Event stream processing
- Caching layer expansion