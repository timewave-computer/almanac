# almanac-core

Core types and traits for the Almanac multi-chain blockchain indexing system.

## Overview

This crate provides the fundamental building blocks used throughout the Almanac ecosystem, including error handling, event definitions, storage traits, and common types for blockchain data.

## Key Components

### Error Handling
- `Error` enum with variants for different error types (Storage, Chain, Event, Processing)
- `Result<T>` type alias for consistent error handling across crates
- Helper methods for creating specific error types

### Event System
- `Event` trait defining the interface for blockchain events
- `UnifiedEvent` struct for normalized cross-chain event representation
- `EventFilter` builder for querying events with rich filtering capabilities

### Storage Abstraction
- `StorageWriter` trait for write operations (e.g., storing events, updating block status)
- `StorageReader` trait for read operations (e.g., querying events, retrieving latest blocks)
- `Storage` trait as a convenience for types implementing both `StorageWriter` and `StorageReader`
- Extensible design for different storage backends

### Common Types
- `ChainId` enum for supported blockchain networks (Ethereum, Cosmos, Solana)
- `BlockStatus` enum for finality levels (Confirmed, Safe, Justified, Finalized)
- Common chain identifiers (e.g., `ChainId::Ethereum`, `ChainId::Cosmos`)

## Usage

```rust
use almanac_core::{ChainId, EventFilter, Error, Result};

// Create an event filter
let filter = EventFilter {
    chain: Some(ChainId::Ethereum.to_string()),
    block_range: Some((1000, 2000)),
    event_type: Some("Transfer".to_string()),
    limit: Some(100),
    offset: Some(0),
    ..Default::default()
};

// Handle errors consistently
fn process_data() -> Result<()> {
    // ... processing logic
    Ok(())
}
```

## Features

- **Cross-chain compatibility**: Unified interfaces for different blockchain networks
- **Flexible filtering**: Rich querying capabilities for events
- **Type safety**: Strong typing for blockchain-specific data
- **Error propagation**: Comprehensive error handling with context
- **Async support**: Built for async/await patterns

## Testing

Run tests with:
```bash
nix develop -c cargo test -p almanac-core
```

The test suite covers:
- Error type conversions and helper methods
- Event filter building and validation
- Chain ID string conversions and serialization
- Block status enum behavior