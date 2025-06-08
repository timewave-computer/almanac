# Almanac End-to-End Tests

This directory contains comprehensive end-to-end tests for the Almanac CLI that verify all commands documented in the `docs/` directory and README with appropriate mock data.

## Overview

The e2e test suite tests all CLI commands listed in the documentation to ensure they work correctly with real-world examples and edge cases. It includes:

- **Comprehensive CLI Tests**: Tests all commands from documentation with mock data
- **Cosmos CLI Tests**: CosmWasm contract code generation testing
- **Ethereum CLI Tests**: Ethereum contract code generation testing
- **Configuration Tests**: Configuration file and environment testing
- **Validation Tests**: Code generation output validation
- **Basic CLI Tests**: Core CLI functionality testing

## Quick Start

### Run All Tests

```bash
# From project root
./scripts/run_e2e_tests.sh
```

### Run Specific Test Suites

```bash
# Run only comprehensive CLI tests (all documented commands)
./scripts/run_e2e_tests.sh --comprehensive

# Run only cosmos-related tests
./scripts/run_e2e_tests.sh --cosmos

# Run only ethereum-related tests
./scripts/run_e2e_tests.sh --ethereum

# Run tests matching a pattern
./scripts/run_e2e_tests.sh --filter="generate-contract"
```

### Development Options

```bash
# Verbose output for debugging
./scripts/run_e2e_tests.sh --verbose

# Skip building binary (use existing)
./scripts/run_e2e_tests.sh --no-build

# Keep test output files for inspection
./scripts/run_e2e_tests.sh --keep-outputs

# Combine options
./scripts/run_e2e_tests.sh --verbose --keep-outputs --filter cosmos
```

## Test Structure

### Comprehensive CLI Tests (`cli_comprehensive.rs`)

This is the main test suite that covers **all CLI commands documented in the docs and README**:

#### Documentation Examples Tested

**From README.md:**
- Cosmos Valence Base Account generation
- Ethereum USDC token generation

**From cosmos_cli_reference.md:**
- Basic contract generation
- Dry run functionality
- Custom namespace usage
- Different chain IDs

**From ethereum_cli_reference.md:**
- Basic ERC20 generation
- Full feature generation
- Polygon network usage
- Complex contract examples

#### Feature Coverage Tests

- Individual feature testing: `client`, `storage`, `api`, `migrations`
- Feature combinations: `client,storage`, `client,api`, etc.
- Chain-specific testing for all documented networks

#### Error Condition Tests

- Missing required arguments
- Invalid file paths
- Invalid contract addresses
- Invalid chain IDs
- Invalid feature names

### Test Fixtures

Mock data for testing:

```
e2e/fixtures/
├── schemas/
│   └── valence_base_account.json    # CosmWasm schema
└── abis/
    ├── erc20.json                   # Standard ERC20 ABI
    └── usdc.json                    # USDC token ABI with additional features
```

### Generated Test Data

Each test creates realistic test scenarios:

- **Cosmos**: Uses Valence Base Account schema with proper message types
- **Ethereum**: Uses standard ERC20 and USDC ABIs with real function signatures
- **Addresses**: Valid format addresses for each chain type
- **Chain IDs**: Real chain IDs from documentation (mainnet, testnet, L2s)

## Running Tests Manually

### Using Cargo

```bash
# Run all tests
cd e2e
cargo run

# Run with filter
cargo run -- --filter=comprehensive

# Run with verbose output
cargo run -- --verbose
```

### Using Nix

```bash
# Build and run in nix environment
nix develop --command cargo run --manifest-path e2e/Cargo.toml
```

## Test Output

### Success Output

```
Almanac CLI End-to-End Test Suite
==================================================
✓ Almanac binary ready
✓ Test fixtures ready

Running test suites...

📚 Comprehensive CLI Tests (All Documented Commands)
  ✓ 127/127 tests passed (100.0%)

🌌 Cosmos CLI Tests
  ✓ 23/23 tests passed (100.0%)

⚡ Ethereum CLI Tests
  ✓ 25/25 tests passed (100.0%)

🎉 All tests passed!
```

### Failure Output

```
📚 Comprehensive CLI Tests (All Documented Commands)
  ⚠ 125/127 tests passed, 2 failed (98.4%)
    Failed tests:
      - cosmos_doc_basic_example
      - eth_invalid_address
```

## Test Development

### Adding New Tests

1. **For new CLI commands**: Add to `cli_comprehensive.rs`
2. **For specific functionality**: Add to appropriate module (`cosmos.rs`, `ethereum.rs`)
3. **For new fixtures**: Add mock data to `fixtures/`

### Test Utilities

Available in `utils/`:
- `run_cli_command()`: Execute CLI commands with capture
- `validate_generated_files()`: Check generated code structure
- `validate_rust_code_patterns()`: Verify code contains expected patterns
- `validate_sql_migration()`: Check SQL migration files

### Writing Tests

```rust
// Test a specific CLI command
let result = run_cli_command(
    config,
    &[
        "cosmos",
        "generate-contract",
        &schema_path.to_string_lossy(),
        "--address", "cosmos1test123456789",
        "--chain", "cosmoshub-4",
        "--features", "client,storage"
    ],
    "test_name"
).await;
results.add_result(result);

// Validate generated output
if output_dir.exists() {
    let validation_result = validate_generated_files(
        &output_dir,
        &["client", "storage"],
        "validation_test_name"
    );
    results.add_result(validation_result);
}
```

## Continuous Integration

The e2e tests are designed to run in CI environments:

- **Nix-based**: Uses project's nix flake for reproducible builds
- **Isolated**: Each test uses separate output directories
- **Fast**: Extensive use of dry-run mode for validation-only tests
- **Reliable**: Comprehensive error handling and graceful failures

### CI Configuration

```yaml
# Example GitHub Actions
- name: Run E2E Tests
  run: |
    nix develop --command ./scripts/run_e2e_tests.sh --verbose
```

## Troubleshooting

### Common Issues

1. **Binary not found**: Ensure almanac is built with `nix build .#indexer-api`
2. **Fixture missing**: Check that `e2e/fixtures/` contains required files
3. **Permission errors**: Ensure write access to test output directories
4. **Nix environment**: Run from project root in nix develop shell

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug ./scripts/run_e2e_tests.sh --verbose

# Keep outputs for inspection
./scripts/run_e2e_tests.sh --keep-outputs

# Run specific failing test
./scripts/run_e2e_tests.sh --filter="failing_test_name"
```

### Test Isolation

Each test creates isolated output directories:
```
e2e/test_outputs/
├── cosmos_generate_contract_test/
├── eth_doc_usdc/
├── comprehensive_test_output/
└── ...
```

## Coverage

The comprehensive test suite covers:

- ✅ **100% of documented CLI commands**
- ✅ **All examples from README.md**
- ✅ **All examples from docs/cosmos_cli_reference.md**
- ✅ **All examples from docs/ethereum_cli_reference.md**
- ✅ **Error conditions from troubleshooting guides**
- ✅ **All supported chains and networks**
- ✅ **All feature combinations**
- ✅ **Edge cases and boundary conditions**

This ensures that all documented functionality actually works and that the documentation examples are accurate and up-to-date. 