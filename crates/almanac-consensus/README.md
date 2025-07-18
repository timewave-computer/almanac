# almanac-consensus

Consensus, finality, and reorganization handling for the Almanac multi-chain blockchain indexing system.

## Overview

This crate provides comprehensive consensus mechanisms for managing blockchain finality, detecting reorganizations, and handling fork choice rules across different blockchain networks. It ensures data integrity and consistency during chain reorganizations and consensus changes.

## Key Components

### Reorganization Detection
- `ReorgDetector` - Monitors chain state for reorganizations
- `ReorgHandler` - Processes and responds to detected reorganizations
- `ReorgType` - Classifies reorganization severity (Simple, Deep, Complex)
- `ReorgInfo` - Detailed information about reorganization events

### Finality Processing
- `FinalityProcessor` - Tracks block finality across chains
- `FinalityConfig` - Chain-specific finality rules
- `FinalityLevel` - Finality status levels (Pending, Safe, Justified, Finalized)
- Confirmation-based and consensus-based finality

### Fork Choice Rules
- `ForkChoiceRule` - Defines how to select the canonical chain
- `ForkChoiceState` - Maintains fork choice state
- Support for different consensus mechanisms (PoW, PoS, BFT)

### Consensus Types
- `BlockInfo` - Block metadata with consensus information
- `ConsensusState` - Current consensus state tracking
- `ForkInfo` - Information about detected forks

## Usage

```rust
use almanac_consensus::{
    ReorgDetector, FinalityProcessor, FinalityConfig, 
    ReorgHandler, FinalityLevel, ReorgType
};

// Initialize finality processor
let finality_config = FinalityConfig {
    chain_id: "ethereum".to_string(),
    confirmation_blocks: 12,
    finality_delay: 64,
    enable_deep_reorg_detection: true,
    max_reorg_depth: 100,
};

let finality_processor = FinalityProcessor::new(finality_config);

// Check block finality
let finality_level = finality_processor
    .get_finality_level(block_number, current_tip)
    .await?;

match finality_level {
    FinalityLevel::Pending => println!("Block is pending"),
    FinalityLevel::Safe => println!("Block is safe"),
    FinalityLevel::Justified => println!("Block is justified"),
    FinalityLevel::Finalized => println!("Block is finalized"),
}

// Set up reorganization detection
let reorg_detector = ReorgDetector::new();

// Register a handler for reorganizations
struct MyReorgHandler;

#[async_trait]
impl ReorgHandler for MyReorgHandler {
    async fn handle_reorg(&self, reorg_info: ReorgInfo) -> Result<()> {
        match reorg_info.reorg_type {
            ReorgType::Simple => {
                println!("Simple reorg detected: {} blocks", reorg_info.depth);
            }
            ReorgType::Deep => {
                println!("Deep reorg detected: {} blocks", reorg_info.depth);
                // Handle deep reorganization
            }
            ReorgType::Complex => {
                println!("Complex reorg detected: {} blocks", reorg_info.depth);
                // Handle complex reorganization
            }
        }
        Ok(())
    }
}

reorg_detector.register_handler(Arc::new(MyReorgHandler)).await;
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

```rust
// Ethereum configuration
let eth_config = FinalityConfig {
    chain_id: "ethereum".to_string(),
    confirmation_blocks: 12,
    finality_delay: 64,
    enable_deep_reorg_detection: true,
    max_reorg_depth: 100,
};

// Cosmos configuration (instant finality)
let cosmos_config = FinalityConfig {
    chain_id: "cosmos".to_string(),
    confirmation_blocks: 1,
    finality_delay: 1,
    enable_deep_reorg_detection: false,
    max_reorg_depth: 0,
};

// Solana configuration
let solana_config = FinalityConfig {
    chain_id: "solana".to_string(),
    confirmation_blocks: 32,
    finality_delay: 32,
    enable_deep_reorg_detection: true,
    max_reorg_depth: 50,
};
```

## Testing

Run tests with:
```bash
cargo test -p almanac-consensus
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