# Valence Contract Integration

This document describes how to integrate and test with real Valence protocol contracts in the Almanac cross-chain indexer.

## Overview

The integration allows testing of the indexer with real Valence smart contracts rather than mocks. This provides more accurate validation of the indexer's ability to process events from Valence contracts deployed on both Ethereum and Cosmos chains.

## Prerequisites

- Nix development environment (provided by the project's flake.nix)
- Access to the Valence Protocol repositories (already included in the project)

## Setup

The setup process is automated through the Nix flake:

```bash
nix run .#valence-contract-integration
```

This command:

1. Ensures the Valence Protocol repository is up-to-date
2. Configures test networks (wasmd for Cosmos, Anvil for Ethereum)
3. Sets up Nix configuration for building Valence contracts
4. Creates deployment configurations for both ecosystems
5. Implements network state persistence for reproducible testing

## Testing Environment

### Cosmos (wasmd) Environment

A local wasmd node is used for testing Cosmos contracts. Configuration:

- Chain ID: `wasmchain`
- RPC URL: `http://localhost:26657`
- API URL: `http://localhost:1317`
- Keyring backend: `test`
- Gas prices: `0.025stake`

### Ethereum (Anvil) Environment

A local Anvil node is used for testing Ethereum contracts. Configuration:

- RPC URL: `http://localhost:8545`
- Chain ID: `31337`
- Pre-configured accounts with test ETH

## Usage

### Building Contracts

Build all Valence contracts using:

```bash
nix run .#build-valence-contracts
```

This compiles both Ethereum Solidity contracts and Cosmos CosmWasm contracts.

### Deploying Contracts

#### Cosmos Contracts

Start the wasmd node and deploy Cosmos contracts:

```bash
# Start wasmd node
nix run .#wasmd-node

# Deploy all Cosmos contracts at once
nix run .#deploy-valence-cosmos-contracts

# Or deploy individual contracts
nix run .#deploy-valence-account
nix run .#deploy-valence-processor
nix run .#deploy-valence-authorization
nix run .#deploy-valence-library
```

#### Ethereum Contracts

Start the Anvil node and deploy Ethereum contracts:

```bash
# Start Anvil node
nix run .#start-anvil

# Deploy contracts
nix run .#deploy-valence-ethereum-contracts
```

### State Persistence

For reproducible testing, you can save and restore network state using the persistence scripts:

#### wasmd State

```bash
# Save current state
./scripts/persist_wasmd_state.sh save [snapshot_name]

# Restore a saved state
./scripts/persist_wasmd_state.sh restore [snapshot_name]

# List available snapshots
./scripts/persist_wasmd_state.sh list
```

#### Anvil State

```bash
# Save current state
./scripts/persist_anvil_state.sh save [snapshot_name]

# Restore a saved state
./scripts/persist_anvil_state.sh restore [snapshot_name]

# List available snapshots
./scripts/persist_anvil_state.sh list
```

## Integration Testing

After deploying contracts to both chains, you can run integration tests that verify the indexer correctly processes events from real Valence contracts:

```bash
nix run .#test-valence-real-contracts
```

This runs a comprehensive test suite that:

1. Deploys all Valence contracts to test nodes
2. Executes various contract interactions
3. Verifies that all state changes are correctly indexed
4. Tests cross-chain queries and event processing

## Contract Details

### Cosmos Contracts

The following Valence Cosmos contracts are deployed:

- **Account**: Manages user accounts and permissions
- **Processor**: Handles cross-chain message processing
- **Authorization**: Manages authorization policies and decisions
- **Library**: Stores reusable code libraries

### Ethereum Contracts

The following Valence Ethereum contracts are deployed:

- **Bridge contracts**: Handle communication between Ethereum and Cosmos
- **Account abstraction**: Provide account features on Ethereum
- **Message verification**: Validate cross-chain messages

## Troubleshooting

### wasmd Issues

- **Node not starting**: Check `~/.wasmd-test/node.log` for errors
- **Transaction failures**: Ensure validator key is properly set up

### Anvil Issues

- **RPC connection errors**: Verify Anvil is running and accessible
- **Deployment failures**: Check for contract compilation errors

## Further Development

To add new contract testing functionality:

1. Update the deployment functions in `nix/valence-contracts.nix`
2. Add test scenarios in the `test-valence-real-contracts` script
3. Update this documentation with new contract details 