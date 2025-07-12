# Almanac Contract Code Generation

This document provides comprehensive documentation for the contract code generation system in the Almanac indexer, covering both Ethereum and Cosmos contract integration.

## Overview

The Almanac codegen system automatically generates everything needed to integrate blockchain contracts into an indexer node. It supports both Ethereum ABI files and CosmWasm schema files, generating:

- **Client code**: Type-safe contract interaction methods
- **Storage models**: Database schemas and storage traits
- **API endpoints**: REST and GraphQL endpoints
- **Database migrations**: SQL migration files
- **Integration tests**: Test templates for generated code

## Core Features

### Universal Capabilities

- **Schema Parsing**: Automatic parsing of both Ethereum ABIs and CosmWasm schemas
- **Type Generation**: Rust type definitions for all contract types and functions
- **Storage Generation**: PostgreSQL schemas and RocksDB key-value stores
- **API Generation**: REST endpoints, GraphQL schemas, and WebSocket subscriptions
- **Migration Generation**: Database migration files with versioning
- **Template System**: Customizable Handlebars templates for specialized code generation

### Ethereum Support

- **ABI Parsing**: Complete Ethereum ABI JSON format support
- **Function Generation**: Both view (read-only) and state-changing functions
- **Event Parsing**: Event parsing and indexing with typed parameters
- **Complex Types**: Structs, arrays, mappings, and custom types
- **Multiple Contracts**: Support for multi-contract systems

### Cosmos Support

- **Message Types**: InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg support
- **Schema Validation**: Complete CosmWasm JSON schema validation
- **Complex Structures**: Enums, structs, arrays, and optional fields
- **Reference Resolution**: $ref resolution within schemas

## Getting Started

### CLI Usage

The code generation system uses a unified CLI interface:

```bash
# Ethereum contract generation
almanac ethereum generate-contract <abi-file> --address <contract-addr> --chain <chain-id>

# Cosmos contract generation  
almanac cosmos generate-contract <schema-file> --address <contract-addr> --chain <chain-id>
```

### Feature Selection

Both systems support the same feature set:

- `client`: Generate contract client code
- `storage`: Generate storage models and schemas
- `api`: Generate REST and GraphQL endpoints
- `migrations`: Generate database migration files

Example with full features:
```bash
almanac ethereum generate-contract usdc_abi.json \
  --address 0xa0b86a33e6dc39c9c6c7c7ccf9c2e9c5c2c8c0 \
  --chain 1 \
  --features client,storage,api,migrations \
  --output-dir ./contracts/usdc

almanac cosmos generate-contract cw20_schema.json \
  --address cosmos1abc123... \
  --chain cosmoshub-4 \
  --features client,storage,api \
  --output-dir ./contracts/cw20
```

## Generated Code Structure

Both Ethereum and Cosmos generation follow the same output structure:

```
generated/
├── client/
│   ├── mod.rs              # Client module definition
│   ├── types.rs            # Generated type definitions
│   ├── functions.rs        # Contract function calls (Ethereum)
│   ├── execute.rs          # Execute methods (Cosmos)
│   ├── query.rs            # Query methods (Cosmos)
│   └── events.rs           # Event parsing methods
├── storage/
│   ├── mod.rs              # Storage module definition
│   ├── postgres_schema.sql # PostgreSQL table definitions
│   ├── rocksdb.rs          # RocksDB key-value schemas
│   └── traits.rs           # Storage trait implementations
├── api/
│   ├── mod.rs              # API module definition
│   ├── rest.rs             # REST endpoint definitions
│   ├── graphql.rs          # GraphQL schema and resolvers
│   └── websocket.rs        # WebSocket subscription handlers
└── migrations/
    └── 20240101120000_contract_setup.sql
```

## Client Code Generation

### Ethereum Client Example

Generated Ethereum clients provide type-safe contract interaction:

```rust
use indexer_core::Result;
use alloy_primitives::{Address, U256};

pub struct UsdcClient {
    ethereum_client: EthereumClient,
    contract_address: Address,
}

impl UsdcClient {
    pub fn new(ethereum_client: EthereumClient, contract_address: Address) -> Self {
        Self { ethereum_client, contract_address }
    }

    // View functions (read-only)
    pub async fn balance_of(&self, account: Address) -> Result<U256> {
        let call_data = encode_function_call("balanceOf", &[Token::Address(account)])?;
        let result = self.ethereum_client.call(&self.contract_address, &call_data).await?;
        Ok(U256::from_be_bytes(&result))
    }

    // State-changing functions
    pub async fn transfer(&self, to: Address, amount: U256) -> Result<TxHash> {
        let call_data = encode_function_call("transfer", &[
            Token::Address(to), Token::Uint(amount)
        ])?;
        self.ethereum_client.send_transaction(&self.contract_address, &call_data, None).await
    }

    // Event parsing
    pub async fn parse_transfer_events(&self, logs: Vec<Log>) -> Result<Vec<TransferEvent>> {
        let mut events = Vec::new();
        for log in logs {
            if let Ok(event) = TransferEvent::from_log(&log) {
                events.push(event);
            }
        }
        Ok(events)
    }
}
```

### Cosmos Client Example

Generated Cosmos clients provide CosmWasm contract interaction:

```rust
use indexer_core::Result;
use cosmwasm_std::{Addr, Uint128};

pub struct Cw20Client {
    cosmos_client: CosmosClient,
    contract_address: Addr,
}

impl Cw20Client {
    pub fn new(cosmos_client: CosmosClient, contract_address: Addr) -> Self {
        Self { cosmos_client, contract_address }
    }

    // Query methods
    pub async fn balance(&self, address: Addr) -> Result<Uint128> {
        let query_msg = QueryMsg::Balance { address };
        self.cosmos_client.query_smart(&self.contract_address, &query_msg).await
    }

    pub async fn token_info(&self) -> Result<TokenInfoResponse> {
        let query_msg = QueryMsg::TokenInfo {};
        self.cosmos_client.query_smart(&self.contract_address, &query_msg).await
    }

    // Execute methods
    pub async fn transfer(&self, recipient: Addr, amount: Uint128) -> Result<TxResponse> {
        let execute_msg = ExecuteMsg::Transfer { recipient, amount };
        self.cosmos_client.execute(&self.contract_address, &execute_msg, &[]).await
    }
}
```

## Type System

### Ethereum Type Mappings

The system maps Ethereum ABI types to Rust types:

- `uint256` → `U256`
- `int256` → `I256`
- `address` → `Address`
- `bytes32` → `FixedBytes<32>`
- `bytes` → `Bytes`
- `string` → `String`
- `bool` → `bool`
- `uint256[]` → `Vec<U256>`
- Complex structs → Generated Rust structs

### Cosmos Type Mappings

CosmWasm schema types map to Rust types:

- JSON Schema `string` → `String`
- JSON Schema `integer` → `u64`, `i64`, `Uint128`, etc.
- JSON Schema `boolean` → `bool`
- JSON Schema `array` → `Vec<T>`
- JSON Schema `object` → Generated Rust structs
- Schema enums → Rust enums with proper variant handling

## Storage Integration

### Database Schema Generation

Both systems generate consistent PostgreSQL schemas:

```sql
-- Ethereum contract events table
CREATE TABLE contract_usdc_transfers (
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

-- Cosmos contract state table
CREATE TABLE contract_cw20_balances (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(255) NOT NULL,
    account_address VARCHAR(255) NOT NULL,
    balance NUMERIC NOT NULL,
    block_height BIGINT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    UNIQUE(contract_address, account_address)
);
```

### RocksDB Integration

Generated RocksDB schemas provide efficient key-value storage:

```rust
// Common patterns for both chains
pub fn contract_state_key(chain_id: &str, contract_addr: &str, key: &str) -> Vec<u8> {
    format!("contract:{}:{}:{}", chain_id, contract_addr, key).into_bytes()
}

// Ethereum-specific event keys
pub fn ethereum_event_key(tx_hash: &str, log_index: u32) -> Vec<u8> {
    format!("eth_event:{}:{}", tx_hash, log_index).into_bytes()
}

// Cosmos-specific state keys
pub fn cosmos_state_key(contract_addr: &str, state_key: &str) -> Vec<u8> {
    format!("cosmos_state:{}:{}", contract_addr, state_key).into_bytes()
}
```

## API Generation

### REST Endpoints

Generated REST endpoints follow OpenAPI 3.0 specification:

```
# Ethereum endpoints
GET    /api/v1/ethereum/{chain}/contracts/{address}/balance/{account}
POST   /api/v1/ethereum/{chain}/contracts/{address}/transfer
GET    /api/v1/ethereum/{chain}/contracts/{address}/events

# Cosmos endpoints
GET    /api/v1/cosmos/{chain}/contracts/{address}/balance/{account}
POST   /api/v1/cosmos/{chain}/contracts/{address}/execute
GET    /api/v1/cosmos/{chain}/contracts/{address}/token-info
```

### GraphQL Schema

Generated GraphQL provides unified type-safe queries:

```graphql
type Query {
  ethereumContract(address: String!, chain: String!): EthereumContract
  cosmosContract(address: String!, chain: String!): CosmosContract
}

type EthereumContract {
  address: String!
  totalSupply: String
  balanceOf(account: String!): String
  transfers(limit: Int, offset: Int): [EthereumTransfer!]!
}

type CosmosContract {
  address: String!
  tokenInfo: TokenInfo
  balance(account: String!): String
  transfers(limit: Int, offset: Int): [CosmosTransfer!]!
}
```

### WebSocket Subscriptions

Real-time event subscriptions work across both chains:

```graphql
subscription {
  contractEvents(
    chain: "ethereum"
    address: "0xa0b86a33e6dc39c9c6c7c7ccf9c2e9c5c2c8c0"
    eventTypes: ["Transfer", "Approval"]
  ) {
    type
    data
    blockNumber
    transactionHash
    timestamp
  }
}

subscription {
  cosmosContractEvents(
    chain: "cosmoshub-4" 
    address: "cosmos1abc123..."
  ) {
    type
    data
    blockHeight
    timestamp
  }
}
```

## Configuration

### Unified Configuration

Both systems support similar configuration patterns:

```toml
# ethereum_codegen.toml
[ethereum]
provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
default_chain = "1"
output_dir = "./generated"

[ethereum.database]
postgres_schema = "public"
table_prefix = "contract_"

[ethereum.templates]
client_template = "custom_client.hbs"
storage_template = "custom_storage.hbs"

[[ethereum.contracts]]
name = "usdc"
address = "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0"
abi_path = "./abis/usdc.json"
features = ["client", "storage", "api"]
```

```toml  
# cosmos_codegen.toml
[cosmos]
grpc_url = "https://rpc.cosmos.network:443"
default_chain = "cosmoshub-4"
output_dir = "./generated"

[cosmos.database]
postgres_schema = "public"  
table_prefix = "contract_"

[cosmos.templates]
client_template = "custom_client.hbs"
storage_template = "custom_storage.hbs"

[[cosmos.contracts]]
name = "cw20"
address = "cosmos1abc123..."
schema_path = "./schemas/cw20.json"
features = ["client", "storage"]
```

## Testing Integration

### Generated Test Templates

Both systems generate similar test structures:

```rust
// Ethereum contract tests
#[tokio::test]
async fn test_ethereum_transfer() {
    let client = setup_ethereum_client().await;
    
    let from = Address::from_str("0x...").unwrap();
    let to = Address::from_str("0x...").unwrap();
    let amount = U256::from(1000000);
    
    let tx_hash = client.transfer(to, amount).await.unwrap();
    let receipt = client.ethereum_client.wait_for_transaction(tx_hash).await.unwrap();
    assert_eq!(receipt.status, Some(1.into()));
}

// Cosmos contract tests
#[tokio::test]
async fn test_cosmos_transfer() {
    let client = setup_cosmos_client().await;
    
    let response = client.transfer(
        Addr::unchecked("cosmos1recipient"),
        Uint128::new(1000000)
    ).await.unwrap();
    
    assert_eq!(response.code, 0);
}
```

## Custom Templates

### Template System

Both systems use Handlebars templates with shared helpers:

```handlebars
{{! ethereum_client.hbs }}
//! Generated Ethereum client for {{contract_name}}

use indexer_core::Result;
use alloy_primitives::{Address, U256};

pub struct {{pascal_case contract_name}}Client {
    ethereum_client: EthereumClient,
    contract_address: Address,
}

impl {{pascal_case contract_name}}Client {
    {{#each functions}}
    pub async fn {{snake_case name}}(&self{{#each inputs}}, {{name}}: {{rust_type type}}{{/each}}) -> Result<{{#if outputs}}{{rust_type (first outputs).type}}{{else}}TxHash{{/if}}> {
        // Generated function implementation
    }
    {{/each}}
}
```

```handlebars
{{! cosmos_client.hbs }}
//! Generated Cosmos client for {{contract_name}}

use indexer_core::Result;
use cosmwasm_std::{Addr, Uint128};

pub struct {{pascal_case contract_name}}Client {
    cosmos_client: CosmosClient,
    contract_address: Addr,
}

impl {{pascal_case contract_name}}Client {
    {{#each query_messages}}
    pub async fn {{snake_case name}}(&self{{#each properties}}, {{@key}}: {{rust_type this}}{{/each}}) -> Result<{{response_type name}}> {
        let query_msg = QueryMsg::{{pascal_case name}} { {{#each properties}}{{@key}}{{#unless @last}}, {{/unless}}{{/each}} };
        self.cosmos_client.query_smart(&self.contract_address, &query_msg).await
    }
    {{/each}}
}
```

## Advanced Usage

### Multi-Contract Integration

Handle related contracts across chains:

```rust
pub struct CrossChainProtocolClient {
    ethereum_token: UsdcClient,
    cosmos_token: Cw20Client,
    bridge_contract: BridgeClient,
}

impl CrossChainProtocolClient {
    pub async fn bridge_tokens(&self, amount: U256, cosmos_recipient: Addr) -> Result<()> {
        // Lock tokens on Ethereum
        let lock_tx = self.ethereum_token.lock_for_bridge(amount).await?;
        
        // Wait for confirmation
        self.wait_for_ethereum_confirmation(lock_tx).await?;
        
        // Mint tokens on Cosmos
        let mint_msg = self.cosmos_token.mint(cosmos_recipient, amount.into()).await?;
        
        Ok(())
    }
}
```

### Performance Optimization

Both systems include optimization features:

1. **Selective Generation**: Only generate needed features
2. **Batch Processing**: Group multiple contract operations
3. **Efficient Indexing**: Strategic database indexes
4. **Caching**: Client-side caching for frequently accessed data

## Best Practices

### Schema Design

- Use clear, descriptive type names
- Minimize deeply nested structures  
- Include comprehensive documentation
- Use consistent naming conventions across chains

### Code Organization

- Group related functionality in modules
- Use meaningful file and directory names
- Include comprehensive error handling
- Add documentation comments to generated code

### Testing Strategy

- Test both success and error cases
- Include integration tests for end-to-end workflows
- Use property-based testing for complex types
- Test with realistic data volumes across both chains

This unified code generation system provides consistent, type-safe integration for both Ethereum and Cosmos contracts while maintaining the flexibility to handle chain-specific requirements and patterns. 