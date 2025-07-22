# almanac-consensus

Consensus, finality, and reorganization handling for the Almanac multi-chain blockchain indexing system.

## Overview

This crate provides comprehensive consensus mechanisms for managing blockchain finality, detecting reorganizations, and handling fork choice rules across different blockchain networks. It ensures data integrity and consistency during chain reorganizations and consensus changes.

## Key Components

### Fork Choice Rules
- `ForkChoiceRule` - Defines how to select the canonical chain
- `ForkChoice` trait - Primary interface for consensus algorithms
- `create_fork_choice` function - Factory for creating `ForkChoice` implementations
- Support for different consensus mechanisms (PoW, PoS, BFT)

### Finality Processing (Internal)
- `FinalityProcessor` - Tracks block finality across chains
- `FinalityConfig` - Chain-specific finality rules
- `FinalityLevel` - Finality status levels (Pending, Safe, Justified, Finalized)
- Confirmation-based and consensus-based finality

### Reorganization Detection (Internal)
- `ReorgDetector` - Monitors chain state for reorganizations
- `ReorgHandler` - Processes and responds to detected reorganizations
- `ReorgType` - Classifies reorganization severity (Simple, Deep, Complex)
- `ReorgInfo` - Detailed information about reorganization events

### Consensus Types
- `BlockInfo` - Block metadata with consensus information
- `ConsensusState` - Current consensus state tracking
- `ForkInfo` - Information about detected forks

## Usage

```rust
use almanac_consensus::{create_fork_choice, ForkChoiceRule};
use almanac_consensus::types::BlockInfo;
use std::time::SystemTime;

// Create an initial block for the fork choice
let initial_block = BlockInfo {
    number: 0,
    hash: "0x000...".to_string(),
    parent_hash: "0x000...".to_string(),
    timestamp: SystemTime::now(),
    confirmations: 0,
};

// Initialize a fork choice implementation (e.g., LongestChain)
let fork_choice = create_fork_choice(ForkChoiceRule::LongestChain, initial_block);

// Process a new block
let new_block = BlockInfo {
    number: 1,
    hash: "0x111...".to_string(),
    parent_hash: "0x000...".to_string(),
    timestamp: SystemTime::now(),
    confirmations: 0,
};

let update = fork_choice.process_block(new_block).await.unwrap();

if update.head_changed {
    println!("Canonical head changed to block {}", update.new_head.number);
}

// Get the current canonical head
let current_head = fork_choice.get_canonical_head().await.unwrap();
println!("Current canonical head: {}", current_head.number);
```

## Features

- **Multi-chain consensus**: Support for different consensus mechanisms
- **Reorganization detection**: Automatic detection of chain reorganizations
- **Finality tracking**: Monitor block finality across different chains
- **Fork choice rules**: Implement chain-specific fork choice algorithms
- **Configurable thresholds**: Adjust finality and reorganization parameters
- **Event-driven**: Async event handling for consensus changes

## Chain-Specific Behavior

### Ethereum
- **Finality**: 12+ confirmations for safety, 64+ for finality
- **Reorganizations**: Supports uncle blocks and deep reorgs
- **Fork choice**: Longest chain rule (PoW) or LMD-GHOST (PoS)

### Cosmos
- **Finality**: Instant finality with BFT consensus
- **Reorganizations**: Limited to validator set changes
- **Fork choice**: Tendermint consensus rules

### Solana
- **Finality**: 32+ confirmations for finality
- **Reorganizations**: Slot-based reorganizations
- **Fork choice**: Tower BFT consensus

## Configuration

Configuration for consensus, finality, and reorg handling is managed through the main `almanac-config` crate. Specific parameters are defined within the `Config` struct and loaded from TOML files or environment variables.

## Testing

Run tests with:
```bash
nix develop -c cargo test -p almanac-consensus
```

The test suite covers:
- Reorganization detection algorithms
- Finality level calculations
- Fork choice rule implementations
- Consensus state management
- Error handling and edge cases

## Reorganization Types

### Simple Reorganization
- Depth: 1-3 blocks
- Confidence: >95%
- Handling: Automatic rollback and reprocessing

### Deep Reorganization  
- Depth: 4-50 blocks
- Confidence: 80-95%
- Handling: Manual intervention may be required

### Complex Reorganization
- Depth: >50 blocks
- Confidence: <80%
- Handling: Requires careful analysis and recovery

## Performance Considerations

- Efficient block tracking with minimal memory usage
- Batched reorganization processing
- Configurable detection thresholds
- Async processing for non-blocking operations
- Optimized fork choice algorithms