# Almanac CLI Reference

This document provides a complete command-line interface reference for the Almanac indexer, covering contract code generation, service management, and blockchain operations.

## Installation

Install the Almanac CLI with full blockchain support:

```bash
# Install from source
git clone https://github.com/timewave-computer/almanac.git
cd almanac
cargo install --path . --features ethereum,cosmos

# Or use pre-built binary
wget https://releases.almanac.com/latest/almanac-linux-x64
chmod +x almanac-linux-x64
sudo mv almanac-linux-x64 /usr/local/bin/almanac
```

## Global Options

These options are available for all commands:

- `--verbose, -v`: Enable verbose output
- `--quiet, -q`: Suppress non-error output  
- `--config <FILE>`: Use custom configuration file
- `--help, -h`: Display help information
- `--version, -V`: Display version information

## Service Management

### Start Indexer Service

```bash
almanac start [OPTIONS]
```

**Options:**
- `--config <FILE>`: Configuration file path (default: `config.toml`)
- `--env <ENV>`: Environment to run in (development, staging, production, test)

**Examples:**
```bash
# Start with default configuration
almanac start

# Start with specific environment
almanac start --env production --config prod_config.toml
```

### Service Control

```bash
# Stop the indexer service
almanac stop

# Check indexer status
almanac status

# Validate configuration
almanac validate-config --config config.toml
```

### Database Management

```bash
# Initialize database schema
almanac init-db --database-url postgresql://localhost/indexer

# Run database migrations
almanac db migrate

# Reset database (WARNING: Deletes all data)
almanac db reset
```

## Contract Code Generation

### Ethereum Contracts

Generate Rust code for Ethereum contracts from ABI files:

```bash
almanac ethereum generate-contract <ABI_FILE> [OPTIONS]
```

**Required Arguments:**
- `<ABI_FILE>`: Path to the contract ABI JSON file

**Required Options:**
- `--address <ADDRESS>`: Contract address (0x followed by 40 hex characters)
- `--chain <CHAIN_ID>`: Blockchain chain ID (1=Ethereum, 137=Polygon, etc.)

**Optional Parameters:**
- `--output-dir <DIR>`: Output directory for generated code (default: `./generated`)
- `--namespace <NAME>`: Namespace for generated modules
- `--features <LIST>`: Features to generate (client, storage, api, migrations)
- `--dry-run`: Preview generation without creating files
- `--overwrite`: Overwrite existing files without confirmation

**Examples:**

Basic generation:
```bash
almanac ethereum generate-contract usdc_abi.json \
  --address 0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0 \
  --chain 1
```

Full feature generation:
```bash
almanac ethereum generate-contract uniswap_v3_pool.json \
  --address 0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8 \
  --chain 1 \
  --features client,storage,api,migrations \
  --output-dir ./contracts/uniswap \
  --namespace uniswap_pool
```

Dry run preview:
```bash
almanac ethereum generate-contract token.json \
  --address 0x123... \
  --chain 5 \
  --dry-run
```

### Cosmos Contracts

Generate code for CosmWasm contracts from schema files:

```bash
almanac cosmos generate-contract <SCHEMA_FILE> [OPTIONS]
```

**Required Arguments:**
- `<SCHEMA_FILE>`: Path to the CosmWasm message schema JSON file

**Required Options:**
- `--address <CONTRACT_ADDRESS>`: The contract address on the blockchain
- `--chain <CHAIN_ID>`: The chain ID where the contract is deployed

**Optional Parameters:**
- `--output-dir <DIRECTORY>`: Output directory (default: `./generated`)
- `--namespace <NAME>`: Namespace for generated modules
- `--features <FEATURE_LIST>`: Features to generate (client, storage, api, migrations)
- `--dry-run`: Preview generation without creating files

**Examples:**

Basic usage:
```bash
almanac cosmos generate-contract schema.json \
  --address cosmos1abc123... \
  --chain cosmoshub-4
```

Generate specific features:
```bash
almanac cosmos generate-contract valence_base_account_schema.json \
  --address cosmos1valencebaseaccountexample... \
  --chain cosmoshub-4 \
  --features client,storage,api \
  --output-dir ./contracts/valence_base_account
```

Custom namespace:
```bash
almanac cosmos generate-contract cw721_schema.json \
  --address cosmos1nft... \
  --chain stargaze-1 \
  --namespace my_nft_collection \
  --features client,storage,api,migrations
```

## Validation Commands

### Validate ABI Files (Ethereum)

```bash
almanac ethereum validate-abi <ABI_FILE> [OPTIONS]
```

**Options:**
- `--strict`: Enable strict validation mode
- `--output-format <FORMAT>`: Output format (json, yaml, table)
- `--check-completeness`: Verify ABI contains standard functions

**Example:**
```bash
almanac ethereum validate-abi erc20.json --strict --output-format json
```

### Validate Schema Files (Cosmos)

```bash
almanac cosmos validate-schema <SCHEMA_FILE> [OPTIONS]
```

**Options:**
- `--strict`: Enable strict validation mode
- `--output-format <FORMAT>`: Output format (text, json)

**Example:**
```bash
almanac cosmos validate-schema cw20_schema.json --strict
```

## Utility Commands

### Fetch Contract ABI (Ethereum)

Download contract ABI from blockchain explorers:

```bash
almanac ethereum fetch-abi <ADDRESS> [OPTIONS]
```

**Options:**
- `--chain <CHAIN_ID>`: Blockchain to fetch from
- `--output <FILE>`: Save ABI to file
- `--explorer <EXPLORER>`: Explorer to use (etherscan, polygonscan, bscscan)
- `--api-key <KEY>`: Explorer API key

**Example:**
```bash
almanac ethereum fetch-abi 0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0 \
  --chain 1 \
  --output usdc_abi.json \
  --api-key YOUR_ETHERSCAN_KEY
```

### List Templates

View available code generation templates:

```bash
# List Ethereum templates
almanac ethereum list-templates [--detailed] [--format json]

# List Cosmos templates  
almanac cosmos list-templates [--category client] [--format table]
```

### Initialize Configuration

Create configuration files for projects:

```bash
# Initialize Ethereum configuration
almanac ethereum init-config [--interactive] [--template defi]

# Initialize Cosmos configuration
almanac cosmos init-config [--output config.toml] [--template minimal]
```

## Chain Management

### Add Blockchain Networks

```bash
# Add Ethereum network
almanac chains add ethereum \
  --rpc-url https://mainnet.infura.io/v3/YOUR_API_KEY \
  --chain-id 1

# Add Cosmos network
almanac chains add cosmos \
  --grpc-url https://rpc.cosmos.network:443 \
  --chain-id cosmoshub-4
```

### Monitor Chain Status

```bash
# List all configured chains
almanac chains list

# View specific chain status
almanac chains status ethereum
almanac chains status cosmos

# Pause/resume chain indexing
almanac chains pause ethereum
almanac chains resume ethereum
```

## Event Querying

### Query by Address

```bash
almanac events address <CHAIN> <ADDRESS> [OPTIONS]
```

**Options:**
- `--limit <N>`: Maximum number of events (default: 100)
- `--from-block <N>`: Start block number
- `--to-block <N>`: End block number
- `--event-type <TYPE>`: Filter by event type

**Examples:**
```bash
# Query Ethereum contract events
almanac events address ethereum 0x1234567890abcdef1234567890abcdef12345678 \
  --limit 10 --event-type Transfer

# Query Cosmos contract events
almanac events address cosmos cosmos1abc123... \
  --from-block 15000000 --limit 50
```

### Query by Chain

```bash
almanac events chain <CHAIN> [OPTIONS]
```

**Examples:**
```bash
# Query Ethereum events in block range
almanac events chain ethereum \
  --from-block 18000000 \
  --to-block 18001000 \
  --limit 100

# Query recent Cosmos events
almanac events chain cosmos \
  --limit 50 \
  --event-type wasm
```

### Custom Event Filters

```bash
almanac events filter [OPTIONS]
```

**Options:**
- `--chain <CHAIN>`: Blockchain network
- `--type <TYPE>`: Event type filter
- `--contract <ADDRESS>`: Contract address filter
- `--from-block <N>`: Start block
- `--to-block <N>`: End block
- `--limit <N>`: Maximum results

**Example:**
```bash
almanac events filter \
  --chain ethereum \
  --type Transfer \
  --from-block 18000000 \
  --limit 20
```

### Export Events

```bash
almanac events export [OPTIONS]
```

**Options:**
- `--chain <CHAIN>`: Blockchain network
- `--output <FILE>`: Output file path
- `--format <FORMAT>`: Export format (json, csv)
- `--from-block <N>`: Start block
- `--to-block <N>`: End block

**Example:**
```bash
almanac events export \
  --chain ethereum \
  --from-block 18000000 \
  --output events.json \
  --format json
```

## Configuration

### Global Configuration

Create configuration at `~/.config/almanac/config.toml`:

```toml
[database]
url = "postgresql://localhost/almanac"

[ethereum]
provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
chain_id = 1

[cosmos]
grpc_url = "https://rpc.cosmos.network:443"
chain_id = "cosmoshub-4"

[api]
host = "0.0.0.0"
port = 8080

[logging]
level = "info"
format = "json"
```

### Project Configuration

Create project-specific configuration files:

```toml
# ethereum_codegen.toml
[ethereum]
provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
output_dir = "./generated"
default_features = ["client", "storage"]

[[ethereum.contracts]]
name = "usdc"
address = "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0"
abi_path = "./abis/usdc.json"
features = ["client", "storage", "api"]
```

## Environment Variables

Override configuration with environment variables:

### Database Configuration
- `DATABASE_URL`: PostgreSQL connection string
- `ROCKSDB_PATH`: RocksDB data directory

### Network Configuration
- `ETHEREUM_PROVIDER_URL`: Ethereum node provider URL
- `ETHEREUM_CHAIN_ID`: Default Ethereum chain ID
- `COSMOS_GRPC_URL`: Cosmos gRPC endpoint
- `COSMOS_CHAIN_ID`: Default Cosmos chain ID

### API Configuration
- `API_HOST`: API server host
- `API_PORT`: API server port

### Explorer Configuration
- `ETHERSCAN_API_KEY`: Etherscan API key
- `POLYGONSCAN_API_KEY`: PolygonScan API key

### Development Configuration
- `RUST_LOG`: Rust logging configuration
- `ALMANAC_VERBOSE`: Enable verbose logging
- `ALMANAC_DRY_RUN`: Default to dry-run mode

## Advanced Usage

### Batch Contract Generation

Process multiple contracts using configuration files:

```bash
# Generate all contracts from config file
almanac generate-contracts --config multi_contract_config.toml

# Or use shell script
for contract in usdc weth dai; do
  almanac ethereum generate-contract "./abis/${contract}.json" \
    --address "$(cat ./addresses/${contract}.txt)" \
    --chain 1 \
    --output-dir "./generated/${contract}"
done
```

### Integration with Build Systems

Add to `build.rs` for automatic code generation:

```rust
use std::process::Command;

fn main() {
    let output = Command::new("almanac")
        .args(&[
            "ethereum", "generate-contract",
            "abis/usdc.json",
            "--address", "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0",
            "--chain", "1",
            "--output-dir", "src/generated",
            "--features", "client"
        ])
        .output()
        .expect("Failed to run almanac");

    if !output.status.success() {
        panic!("Code generation failed");
    }
}
```

### Makefile Integration

```makefile
.PHONY: generate-contracts
generate-contracts:
	@for contract in ethereum/*.json; do \
		almanac ethereum generate-contract $$contract \
			--address $$(cat addresses/$$(basename $$contract .json).txt) \
			--chain 1 \
			--output-dir generated/$$(basename $$contract .json); \
	done
	@for contract in cosmos/*.json; do \
		almanac cosmos generate-contract $$contract \
			--address $$(cat addresses/$$(basename $$contract .json).txt) \
			--chain cosmoshub-4 \
			--output-dir generated/$$(basename $$contract .json); \
	done
```

## Troubleshooting

### Common Issues

**Permission Errors:**
```bash
# Ensure write permissions
chmod -R u+w ./generated

# Or specify different output directory
almanac ethereum generate-contract contract.json \
  --address 0x123... \
  --chain 1 \
  --output-dir ~/my-contracts
```

**Network Errors:**
```bash
# Use local files instead of remote fetching
almanac ethereum generate-contract ./local_abi.json \
  --address 0x123... \
  --chain 1

# Or configure proxy
export HTTP_PROXY=http://proxy.example.com:8080
```

**Validation Errors:**
```bash
# Check file format
almanac ethereum validate-abi contract.json --strict

# Common fixes:
# - Ensure JSON is valid
# - Check required fields are present
# - Verify contract address format
```

### Known Issues

⚠️ **CURRENT LIMITATIONS** ⚠️

- **Configuration Validation**: Some configuration validation may be incomplete
- **Error Messages**: Error messages may not always be user-friendly
- **Auto-completion**: CLI auto-completion may not work for all shells
- **Service Management**: Service restart functionality may be limited
- **Database Commands**: Some database management commands may require manual intervention

### Debug Mode

Enable debug logging for detailed information:

```bash
export RUST_LOG=debug
almanac ethereum generate-contract contract.json \
  --address 0x123... \
  --chain 1 \
  --verbose
```

## Exit Codes

- `0`: Success
- `1`: General error
- `2`: Invalid arguments or configuration
- `3`: File validation failed
- `4`: File system error (permissions, disk space)
- `5`: Network error
- `6`: Template or generation error

## Getting Help

- Use `--help` with any command for detailed usage information
- Check project documentation: https://docs.almanac.com
- Report issues: https://github.com/timewave-computer/almanac/issues
- Join the community Discord for support and discussions

This CLI reference provides comprehensive guidance for all Almanac operations, from basic service management to advanced contract integration across both Ethereum and Cosmos ecosystems. 