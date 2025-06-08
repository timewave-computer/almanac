# Cosmos Contract Code Generation

This document provides comprehensive documentation for the Cosmos contract code generation system in the Almanac indexer.

## Overview

The Cosmos codegen system automatically generates everything needed to integrate a CosmWasm contract into an Almanac indexer node. It parses CosmWasm message schema files (`*_msg.json`) and generates:

- **Client code**: Type-safe contract interaction methods
- **Storage models**: Database schemas and storage traits  
- **API endpoints**: REST and GraphQL endpoints
- **Database migrations**: SQL migration files
- **Integration tests**: Test templates for generated code

## Features

### Core Capabilities

- **Schema Parsing**: Automatic parsing of CosmWasm message schemas
- **Type Generation**: Rust type definitions for all contract types
- **Client Generation**: Methods for contract queries, executions, and instantiation
- **Storage Generation**: PostgreSQL schemas and RocksDB key-value stores
- **API Generation**: REST endpoints, GraphQL schemas, and WebSocket subscriptions
- **Migration Generation**: Database migration files with versioning

### Supported Message Types

- `InstantiateMsg`: Contract instantiation messages
- `ExecuteMsg`: Contract execution messages  
- `QueryMsg`: Contract query messages
- `MigrateMsg`: Contract migration messages (optional)
- Custom event definitions and complex nested types

## CLI Usage

### Basic Command

```bash
almanac cosmos generate-contract <msg-file> --address <contract-addr> --chain <chain-id> [options]
```

### Command Options

- `--address <CONTRACT_ADDRESS>`: Contract address on the blockchain (required)
- `--chain <CHAIN_ID>`: Chain ID where contract is deployed (required)
- `--output-dir <DIR>`: Output directory for generated code (default: `./generated`)
- `--namespace <NAME>`: Namespace for generated code modules
- `--features <LIST>`: Comma-separated list of features to generate
- `--dry-run`: Preview generation without creating files
- `--verbose`: Enable verbose logging

### Feature Selection

Available features:
- `client`: Generate contract client code
- `storage`: Generate storage models and schemas
- `api`: Generate REST and GraphQL endpoints  
- `migrations`: Generate database migration files

Example:
```bash
almanac cosmos generate-contract valence_base_account_schema.json \
  --address cosmos1valencebaseaccountexample... \
  --chain cosmoshub-4 \
  --features client,storage,api \
  --output-dir ./contracts/valence_base_account
```

## Configuration

### Configuration File

Create a `cosmos_codegen.toml` file for advanced configuration:

```toml
[cosmos]
# Default chain configuration
default_chain = "cosmoshub-4"

# Template customization
[cosmos.templates]
client_template = "custom_client.hbs"
storage_template = "custom_storage.hbs"

# Database configuration
[cosmos.database]
postgres_schema = "public"
table_prefix = "contract_"

# API configuration  
[cosmos.api]
base_path = "/api/v1/cosmos"
enable_websockets = true
rate_limiting = true
```

### Environment Variables

- `COSMOS_CODEGEN_OUTPUT_DIR`: Default output directory
- `COSMOS_CODEGEN_FEATURES`: Default features to enable
- `COSMOS_CODEGEN_VERBOSE`: Enable verbose logging

## Generated Code Structure

### Directory Layout

```
generated/
├── client/
│   ├── mod.rs              # Client module definition
│   ├── types.rs            # Generated type definitions
│   ├── execute.rs          # Execute message methods
│   ├── query.rs            # Query message methods
│   ├── instantiate.rs      # Instantiation methods
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

### Example Generated Client

```rust
use indexer_core::Result;
use cosmwasm_std::{Addr, Uint128};

pub struct ValenceBaseAccountClient {
    cosmos_client: CosmosClient,
    contract_address: Addr,
}

impl ValenceBaseAccountClient {
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

    pub async fn mint(&self, recipient: Addr, amount: Uint128) -> Result<TxResponse> {
        let execute_msg = ExecuteMsg::Mint { recipient, amount };
        self.cosmos_client.execute(&self.contract_address, &execute_msg, &[]).await
    }
}
```

## Schema Parsing

### Supported Schema Formats

The codegen system supports standard CosmWasm JSON schema formats:

```json
{
  "instantiate": {
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "InstantiateMsg",
    "type": "object",
    "properties": {
      "name": { "type": "string" },
      "symbol": { "type": "string" },
      "decimals": { "type": "integer" },
      "initial_balances": {
        "type": "array", 
        "items": { "$ref": "#/definitions/Cw20Coin" }
      }
    }
  },
  "execute": { /* ExecuteMsg schema */ },
  "query": { /* QueryMsg schema */ },
  "definitions": { /* Type definitions */ }
}
```

### Complex Type Support

- **Enums**: Rust enums with proper variant handling
- **Structs**: Complex nested structures  
- **Arrays**: Vec<T> types with proper bounds
- **Optional Fields**: Option<T> types
- **References**: $ref resolution within schema

## Storage Integration

### PostgreSQL Schema Generation

Generated SQL creates tables for:
- Contract state tracking
- Transaction history
- Event logs with indexed columns
- Type-safe column definitions

Example generated schema:
```sql
CREATE TABLE contract_valence_base_account_libraries (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(255) NOT NULL,
    account_address VARCHAR(255) NOT NULL,
    balance NUMERIC NOT NULL,
    block_height BIGINT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    UNIQUE(contract_address, account_address)
);

CREATE INDEX idx_valence_libraries_contract ON contract_valence_base_account_libraries(contract_address);
CREATE INDEX idx_valence_libraries_account ON contract_valence_base_account_libraries(account_address);
```

### RocksDB Integration

Generated RocksDB schemas provide:
- Efficient key-value storage patterns
- Prefix-based organization  
- Serialization/deserialization helpers
- Batch operation support

## API Generation

### REST Endpoints

Generated REST endpoints follow OpenAPI 3.0 specification:

```
GET    /api/v1/cosmos/{chain}/contracts/{address}/balance/{account}
POST   /api/v1/cosmos/{chain}/contracts/{address}/execute
GET    /api/v1/cosmos/{chain}/contracts/{address}/token-info
GET    /api/v1/cosmos/{chain}/contracts/{address}/events
```

### GraphQL Schema

Generated GraphQL provides type-safe queries:

```graphql
type Query {
  cosmosContract(address: String!, chain: String!): CosmosContract
}

type CosmosContract {
  address: String!
  tokenInfo: TokenInfo
  balance(account: String!): String
  transfers(limit: Int, offset: Int): [Transfer!]!
}
```

### WebSocket Subscriptions

Real-time event subscriptions:

```graphql  
subscription {
  cosmosContractEvents(address: "cosmos1...", chain: "cosmoshub-4") {
    type
    data
    blockHeight
    timestamp
  }
}
```

## Testing Integration

### Generated Test Templates

The codegen creates test templates for:
- Contract interaction testing
- Storage layer testing
- API endpoint testing
- Integration test examples

### Example Test Usage

```rust
#[tokio::test]
async fn test_valence_base_account_library_approval() {
    let client = setup_test_client().await;
    
    // Test transfer functionality
    let response = client.transfer(
        Addr::unchecked("cosmos1recipient"),
        Uint128::new(1000000)
    ).await.unwrap();
    
    assert_eq!(response.code, 0);
    
    // Verify balance updated
    let balance = client.balance(
        Addr::unchecked("cosmos1recipient")
    ).await.unwrap();
    
    assert_eq!(balance, Uint128::new(1000000));
}
```

## Performance Considerations

### Benchmarking

The system includes built-in performance benchmarks:
- Schema parsing performance
- Code generation speed
- Memory usage tracking
- Scaling characteristics

### Optimization Tips

1. **Selective Feature Generation**: Only generate needed features
2. **Schema Optimization**: Minimize complex nested types when possible
3. **Indexing Strategy**: Use appropriate database indexes for query patterns
4. **Caching**: Enable client-side caching for frequently accessed data

## Troubleshooting

### Common Issues

**Schema Parsing Errors**
```
Error: Failed to parse schema: Invalid JSON format
Solution: Validate JSON schema format and fix syntax errors
```

**Type Generation Failures**  
```
Error: Unsupported type in schema definition
Solution: Simplify complex type definitions or use generic types
```

**File Permission Errors**
```
Error: Permission denied writing to output directory  
Solution: Ensure write permissions for output directory
```

### Debug Mode

Enable debug mode for detailed logging:
```bash
RUST_LOG=debug almanac cosmos generate-contract --verbose schema.json
```

## Best Practices

### Schema Design
- Use clear, descriptive type names
- Minimize deeply nested structures
- Include comprehensive documentation in schemas
- Use consistent naming conventions

### Code Organization
- Group related functionality in modules
- Use meaningful file and directory names  
- Include comprehensive error handling
- Add documentation comments to generated code

### Testing Strategy
- Test both success and error cases
- Include integration tests for end-to-end workflows
- Use property-based testing for complex types
- Test with realistic data volumes

## Advanced Usage

### Custom Templates

Create custom Handlebars templates for specialized code generation:

```handlebars
{{! custom_client.hbs }}
//! Generated client for {{contract_name}}
use super::types::*;

pub struct {{pascal_case contract_name}}Client {
    // Custom client implementation
}
```

### Plugin System

Extend codegen with custom plugins:

```rust
#[plugin]
impl CustomGenerator {
    fn generate_custom_code(&self, schema: &Schema) -> Result<String> {
        // Custom code generation logic
    }
}
```

## Migration Guide

### Upgrading from Previous Versions

When upgrading, review breaking changes:
1. Check generated API changes
2. Update database migrations
3. Verify client code compatibility
4. Run integration tests

### Schema Evolution

Handle schema changes gracefully:
1. Use versioning for breaking changes
2. Maintain backward compatibility when possible
3. Provide migration paths for existing data
4. Document all schema changes

## Support and Community

- **Documentation**: https://docs.almanac.com/cosmos-codegen
- **Examples**: https://github.com/timewave-computer/almanac/tree/main/examples/cosmos
- **Issues**: https://github.com/timewave-computer/almanac/issues
- **Discord**: https://discord.gg/almanac 