# Cosmos Code Integration Tutorial

This tutorial walks you through integrating generated Cosmos contract code into an Almanac indexer application from scratch.

## Prerequisites

- Rust 1.75 or later
- PostgreSQL 12+ 
- RocksDB
- Basic understanding of CosmWasm contracts
- Almanac indexer development environment

## Project Setup

### 1. Create New Indexer Project

```bash
# Create new Rust project
cargo new my-cosmos-indexer --lib
cd my-cosmos-indexer

# Initialize git repository
git init
```

### 2. Configure Dependencies

Add to `Cargo.toml`:

```toml
[package]
name = "my-cosmos-indexer"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core indexer dependencies
indexer-core = { path = "../almanac/crates/core" }
indexer-cosmos = { path = "../almanac/crates/cosmos" }

# Async runtime
tokio = { version = "1.0", features = ["full"] }

# Database
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls", "chrono", "uuid"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# CosmWasm types
cosmwasm-std = "2.0"
cosmwasm-schema = "2.0"

# Date/time handling
chrono = { version = "0.4", features = ["serde"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Configuration
config = "0.14"

[dev-dependencies]
tempfile = "3.8"
```

### 3. Project Structure

Create the following directory structure:

```
my-cosmos-indexer/
├── src/
│   ├── lib.rs
│   ├── config.rs
│   ├── error.rs
│   └── contracts/
│       └── mod.rs
├── contracts/          # Generated contract code goes here
├── schemas/            # CosmWasm message schemas
├── migrations/         # Database migrations
├── tests/
├── examples/
└── Cargo.toml
```

## Step 1: Configuration System

### Create Configuration Module

Create `src/config.rs`:

```rust
//! Configuration management for the cosmos indexer
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// Cosmos RPC configuration
    pub cosmos: CosmosConfig,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// API server configuration
    pub api: ApiConfig,
    
    /// Contracts to monitor
    pub contracts: Vec<ContractConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub migration_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosConfig {
    pub rpc_url: String,
    pub chain_id: String,
    pub gas_limit: u64,
    pub gas_price: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub rocksdb_path: PathBuf,
    pub cache_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub bind_address: String,
    pub port: u16,
    pub enable_graphql: bool,
    pub enable_websockets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    pub address: String,
    pub name: String,
    pub schema_path: PathBuf,
    pub start_height: Option<u64>,
    pub features: Vec<String>,
}

impl IndexerConfig {
    /// Load configuration from file and environment
    pub fn load() -> Result<Self, config::ConfigError> {
        let mut config_builder = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::Environment::with_prefix("INDEXER").separator("_"));

        // Override with local config if it exists
        if std::path::Path::new("config/local.toml").exists() {
            config_builder = config_builder.add_source(config::File::with_name("config/local"));
        }

        config_builder.build()?.try_deserialize()
    }
    
    /// Create a default configuration for development
    pub fn default_dev() -> Self {
        Self {
            database: DatabaseConfig {
                url: "postgresql://localhost/cosmos_indexer_dev".to_string(),
                max_connections: 10,
                migration_path: "migrations".into(),
            },
            cosmos: CosmosConfig {
                rpc_url: "https://rpc.cosmos.network:443".to_string(),
                chain_id: "cosmoshub-4".to_string(),
                gas_limit: 200000,
                gas_price: "0.025uatom".to_string(),
            },
            storage: StorageConfig {
                rocksdb_path: "data/rocksdb".into(),
                cache_size: 64 * 1024 * 1024, // 64MB
            },
            api: ApiConfig {
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
                enable_graphql: true,
                enable_websockets: true,
            },
            contracts: vec![],
        }
    }
}
```

### Create Error Handling

Create `src/error.rs`:

```rust
//! Error types for the cosmos indexer
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Cosmos client error: {0}")]
    Cosmos(#[from] indexer_cosmos::Error),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Contract error: {0}")]
    Contract(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, IndexerError>;
```

## Step 2: Generate Contract Code

### Download Contract Schema

For this tutorial, we'll use a CW721 NFT contract:

```bash
# Create schemas directory
mkdir -p schemas

# Download CW721 schema (or create your own)
curl -o schemas/cw721_schema.json https://raw.githubusercontent.com/CosmWasm/cw-nfts/main/packages/cw721/schema/cw721.json
```

### Generate Contract Code

```bash
# Generate code for CW721 contract
almanac cosmos generate-contract schemas/cw721_schema.json \
  --address cosmos1abc123... \
  --chain cosmoshub-4 \
  --features client,storage,api \
  --output-dir contracts/cw721 \
  --namespace nft_collection
```

This creates:
```
contracts/cw721/
├── client/
│   ├── mod.rs
│   ├── types.rs
│   ├── execute.rs
│   ├── query.rs
│   └── events.rs
├── storage/
│   ├── mod.rs
│   ├── postgres_schema.sql
│   ├── rocksdb.rs
│   └── traits.rs
└── api/
    ├── mod.rs
    ├── rest.rs
    ├── graphql.rs
    └── websocket.rs
```

## Step 3: Integrate Generated Code

### Update Contract Module

Create `src/contracts/mod.rs`:

```rust
//! Contract integration modules

pub mod cw721;

use crate::{config::ContractConfig, error::Result};
use indexer_core::CosmosClient;
use std::sync::Arc;

/// Contract registry for managing multiple contract integrations
pub struct ContractRegistry {
    clients: Vec<Box<dyn ContractClient>>,
    processors: Vec<Box<dyn EventProcessor>>,
}

/// Trait for contract clients
#[async_trait::async_trait]
pub trait ContractClient: Send + Sync {
    /// Get contract address
    fn address(&self) -> &str;
    
    /// Get contract name
    fn name(&self) -> &str;
    
    /// Initialize contract client
    async fn initialize(&mut self) -> Result<()>;
    
    /// Health check for contract
    async fn health_check(&self) -> Result<bool>;
}

/// Trait for event processors
#[async_trait::async_trait]
pub trait EventProcessor: Send + Sync {
    /// Process a cosmos event
    async fn process_event(&self, event: &cosmwasm_std::Event) -> Result<()>;
    
    /// Get supported event types
    fn supported_event_types(&self) -> Vec<String>;
}

impl ContractRegistry {
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
            processors: Vec::new(),
        }
    }
    
    /// Add a contract client
    pub fn add_client(&mut self, client: Box<dyn ContractClient>) {
        self.clients.push(client);
    }
    
    /// Add an event processor
    pub fn add_processor(&mut self, processor: Box<dyn EventProcessor>) {
        self.processors.push(processor);
    }
    
    /// Initialize all registered contracts
    pub async fn initialize_all(&mut self) -> Result<()> {
        for client in &mut self.clients {
            client.initialize().await?;
        }
        Ok(())
    }
    
    /// Process an event with all relevant processors
    pub async fn process_event(&self, event: &cosmwasm_std::Event) -> Result<()> {
        for processor in &self.processors {
            if processor.supported_event_types().contains(&event.ty) {
                processor.process_event(event).await?;
            }
        }
        Ok(())
    }
}

/// Factory for creating contract clients from configuration
pub async fn create_contract_client(
    config: &ContractConfig,
    cosmos_client: Arc<CosmosClient>,
) -> Result<Box<dyn ContractClient>> {
    match config.name.as_str() {
        "cw721" => {
            let client = cw721::Cw721ClientWrapper::new(config, cosmos_client).await?;
            Ok(Box::new(client))
        }
        _ => Err(crate::error::IndexerError::Contract(
            format!("Unknown contract type: {}", config.name)
        )),
    }
}
```

### Create CW721 Integration

Create `src/contracts/cw721.rs`:

```rust
//! CW721 NFT contract integration

use super::{ContractClient, EventProcessor};
use crate::{config::ContractConfig, error::Result};
use indexer_core::CosmosClient;
use std::sync::Arc;

// Import generated types and client
use crate::contracts::cw721::{
    client::{Cw721Client, QueryMsg, ExecuteMsg},
    storage::Cw721Storage,
    types::{TokenInfo, NftInfo, OwnerOfResponse},
};

/// Wrapper for the generated CW721 client that implements our traits
pub struct Cw721ClientWrapper {
    client: Cw721Client,
    config: ContractConfig,
    storage: Cw721Storage,
}

impl Cw721ClientWrapper {
    pub async fn new(
        config: &ContractConfig,
        cosmos_client: Arc<CosmosClient>,
    ) -> Result<Self> {
        let contract_address = cosmwasm_std::Addr::unchecked(&config.address);
        let client = Cw721Client::new((*cosmos_client).clone(), contract_address);
        
        // Initialize storage (this would connect to your databases)
        let storage = Cw721Storage::new().await?;
        
        Ok(Self {
            client,
            config: config.clone(),
            storage,
        })
    }
    
    /// Get all tokens owned by an address
    pub async fn get_user_tokens(&self, owner: &str) -> Result<Vec<String>> {
        // This would use pagination in a real implementation
        let tokens = self.client.tokens(
            Some(owner.to_string()),
            None, // start_after
            Some(100) // limit
        ).await?;
        
        Ok(tokens.tokens)
    }
    
    /// Get detailed NFT information
    pub async fn get_nft_details(&self, token_id: &str) -> Result<NftDetails> {
        let nft_info = self.client.nft_info(token_id.to_string()).await?;
        let owner_response = self.client.owner_of(token_id.to_string(), None).await?;
        
        Ok(NftDetails {
            token_id: token_id.to_string(),
            owner: owner_response.owner,
            name: nft_info.name,
            description: nft_info.description,
            image: nft_info.image,
            attributes: nft_info.attributes.unwrap_or_default(),
        })
    }
}

#[async_trait::async_trait]
impl ContractClient for Cw721ClientWrapper {
    fn address(&self) -> &str {
        &self.config.address
    }
    
    fn name(&self) -> &str {
        &self.config.name
    }
    
    async fn initialize(&mut self) -> Result<()> {
        // Perform any necessary initialization
        // For example, sync existing tokens from the contract
        self.sync_existing_tokens().await?;
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool> {
        // Simple health check - try to query contract info
        match self.client.contract_info().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl Cw721ClientWrapper {
    /// Sync existing tokens from the contract (for initial setup)
    async fn sync_existing_tokens(&self) -> Result<()> {
        let mut start_after: Option<String> = None;
        let limit = 100;
        
        loop {
            let response = self.client.all_tokens(start_after.clone(), Some(limit)).await?;
            
            if response.tokens.is_empty() {
                break;
            }
            
            // Index each token
            for token_id in &response.tokens {
                self.index_token(token_id).await?;
            }
            
            // Set up for next iteration
            start_after = response.tokens.last().cloned();
            
            if response.tokens.len() < limit as usize {
                break;
            }
        }
        
        Ok(())
    }
    
    /// Index a single token
    async fn index_token(&self, token_id: &str) -> Result<()> {
        let nft_details = self.get_nft_details(token_id).await?;
        
        // Store in database using generated storage layer
        self.storage.upsert_token(&nft_details).await?;
        
        Ok(())
    }
}

/// Event processor for CW721 events
pub struct Cw721EventProcessor {
    client: Cw721ClientWrapper,
}

impl Cw721EventProcessor {
    pub fn new(client: Cw721ClientWrapper) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl EventProcessor for Cw721EventProcessor {
    async fn process_event(&self, event: &cosmwasm_std::Event) -> Result<()> {
        match event.ty.as_str() {
            "wasm-mint" => {
                self.process_mint_event(event).await?;
            }
            "wasm-transfer_nft" => {
                self.process_transfer_event(event).await?;
            }
            "wasm-burn" => {
                self.process_burn_event(event).await?;
            }
            _ => {
                // Ignore unknown events
            }
        }
        Ok(())
    }
    
    fn supported_event_types(&self) -> Vec<String> {
        vec![
            "wasm-mint".to_string(),
            "wasm-transfer_nft".to_string(),
            "wasm-burn".to_string(),
        ]
    }
}

impl Cw721EventProcessor {
    async fn process_mint_event(&self, event: &cosmwasm_std::Event) -> Result<()> {
        // Extract token_id from event attributes
        let token_id = self.extract_attribute(event, "token_id")?;
        
        // Index the newly minted token
        self.client.index_token(&token_id).await?;
        
        tracing::info!("Processed mint event for token: {}", token_id);
        Ok(())
    }
    
    async fn process_transfer_event(&self, event: &cosmwasm_std::Event) -> Result<()> {
        let token_id = self.extract_attribute(event, "token_id")?;
        
        // Re-index the token to update owner information
        self.client.index_token(&token_id).await?;
        
        tracing::info!("Processed transfer event for token: {}", token_id);
        Ok(())
    }
    
    async fn process_burn_event(&self, event: &cosmwasm_std::Event) -> Result<()> {
        let token_id = self.extract_attribute(event, "token_id")?;
        
        // Remove token from storage
        self.client.storage.delete_token(&token_id).await?;
        
        tracing::info!("Processed burn event for token: {}", token_id);
        Ok(())
    }
    
    fn extract_attribute(&self, event: &cosmwasm_std::Event, key: &str) -> Result<String> {
        event.attributes
            .iter()
            .find(|attr| attr.key == key)
            .map(|attr| attr.value.clone())
            .ok_or_else(|| crate::error::IndexerError::Contract(
                format!("Missing attribute '{}' in event", key)
            ))
    }
}

/// Detailed NFT information combining multiple queries
#[derive(Debug, Clone)]
pub struct NftDetails {
    pub token_id: String,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub image: Option<String>,
    pub attributes: Vec<cosmwasm_std::Attribute>,
}
```

## Step 4: Main Application

### Create Main Library

Update `src/lib.rs`:

```rust
//! Cosmos contract indexer library

pub mod config;
pub mod contracts;
pub mod error;

use crate::{
    config::IndexerConfig,
    contracts::{ContractRegistry, create_contract_client},
    error::Result,
};
use indexer_core::{CosmosClient, EventStream};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error, warn};

/// Main indexer application
pub struct CosmosIndexer {
    config: IndexerConfig,
    cosmos_client: Arc<CosmosClient>,
    db_pool: PgPool,
    contract_registry: ContractRegistry,
}

impl CosmosIndexer {
    /// Create a new indexer instance
    pub async fn new(config: IndexerConfig) -> Result<Self> {
        // Initialize database connection
        let db_pool = PgPool::connect(&config.database.url).await?;
        
        // Run database migrations
        sqlx::migrate!(&config.database.migration_path)
            .run(&db_pool)
            .await?;
        
        // Initialize cosmos client
        let cosmos_client = Arc::new(
            CosmosClient::new(&config.cosmos.rpc_url, &config.cosmos.chain_id).await?
        );
        
        // Initialize contract registry
        let mut contract_registry = ContractRegistry::new();
        
        // Register all configured contracts
        for contract_config in &config.contracts {
            info!("Registering contract: {} at {}", contract_config.name, contract_config.address);
            
            let client = create_contract_client(contract_config, cosmos_client.clone()).await?;
            contract_registry.add_client(client);
        }
        
        Ok(Self {
            config,
            cosmos_client,
            db_pool,
            contract_registry,
        })
    }
    
    /// Start the indexer
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Cosmos indexer...");
        
        // Initialize all contracts
        self.contract_registry.initialize_all().await?;
        
        // Start event processing
        self.start_event_processing().await?;
        
        // Start API server if enabled
        if self.config.api.enable_graphql || self.config.api.enable_websockets {
            self.start_api_server().await?;
        }
        
        // Keep the application running
        self.run_main_loop().await?;
        
        Ok(())
    }
    
    /// Start processing blockchain events
    async fn start_event_processing(&self) -> Result<()> {
        let (event_tx, mut event_rx) = mpsc::channel(1000);
        
        // Start event stream
        let cosmos_client = self.cosmos_client.clone();
        let contract_addresses: Vec<String> = self.config.contracts
            .iter()
            .map(|c| c.address.clone())
            .collect();
        
        tokio::spawn(async move {
            let mut event_stream = EventStream::new(cosmos_client, contract_addresses);
            
            loop {
                match event_stream.next_event().await {
                    Ok(event) => {
                        if let Err(e) = event_tx.send(event).await {
                            error!("Failed to send event: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Event stream error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });
        
        // Process events
        let contract_registry = &self.contract_registry;
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if let Err(e) = contract_registry.process_event(&event).await {
                    error!("Failed to process event: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    /// Start the API server
    async fn start_api_server(&self) -> Result<()> {
        use axum::{Router, routing::get};
        
        let app = Router::new()
            .route("/health", get(health_check))
            .route("/metrics", get(metrics));
        
        let addr = format!("{}:{}", self.config.api.bind_address, self.config.api.port);
        info!("Starting API server on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                error!("API server error: {}", e);
            }
        });
        
        Ok(())
    }
    
    /// Main application loop
    async fn run_main_loop(&self) -> Result<()> {
        let mut health_check_interval = tokio::time::interval(
            tokio::time::Duration::from_secs(30)
        );
        
        loop {
            tokio::select! {
                _ = health_check_interval.tick() => {
                    self.perform_health_checks().await;
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal");
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Perform health checks on all components
    async fn perform_health_checks(&self) {
        // Check database connectivity
        if let Err(e) = sqlx::query("SELECT 1").execute(&self.db_pool).await {
            error!("Database health check failed: {}", e);
        }
        
        // Check cosmos client connectivity
        if let Err(e) = self.cosmos_client.get_latest_block().await {
            error!("Cosmos client health check failed: {}", e);
        }
        
        // Check contract health
        for client in &self.contract_registry.clients {
            match client.health_check().await {
                Ok(true) => {},
                Ok(false) => warn!("Contract {} health check failed", client.name()),
                Err(e) => error!("Contract {} health check error: {}", client.name(), e),
            }
        }
    }
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Metrics endpoint  
async fn metrics() -> &'static str {
    "# Metrics would go here"
}
```

## Step 5: Application Entry Point

### Create Main Binary

Create `src/main.rs`:

```rust
//! Cosmos indexer application entry point

use my_cosmos_indexer::{config::IndexerConfig, CosmosIndexer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Load configuration
    let config = IndexerConfig::load()
        .unwrap_or_else(|_| {
            tracing::warn!("Failed to load configuration, using defaults");
            IndexerConfig::default_dev()
        });
    
    // Create and start indexer
    let mut indexer = CosmosIndexer::new(config).await?;
    indexer.start().await?;
    
    Ok(())
}
```

## Step 6: Configuration Files

### Create Default Configuration

Create `config/default.toml`:

```toml
[database]
url = "postgresql://localhost/cosmos_indexer"
max_connections = 10
migration_path = "migrations"

[cosmos]
rpc_url = "https://rpc.cosmos.network:443"
chain_id = "cosmoshub-4"
gas_limit = 200000
gas_price = "0.025uatom"

[storage]
rocksdb_path = "data/rocksdb"
cache_size = 67108864  # 64MB

[api]
bind_address = "127.0.0.1"
port = 8080
enable_graphql = true
enable_websockets = true

# Example contract configurations
[[contracts]]
address = "cosmos1abc123..."
name = "cw721"
schema_path = "schemas/cw721_schema.json"
features = ["client", "storage", "api"]

[[contracts]]
address = "cosmos1def456..."
name = "valence_base_account"
schema_path = "schemas/valence_base_account_schema.json"
features = ["client", "storage"]
```

## Step 7: Database Migrations

### Create Initial Migration

Create `migrations/20240315000001_initial.sql`:

```sql
-- Initial schema for cosmos indexer

-- Indexer metadata
CREATE TABLE indexer_metadata (
    key VARCHAR(255) PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Block tracking
CREATE TABLE indexed_blocks (
    chain_id VARCHAR(255) NOT NULL,
    height BIGINT NOT NULL,
    hash VARCHAR(255) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    indexed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (chain_id, height)
);

-- Contract registry
CREATE TABLE contracts (
    address VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    chain_id VARCHAR(255) NOT NULL,
    first_seen_height BIGINT,
    last_activity_height BIGINT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Event processing log
CREATE TABLE processed_events (
    id SERIAL PRIMARY KEY,
    tx_hash VARCHAR(255) NOT NULL,
    event_index INTEGER NOT NULL,
    event_type VARCHAR(255) NOT NULL,
    contract_address VARCHAR(255),
    block_height BIGINT NOT NULL,
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE (tx_hash, event_index)
);

-- Indexes for performance
CREATE INDEX idx_indexed_blocks_chain_height ON indexed_blocks(chain_id, height);
CREATE INDEX idx_contracts_chain ON contracts(chain_id);
CREATE INDEX idx_processed_events_contract ON processed_events(contract_address);
CREATE INDEX idx_processed_events_height ON processed_events(block_height);
```

## Step 8: Testing

### Create Integration Tests

Create `tests/integration_test.rs`:

```rust
//! Integration tests for the cosmos indexer

use my_cosmos_indexer::{config::IndexerConfig, CosmosIndexer};
use sqlx::PgPool;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_indexer_startup() {
    // Create test configuration
    let temp_dir = TempDir::new().unwrap();
    let mut config = IndexerConfig::default_dev();
    config.database.url = "postgresql://localhost/cosmos_indexer_test".to_string();
    config.storage.rocksdb_path = temp_dir.path().to_path_buf();
    
    // Create indexer (this will fail without a real database, but tests the setup)
    let result = CosmosIndexer::new(config).await;
    
    // In a real test environment with a test database, this would succeed
    assert!(result.is_err()); // Expected to fail without test DB setup
}

#[tokio::test]
async fn test_contract_registration() {
    // Test contract registration and initialization
    // This would require mock implementations or a test environment
}

#[tokio::test]
async fn test_event_processing() {
    // Test event processing pipeline
    // This would require mock events and verification
}
```

## Step 9: Running the Indexer

### Development Setup

1. **Set up PostgreSQL**:
```bash
# Create database
createdb cosmos_indexer

# Run migrations
sqlx migrate run --database-url postgresql://localhost/cosmos_indexer
```

2. **Create local configuration**:

Create `config/local.toml` with your specific settings:
```toml
[database]
url = "postgresql://localhost/cosmos_indexer"

[cosmos]
rpc_url = "https://your-preferred-rpc-endpoint"
chain_id = "cosmoshub-4"

[[contracts]]
address = "your-contract-address"
name = "cw721"
schema_path = "schemas/cw721_schema.json"
```

3. **Run the indexer**:
```bash
# Set log level
export RUST_LOG=info

# Start the indexer
cargo run
```

### Production Deployment

1. **Docker setup**:

Create `Dockerfile`:
```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/my-cosmos-indexer /usr/local/bin/
COPY --from=builder /app/config /app/config
COPY --from=builder /app/migrations /app/migrations

WORKDIR /app
CMD ["my-cosmos-indexer"]
```

2. **Docker Compose setup**:

Create `docker-compose.yml`:
```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: cosmos_indexer
      POSTGRES_USER: indexer
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  indexer:
    build: .
    depends_on:
      - postgres
    environment:
      INDEXER_DATABASE_URL: postgresql://indexer:password@postgres/cosmos_indexer
      RUST_LOG: info
    volumes:
      - indexer_data:/app/data
    ports:
      - "8080:8080"

volumes:
  postgres_data:
  indexer_data:
```

## Step 10: Monitoring and Observability

### Add Metrics

Add to your `Cargo.toml`:
```toml
prometheus = "0.13"
metrics = "0.21"
metrics-prometheus = "0.6"
```

### Create Metrics Module

Create `src/metrics.rs`:
```rust
//! Metrics collection for the indexer

use metrics::{counter, gauge, histogram};
use std::time::Instant;

pub struct IndexerMetrics;

impl IndexerMetrics {
    pub fn event_processed(contract: &str, event_type: &str) {
        counter!("events_processed_total", "contract" => contract.to_string(), "type" => event_type.to_string()).increment(1);
    }
    
    pub fn block_processed(height: u64) {
        gauge!("latest_block_height").set(height as f64);
    }
    
    pub fn processing_duration(duration: std::time::Duration) {
        histogram!("event_processing_duration_seconds").record(duration.as_secs_f64());
    }
    
    pub fn contract_health(contract: &str, healthy: bool) {
        gauge!("contract_health", "contract" => contract.to_string()).set(if healthy { 1.0 } else { 0.0 });
    }
}

pub fn record_processing_time<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    IndexerMetrics::processing_duration(start.elapsed());
    result
}
```

## Conclusion

This tutorial demonstrated how to:

1. **Set up a new Cosmos indexer project** with proper structure and dependencies
2. **Generate contract code** using the Almanac codegen system
3. **Integrate generated code** into a cohesive application
4. **Handle configuration and error management** appropriately
5. **Process blockchain events** in real-time
6. **Set up storage and APIs** for the indexed data
7. **Deploy and monitor** the application in production

The resulting indexer provides:
- ✅ Type-safe contract interactions
- ✅ Automated storage layer
- ✅ REST and GraphQL APIs
- ✅ Real-time event processing
- ✅ Health monitoring and metrics
- ✅ Production-ready deployment

### Next Steps

1. **Add more contracts** by generating additional contract modules
2. **Customize the API layer** to meet your specific requirements
3. **Implement advanced features** like cross-contract queries
4. **Set up monitoring dashboards** using Prometheus and Grafana
5. **Add comprehensive testing** for your specific use cases

The modular design allows you to easily extend the indexer with additional contracts and features as your needs evolve. 