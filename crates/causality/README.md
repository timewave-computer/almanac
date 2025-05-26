# Indexer Causality

A high-performance Sparse Merkle Tree (SMT) based causality indexing system for the Almanac cross-chain indexer. This crate provides comprehensive tracking of causal relationships between events, resources, and entities across multiple blockchain domains.

## Overview

The causality crate implements a content-addressed indexing system compatible with the [reverse-causality](https://github.com/anoma/reverse-causality) framework. It enables verifiable cross-domain computations and zero-knowledge proofs by maintaining cryptographic proofs of causal relationships between blockchain events.

## Features

### Core Capabilities

- **Content-Addressed Entities**: All entities (Resources, Effects, Intents, Handlers, Transactions, Domains) use SHA256-based content addressing
- **Sparse Merkle Trees**: Efficient SMT implementation with SHA256 and Blake3 hasher support
- **Domain-Scoped Indexing**: Organize entities by execution domains for efficient querying
- **Causal Relationship Tracking**: Track and verify causal dependencies between entities
- **Cross-Chain Support**: Index events and relationships across multiple blockchain networks
- **Proof Generation**: Generate and verify cryptographic proofs of causal relationships

### Reverse-Causality Compatibility

- **Entity Types**: Full support for Resource Model entities (Resources, Effects, Intents, Handlers, Transactions, Domains)
- **SSZ Serialization**: Compatible serialization format for content addressing
- **TypedDomains**: Support for VerifiableDomain and ServiceDomain execution environments
- **Content Addressing**: SHA256-based content addressing matching reverse-causality specifications

### Storage Backends

- **Memory Backend**: In-memory storage for testing and development
- **PostgreSQL Backend**: Persistent storage with complex query capabilities
- **Hybrid Storage**: Combined causality data and SMT node storage

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Causality Indexer                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐  │
│  │   Entity    │ │  Causality  │ │     SMT     │ │  Storage  │  │
│  │   Types     │ │   Tracker   │ │   Engine    │ │  Backend  │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └───────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐  │
│  │   Domain    │ │Cross-Chain  │ │   Proof     │ │   Query   │  │
│  │  Indexing   │ │ Causality   │ │ Generation  │ │  Engine   │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └───────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Basic Usage

```rust
use indexer_causality::{
    CausalityIndexer, CausalityIndexerConfig, 
    MemorySmtBackend, MemoryCausalityStorage
};

// Create a causality indexer with default configuration
let config = CausalityIndexerConfig::default();
let storage = Box::new(MemoryCausalityStorage::new());
let smt_backend = MemorySmtBackend::new();

let mut indexer = CausalityIndexer::new(config, storage, smt_backend)?;

// Index a causality event
let event = CausalityEvent {
    id: "event-1".to_string(),
    chain_id: ChainId("ethereum".to_string()),
    block_number: 12345,
    tx_hash: "0xabc123...".to_string(),
    event_type: CausalityEventType::ResourceEvent,
    timestamp: SystemTime::now(),
    data: CausalityEventData::Resource(resource),
};

indexer.index_event(event).await?;
```

### Working with Entities

```rust
use indexer_causality::{
    CausalityResource, CausalityEffect, EntityId, DomainId
};

// Create a content-addressed resource
let resource = CausalityResource {
    id: EntityId::new(content_hash),
    name: "Token Balance".to_string(),
    domain_id: DomainId::new(domain_hash),
    resource_type: "token".to_string(),
    quantity: 1000,
    timestamp: SystemTime::now(),
};

// Create an effect that consumes/produces resources
let effect = CausalityEffect {
    id: EntityId::new(effect_hash),
    name: "Token Transfer".to_string(),
    domain_id: DomainId::new(domain_hash),
    effect_type: "transfer".to_string(),
    inputs: vec![input_flow],
    outputs: vec![output_flow],
    expression: Some(ExprId::new(expr_hash)),
    timestamp: SystemTime::now(),
    scoped_by: HandlerId::new(handler_hash),
    intent_id: Some(IntentId::new(intent_hash)),
    source_typed_domain: TypedDomain::VerifiableDomain {
        domain_id: source_domain,
        capabilities: vec!["zk-proof".to_string()],
    },
    target_typed_domain: TypedDomain::ServiceDomain {
        domain_id: target_domain,
        service_type: "http".to_string(),
        endpoint: Some("https://api.example.com".to_string()),
    },
    originating_dataflow_instance: None,
};
```

### SMT Operations

```rust
use indexer_causality::{SparseMerkleTree, Sha256SmtHasher, MemorySmtBackend};

// Create an SMT with SHA256 hasher (reverse-causality compatible)
let backend = MemorySmtBackend::new();
let smt = SparseMerkleTree::with_sha256(backend);

// Insert data into the SMT
let key = hasher.key("resource", resource_id.as_bytes());
let data = resource.to_bytes()?;
let new_root = smt.insert(empty_hash(), &key, &data).await?;

// Generate a proof
let proof = smt.get_proof(new_root, &key).await?;

// Verify the proof
let is_valid = proof.unwrap().verify(&new_root, &key, &data, &hasher);
```

### Domain-Based Querying

```rust
use indexer_causality::{CausalityTracker, DomainId};

let mut tracker = CausalityTracker::new(Box::new(Sha256SmtHasher));

// Add entities to different domains
tracker.graph_mut().add_resource(resource)?;
tracker.graph_mut().add_effect(effect)?;

// Query entities by domain
let domain_id = DomainId::new(domain_hash);
let entities = tracker.graph().get_domain_entities(&domain_id);

// Analyze causal relationships
let analysis = tracker.analyze_event_causality("event-1");
println!("Causal depth: {}", analysis.unwrap().causal_depth);
```

## Entity Types

### Core Entities

- **CausalityResource**: Represents resources (tokens, compute credits, bandwidth) with quantities
- **CausalityEffect**: Computational effects that transform resources
- **CausalityIntent**: Commitments to transform resources with constraints
- **CausalityHandler**: Logic for processing specific effect types
- **CausalityTransaction**: Collections of effects and intents
- **CausalityDomain**: Execution environments (verifiable or service domains)
- **CausalityNullifier**: Proofs that resources have been consumed

### Relationship Types

- **DirectDependency**: Direct causal dependencies
- **ResourceFlow**: Resource transformation relationships
- **CrossChain**: Cross-blockchain dependencies
- **Temporal**: Happens-before relationships
- **State**: State-based dependencies

## Configuration

### CausalityIndexerConfig

```rust
let config = CausalityIndexerConfig {
    enable_smt: true,
    enable_causality_tracking: true,
    max_smt_depth: 256,
    batch_size: 100,
    enable_cross_chain: true,
    indexed_chains: vec![
        ChainId("ethereum".to_string()),
        ChainId("cosmos".to_string()),
    ],
    hasher_type: HasherType::Sha256, // Default for reverse-causality compatibility
};
```

### Storage Configuration

```rust
// Memory storage for development
let storage = Box::new(MemoryCausalityStorage::new());

// PostgreSQL storage for production
let pg_storage = Box::new(PostgresCausalityStorage::new(pg_pool));

// Combined storage with SMT backend
let combined = CausalityStorage::new(storage, smt_backend);
```

## Performance

### Benchmarks

- **Entity Indexing**: ~10,000 entities/second
- **SMT Operations**: ~5,000 insertions/second
- **Proof Generation**: ~100 proofs/second
- **Query Latency**: <10ms for entity lookups, <50ms for causal path queries

### Optimization Tips

1. **Batch Operations**: Use batch processing for multiple entities
2. **Domain Partitioning**: Organize entities by domain for efficient queries
3. **Memory Backend**: Use memory backend for testing and development
4. **PostgreSQL**: Use PostgreSQL backend for complex queries and persistence

## Testing

Run the test suite:

```bash
# Run all tests
cargo test -p indexer-causality

# Run specific test modules
cargo test -p indexer-causality smt::tests
cargo test -p indexer-causality storage::tests
cargo test -p indexer-causality indexer::tests

# Run with verbose output
cargo test -p indexer-causality --verbose
```

## Integration with Reverse-Causality

This crate is designed for seamless integration with the reverse-causality framework:

1. **Content Addressing**: All entities use SHA256-based content addressing
2. **SSZ Serialization**: Compatible serialization format
3. **Entity Types**: Full support for Resource Model entities
4. **Domain Scoping**: Proper domain-based organization
5. **Proof System**: Compatible SMT proof generation and verification

## Examples

See the `examples/` directory for complete usage examples:

- `basic_indexing.rs`: Basic entity indexing and querying
- `cross_chain.rs`: Cross-chain causality tracking
- `proof_generation.rs`: SMT proof generation and verification
- `domain_queries.rs`: Domain-based entity queries

## Contributing

1. Ensure all tests pass: `cargo test -p indexer-causality`
2. Run clippy: `cargo clippy -p indexer-causality`
3. Format code: `cargo fmt`
4. Update documentation as needed

## License

Licensed under the Apache License, Version 2.0. 