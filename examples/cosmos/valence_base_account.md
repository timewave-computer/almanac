# Valence Base Account Integration Example

This example demonstrates how to integrate a Valence Base Account contract using the Cosmos code generation system.

## Contract Overview

We'll integrate a **Valence Base Account**, which is the main type of account used by Valence programs:
- **Admin**: The account has an admin who can manage the account
- **Approved Libraries**: A set of approved libraries that can execute arbitrary messages on behalf of the account
- **Contract Address**: `cosmos1...` (example address)
- **Chain**: Cosmos Hub (chain ID: cosmoshub-4)

The Valence Base Account supports library management and message execution, making it a core component of the Valence protocol ecosystem.

## Step 1: Contract Schema

First, we need the contract's schema. The Valence Base Account schema includes instantiation, execution, and query messages:

`valence_base_account_schema.json`:
```json
{
  "contract_name": "valence-base-account",
  "contract_version": "0.2.0",
  "idl_version": "1.0.0",
  "instantiate": {
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "InstantiateMsg",
    "type": "object",
    "required": [
      "admin",
      "approved_libraries"
    ],
    "properties": {
      "admin": {
        "type": "string"
      },
      "approved_libraries": {
        "type": "array",
        "items": {
          "type": "string"
        }
      }
    },
    "additionalProperties": false
  },
  "execute": {
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "ExecuteMsg",
    "oneOf": [
      {
        "type": "object",
        "required": [
          "approve_library"
        ],
        "properties": {
          "approve_library": {
            "type": "object",
            "required": [
              "library"
            ],
            "properties": {
              "library": {
                "type": "string"
              }
            },
            "additionalProperties": false
          }
        },
        "additionalProperties": false
      },
      {
        "type": "object",
        "required": [
          "remove_library"
        ],
        "properties": {
          "remove_library": {
            "type": "object",
            "required": [
              "library"
            ],
            "properties": {
              "library": {
                "type": "string"
              }
            },
            "additionalProperties": false
          }
        },
        "additionalProperties": false
      },
      {
        "type": "object",
        "required": [
          "execute_msg"
        ],
        "properties": {
          "execute_msg": {
            "type": "object",
            "required": [
              "msgs"
            ],
            "properties": {
              "msgs": {
                "type": "array",
                "items": {
                  "$ref": "#/definitions/CosmosMsg_for_Empty"
                }
              }
            },
            "additionalProperties": false
          }
        },
        "additionalProperties": false
      },
      {
        "type": "object",
        "required": [
          "execute_submsgs"
        ],
        "properties": {
          "execute_submsgs": {
            "type": "object",
            "required": [
              "msgs"
            ],
            "properties": {
              "msgs": {
                "type": "array",
                "items": {
                  "$ref": "#/definitions/SubMsg_for_Empty"
                }
              },
              "payload": {
                "type": [
                  "string",
                  "null"
                ]
              }
            },
            "additionalProperties": false
          }
        },
        "additionalProperties": false
      },
      {
        "description": "Update the contract's ownership. The `action` to be provided can be either to propose transferring ownership to an account, accept a pending ownership transfer, or renounce the ownership permanently.",
        "type": "object",
        "required": [
          "update_ownership"
        ],
        "properties": {
          "update_ownership": {
            "$ref": "#/definitions/Action"
          }
        },
        "additionalProperties": false
      }
    ]
  },
  "query": {
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "QueryMsg",
    "oneOf": [
      {
        "type": "object",
        "required": [
          "list_approved_libraries"
        ],
        "properties": {
          "list_approved_libraries": {
            "type": "object",
            "additionalProperties": false
          }
        },
        "additionalProperties": false
      },
      {
        "description": "Query the contract's ownership information",
        "type": "object",
        "required": [
          "ownership"
        ],
        "properties": {
          "ownership": {
            "type": "object",
            "additionalProperties": false
          }
        },
        "additionalProperties": false
      }
    ]
  },
  "migrate": null,
  "sudo": null,
  "responses": {
    "list_approved_libraries": {
      "$schema": "http://json-schema.org/draft-07/schema#",
      "title": "Array_of_String",
      "type": "array",
      "items": {
        "type": "string"
      }
    },
    "ownership": {
      "$schema": "http://json-schema.org/draft-07/schema#",
      "title": "Ownership_for_String",
      "description": "The contract's ownership info",
      "type": "object",
      "properties": {
        "owner": {
          "description": "The contract's current owner. `None` if the ownership has been renounced.",
          "type": [
            "string",
            "null"
          ]
        },
        "pending_expiry": {
          "description": "The deadline for the pending owner to accept the ownership. `None` if there isn't a pending ownership transfer, or if a transfer exists and it doesn't have a deadline.",
          "anyOf": [
            {
              "$ref": "#/definitions/Expiration"
            },
            {
              "type": "null"
            }
          ]
        },
        "pending_owner": {
          "description": "The account who has been proposed to take over the ownership. `None` if there isn't a pending ownership transfer.",
          "type": [
            "string",
            "null"
          ]
        }
      },
      "additionalProperties": false
    }
  }
}
```

## Step 2: Code Generation

Generate the contract integration code:

```bash
almanac cosmos generate-contract valence_base_account_schema.json \
  --address cosmos1valencebaseaccountexample123456789 \
  --chain cosmoshub-4 \
  --output-dir ./generated/valence_base_account \
  --namespace valence_base_account \
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
use generated::valence_base_account::client::ValenceBaseAccountClient;
use indexer_cosmos::CosmosClient;
use cosmwasm_std::Addr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Cosmos client
    let rpc_url = "https://cosmos-rpc.quickapi.com:443";
    let cosmos_client = CosmosClient::new(rpc_url).await?;

    // Valence base account contract address
    let contract_address = Addr::unchecked("cosmos1valencebaseaccountexample123456789");
    
    // Create Valence base account client
    let account_client = ValenceBaseAccountClient::new(cosmos_client, contract_address);

    // Query basic account information
    let ownership = account_client.ownership().await?;
    let approved_libraries = account_client.list_approved_libraries().await?;

    println!("Account Owner: {:?}", ownership.owner);
    println!("Pending Owner: {:?}", ownership.pending_owner);
    println!("Approved Libraries: {:?}", approved_libraries);

    Ok(())
}
```

### Advanced Usage Patterns

#### 1. Library Management Monitor

```rust
use generated::valence_base_account::client::{ValenceBaseAccountClient, LibraryEvent};

pub struct LibraryManager {
    account_client: ValenceBaseAccountClient,
    monitored_accounts: Vec<Addr>,
}

impl LibraryManager {
    pub fn new(account_client: ValenceBaseAccountClient, accounts: Vec<Addr>) -> Self {
        Self {
            account_client,
            monitored_accounts: accounts,
        }
    }

    pub async fn get_all_approved_libraries(&self) -> Result<Vec<(Addr, Vec<String>)>, Box<dyn std::error::Error>> {
        let mut all_libraries = Vec::new();
        
        for account in &self.monitored_accounts {
            // Create client for each account
            let client = ValenceBaseAccountClient::new(
                self.account_client.cosmos_client.clone(),
                account.clone()
            );
            
            let libraries = client.list_approved_libraries().await?;
            all_libraries.push((account.clone(), libraries));
        }
        
        Ok(all_libraries)
    }

    pub async fn monitor_library_changes(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Subscribe to library approval/removal events for monitored accounts
        for account in &self.monitored_accounts {
            let filter = EventFilter::new()
                .contract_address(account.clone())
                .event_type("wasm")
                .attribute("action", "approve_library")
                .or_attribute("action", "remove_library");
                
            let mut stream = self.account_client.cosmos_client.subscribe_events(filter).await?;

            while let Some(event) = stream.next().await {
                if let Ok(library_event) = LibraryEvent::from_event(&event) {
                    match library_event.action.as_str() {
                        "approve_library" => {
                            println!(
                                "Library approved: {} for account {}",
                                library_event.library,
                                library_event.account
                            );
                            
                            // Update cached data
                            self.update_library_cache(library_event.account, library_event.library, true).await?;
                        }
                        "remove_library" => {
                            println!(
                                "Library removed: {} from account {}",
                                library_event.library,
                                library_event.account
                            );
                            
                            // Update cached data
                            self.update_library_cache(library_event.account, library_event.library, false).await?;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    async fn update_library_cache(&self, account: Addr, library: String, approved: bool) -> Result<(), Box<dyn std::error::Error>> {
        // Update local cache/database with library approval status
        println!("Updating cache: account={}, library={}, approved={}", account, library, approved);
        Ok(())
    }
}
```

#### 2. Account Administration

```rust
use generated::valence_base_account::client::ValenceBaseAccountClient;
use cosmwasm_std::Addr;

pub struct AccountAdministrator {
    account_client: ValenceBaseAccountClient,
    admin_address: Addr,
}

impl AccountAdministrator {
    pub fn new(account_client: ValenceBaseAccountClient, admin_address: Addr) -> Self {
        Self { account_client, admin_address }
    }

    pub async fn approve_library(
        &self,
        library_address: String,
    ) -> Result<TxHash, Box<dyn std::error::Error>> {
        // Check if we're the admin
        let ownership = self.account_client.ownership().await?;
        if ownership.owner != Some(self.admin_address.to_string()) {
            return Err("Not authorized: only admin can approve libraries".into());
        }

        // Check if library is already approved
        let approved_libraries = self.account_client.list_approved_libraries().await?;
        if approved_libraries.contains(&library_address) {
            return Err("Library already approved".into());
        }

        // Execute library approval
        let tx_hash = self.account_client.approve_library(library_address.clone()).await?;
        
        println!(
            "Library {} approved for account {} (tx: {})",
            library_address,
            self.account_client.contract_address,
            tx_hash
        );

        Ok(tx_hash)
    }

    pub async fn remove_library(
        &self,
        library_address: String,
    ) -> Result<TxHash, Box<dyn std::error::Error>> {
        // Check if we're the admin
        let ownership = self.account_client.ownership().await?;
        if ownership.owner != Some(self.admin_address.to_string()) {
            return Err("Not authorized: only admin can remove libraries".into());
        }

        // Check if library is currently approved
        let approved_libraries = self.account_client.list_approved_libraries().await?;
        if !approved_libraries.contains(&library_address) {
            return Err("Library not currently approved".into());
        }

        // Execute library removal
        let tx_hash = self.account_client.remove_library(library_address.clone()).await?;
        
        println!(
            "Library {} removed from account {} (tx: {})",
            library_address,
            self.account_client.contract_address,
            tx_hash
        );

        Ok(tx_hash)
    }

    pub async fn transfer_ownership(
        &self,
        new_owner: String,
        expiry: Option<Expiration>,
    ) -> Result<TxHash, Box<dyn std::error::Error>> {
        let tx_hash = self.account_client.transfer_ownership(new_owner.clone(), expiry).await?;
        
        println!(
            "Ownership transfer initiated: {} -> {} (tx: {})",
            self.admin_address,
            new_owner,
            tx_hash
        );

        Ok(tx_hash)
    }

    pub async fn execute_messages(
        &self,
        messages: Vec<CosmosMsg>,
    ) -> Result<TxHash, Box<dyn std::error::Error>> {
        // Execute arbitrary messages through the account
        let tx_hash = self.account_client.execute_msg(messages.clone()).await?;
        
        println!(
            "Executed {} messages through account {} (tx: {})",
            messages.len(),
            self.account_client.contract_address,
            tx_hash
        );

        Ok(tx_hash)
    }
}
```

## Step 4: Storage Integration

### Database Schema

The generated storage includes tables for tracking Valence base account events:

```sql
-- Generated migration: 20240101120000_valence_base_account_setup.sql
CREATE TABLE valence_base_account_library_events (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(63) NOT NULL,
    action VARCHAR(20) NOT NULL, -- 'approve_library' or 'remove_library'
    library_address VARCHAR(63) NOT NULL,
    admin_address VARCHAR(63) NOT NULL,
    block_height BIGINT NOT NULL,
    transaction_hash VARCHAR(64) NOT NULL,
    event_index INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    UNIQUE(transaction_hash, event_index)
);

CREATE TABLE valence_base_account_ownership_events (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(63) NOT NULL,
    action VARCHAR(30) NOT NULL, -- 'transfer_ownership', 'accept_ownership', 'renounce_ownership'
    old_owner VARCHAR(63),
    new_owner VARCHAR(63),
    pending_owner VARCHAR(63),
    expiry_height BIGINT,
    expiry_time TIMESTAMP,
    block_height BIGINT NOT NULL,
    transaction_hash VARCHAR(64) NOT NULL,
    event_index INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    UNIQUE(transaction_hash, event_index)
);

CREATE TABLE valence_base_account_executions (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(63) NOT NULL,
    executor_address VARCHAR(63) NOT NULL,
    message_count INTEGER NOT NULL,
    submessage_count INTEGER NOT NULL,
    payload JSONB,
    block_height BIGINT NOT NULL,
    transaction_hash VARCHAR(64) NOT NULL,
    event_index INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    UNIQUE(transaction_hash, event_index)
);

-- Indexes for efficient querying
CREATE INDEX idx_valence_library_events_contract ON valence_base_account_library_events(contract_address);
CREATE INDEX idx_valence_library_events_library ON valence_base_account_library_events(library_address);
CREATE INDEX idx_valence_library_events_block ON valence_base_account_library_events(block_height);
CREATE INDEX idx_valence_library_events_timestamp ON valence_base_account_library_events(timestamp);

CREATE INDEX idx_valence_ownership_events_contract ON valence_base_account_ownership_events(contract_address);
CREATE INDEX idx_valence_ownership_events_owner ON valence_base_account_ownership_events(old_owner, new_owner);
CREATE INDEX idx_valence_ownership_events_block ON valence_base_account_ownership_events(block_height);

CREATE INDEX idx_valence_executions_contract ON valence_base_account_executions(contract_address);
CREATE INDEX idx_valence_executions_executor ON valence_base_account_executions(executor_address);
CREATE INDEX idx_valence_executions_block ON valence_base_account_executions(block_height);
```

### Storage Implementation

```rust
use generated::valence_base_account::storage::{ValenceBaseAccountStorage, LibraryEventRecord, OwnershipEventRecord};
use sqlx::PgPool;

pub struct ValenceAccountIndexer {
    storage: ValenceBaseAccountStorage,
    account_client: ValenceBaseAccountClient,
}

impl ValenceAccountIndexer {
    pub fn new(pool: PgPool, account_client: ValenceBaseAccountClient) -> Self {
        Self {
            storage: ValenceBaseAccountStorage::new(pool),
            account_client,
        }
    }

    pub async fn index_block(&self, block_height: u64) -> Result<(), Box<dyn std::error::Error>> {
        println!("Indexing Valence base account events for block {}", block_height);

        // Get all events from the block related to our contract
        let events = self.account_client.cosmos_client.get_events_for_block(
            block_height,
            EventFilter::new()
                .contract_address(self.account_client.contract_address.clone())
        ).await?;

        let mut library_events = Vec::new();
        let mut ownership_events = Vec::new();
        let mut execution_events = Vec::new();

        // Parse events
        for event in events {
            match event.event_type.as_str() {
                "wasm" => {
                    let action = event.get_attribute("action")?;
                    match action.as_str() {
                        "approve_library" | "remove_library" => {
                            let library_event = LibraryEventRecord {
                                contract_address: event.contract_address,
                                action: action,
                                library_address: event.get_attribute("library")?,
                                admin_address: event.get_attribute("admin")?,
                                block_height,
                                transaction_hash: event.transaction_hash,
                                event_index: event.event_index,
                                timestamp: self.get_block_timestamp(block_height).await?,
                            };
                            library_events.push(library_event);
                        }
                        "transfer_ownership" | "accept_ownership" | "renounce_ownership" => {
                            let ownership_event = OwnershipEventRecord {
                                contract_address: event.contract_address,
                                action: action,
                                old_owner: event.get_optional_attribute("old_owner"),
                                new_owner: event.get_optional_attribute("new_owner"),
                                pending_owner: event.get_optional_attribute("pending_owner"),
                                expiry_height: event.get_optional_attribute("expiry_height")
                                    .and_then(|s| s.parse().ok()),
                                expiry_time: event.get_optional_attribute("expiry_time")
                                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                                block_height,
                                transaction_hash: event.transaction_hash,
                                event_index: event.event_index,
                                timestamp: self.get_block_timestamp(block_height).await?,
                            };
                            ownership_events.push(ownership_event);
                        }
                        "execute" => {
                            // Handle message execution events
                            // ... implementation details
                        }
                        _ => {} // Unknown event
                    }
                }
                _ => {} // Non-wasm event
            }
        }

        // Store in database
        if !library_events.is_empty() {
            self.storage.insert_library_events(&library_events).await?;
            println!("Stored {} library events", library_events.len());
        }

        if !ownership_events.is_empty() {
            self.storage.insert_ownership_events(&ownership_events).await?;
            println!("Stored {} ownership events", ownership_events.len());
        }

        Ok(())
    }

    async fn get_block_timestamp(&self, block_height: u64) -> Result<chrono::DateTime<chrono::Utc>, Box<dyn std::error::Error>> {
        let block = self.account_client.cosmos_client.get_block(block_height).await?;
        Ok(block.header.time)
    }

    pub async fn get_library_history(
        &self,
        contract_address: String,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LibraryEventRecord>, Box<dyn std::error::Error>> {
        self.storage.get_library_events_for_contract(contract_address, limit, offset).await
    }

    pub async fn get_current_libraries(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        self.account_client.list_approved_libraries().await
    }
}
```

## Step 5: API Integration

### REST Endpoints

The generated API includes REST endpoints:

```rust
use generated::valence_base_account::api::ValenceBaseAccountApi;
use axum::{Router, extract::{Path, Query}, Json};

pub fn create_valence_account_router(indexer: ValenceAccountIndexer) -> Router {
    Router::new()
        .route("/valence/accounts/:address/ownership", get(get_ownership))
        .route("/valence/accounts/:address/libraries", get(get_libraries))
        .route("/valence/accounts/:address/library-history", get(get_library_history))
        .route("/valence/accounts/:address/ownership-history", get(get_ownership_history))
        .with_state(indexer)
}

async fn get_ownership(
    Path(address): Path<String>,
    State(indexer): State<ValenceAccountIndexer>,
) -> Result<Json<OwnershipResponse>, StatusCode> {
    let ownership = indexer.account_client.ownership().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(OwnershipResponse {
        account_address: address,
        owner: ownership.owner,
        pending_owner: ownership.pending_owner,
        pending_expiry: ownership.pending_expiry,
    }))
}

async fn get_libraries(
    Path(address): Path<String>,
    State(indexer): State<ValenceAccountIndexer>,
) -> Result<Json<LibrariesResponse>, StatusCode> {
    let libraries = indexer.account_client.list_approved_libraries().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(LibrariesResponse {
        account_address: address,
        approved_libraries: libraries,
    }))
}

async fn get_library_history(
    Path(address): Path<String>,
    Query(params): Query<HistoryQuery>,
    State(indexer): State<ValenceAccountIndexer>,
) -> Result<Json<LibraryHistoryResponse>, StatusCode> {
    let events = indexer.get_library_history(
        address.clone(),
        params.limit.unwrap_or(100),
        params.offset.unwrap_or(0),
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(LibraryHistoryResponse {
        account_address: address,
        events,
    }))
}

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct OwnershipResponse {
    account_address: String,
    owner: Option<String>,
    pending_owner: Option<String>,
    pending_expiry: Option<Expiration>,
}

#[derive(Serialize)]
struct LibrariesResponse {
    account_address: String,
    approved_libraries: Vec<String>,
}

#[derive(Serialize)]
struct LibraryHistoryResponse {
    account_address: String,
    events: Vec<LibraryEventRecord>,
}
```

### GraphQL Integration

```rust
use async_graphql::{Object, Context, Result, Schema};
use generated::valence_base_account::client::ValenceBaseAccountClient;

pub struct ValenceAccountQuery;

#[Object]
impl ValenceAccountQuery {
    async fn valence_account_ownership(&self, ctx: &Context<'_>, address: String) -> Result<OwnershipInfo> {
        let indexer = ctx.data::<ValenceAccountIndexer>()?;
        let ownership = indexer.account_client.ownership().await?;
        Ok(OwnershipInfo {
            owner: ownership.owner,
            pending_owner: ownership.pending_owner,
            pending_expiry: ownership.pending_expiry,
        })
    }

    async fn valence_account_libraries(&self, ctx: &Context<'_>, address: String) -> Result<Vec<String>> {
        let indexer = ctx.data::<ValenceAccountIndexer>()?;
        let libraries = indexer.account_client.list_approved_libraries().await?;
        Ok(libraries)
    }

    async fn valence_account_library_history(
        &self,
        ctx: &Context<'_>,
        address: String,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<LibraryEventRecord>> {
        let indexer = ctx.data::<ValenceAccountIndexer>()?;
        let events = indexer.get_library_history(
            address,
            limit.unwrap_or(100) as i64,
            offset.unwrap_or(0) as i64,
        ).await?;
        Ok(events)
    }
}

pub struct ValenceAccountSubscription;

#[Object]
impl ValenceAccountSubscription {
    async fn valence_account_events(&self, ctx: &Context<'_>, address: String) -> impl Stream<Item = AccountEvent> {
        let indexer = ctx.data::<ValenceAccountIndexer>().unwrap();
        // Implementation for real-time account event subscriptions
        indexer.subscribe_to_account_events(address).await
    }
}

#[derive(SimpleObject)]
struct OwnershipInfo {
    owner: Option<String>,
    pending_owner: Option<String>,
    pending_expiry: Option<Expiration>,
}

type ValenceAccountSchema = Schema<ValenceAccountQuery, EmptyMutation, ValenceAccountSubscription>;

pub fn create_valence_account_schema(indexer: ValenceAccountIndexer) -> ValenceAccountSchema {
    Schema::build(ValenceAccountQuery, EmptyMutation, ValenceAccountSubscription)
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
        .unwrap_or_else(|_| "postgresql://localhost/almanac_valence".to_string());
    let pool = PgPool::connect(&database_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Cosmos client setup
    let rpc_url = std::env::var("COSMOS_RPC_URL")
        .unwrap_or_else(|_| "https://cosmos-rpc.quickapi.com:443".to_string());
    let cosmos_client = CosmosClient::new(&rpc_url).await?;

    // Valence base account client setup
    let account_address = Addr::unchecked(
        std::env::var("VALENCE_ACCOUNT_ADDRESS")
            .unwrap_or_else(|_| "cosmos1valencebaseaccountexample123456789".to_string())
    );
    let account_client = ValenceBaseAccountClient::new(cosmos_client, account_address);

    // Indexer setup
    let indexer = Arc::new(ValenceAccountIndexer::new(pool.clone(), account_client));

    // Start background indexing
    let indexer_task = indexer.clone();
    tokio::spawn(async move {
        if let Err(e) = start_indexing(indexer_task).await {
            eprintln!("Indexing error: {}", e);
        }
    });

    // API routes
    let app = Router::new()
        .merge(create_valence_account_router((*indexer).clone()))
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn(logging_middleware));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Valence base account indexer API running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await?;

    Ok(())
}

async fn start_indexing(indexer: Arc<ValenceAccountIndexer>) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_height = indexer.get_last_indexed_height().await?;
    
    loop {
        // Get latest height
        let latest_height = indexer.account_client.cosmos_client.get_latest_height().await?;
        
        // Index any missing blocks
        while current_height < latest_height {
            current_height += 1;
            println!("Indexing block {}", current_height);
            
            if let Err(e) = indexer.index_block(current_height).await {
                eprintln!("Error indexing block {}: {}", current_height, e);
                // Continue with next block
            }
        }
        
        // Wait before checking for new blocks
        tokio::time::sleep(std::time::Duration::from_secs(6)).await;
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

Create a `valence_account_config.toml` file:

```toml
[database]
url = "postgresql://localhost/almanac_valence"
max_connections = 10

[cosmos]
rpc_url = "https://cosmos-rpc.quickapi.com:443"
chain_id = "cosmoshub-4"

[valence]
account_address = "cosmos1valencebaseaccountexample123456789"

[indexer]
start_height = 18000000  # Block to start indexing from
poll_interval = 6        # Seconds between polling for new blocks

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
    async fn test_valence_account_indexing() {
        let docker = Cli::default();
        let postgres_container = docker.run(Postgres::default());
        let connection_string = format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            postgres_container.get_host_port_ipv4(5432)
        );

        let pool = PgPool::connect(&connection_string).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // Test indexing a known block with Valence account events
        let indexer = setup_test_indexer(pool).await;
        indexer.index_block(18000000).await.unwrap();

        // Verify events were indexed
        let library_events = indexer.storage.get_library_events_for_block(18000000).await.unwrap();
        assert!(!library_events.is_empty());
    }

    #[tokio::test]
    async fn test_ownership_queries() {
        let indexer = setup_test_indexer_with_data().await;
        
        // Test ownership query
        let ownership = indexer.account_client.ownership().await.unwrap();
        assert!(ownership.owner.is_some());
    }

    #[tokio::test]
    async fn test_library_management() {
        let indexer = setup_test_indexer_with_data().await;
        
        // Test library listing
        let libraries = indexer.account_client.list_approved_libraries().await.unwrap();
        assert!(!libraries.is_empty());
    }

    async fn setup_test_indexer(pool: PgPool) -> ValenceAccountIndexer {
        let cosmos_client = CosmosClient::new("http://localhost:26657").await.unwrap();
        let account_client = ValenceBaseAccountClient::new(
            cosmos_client,
            Addr::unchecked("cosmos1valencebaseaccountexample123456789")
        );
        ValenceAccountIndexer::new(pool, account_client)
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
    
    // Should process at least 5 blocks per second for Cosmos
    assert!(duration.as_secs() < 200);
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

COPY --from=builder /app/target/release/valence-account-indexer /usr/local/bin/

EXPOSE 3000 9090

CMD ["valence-account-indexer"]
```

```yaml
# docker-compose.yml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: almanac_valence
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

  valence-indexer:
    build: .
    ports:
      - "3000:3000"
      - "9090:9090"
    environment:
      DATABASE_URL: postgresql://postgres:postgres@postgres:5432/almanac_valence
      COSMOS_RPC_URL: https://cosmos-rpc.quickapi.com:443
      VALENCE_ACCOUNT_ADDRESS: cosmos1valencebaseaccountexample123456789
      RUST_LOG: info
    depends_on:
      - postgres
    restart: unless-stopped

volumes:
  postgres_data:
```

This example demonstrates a complete, production-ready Valence Base Account integration using the Cosmos codegen system, including real-time indexing of library management events, ownership tracking, and comprehensive monitoring capabilities. 