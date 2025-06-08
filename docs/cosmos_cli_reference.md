# Cosmos CLI Reference

This document provides a complete reference for the Cosmos contract code generation CLI commands.

## Overview

The Cosmos CLI is part of the Almanac indexer and provides commands for generating contract integration code from CosmWasm schema files.

## Installation

The CLI is included with the Almanac indexer installation:

```bash
# Install Almanac (includes cosmos CLI)
cargo install almanac-indexer

# Or build from source
git clone https://github.com/timewave-computer/almanac
cd almanac
cargo build --release
```

## Global Options

These options are available for all commands:

- `--help, -h`: Show help information
- `--version, -V`: Show version information
- `--verbose, -v`: Enable verbose output
- `--quiet, -q`: Suppress output except errors
- `--config <FILE>`: Use custom configuration file

## Commands

### `cosmos generate-contract`

Generate integration code for a CosmWasm contract from its message schema.

#### Synopsis

```bash
almanac cosmos generate-contract <SCHEMA_FILE> [OPTIONS]
```

#### Arguments

- `<SCHEMA_FILE>`: Path to the CosmWasm message schema JSON file

#### Required Options

- `--address <CONTRACT_ADDRESS>`: The contract address on the blockchain
- `--chain <CHAIN_ID>`: The chain ID where the contract is deployed

#### Optional Parameters

- `--output-dir <DIRECTORY>`: Output directory for generated code (default: `./generated`)
- `--namespace <NAME>`: Namespace for generated modules (default: derived from contract address)
- `--features <FEATURE_LIST>`: Comma-separated list of features to generate
- `--dry-run`: Preview generation without creating files
- `--overwrite`: Overwrite existing files without prompting
- `--template-dir <DIRECTORY>`: Custom template directory
- `--config-file <FILE>`: Configuration file for advanced options

#### Feature Options

Use `--features` to specify which components to generate:

- `client`: Generate contract client code
- `storage`: Generate database schemas and storage traits
- `api`: Generate REST and GraphQL endpoints
- `migrations`: Generate database migration files
- `tests`: Generate test templates
- `docs`: Generate documentation

#### Examples

**Basic Usage**:
```bash
almanac cosmos generate-contract schema.json \
  --address cosmos1abc123... \
  --chain cosmoshub-4
```

**Generate Specific Features**:
```bash
almanac cosmos generate-contract valence_base_account_schema.json \
  --address cosmos1valencebaseaccountexample... \
  --chain cosmoshub-4 \
  --features client,storage,api \
  --output-dir ./contracts/valence_base_account
```

**Dry Run (Preview Only)**:
```bash
almanac cosmos generate-contract schema.json \
  --address cosmos1contract... \
  --chain juno-1 \
  --dry-run \
  --verbose
```

**Custom Namespace**:
```bash
almanac cosmos generate-contract cw721_schema.json \
  --address cosmos1nft... \
  --chain stargaze-1 \
  --namespace my_nft_collection \
  --features client,storage,api,migrations
```

**With Configuration File**:
```bash
almanac cosmos generate-contract schema.json \
  --address cosmos1contract... \
  --chain cosmoshub-4 \
  --config-file cosmos_codegen.toml
```

**Generate Valence Base Account contract integration**:
```bash
almanac cosmos generate-contract valence_base_account_schema.json \
  --address cosmos1valencebaseaccountexample... \
  --chain cosmoshub-4 \
  --features client,storage,api \
  --output-dir ./contracts/valence_base_account
```

### `cosmos validate-schema`

Validate a CosmWasm message schema file.

#### Synopsis

```bash
almanac cosmos validate-schema <SCHEMA_FILE> [OPTIONS]
```

#### Arguments

- `<SCHEMA_FILE>`: Path to the schema file to validate

#### Options

- `--strict`: Enable strict validation mode
- `--output-format <FORMAT>`: Output format (text, json) (default: text)

#### Examples

```bash
# Basic validation
almanac cosmos validate-schema cw20_schema.json

# Strict validation with JSON output
almanac cosmos validate-schema schema.json --strict --output-format json

# Strict validation for Valence Base Account
almanac cosmos validate-schema valence_base_account_schema.json --strict
```

### `cosmos list-templates`

List available code generation templates.

#### Synopsis

```bash
almanac cosmos list-templates [OPTIONS]
```

#### Options

- `--category <CATEGORY>`: Filter by template category (client, storage, api, etc.)
- `--format <FORMAT>`: Output format (table, json, yaml) (default: table)

#### Examples

```bash
# List all templates
almanac cosmos list-templates

# List only client templates
almanac cosmos list-templates --category client

# JSON output
almanac cosmos list-templates --format json
```

### `cosmos init-config`

Initialize a configuration file for code generation.

#### Synopsis

```bash
almanac cosmos init-config [OPTIONS]
```

#### Options

- `--output <FILE>`: Output file path (default: cosmos_codegen.toml)
- `--template <TEMPLATE>`: Use predefined template (minimal, full, custom)
- `--interactive`: Interactive configuration wizard

#### Examples

```bash
# Create default configuration
almanac cosmos init-config

# Interactive setup
almanac cosmos init-config --interactive

# Use minimal template
almanac cosmos init-config --template minimal --output config.toml
```

## Configuration File

The CLI supports a configuration file for advanced options:

### Basic Configuration (`cosmos_codegen.toml`)

```toml
[cosmos]
# Default output directory
output_dir = "./generated"

# Default features to enable
default_features = ["client", "storage", "api"]

# Template configuration
[cosmos.templates]
# Custom template directory
template_dir = "./templates"

# Template overrides
[cosmos.templates.overrides]
client = "custom_client.hbs"
storage = "custom_storage.hbs"

# Database configuration
[cosmos.database]
# Database URL for testing generated code
url = "postgresql://localhost/test_db"

# Schema prefix for generated tables
schema_prefix = "contract_"

# API configuration
[cosmos.api]
# Base path for generated API endpoints
base_path = "/api/v1/cosmos"

# Enable rate limiting in generated code
rate_limiting = true

# Enable CORS in generated code
cors = true

# Contract-specific overrides
[[cosmos.contracts]]
name_pattern = "cw20_*"
features = ["client", "storage", "api", "migrations"]
namespace_prefix = "token_"

[[cosmos.contracts]]
name_pattern = "cw721_*"
features = ["client", "storage", "api"]
namespace_prefix = "nft_"

[[cosmos.contracts]]
name_pattern = "valence_base_account_*"
features = ["client", "storage", "api"]
namespace_prefix = "valence_"
```

## Environment Variables

The CLI respects these environment variables:

- `COSMOS_CODEGEN_OUTPUT_DIR`: Default output directory
- `COSMOS_CODEGEN_FEATURES`: Default features (comma-separated)
- `COSMOS_CODEGEN_TEMPLATE_DIR`: Custom template directory
- `COSMOS_CODEGEN_CONFIG`: Path to configuration file
- `COSMOS_CODEGEN_CACHE_DIR`: Directory for caching downloaded schemas
- `RUST_LOG`: Logging level (trace, debug, info, warn, error)

## Exit Codes

- `0`: Success
- `1`: General error
- `2`: Invalid arguments
- `3`: Schema validation failed
- `4`: File system error (permissions, disk space, etc.)
- `5`: Network error (downloading schemas)
- `6`: Template error

## Advanced Usage

### Custom Templates

Create custom Handlebars templates for specialized code generation:

```bash
# Create template directory structure
mkdir -p templates/client
mkdir -p templates/storage
mkdir -p templates/api

# Create custom client template
cat > templates/client/mod.rs.hbs << 'EOF'
//! Generated client for {{contract_name}}
//! Custom template version

use indexer_core::Result;
use cosmwasm_std::Addr;

pub struct {{pascal_case contract_name}}Client {
    // Custom implementation
}
EOF

# Use custom templates
almanac cosmos generate-contract schema.json \
  --address cosmos1... \
  --chain cosmoshub-4 \
  --template-dir ./templates
```

### Batch Processing

Process multiple contracts with a script:

```bash
#!/bin/bash
# generate_all_contracts.sh

CONTRACTS=(
  "cosmos1cw20contract1...:cw20:osmosis-1"
  "cosmos1cw721contract1...:cw721:stargaze-1"
  "cosmos1daocontract1...:cw-dao:juno-1"
  "cosmos1valencebaseaccountexample...:valence_base_account:cosmoshub-4"
)

for contract_info in "${CONTRACTS[@]}"; do
  IFS=':' read -r address name chain <<< "$contract_info"
  
  echo "Generating code for $name on $chain..."
  almanac cosmos generate-contract "schemas/${name}_schema.json" \
    --address "$address" \
    --chain "$chain" \
    --output-dir "contracts/$name" \
    --features client,storage,api
done
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
            "cosmos", "generate-contract",
            "schemas/cw20_schema.json",
            "--address", "cosmos1...",
            "--chain", "osmosis-1",
            "--output-dir", "src/generated",
            "--features", "client,storage"
        ])
        .output()
        .expect("Failed to run almanac");

    if !output.status.success() {
        panic!("Code generation failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Tell Cargo to rerun if schema changes
    println!("cargo:rerun-if-changed=schemas/cw20_schema.json");
}
```

#### Makefile Integration

```makefile
.PHONY: generate-contracts
generate-contracts:
	@echo "Generating contract code..."
	@for schema in schemas/*.json; do \
		contract=$$(basename $$schema .json | sed 's/_schema//'); \
		almanac cosmos generate-contract $$schema \
			--address $$(cat addresses/$$contract.txt) \
			--chain $$(cat chains/$$contract.txt) \
			--output-dir contracts/$$contract \
			--features client,storage,api; \
	done

.PHONY: validate-schemas
validate-schemas:
	@echo "Validating schemas..."
	@for schema in schemas/*.json; do \
		almanac cosmos validate-schema $$schema --strict; \
	done

.PHONY: clean-generated
clean-generated:
	@echo "Cleaning generated code..."
	@rm -rf contracts/*/generated
```

## Troubleshooting

### Common Issues

**Schema Validation Errors**:
```bash
# Check schema format
almanac cosmos validate-schema schema.json --strict

# Common fixes:
# - Ensure JSON is valid
# - Check required fields are present
# - Verify $ref references are correct
```

**Template Errors**:
```bash
# List available templates
almanac cosmos list-templates

# Check template syntax
# Templates use Handlebars syntax: {{variable}}
```

**Permission Errors**:
```bash
# Ensure write permissions to output directory
chmod -R u+w ./generated

# Or specify different output directory
almanac cosmos generate-contract schema.json \
  --address cosmos1... \
  --chain cosmoshub-4 \
  --output-dir ~/my-contracts
```

**Network Errors** (when downloading schemas):
```bash
# Use local schema file instead
almanac cosmos generate-contract ./local_schema.json \
  --address cosmos1... \
  --chain cosmoshub-4

# Or set proxy if needed
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
```

### Debug Mode

Enable debug logging for detailed information:

```bash
export RUST_LOG=debug
almanac cosmos generate-contract schema.json \
  --address cosmos1... \
  --chain cosmoshub-4 \
  --verbose
```

### Getting Help

- Use `--help` with any command for detailed usage
- Check the project documentation: https://docs.almanac.com
- Report issues: https://github.com/timewave-computer/almanac/issues
- Join the community: https://discord.gg/almanac

## Examples Repository

Complete examples are available in the project repository:

```bash
git clone https://github.com/timewave-computer/almanac
cd almanac/examples/cosmos

# Run example generation
make generate-all
```

The examples include:
- CW20 token contracts
- CW721 NFT contracts
- DAO governance contracts
- Custom contract types
- Multi-contract applications

## Version Compatibility

| CLI Version | CosmWasm Version | Schema Version |
|-------------|------------------|----------------|
| 0.1.x       | 1.x              | draft-07       |
| 0.2.x       | 2.x              | draft-07       |
| 1.0.x       | 2.x              | 2019-09        |

## Performance Tips

1. **Use Dry Run**: Test with `--dry-run` before generating large codebases
2. **Selective Features**: Only generate needed features to reduce build time
3. **Template Caching**: Templates are cached; use `--template-dir` for custom templates
4. **Parallel Processing**: Use shell scripts to process multiple contracts in parallel
5. **Incremental Generation**: Only regenerate when schemas change

This CLI reference provides all the information needed to effectively use the Cosmos code generation system. 