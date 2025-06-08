# Almanac API Documentation

This document outlines the stable public interface of Almanac, a cross-chain indexing and event storage system for Valence protocol and blockchain data.

## Core Concepts

Almanac provides a unified way to access blockchain events and state across multiple chains through a consistent API. The core concepts are:

- **Cross-Chain Indexing**: Index events from Ethereum, Cosmos, and future blockchain networks
- **Valence Protocol Support**: Specialized tracking for Valence Accounts, Processors, Authorization, and Libraries
- **Hybrid Storage**: High-performance RocksDB combined with PostgreSQL for complex queries
- **Causality Tracking**: Content-addressed entity relationships with cryptographic proofs
- **Block Finality**: Multi-level finality tracking (Confirmed, Safe, Justified, Finalized)

## API Interfaces

Almanac provides multiple interfaces for accessing indexed data:

1. **Storage Interface**: Core storage abstraction for indexed data
2. **HTTP REST API**: RESTful endpoints for web applications
3. **GraphQL API**: Complex relational queries
4. **WebSocket API**: Real-time event subscriptions

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
    
    // Valence Processor management  
    async fn store_valence_processor_instantiation(&self, processor_info: ValenceProcessorInfo) -> Result<()>;
    async fn store_valence_processor_message(&self, message: ValenceProcessorMessage) -> Result<()>;
    async fn update_valence_processor_message_status(
        &self, message_id: &str, new_status: ValenceMessageStatus, /* ... */
    ) -> Result<()>;
    
    // Valence Authorization management
    async fn store_valence_authorization_instantiation(
        &self, auth_info: ValenceAuthorizationInfo, initial_policy: Option<ValenceAuthorizationPolicy>
    ) -> Result<()>;
    async fn store_valence_authorization_grant(&self, grant: ValenceAuthorizationGrant) -> Result<()>;
    
    // Valence Library management
    async fn store_valence_library_instantiation(
        &self, library_info: ValenceLibraryInfo, initial_version: Option<ValenceLibraryVersion>
    ) -> Result<()>;
    async fn get_valence_library_state(&self, library_id: &str) -> Result<Option<ValenceLibraryState>>;
}
```

## Chain Adapters

Almanac uses `valence-domain-clients` for robust blockchain connectivity:

```rust
// Ethereum chains (Ethereum, Polygon, Base)
use indexer_ethereum::EthereumClient;

let eth_client = EthereumClient::new_with_config(EvmChainConfig::ethereum_mainnet(rpc_url)).await?;
let latest_block = eth_client.get_latest_block().await?;

// Cosmos chains (Noble, Osmosis, Neutron)  
use indexer_cosmos::CosmosClientWrapper;

let cosmos_client = CosmosClientWrapper::new_with_config(
    CosmosChainConfig::noble_mainnet(grpc_url), 
    mnemonic
).await?;
```

## Block Finality Levels

Almanac tracks multiple finality levels for robust application security:

```rust
pub enum BlockStatus {
    Confirmed,  // Included in chain, may be reorganized
    Safe,       // Unlikely to be orphaned  
    Justified,  // Voted by validators (Ethereum PoS)
    Finalized,  // Irreversible consensus
}

// Query by finality level
let finalized_block = storage.get_latest_block_with_status("ethereum", BlockStatus::Finalized).await?;
let finalized_events = storage.get_events_with_status(
    "ethereum", start_block, end_block, BlockStatus::Finalized
).await?;
```

## Valence Contract Types

### Valence Accounts
Track account creation, ownership, library approvals, and execution history:

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

### Valence Processors
Track cross-chain message processing:

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
```

### Valence Authorization
Track authorization policies and grants:

```rust
let grant = ValenceAuthorizationGrant {
    id: "grant_123".to_string(),
    auth_id: "ethereum:0xauth...".to_string(),
    grantee: "0xgrantee...".to_string(),
    permissions: vec!["execute".to_string(), "manage".to_string()],
    // ...
};

storage.store_valence_authorization_grant(grant).await?;
```

### Valence Libraries
Track library deployments and usage:

```rust
let library_info = ValenceLibraryInfo {
    id: "ethereum:0xlibrary...".to_string(),
    chain_id: "ethereum".to_string(),
    contract_address: "0xlibrary...".to_string(),
    library_type: "swap".to_string(),
    // ...
};

storage.store_valence_library_instantiation(library_info, Some(initial_version)).await?;
```

## Storage Backends

Almanac uses a hybrid storage approach:

```rust
// PostgreSQL for complex queries and relationships
let pg_storage = create_postgres_storage(
    "postgresql://postgres:postgres@localhost:5432/indexer"
).await?;

// RocksDB for high-performance reads
let rocks_storage = create_rocks_storage("./data/rocksdb")?;
```

## Cross-Chain Message Tracking

Track message lifecycle across different blockchain networks:

```rust
pub enum ValenceMessageStatus {
    Pending,     // Created but not yet processed
    Processing,  // Currently being processed
    Completed,   // Successfully processed
    Failed,      // Processing failed
    TimedOut,    // Processing timed out
}

// Update message status as it progresses
storage.update_valence_processor_message_status(
    "msg_123",
    ValenceMessageStatus::Completed,
    Some(target_block),
    Some("0xtarget_tx..."),
    None, // retry_count
    None, // next_retry_block
    Some(gas_used),
    None, // error
).await?;
```

## Error Handling

Comprehensive error types for robust applications:

```rust
use indexer_core::{Error, Result};

match storage.get_valence_account_state("invalid_id").await {
    Ok(Some(state)) => println!("Found account: {}", state.account_id),
    Ok(None) => println!("Account not found"),
    Err(Error::Database(msg)) => println!("Database error: {}", msg),
    Err(Error::NotFound(msg)) => println!("Not found: {}", msg),
    Err(Error::Chain { chain, message }) => println!("Chain {} error: {}", chain, message),
    Err(err) => println!("Other error: {}", err),
}
```

## Configuration

Configure Almanac through environment variables or configuration files:

```rust
// Environment-based configuration
std::env::set_var("DATABASE_URL", "postgresql://postgres:postgres@localhost:5432/indexer");
std::env::set_var("ROCKSDB_PATH", "./data/rocksdb");
std::env::set_var("ETHEREUM_RPC_URL", "http://localhost:8545");
std::env::set_var("COSMOS_GRPC_URL", "http://localhost:9090");

// Or JSON configuration
{
  "chains": {
    "ethereum": { "rpc_url": "http://localhost:8545", "chain_id": "1" },
    "cosmos": { "grpc_url": "http://localhost:9090", "chain_id": "cosmoshub-4" }
  },
  "storage": {
    "postgres_url": "postgresql://postgres:postgres@localhost:5432/indexer",
    "rocksdb_path": "./data/rocksdb"
  }
}
```

## Performance Characteristics

Almanac is designed for high performance:

- **Block processing latency**: < 500ms
- **Event indexing latency**: < 1 second from block finality  
- **RocksDB read latency**: < 50ms (99th percentile)
- **PostgreSQL query latency**: < 500ms (95th percentile)
- **Concurrent query support**: 100+ concurrent operations

## Future Extensions

The API is designed to support future enhancements:

- **Additional Chains**: Solana and Move-based blockchain support
- **Enhanced Causality**: Advanced proof generation and verification
- **Cross-Chain Analytics**: Sophisticated cross-chain relationship analysis
- **Real-Time Subscriptions**: WebSocket-based real-time event streams

## Integration Examples

For detailed integration examples and advanced usage patterns, see:

- [Almanac Crate API Documentation](almanac_crate_api.md) - Comprehensive Rust integration guide
- [Almanac Service API Documentation](almanac_service_api.md) - HTTP/WebSocket API reference 