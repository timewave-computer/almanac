# Almanac Test Contracts and Infrastructure

This directory primarily contains smart contract source code and related infrastructure used for testing the Almanac project's chain adapters and indexing capabilities. These contracts are compiled and deployed as part of the integration and end-to-end test suites.

## Running Tests

Tests are typically run via the main project's `nix develop` shell or `nix run` commands, which handle the compilation and execution of these contracts as part of the test setup.

### Using Nix

```bash
# Run all tests (including those that use these contracts)
nix develop --command cargo test

# Run specific E2E tests that deploy these contracts
nix run .#run-e2e-tests
```

## Contract Categories

### Solidity Contracts
- Located in `ethereum-contracts/`
- Used for testing Ethereum chain adapter functionality.
- Compiled and deployed using Foundry.

### CosmWasm Contracts
- Located in `cosmos-contracts/`
- Used for testing Cosmos chain adapter functionality.
- Compiled into WASM binaries and deployed using CosmWasm tools.

## Test Fixtures

General test data and configurations (e.g., ABIs, schemas, example configurations) are located in the project's root-level `fixtures/` directory.

## Adding New Contracts or Tests

1.  **For new Solidity contracts**: Add `.sol` files to `ethereum-contracts/`.
2.  **For new CosmWasm contracts**: Add Rust source files to `cosmos-contracts/`.
3.  **For tests utilizing these contracts**: Refer to the `e2e/` directory for examples of how these contracts are integrated into the end-to-end testing infrastructure.