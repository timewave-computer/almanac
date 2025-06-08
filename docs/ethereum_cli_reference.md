# Ethereum CLI Reference

This document provides a complete command-line interface reference for the Ethereum contract code generation system.

## Installation

Ensure the Almanac indexer with Ethereum support is installed:

```bash
# Install from source
git clone https://github.com/timewave-computer/almanac.git
cd almanac
cargo install --path . --features ethereum

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

## Core Commands

### `almanac ethereum generate-contract`

Generate Rust code for interacting with an Ethereum contract.

#### Syntax

```bash
almanac ethereum generate-contract <ABI_FILE> [OPTIONS]
```

#### Arguments

- `<ABI_FILE>`: Path to the contract ABI JSON file (required)

#### Options

##### Contract Information
- `--address <ADDRESS>`: Contract address on blockchain (required)
- `--chain <CHAIN_ID>`: Blockchain chain ID (required)
  - `1`: Ethereum Mainnet
  - `5`: Goerli Testnet  
  - `11155111`: Sepolia Testnet
  - `137`: Polygon Mainnet
  - `80001`: Polygon Mumbai Testnet

##### Output Configuration
- `--output-dir <DIR>`: Output directory for generated code (default: `./generated`)
- `--namespace <NAME>`: Namespace for generated modules (default: contract name)
- `--overwrite`: Overwrite existing files without confirmation

##### Feature Selection
- `--features <LIST>`: Comma-separated list of features to generate
  - `client`: Contract interaction client code
  - `storage`: Database storage schemas and models
  - `api`: REST and GraphQL API endpoints
  - `migrations`: Database migration files
  - `tests`: Test templates and utilities
  - `docs`: Documentation generation
- `--exclude-features <LIST>`: Features to exclude from generation

##### Code Generation Options
- `--template-dir <DIR>`: Custom template directory
- `--custom-types <FILE>`: Custom type mappings configuration
- `--gas-limit <AMOUNT>`: Default gas limit for transactions
- `--gas-price <PRICE>`: Default gas price in gwei

##### Database Options
- `--db-schema <SCHEMA>`: PostgreSQL schema name (default: `public`)
- `--table-prefix <PREFIX>`: Prefix for generated table names
- `--migration-dir <DIR>`: Migration files directory (default: `./migrations`)

##### Network Options
- `--provider-url <URL>`: Ethereum provider URL (can be set via env var)
- `--timeout <SECONDS>`: Network request timeout (default: 30)
- `--retries <COUNT>`: Number of retry attempts (default: 3)

##### Advanced Options
- `--dry-run`: Preview generation without creating files
- `--format <FORMAT>`: Output format for dry-run (`json`, `yaml`, `table`)
- `--validate-only`: Only validate ABI without generating code
- `--optimization-level <LEVEL>`: Code optimization level (0-3)

#### Examples

##### Basic Generation

```bash
# Generate client code for USDC contract
almanac ethereum generate-contract usdc_abi.json \
  --address 0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0 \
  --chain 1
```

##### Full Feature Generation

```bash
# Generate all features with custom configuration
almanac ethereum generate-contract uniswap_v3_pool.json \
  --address 0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8 \
  --chain 1 \
  --features client,storage,api,migrations \
  --output-dir ./contracts/uniswap \
  --namespace uniswap_pool \
  --db-schema uniswap \
  --table-prefix pool_
```

##### Custom Templates

```bash
# Use custom templates for specialized code generation
almanac ethereum generate-contract erc721.json \
  --address 0x123... \
  --chain 1 \
  --template-dir ./templates/nft \
  --custom-types ./config/nft-types.json
```

##### Development/Testing

```bash
# Preview generation without creating files
almanac ethereum generate-contract token.json \
  --address 0x123... \
  --chain 5 \
  --dry-run \
  --format json

# Generate with test features for development
almanac ethereum generate-contract token.json \
  --address 0x123... \
  --chain 5 \
  --features client,tests \
  --output-dir ./dev-contracts
```

### `almanac ethereum validate-abi`

Validate an Ethereum contract ABI file.

#### Syntax

```bash
almanac ethereum validate-abi <ABI_FILE> [OPTIONS]
```

#### Options

- `--strict`: Enable strict validation mode
- `--output-format <FORMAT>`: Output format (`json`, `yaml`, `table`)
- `--check-completeness`: Verify ABI contains all standard functions

#### Examples

```bash
# Basic validation
almanac ethereum validate-abi erc20.json

# Strict validation with detailed output
almanac ethereum validate-abi erc20.json --strict --output-format json

# Check for ERC20 standard completeness
almanac ethereum validate-abi erc20.json --check-completeness
```

### `almanac ethereum init-config`

Initialize a new Ethereum codegen configuration file.

#### Syntax

```bash
almanac ethereum init-config [OPTIONS]
```

#### Options

- `--interactive, -i`: Interactive configuration setup
- `--template <TEMPLATE>`: Use predefined template
  - `basic`: Basic single-contract setup
  - `defi`: DeFi protocol with multiple contracts  
  - `nft`: NFT collection setup
- `--output <FILE>`: Configuration file path (default: `ethereum_codegen.toml`)

#### Examples

```bash
# Interactive setup
almanac ethereum init-config --interactive

# Create DeFi template configuration
almanac ethereum init-config --template defi --output defi_config.toml

# Basic configuration
almanac ethereum init-config
```

### `almanac ethereum list-templates`

List available code generation templates.

#### Syntax

```bash
almanac ethereum list-templates [OPTIONS]
```

#### Options

- `--format <FORMAT>`: Output format (`table`, `json`, `yaml`)
- `--detailed`: Show detailed template information
- `--category <CATEGORY>`: Filter by template category

#### Examples

```bash
# List all templates
almanac ethereum list-templates

# Detailed template information
almanac ethereum list-templates --detailed --format json

# Filter by category
almanac ethereum list-templates --category erc20
```

### `almanac ethereum fetch-abi`

Fetch contract ABI from blockchain explorers.

#### Syntax

```bash
almanac ethereum fetch-abi <ADDRESS> [OPTIONS]
```

#### Options

- `--chain <CHAIN_ID>`: Blockchain to fetch from
- `--output <FILE>`: Save ABI to file
- `--explorer <EXPLORER>`: Blockchain explorer to use
  - `etherscan`: Etherscan.io (default)
  - `polygonscan`: PolygonScan
  - `bscscan`: BscScan
- `--api-key <KEY>`: Explorer API key

#### Examples

```bash
# Fetch USDC ABI from Etherscan
almanac ethereum fetch-abi 0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0 \
  --chain 1 \
  --output usdc_abi.json

# Fetch from Polygon with API key
almanac ethereum fetch-abi 0x123... \
  --chain 137 \
  --explorer polygonscan \
  --api-key YOUR_API_KEY \
  --output polygon_token.json
```

## Configuration Files

### Global Configuration

Location: `~/.config/almanac/ethereum.toml` or set via `--config`

```toml
[ethereum]
# Default provider URL
provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"

# Default output directory
output_dir = "./generated"

# Default features to generate
default_features = ["client", "storage"]

# Template directory
template_dir = "~/.config/almanac/templates"

[ethereum.database]
# Default database configuration
default_url = "postgresql://localhost/ethereum_indexer"
schema = "public"
table_prefix = "contract_"

[ethereum.api]
# Default API configuration
base_path = "/api/v1/ethereum"
enable_cors = true
enable_websockets = true

[ethereum.explorers]
# API keys for blockchain explorers
etherscan_api_key = "YOUR_ETHERSCAN_API_KEY"
polygonscan_api_key = "YOUR_POLYGONSCAN_API_KEY"
```

### Project Configuration

File: `ethereum_codegen.toml` (created with `init-config`)

```toml
[ethereum]
provider_url = "https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
chain_id = 1
output_dir = "./generated"

# Template configuration
[ethereum.templates]
client_template = "ethereum_client.hbs"
storage_template = "ethereum_storage.hbs"
api_template = "ethereum_api.hbs"

# Database configuration
[ethereum.database]
url = "postgresql://localhost/my_ethereum_indexer"
schema = "contracts"
table_prefix = "eth_"
migration_dir = "./migrations"

# API configuration
[ethereum.api]
host = "0.0.0.0"
port = 3000
base_path = "/api/v1"
enable_cors = true
enable_graphql = true
enable_websockets = true

# Contract definitions
[[ethereum.contracts]]
name = "usdc"
address = "0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0"
abi_path = "./abis/usdc.json"
features = ["client", "storage", "api"]
start_block = 18000000
gas_limit = 100000

[[ethereum.contracts]]
name = "weth"
address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
abi_path = "./abis/weth.json"
features = ["client", "storage"]
start_block = 18000000

# Generation options
[ethereum.generation]
optimization_level = 2
include_docs = true
include_tests = true
namespace_style = "snake_case"  # or "pascal_case"
```

### Custom Type Mappings

File: `custom_types.json`

```json
{
  "type_mappings": {
    "uint256": "U256",
    "int256": "I256", 
    "address": "Address",
    "bytes32": "FixedBytes<32>",
    "bytes": "Bytes",
    "string": "String",
    "bool": "bool"
  },
  "custom_types": {
    "TokenAmount": {
      "type": "uint256",
      "decimals": 18,
      "display_format": "decimal"
    },
    "UsdcAmount": {
      "type": "uint256", 
      "decimals": 6,
      "display_format": "decimal"
    }
  },
  "struct_mappings": {
    "SwapParams": {
      "tokenIn": "Address",
      "tokenOut": "Address", 
      "fee": "u32",
      "recipient": "Address",
      "deadline": "u64",
      "amountIn": "TokenAmount",
      "amountOutMinimum": "TokenAmount",
      "sqrtPriceLimitX96": "U256"
    }
  }
}
```

## Environment Variables

The following environment variables can be used to override configuration:

### Network Configuration
- `ETHEREUM_PROVIDER_URL`: Ethereum node provider URL
- `ETHEREUM_CHAIN_ID`: Default chain ID
- `ETHEREUM_GAS_LIMIT`: Default gas limit
- `ETHEREUM_GAS_PRICE`: Default gas price in gwei

### Database Configuration  
- `DATABASE_URL`: PostgreSQL connection string
- `ETHEREUM_DB_SCHEMA`: Database schema name
- `ETHEREUM_TABLE_PREFIX`: Table name prefix

### API Configuration
- `ETHEREUM_API_HOST`: API server host
- `ETHEREUM_API_PORT`: API server port
- `ETHEREUM_API_BASE_PATH`: API base path

### Output Configuration
- `ETHEREUM_OUTPUT_DIR`: Output directory for generated code
- `ETHEREUM_TEMPLATE_DIR`: Custom template directory
- `ETHEREUM_NAMESPACE`: Default namespace for generated code

### Explorer Configuration
- `ETHERSCAN_API_KEY`: Etherscan API key
- `POLYGONSCAN_API_KEY`: PolygonScan API key
- `BSCSCAN_API_KEY`: BscScan API key

### Development/Debug
- `ETHEREUM_CODEGEN_VERBOSE`: Enable verbose logging
- `ETHEREUM_CODEGEN_DRY_RUN`: Default to dry-run mode
- `RUST_LOG`: Rust logging configuration

## Advanced Usage Patterns

### Batch Contract Generation

Generate code for multiple contracts:

```bash
# Using configuration file
almanac ethereum generate-contracts --config ./multi_contract_config.toml

# Using shell script for batch processing
for contract in usdc weth dai; do
  almanac ethereum generate-contract "./abis/${contract}.json" \
    --address "$(cat ./addresses/${contract}.txt)" \
    --chain 1 \
    --output-dir "./generated/${contract}" \
    --namespace "${contract}"
done
```

### Custom Template Development

Create custom templates for specialized use cases:

```bash
# List available template helpers
almanac ethereum list-templates --detailed

# Generate with custom template
almanac ethereum generate-contract contract.json \
  --address 0x123... \
  --chain 1 \
  --template-dir ./my-templates \
  --features client
```

### Integration with Build Systems

#### Cargo Build Script

Add to `build.rs`:

```rust
use std::process::Command;

fn main() {
    // Generate contract code during build
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
        .expect("Failed to generate contract code");

    if !output.status.success() {
        panic!("Contract generation failed: {}", 
               String::from_utf8_lossy(&output.stderr));
    }

    // Tell Cargo to rerun if ABI changes
    println!("cargo:rerun-if-changed=abis/usdc.json");
}
```

#### Makefile Integration

```makefile
# Generate all contract code
.PHONY: generate-contracts
generate-contracts:
	almanac ethereum generate-contract abis/usdc.json \
		--address 0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0 \
		--chain 1 \
		--output-dir src/generated/usdc \
		--features client,storage,api
	almanac ethereum generate-contract abis/weth.json \
		--address 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 \
		--chain 1 \
		--output-dir src/generated/weth \
		--features client,storage

# Clean generated code
.PHONY: clean-generated
clean-generated:
	rm -rf src/generated/*

# Validate all ABIs
.PHONY: validate-abis
validate-abis:
	for abi in abis/*.json; do \
		almanac ethereum validate-abi "$$abi" --strict; \
	done
```

### CI/CD Integration

#### GitHub Actions

`.github/workflows/contracts.yml`:

```yaml
name: Contract Code Generation

on:
  push:
    paths:
      - 'abis/**'
      - 'contracts/**'

jobs:
  generate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Almanac
        run: |
          wget https://releases.almanac.com/latest/almanac-linux-x64
          chmod +x almanac-linux-x64
          sudo mv almanac-linux-x64 /usr/local/bin/almanac
      
      - name: Generate Contract Code
        run: |
          almanac ethereum generate-contracts --config ethereum_codegen.toml
        env:
          ETHEREUM_PROVIDER_URL: ${{ secrets.ETHEREUM_PROVIDER_URL }}
      
      - name: Check for Changes
        run: |
          if [[ -n $(git status --porcelain) ]]; then
            echo "Generated code has changes"
            git diff
            exit 1
          fi
```

### Testing Integration

Generate test code and run contract tests:

```bash
# Generate with test features
almanac ethereum generate-contract usdc.json \
  --address 0xA0b86a33E6dc39C9c6D7C7CcF9C2e5C2c8C0b0 \
  --chain 1 \
  --features client,tests \
  --output-dir ./tests/generated

# Run generated tests
cargo test --test contract_tests
```

## Troubleshooting

### Common Issues and Solutions

#### ABI Validation Errors

```bash
# Check ABI format
almanac ethereum validate-abi contract.json --strict

# Common issues:
# - Invalid JSON syntax
# - Missing required fields (type, name, inputs, outputs)
# - Malformed function signatures
```

#### Network Connection Issues

```bash
# Test provider connectivity
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  https://mainnet.infura.io/v3/YOUR_PROJECT_ID

# Check environment variables
echo $ETHEREUM_PROVIDER_URL
```

#### Generation Failures

```bash
# Enable verbose output for debugging
almanac ethereum generate-contract contract.json \
  --address 0x123... \
  --chain 1 \
  --verbose

# Validate configuration
almanac ethereum validate-config ethereum_codegen.toml

# Check permissions
ls -la ./generated/
```

#### Template Issues

```bash
# List available templates
almanac ethereum list-templates --detailed

# Test custom templates
almanac ethereum generate-contract contract.json \
  --address 0x123... \
  --chain 1 \
  --template-dir ./templates \
  --dry-run
```

### Debug Mode

Enable comprehensive debugging:

```bash
# Set debug environment
export RUST_LOG=debug
export ETHEREUM_CODEGEN_VERBOSE=true

# Run with maximum verbosity
almanac ethereum generate-contract contract.json \
  --address 0x123... \
  --chain 1 \
  --verbose \
  --dry-run \
  --format json > debug_output.json
```

### Getting Help

#### Built-in Help

```bash
# General help
almanac ethereum --help

# Command-specific help
almanac ethereum generate-contract --help

# List all available commands
almanac ethereum --help
```

#### Version Information

```bash
# Check version and build info
almanac --version

# Check feature support
almanac ethereum --version
```

#### Log Analysis

```bash
# Enable structured logging
export RUST_LOG=almanac_ethereum=trace

# Save logs to file
almanac ethereum generate-contract contract.json \
  --address 0x123... \
  --chain 1 \
  --verbose 2>&1 | tee generation.log
```

## Exit Codes

The CLI uses the following exit codes:

- `0`: Success
- `1`: General error
- `2`: Invalid arguments or configuration
- `3`: Network/connectivity error
- `4`: ABI parsing error
- `5`: File system error (permissions, disk space)
- `6`: Template error
- `7`: Code generation error
- `8`: Database error

## Version Compatibility

| CLI Version | Supported Rust | Supported ABIs | Notes |
|-------------|----------------|----------------|--------|
| 0.1.x       | 1.70+         | Solidity 0.8.x | Initial release |
| 0.2.x       | 1.72+         | Solidity 0.8.x | Added GraphQL support |
| 0.3.x       | 1.75+         | Solidity 0.8.x | Custom templates |
| 1.0.x       | 1.75+         | Solidity 0.8.x | Stable release |

## Migration Guide

### From 0.2.x to 0.3.x

```bash
# Update configuration format
almanac ethereum migrate-config --from 0.2 --to 0.3

# Regenerate with new templates
almanac ethereum generate-contract contract.json \
  --address 0x123... \
  --chain 1 \
  --features client,storage,api \
  --overwrite
```

This comprehensive CLI reference provides all the information needed to effectively use the Ethereum contract code generation system. 