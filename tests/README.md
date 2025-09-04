# Almanac Test Infrastructure

This directory contains test infrastructure for the Almanac project, including smart contracts, environment validation, integration tests, and end-to-end test suites.

## Running Tests

Tests are typically run via the main project's `nix develop` shell or `nix run` commands, which handle the compilation and execution of these contracts as part of the test setup.

### Using Nix

```bash
# Run all tests (including those that use these contracts)
nix develop --command cargo test

# Run specific E2E tests that deploy these contracts
nix run .#run-e2e-tests
```

## Directory Structure

### Environment Tests (`environment/`)
- **`solana/`** - Solana environment validation tests for macOS and NixOS
  - Geyser plugin compatibility testing
  - Nix environment validation  
  - KVM/DigitalOcean deployment checks
  - Build system verification

### Smart Contracts (`contracts/`)

#### Solidity Contracts
- Located in `contracts/solidity/`
- Used for testing Ethereum chain adapter functionality
- Compiled and deployed using Foundry

#### CosmWasm Contracts  
- Located in `contracts/cosmwasm/`
- Used for testing Cosmos chain adapter functionality
- Compiled into WASM binaries and deployed using CosmWasm tools

### Integration Tests (`integration/`)
- Cross-chain event correlation tests
- Storage backend integration tests
- API endpoint validation

### End-to-End Tests (`e2e/`)
- Multi-chain simulation tests
- Account subscription validation
- IDL indexing functionality

## Test Fixtures

General test data and configurations (e.g., ABIs, schemas, example configurations) are located in the project's root-level `fixtures/` directory.

## Adding New Tests

1. **Environment tests**: Add to `environment/` for Nix flake validation and deployment testing
2. **Smart contracts**: 
   - Solidity: Add `.sol` files to `contracts/solidity/`
   - CosmWasm: Add Rust source files to `contracts/cosmwasm/`
3. **Integration tests**: Add to `integration/` for component interaction testing
4. **E2E tests**: Add to `e2e/` for full system validation

## Running Environment Tests

Environment tests validate Nix configurations and deployment compatibility:

```bash
# Run Solana environment tests
cd tests/environment/solana
./test-build.sh            # Test build compatibility (macOS/NixOS)
./test-geyser-setup.sh     # Test Geyser plugin setup
```