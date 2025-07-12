# Almanac Contract Integration Tutorial

This tutorial provides a complete guide for integrating blockchain contracts into your applications using the Almanac indexer. The guide covers both Ethereum and Cosmos chains with shared patterns and chain-specific implementations.

## Prerequisites

- Rust 1.75 or later
- PostgreSQL database
- Blockchain node access (Ethereum RPC or Cosmos gRPC)
- Basic understanding of smart contracts and ABIs/schemas

## Project Setup

### Create New Project

```bash
cargo new my-blockchain-indexer
cd my-blockchain-indexer
```

### Add Dependencies

Update `Cargo.toml` with the core dependencies:

```toml
[package]
name = "my-blockchain-indexer"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core dependencies
indexer-core = { path = "path/to/almanac/crates/core" }
indexer-storage = { path = "path/to/almanac/crates/storage", features = ["postgres", "rocks"] }

# Chain-specific dependencies
indexer-ethereum = { path = "path/to/almanac/crates/ethereum" }
indexer-cosmos = { path = "path/to/almanac/crates/cosmos" }

# Async runtime and utilities
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
thiserror = "1.0"

# Database dependencies
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "postgres", "chrono", "uuid"] }

# Serialization and configuration
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# API server (optional)
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Configuration and CLI
config = "0.14"
clap = { version = "4.0", features = ["derive"] }
```

### Project Structure

Create a well-organized directory structure:

```
my-blockchain-indexer/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   ├── indexer/
│   │   ├── mod.rs
│   │   ├── ethereum.rs
│   │   └── cosmos.rs
│   ├── storage/
│   │   ├── mod.rs
│   │   └── models.rs
│   └── api/
│       ├── mod.rs
│       └── rest.rs
├── config/
│   ├── development.toml
│   └── production.toml
├── contracts/
│   ├── ethereum/
│   │   └── erc20_abi.json
│   └── cosmos/
│       └── cw20_schema.json
├── migrations/
└── tests/
    ├── integration_tests.rs
    └── contract_tests.rs
```

## Configuration System

### Configuration Structure

Create `src/config.rs`:

```rust
//! Configuration management for the blockchain indexer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub indexer: IndexerConfig,
    pub api: ApiConfig,
    pub logging: LoggingConfig,
    pub ethereum: Option<EthereumConfig>,
    pub cosmos: Option<CosmosConfig>,
    pub contracts: Vec<ContractConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IndexerConfig {
    pub start_block: Option<u64>,
    pub block_batch_size: u64,
    pub poll_interval: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EthereumConfig {
    pub provider_url: String,
    pub chain_id: u64,
    pub gas_limit: u64,
    pub confirmations: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CosmosConfig {
    pub grpc_url: String,
    pub chain_id: String,
    pub gas_limit: u64,
    pub gas_price: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContractConfig {
    pub name: String,
    pub chain: String, // "ethereum" or "cosmos"
    pub address: String,
    pub abi_or_schema_path: String,
    pub start_block: Option<u64>,
    pub features: Vec<String>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let config_path = format!("config/{}.toml", environment);
        
        let config_str = std::fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&config_str)?;
        
        config.override_from_env()?;
        Ok(config)
    }
    
    fn override_from_env(&mut self) -> anyhow::Result<()> {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            self.database.url = url;
        }
        
        if let Some(ref mut eth_config) = self.ethereum {
            if let Ok(url) = std::env::var("ETHEREUM_PROVIDER_URL") {
                eth_config.provider_url = url;
            }
        }
        
        if let Some(ref mut cosmos_config) = self.cosmos {
            if let Ok(url) = std::env::var("COSMOS_GRPC_URL") {
                cosmos_config.grpc_url = url;
            }
        }
        
        Ok(())
    }
}
```

### Configuration Files

Create `config/development.toml`:

```toml
[database]
url = "postgresql://localhost/blockchain_indexer_dev"
max_connections = 5
connection_timeout = 30

[indexer]
start_block = 18000000
block_batch_size = 100
poll_interval = 12
max_retries = 3

[api]
host = "127.0.0.1"
port = 3000
enable_cors = true

[logging]
level = "debug"
format = "pretty"

# Ethereum configuration
[ethereum]
provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
chain_id = 1
gas_limit = 6000000
confirmations = 12

# Cosmos configuration  
[cosmos]
grpc_url = "https://rpc.cosmos.network:443"
chain_id = "cosmoshub-4"
gas_limit = 200000
gas_price = "0.025uatom"

# Contract definitions
[[contracts]]
name = "usdc"
chain = "ethereum"
address = "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0"
abi_or_schema_path = "contracts/ethereum/usdc_abi.json"
features = ["client", "storage", "api"]

[[contracts]]
name = "cw20_token"
chain = "cosmos"
address = "cosmos1abc123..."
abi_or_schema_path = "contracts/cosmos/cw20_schema.json"
features = ["client", "storage"]
```

## Error Handling

Create `src/error.rs`:

```rust
//! Error types for the blockchain indexer

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Ethereum client error: {0}")]
    EthereumClient(String),
    
    #[error("Cosmos client error: {0}")]
    CosmosClient(String),
    
    #[error("Contract error: {0}")]
    Contract(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Block not found: {block_number} on chain {chain}")]
    BlockNotFound { chain: String, block_number: u64 },
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, IndexerError>;
```

## Contract Code Generation

Before integrating contracts, generate the necessary code using Almanac's codegen tools.

### Ethereum Contract Generation

```bash
# Generate code for ERC20 contract
almanac ethereum generate-contract contracts/ethereum/usdc_abi.json \
  --address 0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0 \
  --chain 1 \
  --features client,storage,api \
  --output-dir ./generated/ethereum/usdc
```

### Cosmos Contract Generation

```bash
# Generate code for CW20 contract
almanac cosmos generate-contract contracts/cosmos/cw20_schema.json \
  --address cosmos1abc123... \
  --chain cosmoshub-4 \
  --features client,storage \
  --output-dir ./generated/cosmos/cw20
```

## Chain-Specific Integration

### Ethereum Integration

Create `src/indexer/ethereum.rs`:

```rust
//! Ethereum-specific indexer implementation

use crate::{config::EthereumConfig, error::Result};
use indexer_ethereum::EthereumClient;
use std::sync::Arc;

pub struct EthereumIndexer {
    client: Arc<EthereumClient>,
    config: EthereumConfig,
}

impl EthereumIndexer {
    pub async fn new(config: EthereumConfig) -> Result<Self> {
        let client = EthereumClient::new(
            config.chain_id.to_string(),
            config.provider_url.clone(),
        ).await?;
        
        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }
    
    pub async fn start_indexing(&self) -> Result<()> {
        let latest_block = self.client.get_latest_block().await?;
        let start_block = self.config.start_block.unwrap_or(latest_block);
        
        // Implementation details...
        tracing::info!("Started Ethereum indexing from block {}", start_block);
        Ok(())
    }
    
    pub async fn index_block(&self, block_number: u64) -> Result<()> {
        // Block indexing implementation
        Ok(())
    }
}
```

### Cosmos Integration

Create `src/indexer/cosmos.rs`:

```rust
//! Cosmos-specific indexer implementation

use crate::{config::CosmosConfig, error::Result};
use indexer_cosmos::CosmosClientWrapper;
use std::sync::Arc;

pub struct CosmosIndexer {
    client: Arc<CosmosClientWrapper>,
    config: CosmosConfig,
}

impl CosmosIndexer {
    pub async fn new(config: CosmosConfig) -> Result<Self> {
        let client = CosmosClientWrapper::new(
            config.chain_id.clone(),
            config.grpc_url.clone(),
            "abandon abandon abandon...".to_string(), // Use proper mnemonic
        ).await?;
        
        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }
    
    pub async fn start_indexing(&self) -> Result<()> {
        let latest_block = self.client.get_latest_block().await?;
        let start_block = self.config.start_block.unwrap_or(latest_block);
        
        tracing::info!("Started Cosmos indexing from block {}", start_block);
        Ok(())
    }
    
    pub async fn index_block(&self, block_number: u64) -> Result<()> {
        // Block indexing implementation
        Ok(())
    }
}
```

## Storage Integration

Create `src/storage/models.rs`:

```rust
//! Database models for indexed data

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Block {
    pub id: i32,
    pub chain_id: String,
    pub block_number: i64,
    pub block_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub is_finalized: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i32,
    pub chain_id: String,
    pub block_id: i32,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: Option<String>,
    pub status: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ContractEvent {
    pub id: i32,
    pub chain_id: String,
    pub block_id: i32,
    pub transaction_id: i32,
    pub contract_address: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

## Main Application

Create `src/main.rs`:

```rust
//! Main application entry point

use anyhow::Result;
use tracing_subscriber::fmt;

mod config;
mod error;
mod indexer;
mod storage;

use config::Config;
use indexer::{ethereum::EthereumIndexer, cosmos::CosmosIndexer};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    fmt::init();
    
    // Load configuration
    let config = Config::load()?;
    
    // Initialize storage
    let pool = sqlx::postgres::PgPool::connect(&config.database.url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    // Start indexers based on configuration
    let mut handles = Vec::new();
    
    if let Some(eth_config) = config.ethereum {
        let indexer = EthereumIndexer::new(eth_config).await?;
        let handle = tokio::spawn(async move {
            indexer.start_indexing().await
        });
        handles.push(handle);
    }
    
    if let Some(cosmos_config) = config.cosmos {
        let indexer = CosmosIndexer::new(cosmos_config).await?;
        let handle = tokio::spawn(async move {
            indexer.start_indexing().await
        });
        handles.push(handle);
    }
    
    // Wait for all indexers
    for handle in handles {
        handle.await??;
    }
    
    Ok(())
}
```

## Testing Framework

Create `tests/integration_tests.rs`:

```rust
//! Integration tests for the indexer

use my_blockchain_indexer::{config::Config, indexer::*};
use sqlx::PgPool;
use std::sync::Arc;
use tempfile::NamedTempFile;

async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/indexer_test".to_string());
    
    let pool = PgPool::connect(&database_url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn test_ethereum_indexer() {
    let pool = setup_test_db().await;
    
    // Test Ethereum indexer functionality
    // Implementation depends on your specific requirements
    assert!(true);
}

#[tokio::test]
async fn test_cosmos_indexer() {
    let pool = setup_test_db().await;
    
    // Test Cosmos indexer functionality
    assert!(true);
}

#[tokio::test]
async fn test_contract_integration() {
    let pool = setup_test_db().await;
    
    // Test generated contract code integration
    assert!(true);
}
```

## API Server Integration

Create `src/api/rest.rs`:

```rust
//! REST API endpoints

use axum::{extract::Query, http::StatusCode, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct EventQuery {
    chain: String,
    from_block: Option<u64>,
    to_block: Option<u64>,
    limit: Option<u64>,
}

#[derive(Serialize)]
pub struct EventResponse {
    pub events: Vec<serde_json::Value>,
    pub total: u64,
}

pub fn create_router(pool: Arc<PgPool>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/events", get(get_events))
        .with_state(pool)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn get_events(
    Query(params): Query<EventQuery>,
    pool: axum::extract::State<Arc<PgPool>>,
) -> Result<Json<EventResponse>, StatusCode> {
    // Implementation for fetching events
    let events = vec![]; // Fetch from database
    let total = 0; // Count total events
    
    Ok(Json(EventResponse { events, total }))
}
```

## Deployment and Production

### Environment Configuration

Create production configuration in `config/production.toml`:

```toml
[database]
url = "${DATABASE_URL}"
max_connections = 20
connection_timeout = 60

[indexer]
block_batch_size = 1000
poll_interval = 6
max_retries = 5

[api]
host = "0.0.0.0"
port = 8080
enable_cors = false

[logging]
level = "info"
format = "json"
```

### Docker Configuration

Create a `Dockerfile`:

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/my-blockchain-indexer /usr/local/bin/

EXPOSE 8080
CMD ["my-blockchain-indexer"]
```

### Monitoring and Observability

Add health checks and metrics to your application:

```rust
use tracing::{info, warn, error};

// Add to your main indexing loop
async fn index_with_monitoring(&self, block_number: u64) -> Result<()> {
    let start_time = std::time::Instant::now();
    
    match self.index_block(block_number).await {
        Ok(_) => {
            let duration = start_time.elapsed();
            info!(
                block_number = block_number,
                duration_ms = duration.as_millis(),
                "Successfully indexed block"
            );
        }
        Err(e) => {
            error!(
                block_number = block_number,
                error = %e,
                "Failed to index block"
            );
            return Err(e);
        }
    }
    
    Ok(())
}
```

## Best Practices

### Performance Optimization

1. **Batch Processing**: Process multiple blocks together when possible
2. **Connection Pooling**: Use database connection pools efficiently
3. **Async Processing**: Leverage Rust's async capabilities for concurrent operations
4. **Caching**: Implement caching for frequently accessed data

### Error Recovery

1. **Retry Logic**: Implement exponential backoff for transient failures
2. **Circuit Breakers**: Prevent cascading failures
3. **Graceful Degradation**: Handle partial service outages
4. **Data Consistency**: Ensure atomicity of multi-step operations

### Security Considerations

1. **Input Validation**: Validate all external data inputs
2. **Rate Limiting**: Implement rate limiting for API endpoints
3. **Authentication**: Secure API endpoints with proper authentication
4. **Logging**: Log security-relevant events without exposing sensitive data

This comprehensive tutorial provides the foundation for building robust blockchain indexers with Almanac. The patterns shown work across both Ethereum and Cosmos chains, with the generated contract code providing type-safe integration for specific contract interactions. 