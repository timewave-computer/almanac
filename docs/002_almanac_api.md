# Almanac API Documentation

## Overview

Almanac provides a unified way to access blockchain events and state across multiple chains through consistent interfaces. The system supports Ethereum, Cosmos, and future blockchain networks with specialized tracking for Valence protocol contracts.

### Core Concepts

- **Cross-Chain Indexing**: Index events from multiple blockchain networks through unified interfaces
- **Valence Protocol Support**: Specialized tracking for Valence Accounts, Processors, Authorization, and Libraries  
- **Hybrid Storage**: High-performance RocksDB combined with PostgreSQL for complex queries
- **Block Finality**: Multi-level finality tracking (Confirmed, Safe, Justified, Finalized)
- **Real-time Updates**: WebSocket subscriptions for live event streaming

## API Interfaces

Almanac provides three main ways to access indexed data:

### 1. HTTP REST API
RESTful endpoints for web applications and simple integrations.

### 2. Rust Crate Integration
Direct library integration for Rust applications requiring high performance.

### 3. Real-time Subscriptions
WebSocket API for live event streams and real-time updates.

## Getting Started

### Service Setup

Start the Almanac service to access HTTP and WebSocket APIs:

```bash
# Start with default configuration
almanac start --config config.toml

# Check service status
almanac status
```

### Rust Crate Integration

Add Almanac crates to your `Cargo.toml`:

```toml
[dependencies]
indexer-core = { path = "path/to/almanac/crates/core" }
indexer-storage = { path = "path/to/almanac/crates/storage", features = ["postgres", "rocks"] }
indexer-ethereum = { path = "path/to/almanac/crates/ethereum" }
indexer-cosmos = { path = "path/to/almanac/crates/cosmos" }
```

## Storage Interface

The core `Storage` trait provides access to all indexed blockchain data:

```rust
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    // Basic blockchain data
    async fn get_latest_block(&self, chain: &str) -> Result<u64>;
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64>;
    async fn get_events(&self, chain: &str, from_block: u64, to_block: u64) -> Result<Vec<Box<dyn Event>>>;
    
    // Valence Account management
    async fn store_valence_account_instantiation(
        &self, account_info: ValenceAccountInfo, initial_libraries: Vec<ValenceAccountLibrary>
    ) -> Result<()>;
    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>>;
    
    // Cross-chain message tracking
    async fn store_valence_processor_message(&self, message: ValenceProcessorMessage) -> Result<()>;
    async fn update_valence_processor_message_status(
        &self, message_id: &str, new_status: ValenceMessageStatus
    ) -> Result<()>;
}
```

### Creating Storage Connections

```rust
use indexer_storage::{create_postgres_storage, create_rocks_storage};

#[tokio::main]
async fn main() -> Result<()> {
    // PostgreSQL for complex queries
    let pg_storage = create_postgres_storage(
        "postgresql://postgres:postgres@localhost:5432/indexer"
    ).await?;
    
    // RocksDB for high-performance reads
    let rocks_storage = create_rocks_storage("./data/rocksdb")?;
    
    // Query latest block
    let latest_block = pg_storage.get_latest_block("ethereum").await?;
    println!("Latest Ethereum block: {}", latest_block);
    
    Ok(())
}
```

## HTTP REST API

All REST endpoints are prefixed with `/api/v1`. Authentication uses Bearer tokens in the `Authorization` header.

### Chain Status

Get indexing status for a specific blockchain:

```
GET /chains/{chain_id}/status
```

Example response:
```json
{
  "chain_id": "ethereum",
  "latest_height": 18256432,
  "finalized_height": 18256400,
  "is_indexing": true,
  "last_indexed_at": 1684231456
}
```

### Event Queries

Query events by address:
```
GET /events/address/{chain_id}/{address}?limit=100&offset=0
```

Query events by chain and block range:
```
GET /events/chain/{chain_id}?from_height=18000000&to_height=18001000
```

Filter events with complex criteria:
```
POST /events/filter
Content-Type: application/json

{
  "chain_id": "ethereum",
  "event_type": "Transfer",
  "from_height": 18000000,
  "limit": 50
}
```

## WebSocket API

Connect to the WebSocket endpoint for real-time event subscriptions:

```
ws://localhost:8000/api/v1/ws
```

### Subscribe to Events

Send a subscription message:
```json
{
  "action": "subscribe",
  "filter": {
    "chain_id": "ethereum",
    "event_type": "Transfer",
    "address": "0x1234567890abcdef1234567890abcdef12345678"
  }
}
```

### Receive Events

Events matching your filter arrive in real-time:
```json
{
  "type": "event",
  "data": {
    "chain_id": "ethereum",
    "height": 18256542,
    "tx_hash": "0x...",
    "event_type": "Transfer",
    "attributes": {
      "from": "0x...",
      "to": "0x...",
      "value": "5000000000000000000"
    },
    "timestamp": 1684231890
  }
}
```

## Block Finality Support

Almanac tracks multiple finality levels for robust security guarantees:

```rust
pub enum BlockStatus {
    Confirmed,  // Included in chain, may be reorganized
    Safe,       // Unlikely to be orphaned  
    Justified,  // Voted by validators (Ethereum PoS)
    Finalized,  // Irreversible consensus
}
```

Query by finality level:
```rust
// Get latest finalized block
let finalized_block = storage
    .get_latest_block_with_status("ethereum", BlockStatus::Finalized)
    .await?;

// Get only finalized events
let events = storage.get_events_with_status(
    "ethereum", start_block, end_block, BlockStatus::Finalized
).await?;
```

## Valence Contract Integration

### Valence Accounts

Track account creation, ownership, and library approvals:

```rust
let account_info = ValenceAccountInfo {
    id: "ethereum:0x1234...".to_string(),
    chain_id: "ethereum".to_string(),
    contract_address: "0x1234...".to_string(),
    current_owner: Some("0xowner...".to_string()),
    // ...
};

storage.store_valence_account_instantiation(account_info, initial_libraries).await?;
```

### Cross-Chain Message Tracking

Track message lifecycle across blockchain networks:

```rust
let message = ValenceProcessorMessage {
    id: "msg_123".to_string(),
    processor_id: "ethereum:0xprocessor...".to_string(),
    source_chain_id: "ethereum".to_string(),
    target_chain_id: "cosmos".to_string(),
    status: ValenceMessageStatus::Pending,
    // ...
};

storage.store_valence_processor_message(message).await?;

// Update status as processing progresses
storage.update_valence_processor_message_status(
    "msg_123",
    ValenceMessageStatus::Completed,
    Some(target_block),
    Some("0xtarget_tx..."),
    Some(gas_used),
    None, // error
).await?;
```

## Chain Adapters

Access blockchain-specific functionality:

```rust
// Ethereum client
let eth_client = EthereumClient::new(
    "1".to_string(), 
    "http://localhost:8545".to_string(),
).await?;

// Cosmos client
let cosmos_client = CosmosClientWrapper::new(
    "cosmoshub-4".to_string(),
    "http://localhost:9090".to_string(),
    "abandon abandon abandon...".to_string(),
).await?;
```

## Configuration

Configure Almanac through TOML files:

```toml
# config.toml
environment = "development"

[database]
postgres_url = "postgresql://postgres:postgres@localhost:5432/indexer"
rocksdb_path = "./data/rocksdb"

[chains.ethereum]
chain_id = "1"
rpc_url = "http://localhost:8545"
start_block = 18000000

[api]
host = "0.0.0.0"
port = 8080
enable_graphql = true
enable_websockets = true
```

Override with environment variables:
```bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/indexer"
export ETHEREUM_RPC_URL="http://localhost:8545"
```

## Error Handling

Comprehensive error types for robust applications:

```rust
use indexer_core::{Error, Result};

match storage.get_valence_account_state("invalid_id").await {
    Ok(Some(state)) => println!("Found account: {}", state.account_id),
    Ok(None) => println!("Account not found"),
    Err(Error::Database(msg)) => println!("Database error: {}", msg),
    Err(Error::Chain { chain, message }) => println!("Chain {} error: {}", chain, message),
    Err(err) => println!("Other error: {}", err),
}
```

## Performance Characteristics

Almanac is optimized for high performance:

- Block processing latency: < 500ms
- Event indexing latency: < 1 second from block finality  
- RocksDB read latency: < 50ms (99th percentile)
- PostgreSQL query latency: < 500ms (95th percentile)
- Concurrent query support: 100+ concurrent operations

## Command Line Interface

The CLI provides administrative capabilities:

```bash
# Start/stop service
almanac start --config config.toml
almanac stop

# Chain management
almanac chains add ethereum --rpc-url https://mainnet.infura.io/v3/API_KEY
almanac chains status ethereum

# Query events
almanac events address ethereum 0x1234... --limit 10
almanac events chain cosmos --from-height 15000000

# Database management
almanac db init
almanac db migrate
```

## Integration Examples

### Python Integration

```python
import requests
import websockets
import json

# HTTP API usage
response = requests.get("http://localhost:8000/api/v1/chains/ethereum/status")
status = response.json()
print(f"Latest block: {status['latest_height']}")

# WebSocket subscription
async def subscribe_to_events():
    uri = "ws://localhost:8000/api/v1/ws"
    async with websockets.connect(uri) as websocket:
        subscription = {
            "action": "subscribe",
            "filter": {"chain_id": "ethereum", "event_type": "Transfer"}
        }
        await websocket.send(json.dumps(subscription))
        
        while True:
            response = await websocket.recv()
            event = json.loads(response)
            print(f"New event: {event['data']['event_type']}")
```

### Node.js Integration

```javascript
const axios = require('axios');
const WebSocket = require('ws');

// HTTP API usage
async function getChainStatus(chainId) {
    const response = await axios.get(`http://localhost:8000/api/v1/chains/${chainId}/status`);
    console.log(`Latest ${chainId} block: ${response.data.latest_height}`);
}

// WebSocket subscription
const ws = new WebSocket('ws://localhost:8000/api/v1/ws');
ws.on('open', () => {
    ws.send(JSON.stringify({
        action: 'subscribe',
        filter: { chain_id: 'ethereum', event_type: 'Transfer' }
    }));
});

ws.on('message', (data) => {
    const event = JSON.parse(data);
    console.log('New event:', event.data.event_type);
});
```

This unified API documentation provides everything needed to integrate with Almanac, whether using the HTTP service, Rust crates, or real-time subscriptions. The hybrid storage architecture and finality tracking enable both high-performance access and complex analytical queries across multiple blockchain networks. 