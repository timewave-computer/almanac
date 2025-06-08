# ERC20 Token Integration Example

This example demonstrates how to integrate an ERC20 token contract (USDC) using the Ethereum codegen system.

## Contract Overview

We'll integrate USDC (USD Coin), a popular ERC20 stablecoin:
- **Contract Address**: `0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0`
- **Chain**: Ethereum Mainnet (chain ID: 1)
- **Decimals**: 6
- **Symbol**: USDC

## Step 1: Contract ABI

First, we need the contract's ABI. For USDC, the ABI includes standard ERC20 functions plus additional features:

`usdc_abi.json`:
```json
[
  {
    "type": "constructor",
    "inputs": [
      {"name": "_name", "type": "string", "internalType": "string"},
      {"name": "_symbol", "type": "string", "internalType": "string"},
      {"name": "_currency", "type": "string", "internalType": "string"},
      {"name": "_decimals", "type": "uint8", "internalType": "uint8"},
      {"name": "_masterMinter", "type": "address", "internalType": "address"},
      {"name": "_pauser", "type": "address", "internalType": "address"},
      {"name": "_blacklister", "type": "address", "internalType": "address"},
      {"name": "_owner", "type": "address", "internalType": "address"}
    ]
  },
  {
    "type": "function",
    "name": "name",
    "inputs": [],
    "outputs": [{"name": "", "type": "string", "internalType": "string"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "symbol",
    "inputs": [],
    "outputs": [{"name": "", "type": "string", "internalType": "string"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "decimals",
    "inputs": [],
    "outputs": [{"name": "", "type": "uint8", "internalType": "uint8"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "totalSupply",
    "inputs": [],
    "outputs": [{"name": "", "type": "uint256", "internalType": "uint256"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "balanceOf",
    "inputs": [{"name": "account", "type": "address", "internalType": "address"}],
    "outputs": [{"name": "", "type": "uint256", "internalType": "uint256"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "transfer",
    "inputs": [
      {"name": "to", "type": "address", "internalType": "address"},
      {"name": "amount", "type": "uint256", "internalType": "uint256"}
    ],
    "outputs": [{"name": "", "type": "bool", "internalType": "bool"}],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "allowance",
    "inputs": [
      {"name": "owner", "type": "address", "internalType": "address"},
      {"name": "spender", "type": "address", "internalType": "address"}
    ],
    "outputs": [{"name": "", "type": "uint256", "internalType": "uint256"}],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "approve",
    "inputs": [
      {"name": "spender", "type": "address", "internalType": "address"},
      {"name": "amount", "type": "uint256", "internalType": "uint256"}
    ],
    "outputs": [{"name": "", "type": "bool", "internalType": "bool"}],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "transferFrom",
    "inputs": [
      {"name": "from", "type": "address", "internalType": "address"},
      {"name": "to", "type": "address", "internalType": "address"},
      {"name": "amount", "type": "uint256", "internalType": "uint256"}
    ],
    "outputs": [{"name": "", "type": "bool", "internalType": "bool"}],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "mint",
    "inputs": [
      {"name": "_to", "type": "address", "internalType": "address"},
      {"name": "_amount", "type": "uint256", "internalType": "uint256"}
    ],
    "outputs": [{"name": "", "type": "bool", "internalType": "bool"}],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "burn",
    "inputs": [{"name": "_amount", "type": "uint256", "internalType": "uint256"}],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "pause",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "unpause",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "paused",
    "inputs": [],
    "outputs": [{"name": "", "type": "bool", "internalType": "bool"}],
    "stateMutability": "view"
  },
  {
    "type": "event",
    "name": "Transfer",
    "inputs": [
      {"name": "from", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "to", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "value", "type": "uint256", "indexed": false, "internalType": "uint256"}
    ]
  },
  {
    "type": "event",
    "name": "Approval",
    "inputs": [
      {"name": "owner", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "spender", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "value", "type": "uint256", "indexed": false, "internalType": "uint256"}
    ]
  },
  {
    "type": "event",
    "name": "Mint",
    "inputs": [
      {"name": "minter", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "to", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "amount", "type": "uint256", "indexed": false, "internalType": "uint256"}
    ]
  },
  {
    "type": "event",
    "name": "Burn",
    "inputs": [
      {"name": "burner", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "amount", "type": "uint256", "indexed": false, "internalType": "uint256"}
    ]
  },
  {
    "type": "event",
    "name": "Pause",
    "inputs": []
  },
  {
    "type": "event",
    "name": "Unpause",
    "inputs": []
  }
]
```

## Step 2: Code Generation

Generate the contract integration code:

```bash
almanac ethereum generate-contract usdc_abi.json \
  --address 0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0 \
  --chain 1 \
  --output-dir ./generated/usdc \
  --namespace usdc \
  --features client,storage,api \
  --verbose
```

This generates:
- Client code for contract interactions
- Storage schemas for event indexing
- API endpoints for external access
- Database migrations

## Step 3: Generated Client Usage

### Basic Client Setup

```rust
use generated::usdc::client::UsdcClient;
use almanac_ethereum::EthereumClient;
use alloy_primitives::{Address, U256};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Ethereum client
    let provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID";
    let ethereum_client = EthereumClient::new(provider_url).await?;

    // USDC contract address
    let contract_address = Address::from_str("0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0")?;
    
    // Create USDC client
    let usdc_client = UsdcClient::new(ethereum_client, contract_address);

    // Query basic contract information
    let name = usdc_client.name().await?;
    let symbol = usdc_client.symbol().await?;
    let decimals = usdc_client.decimals().await?;
    let total_supply = usdc_client.total_supply().await?;

    println!("Contract: {} ({})", name, symbol);
    println!("Decimals: {}", decimals);
    println!("Total Supply: {} {}", total_supply, symbol);

    Ok(())
}
```

### Advanced Usage Patterns

#### 1. Balance Monitoring

```rust
use generated::usdc::client::{UsdcClient, TransferEvent};

pub struct BalanceMonitor {
    usdc_client: UsdcClient,
    watched_addresses: Vec<Address>,
}

impl BalanceMonitor {
    pub fn new(usdc_client: UsdcClient, addresses: Vec<Address>) -> Self {
        Self {
            usdc_client,
            watched_addresses: addresses,
        }
    }

    pub async fn get_balances(&self) -> Result<Vec<(Address, U256)>, Box<dyn std::error::Error>> {
        let mut balances = Vec::new();
        
        for address in &self.watched_addresses {
            let balance = self.usdc_client.balance_of(*address).await?;
            balances.push((*address, balance));
        }
        
        Ok(balances)
    }

    pub async fn monitor_transfers(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Subscribe to Transfer events for watched addresses
        let filter = Filter::new()
            .address(self.usdc_client.contract_address)
            .topic0(TransferEvent::signature())
            .topic1_any(&self.watched_addresses); // from addresses
            
        let mut stream = self.usdc_client.ethereum_client.subscribe_logs(filter).await?;

        while let Some(log) = stream.next().await {
            if let Ok(event) = TransferEvent::from_log(&log) {
                println!(
                    "Transfer detected: {} USDC from {} to {} (tx: {})",
                    self.format_usdc_amount(event.value),
                    event.from,
                    event.to,
                    event.transaction_hash
                );

                // Update cached balances
                self.update_balance_cache(event.from).await?;
                self.update_balance_cache(event.to).await?;
            }
        }

        Ok(())
    }

    fn format_usdc_amount(&self, amount: U256) -> String {
        // USDC has 6 decimals
        let whole = amount / U256::from(1_000_000);
        let fractional = amount % U256::from(1_000_000);
        format!("{}.{:06}", whole, fractional)
    }

    async fn update_balance_cache(&self, address: Address) -> Result<(), Box<dyn std::error::Error>> {
        let balance = self.usdc_client.balance_of(address).await?;
        // Store in cache/database
        println!("Updated balance for {}: {} USDC", address, self.format_usdc_amount(balance));
        Ok(())
    }
}
```

#### 2. Transaction Execution

```rust
use generated::usdc::client::UsdcClient;
use alloy_primitives::{Address, U256};

pub struct UsdcTransactionManager {
    usdc_client: UsdcClient,
    wallet_address: Address,
}

impl UsdcTransactionManager {
    pub fn new(usdc_client: UsdcClient, wallet_address: Address) -> Self {
        Self { usdc_client, wallet_address }
    }

    pub async fn transfer_usdc(
        &self,
        to: Address,
        amount_usdc: f64,
    ) -> Result<TxHash, Box<dyn std::error::Error>> {
        // Convert USDC amount to wei (6 decimals)
        let amount_wei = U256::from((amount_usdc * 1_000_000.0) as u64);

        // Check balance
        let balance = self.usdc_client.balance_of(self.wallet_address).await?;
        if balance < amount_wei {
            return Err("Insufficient balance".into());
        }

        // Execute transfer
        let tx_hash = self.usdc_client.transfer(to, amount_wei).await?;
        
        println!(
            "Transfer initiated: {} USDC to {} (tx: {})",
            amount_usdc, to, tx_hash
        );

        // Wait for confirmation
        let receipt = self.usdc_client.ethereum_client
            .wait_for_transaction(tx_hash)
            .await?;

        if receipt.status == Some(1.into()) {
            println!("Transfer confirmed in block {}", receipt.block_number.unwrap());
        } else {
            return Err("Transaction failed".into());
        }

        Ok(tx_hash)
    }

    pub async fn approve_spending(
        &self,
        spender: Address,
        amount_usdc: f64,
    ) -> Result<TxHash, Box<dyn std::error::Error>> {
        let amount_wei = U256::from((amount_usdc * 1_000_000.0) as u64);

        let tx_hash = self.usdc_client.approve(spender, amount_wei).await?;
        
        println!(
            "Approval set: {} can spend {} USDC (tx: {})",
            spender, amount_usdc, tx_hash
        );

        Ok(tx_hash)
    }

    pub async fn batch_transfers(
        &self,
        transfers: Vec<(Address, f64)>, // (to, amount_usdc)
    ) -> Result<Vec<TxHash>, Box<dyn std::error::Error>> {
        let mut tx_hashes = Vec::new();

        for (to, amount_usdc) in transfers {
            let tx_hash = self.transfer_usdc(to, amount_usdc).await?;
            tx_hashes.push(tx_hash);
            
            // Small delay between transactions to avoid nonce conflicts
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Ok(tx_hashes)
    }
}
```

## Step 4: Storage Integration

### Database Schema

The generated storage includes tables for tracking USDC events:

```sql
-- Generated migration: 20240101120000_usdc_setup.sql
CREATE TABLE usdc_transfers (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(42) NOT NULL,
    from_address VARCHAR(42) NOT NULL,
    to_address VARCHAR(42) NOT NULL,
    value NUMERIC NOT NULL,
    block_number BIGINT NOT NULL,
    transaction_hash VARCHAR(66) NOT NULL,
    log_index INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    UNIQUE(transaction_hash, log_index)
);

CREATE TABLE usdc_approvals (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(42) NOT NULL,
    owner_address VARCHAR(42) NOT NULL,
    spender_address VARCHAR(42) NOT NULL,
    value NUMERIC NOT NULL,
    block_number BIGINT NOT NULL,
    transaction_hash VARCHAR(66) NOT NULL,
    log_index INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    UNIQUE(transaction_hash, log_index)
);

CREATE TABLE usdc_mint_burn (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(42) NOT NULL,
    event_type VARCHAR(10) NOT NULL, -- 'mint' or 'burn'
    address VARCHAR(42) NOT NULL,
    amount NUMERIC NOT NULL,
    block_number BIGINT NOT NULL,
    transaction_hash VARCHAR(66) NOT NULL,
    log_index INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    UNIQUE(transaction_hash, log_index)
);

-- Indexes for efficient querying
CREATE INDEX idx_usdc_transfers_from ON usdc_transfers(from_address);
CREATE INDEX idx_usdc_transfers_to ON usdc_transfers(to_address);
CREATE INDEX idx_usdc_transfers_block ON usdc_transfers(block_number);
CREATE INDEX idx_usdc_transfers_timestamp ON usdc_transfers(timestamp);

CREATE INDEX idx_usdc_approvals_owner ON usdc_approvals(owner_address);
CREATE INDEX idx_usdc_approvals_spender ON usdc_approvals(spender_address);
CREATE INDEX idx_usdc_approvals_block ON usdc_approvals(block_number);

CREATE INDEX idx_usdc_mint_burn_address ON usdc_mint_burn(address);
CREATE INDEX idx_usdc_mint_burn_type ON usdc_mint_burn(event_type);
CREATE INDEX idx_usdc_mint_burn_block ON usdc_mint_burn(block_number);
```

### Storage Implementation

```rust
use generated::usdc::storage::{UsdcStorage, TransferRecord, ApprovalRecord};
use sqlx::PgPool;

pub struct UsdcIndexer {
    storage: UsdcStorage,
    usdc_client: UsdcClient,
}

impl UsdcIndexer {
    pub fn new(pool: PgPool, usdc_client: UsdcClient) -> Self {
        Self {
            storage: UsdcStorage::new(pool),
            usdc_client,
        }
    }

    pub async fn index_block(&self, block_number: u64) -> Result<(), Box<dyn std::error::Error>> {
        println!("Indexing USDC events for block {}", block_number);

        // Get all USDC events from the block
        let logs = self.usdc_client.ethereum_client.get_logs(
            Filter::new()
                .address(self.usdc_client.contract_address)
                .from_block(block_number)
                .to_block(block_number)
        ).await?;

        let mut transfers = Vec::new();
        let mut approvals = Vec::new();
        let mut mints = Vec::new();
        let mut burns = Vec::new();

        // Parse events
        for log in logs {
            match log.topics[0] {
                topic if topic == TransferEvent::signature() => {
                    if let Ok(event) = TransferEvent::from_log(&log) {
                        transfers.push(TransferRecord {
                            contract_address: event.contract_address.to_string(),
                            from_address: event.from.to_string(),
                            to_address: event.to.to_string(),
                            value: event.value,
                            block_number: event.block_number,
                            transaction_hash: event.transaction_hash.to_string(),
                            log_index: event.log_index as i32,
                            timestamp: self.get_block_timestamp(event.block_number).await?,
                        });
                    }
                }
                topic if topic == ApprovalEvent::signature() => {
                    if let Ok(event) = ApprovalEvent::from_log(&log) {
                        approvals.push(ApprovalRecord {
                            contract_address: event.contract_address.to_string(),
                            owner_address: event.owner.to_string(),
                            spender_address: event.spender.to_string(),
                            value: event.value,
                            block_number: event.block_number,
                            transaction_hash: event.transaction_hash.to_string(),
                            log_index: event.log_index as i32,
                            timestamp: self.get_block_timestamp(event.block_number).await?,
                        });
                    }
                }
                topic if topic == MintEvent::signature() => {
                    if let Ok(event) = MintEvent::from_log(&log) {
                        mints.push(event);
                    }
                }
                topic if topic == BurnEvent::signature() => {
                    if let Ok(event) = BurnEvent::from_log(&log) {
                        burns.push(event);
                    }
                }
                _ => {} // Unknown event
            }
        }

        // Store in database
        if !transfers.is_empty() {
            self.storage.insert_transfers(&transfers).await?;
            println!("Stored {} transfers", transfers.len());
        }

        if !approvals.is_empty() {
            self.storage.insert_approvals(&approvals).await?;
            println!("Stored {} approvals", approvals.len());
        }

        // Handle mints and burns...

        Ok(())
    }

    async fn get_block_timestamp(&self, block_number: u64) -> Result<chrono::DateTime<chrono::Utc>, Box<dyn std::error::Error>> {
        let block = self.usdc_client.ethereum_client.get_block(block_number).await?;
        Ok(chrono::DateTime::from_timestamp(block.timestamp as i64, 0).unwrap())
    }

    pub async fn get_transfer_history(
        &self,
        address: Address,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TransferRecord>, Box<dyn std::error::Error>> {
        self.storage.get_transfers_for_address(address.to_string(), limit, offset).await
    }

    pub async fn get_total_volume(&self) -> Result<U256, Box<dyn std::error::Error>> {
        self.storage.get_total_transfer_volume().await
    }
}
```

## Step 5: API Integration

### REST Endpoints

The generated API includes REST endpoints:

```rust
use generated::usdc::api::UsdcApi;
use axum::{Router, extract::{Path, Query}, Json};

pub fn create_usdc_router(indexer: UsdcIndexer) -> Router {
    Router::new()
        .route("/usdc/balance/:address", get(get_balance))
        .route("/usdc/transfers/:address", get(get_transfers))
        .route("/usdc/total-supply", get(get_total_supply))
        .route("/usdc/stats", get(get_stats))
        .with_state(indexer)
}

async fn get_balance(
    Path(address): Path<String>,
    State(indexer): State<UsdcIndexer>,
) -> Result<Json<BalanceResponse>, StatusCode> {
    let address = Address::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let balance = indexer.usdc_client.balance_of(address).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(BalanceResponse {
        address: address.to_string(),
        balance: balance.to_string(),
        balance_formatted: format_usdc_amount(balance),
    }))
}

async fn get_transfers(
    Path(address): Path<String>,
    Query(params): Query<TransferQuery>,
    State(indexer): State<UsdcIndexer>,
) -> Result<Json<TransfersResponse>, StatusCode> {
    let address = Address::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let transfers = indexer.get_transfer_history(
        address,
        params.limit.unwrap_or(100),
        params.offset.unwrap_or(0),
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(TransfersResponse { transfers }))
}

#[derive(Deserialize)]
struct TransferQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct BalanceResponse {
    address: String,
    balance: String,
    balance_formatted: String,
}

#[derive(Serialize)]
struct TransfersResponse {
    transfers: Vec<TransferRecord>,
}
```

### GraphQL Integration

```rust
use async_graphql::{Object, Context, Result, Schema};
use generated::usdc::client::UsdcClient;

pub struct UsdcQuery;

#[Object]
impl UsdcQuery {
    async fn usdc_balance(&self, ctx: &Context<'_>, address: String) -> Result<String> {
        let indexer = ctx.data::<UsdcIndexer>()?;
        let addr = Address::from_str(&address)?;
        let balance = indexer.usdc_client.balance_of(addr).await?;
        Ok(balance.to_string())
    }

    async fn usdc_total_supply(&self, ctx: &Context<'_>) -> Result<String> {
        let indexer = ctx.data::<UsdcIndexer>()?;
        let supply = indexer.usdc_client.total_supply().await?;
        Ok(supply.to_string())
    }

    async fn usdc_transfers(
        &self,
        ctx: &Context<'_>,
        address: String,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<TransferRecord>> {
        let indexer = ctx.data::<UsdcIndexer>()?;
        let addr = Address::from_str(&address)?;
        let transfers = indexer.get_transfer_history(
            addr,
            limit.unwrap_or(100) as i64,
            offset.unwrap_or(0) as i64,
        ).await?;
        Ok(transfers)
    }
}

pub struct UsdcSubscription;

#[Object]
impl UsdcSubscription {
    async fn usdc_transfers(&self, ctx: &Context<'_>) -> impl Stream<Item = TransferEvent> {
        let indexer = ctx.data::<UsdcIndexer>().unwrap();
        // Implementation for real-time transfer subscriptions
        indexer.subscribe_to_transfers().await
    }
}

type UsdcSchema = Schema<UsdcQuery, EmptyMutation, UsdcSubscription>;

pub fn create_usdc_schema(indexer: UsdcIndexer) -> UsdcSchema {
    Schema::build(UsdcQuery, EmptyMutation, UsdcSubscription)
        .data(indexer)
        .finish()
}
```

## Step 6: Complete Application

### Main Application Setup

```rust
use tokio;
use std::sync::Arc;
use axum::{Router, middleware};
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/almanac_usdc".to_string());
    let pool = PgPool::connect(&database_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Ethereum client setup
    let provider_url = std::env::var("ETH_PROVIDER_URL")
        .unwrap_or_else(|_| "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string());
    let ethereum_client = EthereumClient::new(&provider_url).await?;

    // USDC client setup
    let usdc_address = Address::from_str("0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0")?;
    let usdc_client = UsdcClient::new(ethereum_client, usdc_address);

    // Indexer setup
    let indexer = Arc::new(UsdcIndexer::new(pool.clone(), usdc_client));

    // Start background indexing
    let indexer_task = indexer.clone();
    tokio::spawn(async move {
        if let Err(e) = start_indexing(indexer_task).await {
            eprintln!("Indexing error: {}", e);
        }
    });

    // API routes
    let app = Router::new()
        .merge(create_usdc_router((*indexer).clone()))
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn(logging_middleware));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("USDC indexer API running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await?;

    Ok(())
}

async fn start_indexing(indexer: Arc<UsdcIndexer>) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_block = indexer.get_last_indexed_block().await?;
    
    loop {
        // Get latest block
        let latest_block = indexer.usdc_client.ethereum_client.get_latest_block().await?;
        
        // Index any missing blocks
        while current_block < latest_block {
            current_block += 1;
            println!("Indexing block {}", current_block);
            
            if let Err(e) = indexer.index_block(current_block).await {
                eprintln!("Error indexing block {}: {}", current_block, e);
                // Continue with next block
            }
        }
        
        // Wait before checking for new blocks
        tokio::time::sleep(std::time::Duration::from_secs(12)).await;
    }
}

async fn logging_middleware(
    request: Request<Body>,
    next: Next<Body>,
) -> Result<Response, StatusCode> {
    let start = std::time::Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    let response = next.run(request).await;
    
    let duration = start.elapsed();
    println!("{} {} - {}ms", method, uri, duration.as_millis());
    
    Ok(response)
}
```

### Configuration

Create a `usdc_config.toml` file:

```toml
[database]
url = "postgresql://localhost/almanac_usdc"
max_connections = 10

[ethereum]
provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
contract_address = "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0"
chain_id = 1

[indexer]
start_block = 18000000  # Block to start indexing from
batch_size = 100        # Blocks to process in batch
poll_interval = 12      # Seconds between polling for new blocks

[api]
host = "0.0.0.0"
port = 3000
enable_cors = true
enable_graphql = true

[monitoring]
enable_metrics = true
metrics_port = 9090
log_level = "info"
```

## Step 7: Testing

### Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::{clients::Cli, images::postgres::Postgres, Container};

    #[tokio::test]
    async fn test_usdc_indexing() {
        let docker = Cli::default();
        let postgres_container = docker.run(Postgres::default());
        let connection_string = format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            postgres_container.get_host_port_ipv4(5432)
        );

        let pool = PgPool::connect(&connection_string).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // Test indexing a known block with USDC transfers
        let indexer = setup_test_indexer(pool).await;
        indexer.index_block(18000000).await.unwrap();

        // Verify transfers were indexed
        let transfers = indexer.storage.get_transfers_for_block(18000000).await.unwrap();
        assert!(!transfers.is_empty());
    }

    #[tokio::test]
    async fn test_balance_queries() {
        let indexer = setup_test_indexer_with_data().await;
        
        // Test balance query for known address
        let balance = indexer.usdc_client.balance_of(
            Address::from_str("0x123...").unwrap()
        ).await.unwrap();
        
        assert!(balance > U256::ZERO);
    }

    async fn setup_test_indexer(pool: PgPool) -> UsdcIndexer {
        let ethereum_client = EthereumClient::new("http://localhost:8545").await.unwrap();
        let usdc_client = UsdcClient::new(
            ethereum_client,
            Address::from_str("0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0").unwrap()
        );
        UsdcIndexer::new(pool, usdc_client)
    }
}
```

### Performance Testing

```rust
#[tokio::test]
async fn benchmark_indexing_performance() {
    let indexer = setup_test_indexer().await;
    
    let start = std::time::Instant::now();
    
    // Index 1000 blocks
    for block in 18000000..18001000 {
        indexer.index_block(block).await.unwrap();
    }
    
    let duration = start.elapsed();
    println!("Indexed 1000 blocks in {:?}", duration);
    
    // Should process at least 10 blocks per second
    assert!(duration.as_secs() < 100);
}
```

## Deployment

### Docker Configuration

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/usdc-indexer /usr/local/bin/

EXPOSE 3000 9090

CMD ["usdc-indexer"]
```

```yaml
# docker-compose.yml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: almanac_usdc
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

  usdc-indexer:
    build: .
    ports:
      - "3000:3000"
      - "9090:9090"
    environment:
      DATABASE_URL: postgresql://postgres:postgres@postgres:5432/almanac_usdc
      ETH_PROVIDER_URL: https://mainnet.infura.io/v3/YOUR_PROJECT_ID
      RUST_LOG: info
    depends_on:
      - postgres
    restart: unless-stopped

volumes:
  postgres_data:
```

This example demonstrates a complete, production-ready USDC token integration using the Ethereum codegen system, including real-time indexing, API endpoints, and comprehensive monitoring. 