# Almanac Crate API Documentation

This document outlines how to use Almanac as an imported Rust crate, focusing on its stable public interfaces for cross-chain blockchain indexing.

## Core Concepts

Almanac provides a unified way to access blockchain events and state across multiple chains through a consistent API. The core concepts are:

- **Events**: Structured data emitted by on-chain activities
- **Storage**: Hybrid PostgreSQL/RocksDB storage for performance and flexibility
- **Chains**: Different blockchain networks (Ethereum, Cosmos, etc.)
- **Valence Contracts**: Specialized indexing for Valence protocol contracts
- **Causality**: Content-addressed entity tracking with SMT-based proofs

## API Overview

When using Almanac as a Rust crate, you'll primarily interact with these core interfaces:

1. `Storage`: Main storage interface for indexed data
2. `EventService`: Chain-specific event processing
3. `CausalityIndexer`: Content-addressed causality tracking

## Using the Storage API

The `Storage` trait is the primary interface for accessing indexed blockchain data.

```rust
use indexer_storage::{Storage, BoxedStorage, create_postgres_storage, ValenceAccountInfo};
use indexer_core::{Result, BlockStatus};

/// Main storage trait for accessing indexed data
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    /// Store an event
    async fn store_event(&self, chain: &str, event: Box<dyn Event>) -> Result<()>;
    
    /// Get events by chain and block range
    async fn get_events(&self, chain: &str, from_block: u64, to_block: u64) -> Result<Vec<Box<dyn Event>>>;
    
    /// Get the latest block height for a chain
    async fn get_latest_block(&self, chain: &str) -> Result<u64>;
    
    /// Get the latest block with specific finality status
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64>;
    
    /// Store Valence account information
    async fn store_valence_account_instantiation(
        &self,
        account_info: ValenceAccountInfo,
        initial_libraries: Vec<ValenceAccountLibrary>,
    ) -> Result<()>;
    
    /// Get Valence account state
    async fn get_valence_account_state(&self, account_id: &str) -> Result<Option<ValenceAccountState>>;
}
```

### Getting Started with Almanac Crate

First, add Almanac crates to your Cargo.toml:

```toml
[dependencies]
indexer-core = { path = "path/to/almanac/crates/core" }
indexer-storage = { path = "path/to/almanac/crates/storage", features = ["postgres", "rocks"] }
indexer-ethereum = { path = "path/to/almanac/crates/ethereum" }
indexer-cosmos = { path = "path/to/almanac/crates/cosmos" }
# Or if published to crates.io:
# indexer-storage = { version = "0.1.0", features = ["postgres", "rocks"] }
```

Then, in your code:

```rust
use indexer_storage::{create_postgres_storage, create_rocks_storage, BoxedStorage};
use indexer_core::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create PostgreSQL storage
    let pg_storage = create_postgres_storage(
        "postgresql://postgres:postgres@localhost:5432/indexer"
    ).await?;
    
    // Create RocksDB storage
    let rocks_storage = create_rocks_storage("./data/rocksdb")?;
    
    // Use storage for queries
    let latest_block = pg_storage.get_latest_block("ethereum").await?;
    println!("Latest Ethereum block: {}", latest_block);
    
    Ok(())
}
```

### Working with Valence Account Data

Query and manage Valence account information:

```rust
use indexer_storage::{Storage, ValenceAccountInfo, ValenceAccountState, ValenceAccountLibrary};
use indexer_core::Result;

async fn track_valence_account(
    storage: &dyn Storage, 
    chain_id: &str, 
    contract_address: &str
) -> Result<()> {
    let account_id = format!("{}:{}", chain_id, contract_address);
    
    // Store new account
    let account_info = ValenceAccountInfo {
        id: account_id.clone(),
        chain_id: chain_id.to_string(),
        contract_address: contract_address.to_string(),
        created_at_block: 12345,
        created_at_tx: "0xabc123...".to_string(),
        current_owner: Some("0xowner123...".to_string()),
        pending_owner: None,
        pending_owner_expiry: None,
        last_updated_block: 12345,
        last_updated_tx: "0xabc123...".to_string(),
    };
    
    let initial_libraries = vec![
        ValenceAccountLibrary {
            account_id: account_id.clone(),
            library_address: "0xlibrary123...".to_string(),
            approved_at_block: 12345,
            approved_at_tx: "0xabc123...".to_string(),
        }
    ];
    
    storage.store_valence_account_instantiation(account_info, initial_libraries).await?;
    
    // Retrieve account state
    if let Some(state) = storage.get_valence_account_state(&account_id).await? {
        println!("Account {} has {} approved libraries", 
                 state.account_id, state.libraries.len());
    }
    
    Ok(())
}
```

### Working with Cross-Chain Messages

Track cross-chain message processing:

```rust
use indexer_storage::{Storage, ValenceProcessorMessage, ValenceMessageStatus};

async fn track_processor_message(
    storage: &dyn Storage,
    processor_id: &str,
    source_chain: &str,
    target_chain: &str
) -> Result<()> {
    let message = ValenceProcessorMessage {
        id: "msg_123".to_string(),
        processor_id: processor_id.to_string(),
        source_chain_id: source_chain.to_string(),
        target_chain_id: target_chain.to_string(),
        sender_address: "0xsender123...".to_string(),
        payload: "base64_encoded_payload".to_string(),
        status: ValenceMessageStatus::Pending,
        created_at_block: 12345,
        created_at_tx: "0xsource_tx...".to_string(),
        last_updated_block: 12345,
        processed_at_block: None,
        processed_at_tx: None,
        retry_count: 0,
        next_retry_block: None,
        gas_used: None,
        error: None,
    };
    
    storage.store_valence_processor_message(message).await?;
    
    // Later, update message status
    storage.update_valence_processor_message_status(
        "msg_123",
        ValenceMessageStatus::Completed,
        Some(12350), // processed_block
        Some("0xtarget_tx..."), // processed_tx
        None, // retry_count
        None, // next_retry_block
        Some(21000), // gas_used
        None, // error
    ).await?;
    
    Ok(())
}
```

## Using Chain Adapters

Access blockchain-specific functionality through chain adapters:

```rust
use indexer_ethereum::EthereumClient;
use indexer_cosmos::CosmosClientWrapper;
use indexer_core::service::EventService;

async fn setup_chain_clients() -> Result<()> {
    // Ethereum client
    let eth_client = EthereumClient::new(
        "1".to_string(), // chain_id
        "http://localhost:8545".to_string(), // rpc_url
    ).await?;
    
    let latest_eth_block = eth_client.get_latest_block().await?;
    println!("Latest Ethereum block: {}", latest_eth_block);
    
    // Cosmos client
    let cosmos_client = CosmosClientWrapper::new(
        "cosmoshub-4".to_string(), // chain_id
        "http://localhost:9090".to_string(), // grpc_url
        "abandon abandon abandon...".to_string(), // mnemonic
    ).await?;
    
    let latest_cosmos_block = cosmos_client.get_latest_block().await?;
    println!("Latest Cosmos block: {}", latest_cosmos_block);
    
    Ok(())
}
```

## Using the Causality Indexer

Work with content-addressed entities and causality relationships:

```rust
use indexer_causality::{
    CausalityIndexer, CausalityIndexerConfig, 
    MemorySmtBackend, MemoryCausalityStorage,
    CausalityResource, CausalityEffect, EntityId, DomainId
};

async fn setup_causality_indexing() -> Result<()> {
    // Create causality indexer
    let config = CausalityIndexerConfig::default();
    let storage = Box::new(MemoryCausalityStorage::new());
    let smt_backend = MemorySmtBackend::new();
    
    let mut indexer = CausalityIndexer::new(config, storage, smt_backend)?;
    
    // Create a content-addressed resource
    let resource = CausalityResource {
        id: EntityId::new([0u8; 32]), // content hash
        name: "Token Balance".to_string(),
        domain_id: DomainId::new([1u8; 32]), // domain hash
        resource_type: "token".to_string(),
        quantity: 1000,
        timestamp: std::time::SystemTime::now(),
    };
    
    // Index the resource
    indexer.index_resource(resource).await?;
    
    // Query entities by domain
    let domain_entities = indexer.get_domain_entities(&DomainId::new([1u8; 32])).await?;
    println!("Found {} entities in domain", domain_entities.len());
    
    Ok(())
}
```

## Block Finality Support

Work with different levels of block finality:

```rust
use indexer_core::BlockStatus;
use indexer_storage::Storage;

async fn query_by_finality(storage: &dyn Storage, chain: &str) -> Result<()> {
    // Get latest finalized block
    let finalized_block = storage
        .get_latest_block_with_status(chain, BlockStatus::Finalized)
        .await?;
    
    // Get events only from finalized blocks
    let finalized_events = storage
        .get_events_with_status(
            chain, 
            finalized_block.saturating_sub(100), 
            finalized_block,
            BlockStatus::Finalized
        )
        .await?;
    
    println!("Found {} finalized events in last 100 blocks", finalized_events.len());
    
    // Update block status
    storage.update_block_status(chain, 12345, BlockStatus::Safe).await?;
    
    Ok(())
}
```

## Error Handling

Almanac provides comprehensive error handling:

```rust
use indexer_core::{Error, Result};

async fn handle_indexer_errors(storage: &dyn Storage) {
    match storage.get_latest_block("nonexistent_chain").await {
        Ok(block) => println!("Latest block: {}", block),
        Err(Error::NotFound(msg)) => println!("Chain not found: {}", msg),
        Err(Error::Database(msg)) => println!("Database error: {}", msg),
        Err(Error::Connection(msg)) => println!("Connection error: {}", msg),
        Err(err) => println!("Other error: {}", err),
    }
}
```

## Configuration

Configure storage backends and chain connections:

```rust
use indexer_storage::{create_postgres_storage, create_rocks_storage};

async fn setup_storage_with_config() -> Result<BoxedStorage> {
    // PostgreSQL with custom configuration
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/indexer".to_string());
    
    let storage = create_postgres_storage(&database_url).await?;
    
    // Or RocksDB with custom path
    let rocksdb_path = std::env::var("ROCKSDB_PATH")
        .unwrap_or_else(|_| "./data/rocksdb".to_string());
    
    let rocks_storage = create_rocks_storage(&rocksdb_path)?;
    
    Ok(storage)
}
```

## Testing Support

Almanac provides testing utilities:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use indexer_storage::MemoryCausalityStorage;

    #[tokio::test]
    async fn test_valence_account_lifecycle() {
        let storage = Box::new(MemoryCausalityStorage::new());
        
        // Test account creation and updates
        // ... test implementation
        
        assert!(true); // Your assertions here
    }
}
```

## Performance Considerations

- Use **RocksDB storage** for high-frequency read operations
- Use **PostgreSQL storage** for complex queries and relationships
- Leverage **block finality levels** appropriate for your security requirements
- Batch operations when possible to improve throughput
- Consider **causality indexing** for applications requiring cryptographic proofs 