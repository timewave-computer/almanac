# Ethereum Block Finality Tracking

This document provides a comprehensive guide to the implementation of Ethereum block finality tracking in the indexer.

## Overview

Ethereum's consensus mechanism (post-merge) provides different levels of block finality, ranging from newly mined blocks to fully finalized blocks. The indexer tracks these finality levels to allow applications to make informed decisions about which blocks they consider safe to use.

## Finality Levels

Ethereum blocks can have the following finality statuses:

1. **Confirmed**: Blocks that have been included in the chain but have not yet reached any higher level of finality. These blocks are still subject to reorganizations.

2. **Safe**: Blocks that have enough attestations to be considered safe to use for most applications. These blocks can still be reverted in certain edge cases but are generally considered reliable.

3. **Justified**: Blocks that have been voted on by a supermajority of validators in the current epoch. This is an intermediate step toward finalization.

4. **Finalized**: Blocks that have been irreversibly agreed upon by the consensus mechanism. These blocks cannot be reverted without extraordinary circumstances like a hard fork.

## Implementation Components

### 1. Database Schema

The blocks table has columns to track the finality status of each block:

```sql
ALTER TABLE blocks 
ADD COLUMN is_safe BOOLEAN NOT NULL DEFAULT false;
ADD COLUMN is_justified BOOLEAN NOT NULL DEFAULT false;
ADD COLUMN is_finalized BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX idx_blocks_finalized ON blocks (chain, is_finalized);
CREATE INDEX idx_blocks_chain_finalized_block_number ON blocks (chain, is_finalized, block_number DESC);
```

### 2. Provider Interface

The `EthereumProvider` uses Ethereum RPC methods to query finality status:

```rust
/// Get a block by number with a specific status requirement
pub async fn get_block_by_status(&self, status: BlockStatus) -> Result<(Block<Transaction>, u64)> {
    let block_tag = match status {
        BlockStatus::Confirmed => BlockTag::Latest,
        BlockStatus::Safe => BlockTag::Safe,
        BlockStatus::Justified => BlockTag::Finalized, // Ethereum RPC doesn't expose "justified" directly
        BlockStatus::Finalized => BlockTag::Finalized,
    };
    
    // Query the Ethereum node for a block with this tag
    // ...
}

/// Get latest finalized block number
pub async fn get_finalized_block_number(&self) -> Result<u64> {
    // Query the Ethereum node for the latest finalized block
    // ...
}

/// Get latest safe block number
pub async fn get_safe_block_number(&self) -> Result<u64> {
    // Query the Ethereum node for the latest safe block
    // ...
}
```

### 3. Event Service

The `EthereumEventService` provides methods to access finality information:

```rust
/// Get the latest finalized block number
pub async fn get_latest_finalized_block(&self) -> Result<u64> {
    self.provider.get_finalized_block_number().await
}

/// Get the latest safe block number
pub async fn get_latest_safe_block(&self) -> Result<u64> {
    self.provider.get_safe_block_number().await
}

/// Get a block with a specific finality status
pub async fn get_block_by_status(&self, status: BlockStatus) -> Result<(Block<Transaction>, u64)> {
    self.provider.get_block_by_status(status).await
}
```

### 4. Background Worker

A background task periodically queries the Ethereum node for the latest finality information and updates the database:

```rust
async fn update_ethereum_finality_status(
    service: &EthereumEventService,
    storage: &Arc<dyn Storage>
) -> Result<()> {
    // Get the latest finalized and safe block numbers
    let finalized_block = service.get_latest_finalized_block().await?;
    let safe_block = service.get_latest_safe_block().await?;
    
    // Update the database with this information
    storage.update_block_status("ethereum", finalized_block, BlockStatus::Finalized).await?;
    storage.update_block_status("ethereum", safe_block, BlockStatus::Safe).await?;
    
    Ok(())
}
```

## API Integration

### GraphQL API

The GraphQL API provides methods to query blocks and events based on finality status:

```graphql
# Query the latest finalized block
query {
  latestFinalizedBlock(chain: "ethereum")
}

# Query events only from finalized blocks
query {
  eventsWithStatus(
    chain: "ethereum",
    status: "finalized",
    eventTypes: ["Transfer"]
  ) {
    id
    blockNumber
    eventType
  }
}
```

### HTTP API

The HTTP API provides endpoints to query blocks and events based on finality status:

```
GET /blocks/:chain/latest/finalized
GET /blocks/:chain/latest/safe
GET /blocks/:chain/latest/:status
GET /events/status/:status
```

## Usage Recommendations

Applications should choose the appropriate finality level based on their security requirements:

- **High Security**: Use only finalized blocks for applications handling high-value assets or requiring maximum security.
- **Medium Security**: Use safe blocks for applications that need a balance between recency and security.
- **Low Latency**: Use confirmed blocks for applications prioritizing real-time data where occasional reorgs are acceptable.

## Testing

The finality tracking system is tested at multiple levels:

1. **Unit Tests**: Test individual components like the provider and service methods that interact with block finality.
2. **Integration Tests**: Test the end-to-end flow of finality tracking across multiple components.
3. **API Tests**: Test the GraphQL and HTTP endpoints to ensure they correctly filter by finality status.

## Limitations

- The "justified" status isn't directly exposed by Ethereum RPC, so we use the "finalized" status as a proxy.
- Block finality information is only available on post-merge Ethereum networks (those running the consensus layer).
- The finality status is only as reliable as the connected Ethereum node's implementation. 