## Block Finality Tracking

### Ethereum Finality Levels

The indexer tracks multiple levels of finality for Ethereum blocks:

1. **Confirmed**: Blocks that have been included in the Ethereum chain but have not yet reached a safer level of finality. These blocks are still subject to potential reorganizations.

2. **Safe**: Blocks that have enough attestations to be considered unlikely to be orphaned in most situations, but are not yet fully finalized. This corresponds to the `safe` tag in Ethereum RPC calls.

3. **Justified**: Blocks that have been voted on by validators in the current epoch to move toward finalization. This is specific to Ethereum's proof-of-stake consensus.

4. **Finalized**: Blocks that have been irreversibly agreed upon by the Ethereum consensus (via the beacon chain) and cannot be reverted without slashing a large number of validators. These blocks have the strongest finality guarantees.

### Querying Blocks by Finality Status

The API provides several endpoints to query blocks and events based on their finality status:

#### GraphQL API

```graphql
# Get the latest finalized block for Ethereum
query LatestFinalizedBlock {
  latestFinalizedBlock(chain: "ethereum")
}

# Get events only from finalized blocks
query FinalizedEvents {
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

#### HTTP API

```
# Get the latest finalized block for Ethereum
GET /blocks/ethereum/latest/finalized

# Get the latest safe block for Ethereum
GET /blocks/ethereum/latest/safe

# Get events only from finalized blocks
GET /events/status/finalized?chain=ethereum&event_types=Transfer
```

### Implementation Details

The system periodically polls Ethereum nodes to track the status of blocks using the node's built-in finality indicators. It then updates the database with this status information so that applications can choose to interact only with blocks that have reached their required level of finality.

This is particularly important for applications that need to make security tradeoffs:

- Applications requiring maximum security should use only finalized blocks
- Applications that need a better balance of recency and security can use safe blocks
- Applications prioritizing real-time data can use confirmed blocks

The finality tracking system runs in a background task that continually updates block statuses, ensuring that the indexer always has the most current finality information available. 