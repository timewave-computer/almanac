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

Almanac uses a distributed architecture where chain-specific nodes connect to a shared FoundationDB data layer:

```
┌─────────────────────────────────────────────────────────────────┐
│                         External Clients                        │
└─────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                     almanac-api (Load Balanced)                 │
│  • HTTP API Server        • WebSocket Support                   │
│  • Authentication         • Rate Limiting                       │
│  • OpenAPI Documentation  • Response Caching                    │
└─────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                  FoundationDB Cluster (Shared Storage)          │
│  • Distributed ACID transactions                                │
│  • Automatic replication and sharding                           │
│  • Multi-dimensional tuple indexing                             │
└─────────────────────────────────────────────────────────────────┘
                                 ▲
       ┌─────────────────────────┼─────────────────────────┐
       │                         │                         │
┌──────▼──────────┐     ┌────────▼────────┐     ┌─────────▼────────┐
│ almanac-ethereum│     │ almanac-cosmos  │     │ almanac-solana   │
│  (2-3 replicas) │     │ (2-3 replicas)  │     │ (2-3 replicas)   │
├─────────────────┤     ├─────────────────┤     ├──────────────────┤
│ • Event indexing│     │ • Event indexing│     │ • Event indexing │
│ • Reorg handling│     │ • BFT consensus │     │ • PoH validation │
│ • State proofs  │     │ • IBC tracking  │     │ • Program data   │
│ • EVM traces    │     │ • Module events │     │ • Account updates│
└─────────────────┘     └─────────────────┘     └──────────────────┘
        │                        │                        │
        ▼                        ▼                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                    valence-domain-clients                       │
│  • Standardized blockchain RPC interfaces                       │
│  • Connection pooling and failover                              │
│  • Type-safe client implementations                             │
└─────────────────────────────────────────────────────────────────┘
```

## Crate Structure

### Core Crates

#### almanac-core
The foundational crate containing core types and traits used throughout the system:
- **Event trait**: Base trait for all blockchain events
- **Storage trait**: Abstract storage interface
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

#### Chain-Specific Binaries
Each blockchain has its own binary to avoid dependency conflicts:
- **almanac-ethereum**: Ethereum-specific node using valence-domain-client-ethereum
- **almanac-cosmos**: Cosmos chains using valence-domain-client-cosmos  
- **almanac-solana**: Solana using valence-domain-client-solana
- **Feature flags**: Each binary built with only required dependencies
- **Shared storage**: All nodes write to the same FoundationDB namespace

#### almanac-chains
Core blockchain abstraction layer:
- **ChainAdapter trait**: Unified interface implemented by each chain binary
- **ChainDataIR**: Normalized intermediate representation for all chains
- **State extraction**: Merkle proofs and storage slot tracking
- **Event normalization**: Consistent event format across chains
- **Connection management**: RPC pooling via valence-domain-clients

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

### Distributed Node Architecture
- **Isolation**: Chain-specific binaries prevent dependency conflicts
- **Scalability**: Independent scaling per blockchain
- **Resilience**: Chain failures don't affect other chains
- **Flexibility**: Different resource allocation per chain
- **Shared State**: FoundationDB provides consistent view across nodes

### FoundationDB as Shared Storage
- **ACID Guarantees**: Consistent state across all nodes
- **Horizontal Scaling**: Automatic sharding and replication
- **Multi-Version Concurrency**: Non-blocking reads
- **Tuple Layer**: Efficient multi-dimensional indexing
- **Namespace Isolation**: Logical separation between chains

### Chain-Specific Binaries
- **No Feature Conflicts**: Each binary has only required dependencies
- **Optimized Builds**: Smaller binaries, faster compilation
- **Independent Updates**: Update one chain without affecting others
- **Resource Isolation**: CPU/memory limits per chain
- **Simplified Debugging**: Clear separation of concerns

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