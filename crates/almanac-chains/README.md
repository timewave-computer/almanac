# Almanac Chain Adapters

This crate provides blockchain-specific adapter implementations for the Almanac indexing system.

## Overview

The `almanac-chains` crate defines the `ChainAdapter` trait and provides implementations for different blockchains. Each adapter normalizes blockchain data into a common format (`ChainDataIR`) that can be processed by the indexing system.

## Supported Chains

- **Ethereum** - Full support for state extraction, storage proofs, and finality tracking
- **Cosmos** - Support for Tendermint-based chains with instant finality
- **Solana** - Support for Solana blockchain with account-based state tracking

## Architecture

```
almanac-chains/
├── src/
│   ├── adapters/
│   │   ├── traits.rs      # ChainAdapter trait definition
│   │   ├── types.rs       # Common types (ChainDataIR, etc.)
│   │   ├── ethereum.rs    # Ethereum implementation
│   │   ├── cosmos.rs      # Cosmos implementation
│   │   └── solana.rs      # Solana implementation
│   └── lib.rs
└── tests/
    ├── ethereum_adapter_test.rs
    └── cosmos_adapter_test.rs
```

## Usage

### Adding the Dependency

```toml
[dependencies]
almanac-chains = { path = "../almanac-chains", features = ["ethereum"] }
```

### Creating an Adapter

```rust
use almanac_chains::{ChainAdapter, ChainConfig, EthereumAdapter};

let config = ChainConfig {
    chain_id: "ethereum-mainnet".to_string(),
    rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string(),
    api_key: None,
    confirmations: 6,
    max_retries: 3,
    timeout_secs: 30,
    extra: serde_json::Value::Null,
};

let adapter = EthereumAdapter::new(config).await?;
```

### Fetching Blocks

```rust
// Get latest block number
let latest = adapter.get_latest_block().await?;

// Fetch normalized block data
let block_data = adapter.fetch_block(latest - 1).await?;

// Access normalized data
println!("Block: {}", block_data.block_metadata.block_height);
println!("Events: {}", block_data.state_changes.len());
```

## Implementing a New Chain Adapter

To add support for a new blockchain:

1. Create a new module in `src/adapters/`
2. Implement the `ChainAdapter` trait
3. Add feature flag to `Cargo.toml`
4. Create integration tests

Example structure:

```rust
use async_trait::async_trait;
use crate::adapters::{ChainAdapter, ChainAdapterError, ChainDataIR};

pub struct MyChainAdapter {
    client: MyChainClient,
    config: ChainConfig,
}

#[async_trait]
impl ChainAdapter for MyChainAdapter {
    fn chain_id(&self) -> &str {
        &self.config.chain_id
    }
    
    async fn fetch_block(&self, block_number: u64) -> Result<ChainDataIR, ChainAdapterError> {
        // Implementation
    }
    
    // ... other trait methods
}
```

## Data Normalization

All chain adapters normalize blockchain data into the `ChainDataIR` format:

- **Block Metadata** - Height, hash, timestamp, roots
- **State Changes** - Storage modifications with proofs
- **Consensus Metadata** - Chain-specific consensus data
- **Finality Status** - Pending, Safe, Finalized

## Testing

### Unit Tests

```bash
cargo test -p almanac-chains
```

### Integration Tests

Integration tests require RPC endpoints:

```bash
# Set RPC URLs
export ETHEREUM_TEST_RPC_URL="https://eth-goerli.g.alchemy.com/v2/YOUR_KEY"
export COSMOS_TEST_RPC_URL="https://rpc.testnet.cosmos.network"

# Run integration tests
cargo test -p almanac-chains --features ethereum,cosmos -- --ignored
```

## Features

- `ethereum` - Ethereum adapter with ethers-rs
- `cosmos` - Cosmos adapter with cosmrs and tendermint-rpc
- `solana` - Solana adapter with solana-client and solana-sdk

## Performance Considerations

- Adapters use connection pooling for RPC calls
- Batch operations are supported where possible
- State changes are extracted efficiently using traces/events
- Storage proofs are generated on-demand

## Security

- RPC connections use TLS
- API keys are stored securely
- Input validation on all chain data
- Proof verification for storage claims