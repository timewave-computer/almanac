# Ethereum Contract Code Generation

This document provides comprehensive documentation for the Ethereum contract code generation system in the Almanac indexer.

## Overview

The Ethereum codegen system automatically generates everything needed to integrate an Ethereum contract into an Almanac indexer node. It parses Ethereum ABI (Application Binary Interface) files and generates:

- **Client code**: Type-safe contract interaction methods
- **Storage models**: Database schemas and storage traits
- **API endpoints**: REST and GraphQL endpoints
- **Database migrations**: SQL migration files
- **Integration tests**: Test templates for generated code

## Features

### Core Capabilities

- **ABI Parsing**: Automatic parsing of Ethereum contract ABIs (JSON format)
- **Type Generation**: Rust type definitions for all contract types and functions
- **Client Generation**: Methods for contract calls, transactions, and event monitoring
- **Storage Generation**: PostgreSQL schemas and RocksDB key-value stores
- **API Generation**: REST endpoints, GraphQL schemas, and WebSocket subscriptions
- **Migration Generation**: Database migration files with versioning

### Supported Contract Features

- **Functions**: Both view (read-only) and state-changing functions
- **Events**: Event parsing and indexing with typed parameters
- **Constructor**: Contract deployment and initialization
- **Fallback/Receive**: Special functions for handling Ether transfers
- **Complex Types**: Structs, arrays, mappings, and custom types
- **Multiple Contracts**: Support for multi-contract systems

## CLI Usage

### Basic Command

```bash
almanac ethereum generate-contract <abi-file> --address <contract-addr> --chain <chain-id> [options]
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
almanac ethereum generate-contract usdc_abi.json \
  --address 0xa0b86a33e6dc39c9c6c7c7ccf9c2e9c5c2c8c0 \
  --chain 1 \
  --features client,storage,api \
  --output-dir ./contracts/usdc
```

## Configuration

### Configuration File

Create an `ethereum_codegen.toml` file for advanced configuration:

```toml
[ethereum]
# Default chain configuration
default_chain = "1"  # Ethereum mainnet

# Template customization
[ethereum.templates]
client_template = "custom_client.hbs"
storage_template = "custom_storage.hbs"

# Database configuration
[ethereum.database]
postgres_schema = "public"
table_prefix = "contract_"

# API configuration
[ethereum.api]
base_path = "/api/v1/ethereum"
enable_websockets = true
rate_limiting = true

# Web3 provider configuration
[ethereum.web3]
provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
gas_limit = 6000000
gas_price = "20000000000"  # 20 gwei
```

### Environment Variables

- `ETHEREUM_CODEGEN_OUTPUT_DIR`: Default output directory
- `ETHEREUM_CODEGEN_FEATURES`: Default features to enable
- `ETHEREUM_CODEGEN_VERBOSE`: Enable verbose logging
- `WEB3_PROVIDER_URL`: Ethereum node provider URL
- `ETHEREUM_PRIVATE_KEY`: Private key for transactions (development only)

## Generated Code Structure

### Directory Layout

```
generated/
├── client/
│   ├── mod.rs              # Client module definition
│   ├── types.rs            # Generated type definitions
│   ├── functions.rs        # Contract function calls
│   ├── events.rs           # Event parsing methods
│   └── deploy.rs           # Contract deployment
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
use alloy_primitives::{Address, U256, Bytes};

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

    pub async fn total_supply(&self) -> Result<U256> {
        let call_data = encode_function_call("totalSupply", &[])?;
        let result = self.ethereum_client.call(&self.contract_address, &call_data).await?;
        Ok(U256::from_be_bytes(&result))
    }

    pub async fn allowance(&self, owner: Address, spender: Address) -> Result<U256> {
        let call_data = encode_function_call("allowance", &[
            Token::Address(owner),
            Token::Address(spender)
        ])?;
        let result = self.ethereum_client.call(&self.contract_address, &call_data).await?;
        Ok(U256::from_be_bytes(&result))
    }

    // State-changing functions
    pub async fn transfer(&self, to: Address, amount: U256) -> Result<TxHash> {
        let call_data = encode_function_call("transfer", &[
            Token::Address(to),
            Token::Uint(amount)
        ])?;
        
        self.ethereum_client.send_transaction(&self.contract_address, &call_data, None).await
    }

    pub async fn approve(&self, spender: Address, amount: U256) -> Result<TxHash> {
        let call_data = encode_function_call("approve", &[
            Token::Address(spender),
            Token::Uint(amount)
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

// Generated event types
#[derive(Debug, Clone)]
pub struct TransferEvent {
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub block_number: u64,
    pub transaction_hash: TxHash,
    pub log_index: u64,
}

#[derive(Debug, Clone)]
pub struct ApprovalEvent {
    pub owner: Address,
    pub spender: Address,
    pub value: U256,
    pub block_number: u64,
    pub transaction_hash: TxHash,
    pub log_index: u64,
}
```

## ABI Parsing

### Supported ABI Formats

The codegen system supports standard Ethereum ABI JSON format:

```json
[
  {
    "type": "function",
    "name": "transfer",
    "inputs": [
      {"name": "to", "type": "address", "internalType": "address"},
      {"name": "amount", "type": "uint256", "internalType": "uint256"}
    ],
    "outputs": [
      {"name": "", "type": "bool", "internalType": "bool"}
    ],
    "stateMutability": "nonpayable"
  },
  {
    "type": "event",
    "name": "Transfer",
    "inputs": [
      {"name": "from", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "to", "type": "address", "indexed": true, "internalType": "address"},
      {"name": "value", "type": "uint256", "indexed": false, "internalType": "uint256"}
    ]
  }
]
```

### Complex Type Support

- **Elementary Types**: `uint256`, `int256`, `address`, `bool`, `bytes`, `string`
- **Fixed-Size Arrays**: `uint256[3]`, `bytes32[10]`
- **Dynamic Arrays**: `uint256[]`, `address[]`, `bytes[]`
- **Structs/Tuples**: Complex nested structures
- **Mappings**: Represented as function calls for access

## Storage Integration

### PostgreSQL Schema Generation

Generated SQL creates tables for:
- Contract state tracking
- Function call history
- Event logs with indexed columns
- Type-safe column definitions

Example generated schema:
```sql
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

CREATE INDEX idx_usdc_transfers_from ON contract_usdc_transfers(from_address);
CREATE INDEX idx_usdc_transfers_to ON contract_usdc_transfers(to_address);
CREATE INDEX idx_usdc_transfers_block ON contract_usdc_transfers(block_number);
CREATE INDEX idx_usdc_transfers_timestamp ON contract_usdc_transfers(timestamp);
```

### RocksDB Integration

Generated RocksDB schemas provide:
- Efficient key-value storage patterns
- Block-based organization
- Serialization/deserialization helpers
- Batch operation support

## API Generation

### REST Endpoints

Generated REST endpoints follow OpenAPI 3.0 specification:

```
GET    /api/v1/ethereum/{chain}/contracts/{address}/balance/{account}
POST   /api/v1/ethereum/{chain}/contracts/{address}/transfer
GET    /api/v1/ethereum/{chain}/contracts/{address}/total-supply
GET    /api/v1/ethereum/{chain}/contracts/{address}/events
GET    /api/v1/ethereum/{chain}/contracts/{address}/transactions
```

### GraphQL Schema

Generated GraphQL provides type-safe queries:

```graphql
type Query {
  ethereumContract(address: String!, chain: String!): EthereumContract
}

type EthereumContract {
  address: String!
  totalSupply: String
  balanceOf(account: String!): String
  transfers(limit: Int, offset: Int): [Transfer!]!
  events(types: [String!], fromBlock: Int, toBlock: Int): [Event!]!
}

type Transfer {
  from: String!
  to: String!
  value: String!
  blockNumber: Int!
  transactionHash: String!
  timestamp: DateTime!
}
```

### WebSocket Subscriptions

Real-time event subscriptions:

```graphql
subscription {
  ethereumContractEvents(
    address: "0xa0b86a33e6dc39c9c6c7c7ccf9c2e9c5c2c8c0"
    chain: "1"
    eventTypes: ["Transfer", "Approval"]
  ) {
    type
    data
    blockNumber
    transactionHash
    timestamp
  }
}
```

## Testing Integration

### Generated Test Templates

The codegen creates test templates for:
- Contract interaction testing
- Event parsing testing
- Storage layer testing
- API endpoint testing

### Example Test Usage

```rust
#[tokio::test]
async fn test_usdc_transfer() {
    let client = setup_test_client().await;
    
    // Test transfer functionality
    let from = Address::from_str("0x...").unwrap();
    let to = Address::from_str("0x...").unwrap();
    let amount = U256::from(1000000); // 1 USDC (6 decimals)
    
    let tx_hash = client.transfer(to, amount).await.unwrap();
    
    // Wait for transaction confirmation
    let receipt = client.ethereum_client.wait_for_transaction(tx_hash).await.unwrap();
    assert_eq!(receipt.status, Some(1.into()));
    
    // Verify balance updated
    let balance = client.balance_of(to).await.unwrap();
    assert!(balance >= amount);
}

#[tokio::test]
async fn test_event_parsing() {
    let client = setup_test_client().await;
    
    // Get transfer events from a block
    let logs = client.ethereum_client.get_logs(
        Filter::new()
            .address(client.contract_address)
            .topic0(TransferEvent::signature())
            .from_block(18000000)
            .to_block(18000100)
    ).await.unwrap();
    
    let events = client.parse_transfer_events(logs).await.unwrap();
    assert!(!events.is_empty());
    
    for event in events {
        assert!(!event.from.is_zero() || !event.to.is_zero());
        assert!(event.value > U256::ZERO);
    }
}
```

## Performance Considerations

### Benchmarking

The system includes built-in performance benchmarks:
- ABI parsing performance
- Code generation speed
- Memory usage tracking
- Scaling characteristics

### Optimization Tips

1. **Selective Feature Generation**: Only generate needed features
2. **Event Filtering**: Use indexed parameters for efficient event queries
3. **Batch Processing**: Group multiple contract calls together
4. **Caching**: Enable client-side caching for frequently accessed data

## Advanced Usage

### Multi-Contract Integration

Handle multiple related contracts:

```rust
pub struct DefiProtocolClient {
    token_client: UsdcClient,
    pool_client: UniswapV3PoolClient,
    router_client: UniswapV3RouterClient,
}

impl DefiProtocolClient {
    pub async fn get_pool_price(&self) -> Result<U256> {
        let slot0 = self.pool_client.slot0().await?;
        Ok(slot0.sqrt_price_x96)
    }
    
    pub async fn execute_swap(
        &self,
        amount_in: U256,
        amount_out_minimum: U256,
    ) -> Result<TxHash> {
        // Approve token spending
        self.token_client.approve(
            self.router_client.contract_address,
            amount_in
        ).await?;
        
        // Execute swap
        self.router_client.exact_input_single(SwapParams {
            token_in: self.token_client.contract_address,
            token_out: Address::from_str("0x...")?,
            fee: 3000,
            recipient: self.get_wallet_address(),
            deadline: self.get_deadline(),
            amount_in,
            amount_out_minimum,
            sqrt_price_limit_x96: U256::ZERO,
        }).await
    }
}
```

### Custom Event Processing

```rust
pub struct EventProcessor {
    storage: ContractStorage,
}

impl EventProcessor {
    pub async fn process_transfer_event(&self, event: TransferEvent) -> Result<()> {
        // Update sender balance
        self.update_balance(event.from, event.block_number).await?;
        
        // Update receiver balance
        self.update_balance(event.to, event.block_number).await?;
        
        // Store transfer record
        self.storage.insert_transfer(&event).await?;
        
        // Trigger notifications for large transfers
        if event.value > U256::from(1_000_000_000_000u64) { // > 1M tokens
            self.notify_large_transfer(&event).await?;
        }
        
        Ok(())
    }
    
    async fn update_balance(&self, address: Address, block_number: u64) -> Result<()> {
        let balance = self.token_client.balance_of(address).await?;
        self.storage.upsert_balance(address, balance, block_number).await
    }
}
```

### Contract Deployment

```rust
impl UsdcClient {
    pub async fn deploy(
        ethereum_client: EthereumClient,
        name: String,
        symbol: String,
        decimals: u8,
        initial_supply: U256,
    ) -> Result<(Address, Self)> {
        let constructor_data = encode_constructor(&[
            Token::String(name),
            Token::String(symbol),
            Token::Uint(decimals.into()),
            Token::Uint(initial_supply),
        ])?;
        
        let deployment_tx = ethereum_client.deploy_contract(
            include_bytes!("../bytecode/usdc.bin"),
            &constructor_data,
        ).await?;
        
        let receipt = ethereum_client.wait_for_transaction(deployment_tx).await?;
        let contract_address = receipt.contract_address
            .ok_or_else(|| Error::Generic("No contract address in receipt".into()))?;
        
        let client = Self::new(ethereum_client, contract_address);
        Ok((contract_address, client))
    }
}
```

## Error Handling

### Common Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum EthereumError {
    #[error("Contract call failed: {0}")]
    ContractCall(String),
    
    #[error("Transaction failed: {0}")]
    Transaction(String),
    
    #[error("ABI encoding error: {0}")]
    AbiEncoding(String),
    
    #[error("Event parsing error: {0}")]
    EventParsing(String),
    
    #[error("Network error: {0}")]
    Network(String),
}
```

### Retry Logic

```rust
impl UsdcClient {
    async fn call_with_retry<T>(&self, f: impl Fn() -> Future<Output = Result<T>>) -> Result<T> {
        let mut attempts = 0;
        let max_attempts = 3;
        
        while attempts < max_attempts {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if attempts < max_attempts - 1 => {
                    tokio::time::sleep(Duration::from_millis(1000 * 2_u64.pow(attempts))).await;
                    attempts += 1;
                }
                Err(e) => return Err(e),
            }
        }
        
        unreachable!()
    }
}
```

## Best Practices

### Gas Optimization

1. **Batch Operations**: Group multiple calls into single transaction
2. **Gas Price Estimation**: Use dynamic gas pricing
3. **Transaction Nonce Management**: Handle nonce conflicts properly

### Security Considerations

1. **Input Validation**: Validate all function parameters
2. **Access Control**: Implement proper authorization
3. **Slippage Protection**: Use appropriate slippage tolerances for DEX operations

### Monitoring and Alerting

```rust
pub struct ContractMonitor {
    client: UsdcClient,
    alerts: AlertManager,
}

impl ContractMonitor {
    pub async fn monitor_large_transfers(&self) -> Result<()> {
        let threshold = U256::from(10_000_000_000_000u64); // 10M tokens
        
        let filter = Filter::new()
            .address(self.client.contract_address)
            .topic0(TransferEvent::signature());
        
        let mut stream = self.client.ethereum_client.subscribe_logs(filter).await?;
        
        while let Some(log) = stream.next().await {
            if let Ok(event) = TransferEvent::from_log(&log) {
                if event.value > threshold {
                    self.alerts.send_alert(&format!(
                        "Large transfer: {} tokens from {} to {}",
                        event.value,
                        event.from,
                        event.to
                    )).await?;
                }
            }
        }
        
        Ok(())
    }
}
```

## Troubleshooting

### Common Issues

1. **ABI Parsing Errors**: Ensure ABI is valid JSON format
2. **Gas Estimation Failures**: Check network congestion and gas limits
3. **Event Parsing Issues**: Verify event signatures match ABI
4. **Connection Problems**: Check RPC endpoint and network connectivity

### Debug Mode

Enable debug logging for detailed information:

```bash
export RUST_LOG=debug
almanac ethereum generate-contract usdc_abi.json \
  --address 0xa0b86a33e6dc39c9c6c7c7ccf9c2e9c5c2c8c0 \
  --chain 1 \
  --verbose
```

## Support and Community

- **Documentation**: https://docs.almanac.com/ethereum-codegen
- **Examples**: https://github.com/timewave-computer/almanac/tree/main/examples/ethereum
- **Issues**: https://github.com/timewave-computer/almanac/issues
- **Discord**: https://discord.gg/almanac 