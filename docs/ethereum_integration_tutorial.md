# Ethereum Contract Integration Tutorial

This tutorial provides a complete guide for integrating generated Ethereum contract code into your applications using the Almanac indexer.

## Prerequisites

- Rust 1.75 or later
- PostgreSQL database
- Ethereum node access (Infura, Alchemy, or local node)
- Basic understanding of Ethereum contracts and ABIs

## Project Setup

### 1. Create New Rust Project

```bash
cargo new my-eth-indexer
cd my-eth-indexer
```

### 2. Add Dependencies

Update `Cargo.toml`:

```toml
[package]
name = "my-eth-indexer"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core dependencies
almanac-ethereum = "0.1.0"
almanac-core = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
thiserror = "1.0"

# Ethereum dependencies
alloy-primitives = "0.5"
alloy-rpc-types = "0.1"
ethers = "2.0"

# Database dependencies
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "postgres", "chrono", "uuid"] }
sea-orm = { version = "0.12", features = ["sqlx-postgres", "runtime-tokio-native-tls", "macros"] }

# Async runtime
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# API server
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# GraphQL (optional)
async-graphql = { version = "7.0", features = ["chrono"] }
async-graphql-axum = "7.0"

# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
env_logger = "0.11"

# Configuration
config = "0.14"
clap = { version = "4.0", features = ["derive"] }

# Development dependencies
[dev-dependencies]
testcontainers = "0.15"
test-log = "0.2"
```

### 3. Project Structure

Create the following directory structure:

```
my-eth-indexer/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   ├── indexer/
│   │   ├── mod.rs
│   │   └── ethereum.rs
│   ├── storage/
│   │   ├── mod.rs
│   │   └── models.rs
│   ├── api/
│   │   ├── mod.rs
│   │   ├── rest.rs
│   │   └── graphql.rs
│   └── cli.rs
├── config/
│   ├── development.toml
│   ├── production.toml
│   └── test.toml
├── contracts/
│   └── erc20_abi.json
├── migrations/
└── tests/
    ├── integration_tests.rs
    └── contract_tests.rs
```

## Configuration System

### 1. Configuration Structure

Create `src/config.rs`:

```rust
//! Configuration management for the Ethereum indexer

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub ethereum: EthereumConfig,
    pub indexer: IndexerConfig,
    pub api: ApiConfig,
    pub logging: LoggingConfig,
    pub contracts: Vec<ContractConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
    pub idle_timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EthereumConfig {
    pub provider_url: String,
    pub chain_id: u64,
    pub gas_limit: u64,
    pub gas_price: Option<u64>,
    pub confirmations: u64,
    pub timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IndexerConfig {
    pub start_block: Option<u64>,
    pub end_block: Option<u64>,
    pub block_batch_size: u64,
    pub poll_interval: u64,
    pub max_retries: u32,
    pub retry_delay: u64,
    pub enable_reorg_protection: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
    pub enable_graphql: bool,
    pub rate_limit: Option<RateLimitConfig>,
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // "json" or "pretty"
    pub file: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContractConfig {
    pub name: String,
    pub address: String,
    pub abi_path: String,
    pub start_block: Option<u64>,
    pub features: Vec<String>, // ["client", "storage", "api"]
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        
        let config_path = format!("config/{}.toml", environment);
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file {}: {}", config_path, e))?;
        
        let mut config: Config = toml::from_str(&config_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
        
        // Override with environment variables
        config.override_from_env()?;
        
        Ok(config)
    }
    
    fn override_from_env(&mut self) -> anyhow::Result<()> {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            self.database.url = url;
        }
        
        if let Ok(url) = std::env::var("ETHEREUM_PROVIDER_URL") {
            self.ethereum.provider_url = url;
        }
        
        if let Ok(level) = std::env::var("LOG_LEVEL") {
            self.logging.level = level;
        }
        
        Ok(())
    }
    
    pub fn api_address(&self) -> SocketAddr {
        format!("{}:{}", self.api.host, self.api.port)
            .parse()
            .expect("Invalid API address")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                url: "postgresql://localhost/ethereum_indexer".to_string(),
                max_connections: 10,
                min_connections: 1,
                connection_timeout: 30,
                idle_timeout: 300,
            },
            ethereum: EthereumConfig {
                provider_url: "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string(),
                chain_id: 1,
                gas_limit: 6_000_000,
                gas_price: None,
                confirmations: 12,
                timeout: 30,
            },
            indexer: IndexerConfig {
                start_block: None,
                end_block: None,
                block_batch_size: 100,
                poll_interval: 12,
                max_retries: 3,
                retry_delay: 1,
                enable_reorg_protection: true,
            },
            api: ApiConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                enable_cors: true,
                enable_graphql: true,
                rate_limit: None,
                tls: None,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "pretty".to_string(),
                file: None,
            },
            contracts: vec![],
        }
    }
}
```

### 2. Configuration Files

Create `config/development.toml`:

```toml
[database]
url = "postgresql://localhost/ethereum_indexer_dev"
max_connections = 5
min_connections = 1
connection_timeout = 30
idle_timeout = 300

[ethereum]
provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
chain_id = 1
gas_limit = 6000000
confirmations = 12
timeout = 30

[indexer]
start_block = 18000000
block_batch_size = 100
poll_interval = 12
max_retries = 3
retry_delay = 1
enable_reorg_protection = true

[api]
host = "127.0.0.1"
port = 3000
enable_cors = true
enable_graphql = true

[logging]
level = "debug"
format = "pretty"

[[contracts]]
name = "usdc"
address = "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0"
abi_path = "contracts/usdc_abi.json"
start_block = 18000000
features = ["client", "storage", "api"]

[[contracts]]
name = "weth"
address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
abi_path = "contracts/weth_abi.json"
start_block = 18000000
features = ["client", "storage"]
```

## Error Handling

Create `src/error.rs`:

```rust
//! Error types for the Ethereum indexer

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Ethereum client error: {0}")]
    EthereumClient(String),
    
    #[error("Contract error: {0}")]
    Contract(String),
    
    #[error("ABI parsing error: {0}")]
    AbiParsing(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Block not found: {block_number}")]
    BlockNotFound { block_number: u64 },
    
    #[error("Transaction not found: {tx_hash}")]
    TransactionNotFound { tx_hash: String },
    
    #[error("Invalid address: {address}")]
    InvalidAddress { address: String },
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Timeout error: operation timed out after {seconds}s")]
    Timeout { seconds: u64 },
    
    #[error("Generic error: {0}")]
    Generic(String),
}

impl IndexerError {
    pub fn ethereum_client<E: std::fmt::Display>(error: E) -> Self {
        Self::EthereumClient(error.to_string())
    }
    
    pub fn contract<E: std::fmt::Display>(error: E) -> Self {
        Self::Contract(error.to_string())
    }
    
    pub fn abi_parsing<E: std::fmt::Display>(error: E) -> Self {
        Self::AbiParsing(error.to_string())
    }
    
    pub fn configuration<E: std::fmt::Display>(error: E) -> Self {
        Self::Configuration(error.to_string())
    }
    
    pub fn network<E: std::fmt::Display>(error: E) -> Self {
        Self::Network(error.to_string())
    }
    
    pub fn generic<E: std::fmt::Display>(error: E) -> Self {
        Self::Generic(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, IndexerError>;

// Convenient conversion traits
impl From<anyhow::Error> for IndexerError {
    fn from(error: anyhow::Error) -> Self {
        Self::Generic(error.to_string())
    }
}
```

## Contract Integration Module

Create `src/indexer/mod.rs`:

```rust
//! Indexer module for Ethereum contracts

pub mod ethereum;

use crate::{config::Config, error::Result};
use std::sync::Arc;

pub trait ContractIndexer: Send + Sync {
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn index_block(&self, block_number: u64) -> Result<()>;
    async fn get_latest_indexed_block(&self) -> Result<Option<u64>>;
}

pub struct IndexerManager {
    indexers: Vec<Arc<dyn ContractIndexer>>,
}

impl IndexerManager {
    pub fn new() -> Self {
        Self {
            indexers: Vec::new(),
        }
    }
    
    pub fn add_indexer(&mut self, indexer: Arc<dyn ContractIndexer>) {
        self.indexers.push(indexer);
    }
    
    pub async fn start_all(&self) -> Result<()> {
        for indexer in &self.indexers {
            indexer.start().await?;
        }
        Ok(())
    }
    
    pub async fn stop_all(&self) -> Result<()> {
        for indexer in &self.indexers {
            indexer.stop().await?;
        }
        Ok(())
    }
}
```

Create `src/indexer/ethereum.rs`:

```rust
//! Ethereum-specific indexer implementation

use crate::{
    config::{Config, ContractConfig},
    error::{IndexerError, Result},
    indexer::ContractIndexer,
};
use alloy_primitives::{Address, U256};
use sqlx::PgPool;
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::sleep};
use tracing::{info, warn, error, debug};

pub struct EthereumIndexer {
    config: Config,
    pool: PgPool,
    ethereum_client: Arc<EthereumClient>,
    contracts: HashMap<String, ContractHandler>,
    latest_indexed_block: Arc<RwLock<Option<u64>>>,
    running: Arc<RwLock<bool>>,
}

impl EthereumIndexer {
    pub async fn new(config: Config, pool: PgPool) -> Result<Self> {
        // Initialize Ethereum client
        let ethereum_client = Arc::new(
            EthereumClient::new(&config.ethereum.provider_url)
                .await
                .map_err(IndexerError::ethereum_client)?
        );
        
        // Initialize contract handlers
        let mut contracts = HashMap::new();
        for contract_config in &config.contracts {
            let handler = ContractHandler::new(
                contract_config.clone(),
                ethereum_client.clone(),
                pool.clone(),
            ).await?;
            contracts.insert(contract_config.name.clone(), handler);
        }
        
        // Get latest indexed block from database
        let latest_block = Self::get_latest_block_from_db(&pool).await?;
        
        Ok(Self {
            config,
            pool,
            ethereum_client,
            contracts,
            latest_indexed_block: Arc::new(RwLock::new(latest_block)),
            running: Arc::new(RwLock::new(false)),
        })
    }
    
    async fn get_latest_block_from_db(pool: &PgPool) -> Result<Option<u64>> {
        let row = sqlx::query!("SELECT MAX(block_number) as max_block FROM indexed_blocks")
            .fetch_optional(pool)
            .await?;
        
        Ok(row.and_then(|r| r.max_block.map(|b| b as u64)))
    }
    
    async fn update_indexed_block(&self, block_number: u64) -> Result<()> {
        sqlx::query!(
            "INSERT INTO indexed_blocks (block_number, indexed_at) VALUES ($1, NOW()) 
             ON CONFLICT (block_number) DO UPDATE SET indexed_at = NOW()",
            block_number as i64
        )
        .execute(&self.pool)
        .await?;
        
        let mut latest = self.latest_indexed_block.write().await;
        *latest = Some(block_number);
        
        Ok(())
    }
    
    async fn indexing_loop(&self) -> Result<()> {
        let mut current_block = {
            let latest = self.latest_indexed_block.read().await;
            latest.unwrap_or(
                self.config.indexer.start_block
                    .unwrap_or_else(|| {
                        warn!("No start block configured, starting from latest");
                        0
                    })
            )
        };
        
        info!("Starting indexing from block {}", current_block);
        
        while *self.running.read().await {
            // Get latest block from network
            let latest_network_block = match self.ethereum_client.get_latest_block_number().await {
                Ok(block) => block,
                Err(e) => {
                    error!("Failed to get latest block: {}", e);
                    sleep(Duration::from_secs(self.config.indexer.retry_delay)).await;
                    continue;
                }
            };
            
            // Apply confirmation delay
            let target_block = latest_network_block.saturating_sub(self.config.ethereum.confirmations);
            
            // Index missing blocks in batches
            while current_block <= target_block && *self.running.read().await {
                let batch_end = std::cmp::min(
                    current_block + self.config.indexer.block_batch_size - 1,
                    target_block
                );
                
                info!("Indexing blocks {} to {}", current_block, batch_end);
                
                for block_number in current_block..=batch_end {
                    if let Err(e) = self.index_block(block_number).await {
                        error!("Failed to index block {}: {}", block_number, e);
                        
                        // Retry logic
                        let mut retries = 0;
                        while retries < self.config.indexer.max_retries {
                            sleep(Duration::from_secs(self.config.indexer.retry_delay)).await;
                            
                            match self.index_block(block_number).await {
                                Ok(()) => break,
                                Err(e) => {
                                    retries += 1;
                                    warn!("Retry {}/{} failed for block {}: {}", 
                                          retries, self.config.indexer.max_retries, block_number, e);
                                }
                            }
                        }
                        
                        if retries >= self.config.indexer.max_retries {
                            error!("Failed to index block {} after {} retries", 
                                   block_number, self.config.indexer.max_retries);
                            // Continue with next block instead of failing entirely
                        }
                    }
                }
                
                current_block = batch_end + 1;
            }
            
            // Wait before checking for new blocks
            if current_block > target_block {
                debug!("Caught up to latest block, waiting {} seconds", 
                       self.config.indexer.poll_interval);
                sleep(Duration::from_secs(self.config.indexer.poll_interval)).await;
            }
        }
        
        info!("Indexing loop stopped");
        Ok(())
    }
}

#[async_trait::async_trait]
impl ContractIndexer for EthereumIndexer {
    async fn start(&self) -> Result<()> {
        info!("Starting Ethereum indexer");
        
        {
            let mut running = self.running.write().await;
            *running = true;
        }
        
        // Start indexing loop in background
        let indexer = self.clone(); // Assume Clone is implemented
        tokio::spawn(async move {
            if let Err(e) = indexer.indexing_loop().await {
                error!("Indexing loop failed: {}", e);
            }
        });
        
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        info!("Stopping Ethereum indexer");
        
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        
        Ok(())
    }
    
    async fn index_block(&self, block_number: u64) -> Result<()> {
        debug!("Indexing block {}", block_number);
        
        // Get block data
        let block = self.ethereum_client.get_block(block_number).await
            .map_err(|e| IndexerError::ethereum_client(format!("Failed to get block {}: {}", block_number, e)))?;
        
        // Index each contract
        for (name, handler) in &self.contracts {
            if let Err(e) = handler.index_block(&block).await {
                error!("Failed to index block {} for contract {}: {}", block_number, name, e);
                return Err(e);
            }
        }
        
        // Update indexed block record
        self.update_indexed_block(block_number).await?;
        
        debug!("Successfully indexed block {}", block_number);
        Ok(())
    }
    
    async fn get_latest_indexed_block(&self) -> Result<Option<u64>> {
        let latest = self.latest_indexed_block.read().await;
        Ok(*latest)
    }
}

struct ContractHandler {
    config: ContractConfig,
    ethereum_client: Arc<EthereumClient>,
    storage: Arc<dyn ContractStorage>,
}

impl ContractHandler {
    async fn new(
        config: ContractConfig,
        ethereum_client: Arc<EthereumClient>,
        pool: PgPool,
    ) -> Result<Self> {
        // Load ABI
        let abi_content = std::fs::read_to_string(&config.abi_path)
            .map_err(|e| IndexerError::configuration(format!("Failed to read ABI file {}: {}", config.abi_path, e)))?;
        
        // Initialize storage based on contract type
        let storage = match config.name.as_str() {
            "usdc" => Arc::new(UsdcStorage::new(pool)) as Arc<dyn ContractStorage>,
            "weth" => Arc::new(WethStorage::new(pool)) as Arc<dyn ContractStorage>,
            _ => return Err(IndexerError::configuration(format!("Unknown contract type: {}", config.name))),
        };
        
        Ok(Self {
            config,
            ethereum_client,
            storage,
        })
    }
    
    async fn index_block(&self, block: &Block) -> Result<()> {
        // Get logs for this contract from the block
        let logs = self.ethereum_client.get_logs_for_block(
            block.number,
            &Address::from_str(&self.config.address)
                .map_err(|e| IndexerError::invalid_address(self.config.address.clone()))?
        ).await?;
        
        // Process each log
        for log in logs {
            self.storage.process_log(&log, block).await?;
        }
        
        Ok(())
    }
}

// Storage traits and implementations
#[async_trait::async_trait]
trait ContractStorage: Send + Sync {
    async fn process_log(&self, log: &Log, block: &Block) -> Result<()>;
}

struct UsdcStorage {
    pool: PgPool,
}

impl UsdcStorage {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ContractStorage for UsdcStorage {
    async fn process_log(&self, log: &Log, block: &Block) -> Result<()> {
        // Parse USDC events and store them
        match log.topics[0] {
            topic if topic == TRANSFER_EVENT_SIGNATURE => {
                let event = parse_transfer_event(log)?;
                self.store_transfer(&event, block).await?;
            }
            topic if topic == APPROVAL_EVENT_SIGNATURE => {
                let event = parse_approval_event(log)?;
                self.store_approval(&event, block).await?;
            }
            _ => {
                debug!("Unknown event signature: {:?}", log.topics[0]);
            }
        }
        
        Ok(())
    }
}

impl UsdcStorage {
    async fn store_transfer(&self, event: &TransferEvent, block: &Block) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO usdc_transfers (
                contract_address, from_address, to_address, value,
                block_number, transaction_hash, log_index, timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (transaction_hash, log_index) DO NOTHING
            "#,
            event.contract_address,
            event.from,
            event.to,
            event.value.to_string(),
            block.number as i64,
            event.transaction_hash,
            event.log_index as i32,
            block.timestamp
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn store_approval(&self, event: &ApprovalEvent, block: &Block) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO usdc_approvals (
                contract_address, owner_address, spender_address, value,
                block_number, transaction_hash, log_index, timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (transaction_hash, log_index) DO NOTHING
            "#,
            event.contract_address,
            event.owner,
            event.spender,
            event.value.to_string(),
            block.number as i64,
            event.transaction_hash,
            event.log_index as i32,
            block.timestamp
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}

// Event parsing functions and types would be defined here
// These would typically be generated by the codegen system

struct TransferEvent {
    contract_address: String,
    from: String,
    to: String,
    value: U256,
    transaction_hash: String,
    log_index: u64,
}

struct ApprovalEvent {
    contract_address: String,
    owner: String,
    spender: String,
    value: U256,
    transaction_hash: String,
    log_index: u64,
}

// Event signature constants
const TRANSFER_EVENT_SIGNATURE: [u8; 32] = [0; 32]; // Would be actual signature
const APPROVAL_EVENT_SIGNATURE: [u8; 32] = [0; 32]; // Would be actual signature

fn parse_transfer_event(log: &Log) -> Result<TransferEvent> {
    // Implementation would decode the log data
    todo!("Implement event parsing")
}

fn parse_approval_event(log: &Log) -> Result<ApprovalEvent> {
    // Implementation would decode the log data
    todo!("Implement event parsing")
}

// Placeholder types - these would come from the generated code
struct EthereumClient;
struct Block {
    number: u64,
    timestamp: chrono::DateTime<chrono::Utc>,
}
struct Log {
    topics: Vec<[u8; 32]>,
    data: Vec<u8>,
}

impl EthereumClient {
    async fn new(url: &str) -> Result<Self> {
        // Implementation
        Ok(Self)
    }
    
    async fn get_latest_block_number(&self) -> Result<u64> {
        // Implementation
        Ok(0)
    }
    
    async fn get_block(&self, number: u64) -> Result<Block> {
        // Implementation
        Ok(Block {
            number,
            timestamp: chrono::Utc::now(),
        })
    }
    
    async fn get_logs_for_block(&self, block: u64, address: &Address) -> Result<Vec<Log>> {
        // Implementation
        Ok(vec![])
    }
}

struct WethStorage {
    pool: PgPool,
}

impl WethStorage {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ContractStorage for WethStorage {
    async fn process_log(&self, log: &Log, block: &Block) -> Result<()> {
        // WETH-specific event processing
        Ok(())
    }
}
```

## API Layer

Create `src/api/mod.rs`:

```rust
//! API module for serving indexed data

pub mod rest;
pub mod graphql;

use crate::{config::Config, error::Result, indexer::IndexerManager};
use axum::{
    middleware,
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub async fn create_app(
    config: Config,
    indexer_manager: Arc<IndexerManager>,
) -> Result<Router> {
    let app = Router::new()
        .route("/health", get(health_check))
        .merge(rest::create_routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(indexer_manager);
    
    if config.api.enable_graphql {
        let graphql_app = graphql::create_schema().await?;
        app = app.merge(graphql_app);
    }
    
    Ok(app)
}

async fn health_check() -> &'static str {
    "OK"
}
```

## Testing

Create `tests/integration_tests.rs`:

```rust
//! Integration tests for the Ethereum indexer

use my_eth_indexer::{config::Config, indexer::ethereum::EthereumIndexer};
use sqlx::PgPool;
use testcontainers::{clients::Cli, images::postgres::Postgres};

#[tokio::test]
async fn test_ethereum_indexer_basic_flow() {
    // Setup test database
    let docker = Cli::default();
    let postgres_container = docker.run(Postgres::default());
    let connection_string = format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        postgres_container.get_host_port_ipv4(5432)
    );
    
    let pool = PgPool::connect(&connection_string).await.unwrap();
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    
    // Create test config
    let mut config = Config::default();
    config.database.url = connection_string;
    config.ethereum.provider_url = "http://localhost:8545".to_string(); // Local testnet
    
    // Initialize indexer
    let indexer = EthereumIndexer::new(config, pool).await.unwrap();
    
    // Test indexing a specific block
    indexer.index_block(18000000).await.unwrap();
    
    // Verify data was stored
    let latest_block = indexer.get_latest_indexed_block().await.unwrap();
    assert_eq!(latest_block, Some(18000000));
}

#[tokio::test]
async fn test_contract_event_processing() {
    // Test specific contract event processing
    todo!("Implement contract event tests");
}

#[tokio::test]  
async fn test_api_endpoints() {
    // Test API endpoints
    todo!("Implement API tests");
}
```

## Main Application

Create `src/main.rs`:

```rust
//! Main application entry point

use anyhow::Result;
use clap::Parser;
use my_eth_indexer::{
    api,
    config::Config,
    indexer::{ethereum::EthereumIndexer, IndexerManager},
};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "eth-indexer")]
#[command(about = "Ethereum contract indexer")]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,
    
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser)]
enum Commands {
    /// Run the indexer
    Run,
    /// Generate contract code from ABI
    Generate {
        #[arg(short, long)]
        abi: String,
        #[arg(short, long)]
        address: String,
        #[arg(short, long)]
        chain: u64,
    },
    /// Database migrations
    Migrate,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let subscriber = tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        );
    tracing::subscriber::set_global_default(subscriber)?;
    
    // Load configuration
    let config = Config::load()?;
    
    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => run_indexer(config).await,
        Commands::Generate { abi, address, chain } => {
            generate_contract_code(abi, address, chain).await
        }
        Commands::Migrate => run_migrations(config).await,
    }
}

async fn run_indexer(config: Config) -> Result<()> {
    info!("Starting Ethereum indexer");
    
    // Connect to database
    let pool = PgPool::connect(&config.database.url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    // Initialize indexer
    let ethereum_indexer = Arc::new(EthereumIndexer::new(config.clone(), pool.clone()).await?);
    
    let mut indexer_manager = IndexerManager::new();
    indexer_manager.add_indexer(ethereum_indexer);
    
    // Start indexing
    indexer_manager.start_all().await?;
    
    // Create and start API server
    let app = api::create_app(config.clone(), Arc::new(indexer_manager)).await?;
    let listener = tokio::net::TcpListener::bind(config.api_address()).await?;
    
    info!("API server starting on {}", config.api_address());
    
    // Graceful shutdown handling
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    
    Ok(())
}

async fn generate_contract_code(abi: String, address: String, chain: u64) -> Result<()> {
    info!("Generating contract code for {}", address);
    
    // This would integrate with the codegen system
    // almanac ethereum generate-contract {abi} --address {address} --chain {chain}
    
    println!("Contract code generation completed!");
    Ok(())
}

async fn run_migrations(config: Config) -> Result<()> {
    info!("Running database migrations");
    
    let pool = PgPool::connect(&config.database.url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    info!("Migrations completed successfully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}
```

## Database Migrations

Create migration files in `migrations/`:

`migrations/20240101000001_initial_setup.sql`:
```sql
-- Initial database setup
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Track indexed blocks
CREATE TABLE indexed_blocks (
    block_number BIGINT PRIMARY KEY,
    indexed_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_indexed_blocks_indexed_at ON indexed_blocks(indexed_at);

-- USDC contract events
CREATE TABLE usdc_transfers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    contract_address VARCHAR(42) NOT NULL,
    from_address VARCHAR(42) NOT NULL,
    to_address VARCHAR(42) NOT NULL,
    value NUMERIC NOT NULL,
    block_number BIGINT NOT NULL,
    transaction_hash VARCHAR(66) NOT NULL,
    log_index INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(transaction_hash, log_index)
);

CREATE INDEX idx_usdc_transfers_from ON usdc_transfers(from_address);
CREATE INDEX idx_usdc_transfers_to ON usdc_transfers(to_address);
CREATE INDEX idx_usdc_transfers_block ON usdc_transfers(block_number);
CREATE INDEX idx_usdc_transfers_timestamp ON usdc_transfers(timestamp);

CREATE TABLE usdc_approvals (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    contract_address VARCHAR(42) NOT NULL,
    owner_address VARCHAR(42) NOT NULL,
    spender_address VARCHAR(42) NOT NULL,
    value NUMERIC NOT NULL,
    block_number BIGINT NOT NULL,
    transaction_hash VARCHAR(66) NOT NULL,
    log_index INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(transaction_hash, log_index)
);

CREATE INDEX idx_usdc_approvals_owner ON usdc_approvals(owner_address);
CREATE INDEX idx_usdc_approvals_spender ON usdc_approvals(spender_address);
CREATE INDEX idx_usdc_approvals_block ON usdc_approvals(block_number);
```

## Development Workflow

### 1. Code Generation

```bash
# Generate contract integration code
almanac ethereum generate-contract contracts/usdc_abi.json \
  --address 0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0 \
  --chain 1 \
  --output-dir src/generated/usdc \
  --features client,storage,api
```

### 2. Development Setup

```bash
# Start PostgreSQL (via Docker)
docker run --name postgres-dev \
  -e POSTGRES_DB=ethereum_indexer_dev \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -p 5432:5432 \
  -d postgres:15

# Set environment variables
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/ethereum_indexer_dev"
export ETHEREUM_PROVIDER_URL="https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
export ENVIRONMENT="development"

# Run migrations
cargo run -- migrate

# Start the indexer
cargo run -- run
```

### 3. Testing

```bash
# Run unit tests
cargo test

# Run integration tests with test database
ENVIRONMENT=test cargo test --test integration_tests

# Run specific test
cargo test test_ethereum_indexer_basic_flow
```

### 4. API Testing

```bash
# Health check
curl http://localhost:3000/health

# Get USDC balance
curl http://localhost:3000/api/v1/ethereum/1/contracts/0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0/balance/0x123...

# Get transfer history
curl "http://localhost:3000/api/v1/ethereum/1/contracts/0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0/transfers?limit=10&offset=0"
```

## Production Deployment

### Docker Configuration

`Dockerfile`:
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

COPY --from=builder /app/target/release/my-eth-indexer /usr/local/bin/
COPY --from=builder /app/config /app/config
COPY --from=builder /app/migrations /app/migrations

WORKDIR /app

EXPOSE 3000

CMD ["my-eth-indexer", "run"]
```

`docker-compose.yml`:
```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: ethereum_indexer
      POSTGRES_USER: indexer
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  indexer:
    build: .
    environment:
      DATABASE_URL: postgresql://indexer:${POSTGRES_PASSWORD}@postgres:5432/ethereum_indexer
      ETHEREUM_PROVIDER_URL: ${ETHEREUM_PROVIDER_URL}
      ENVIRONMENT: production
      RUST_LOG: info
    ports:
      - "3000:3000"
    depends_on:
      - postgres
    restart: unless-stopped
    volumes:
      - ./config:/app/config:ro

volumes:
  postgres_data:
```

### Monitoring

Add monitoring with Prometheus metrics:

```rust
// In Cargo.toml
[dependencies]
prometheus = "0.13"
axum-prometheus = "0.4"

// In main.rs
use axum_prometheus::PrometheusMetricLayer;

let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

let app = Router::new()
    .route("/metrics", get(|| async move { metric_handle.render() }))
    .layer(prometheus_layer)
    // ... other routes
```

This tutorial provides a comprehensive foundation for integrating Ethereum contracts using generated code, with proper error handling, configuration management, testing, and production deployment practices. 