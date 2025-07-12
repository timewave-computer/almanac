# Reth Ethereum Node Integration

## Overview

This document describes how to use the Reth Ethereum node implementation in place of Anvil for local development and testing. Reth is a Rust implementation of the Ethereum protocol that offers robust performance and stability.

## Requirements

- Nix development environment (provided by project's flake.nix)
- No additional requirements as reth is installed via Nix

## Getting Started

### Starting a Reth Node

Start a Reth development node with:

```bash
nix run .#start-reth
```

This will:
1. Create a development Ethereum node with a pre-configured genesis block
2. Set up default accounts with test ETH
3. Configure JSON-RPC and WebSocket endpoints
4. Export configuration to the project format
5. Configure mining for automatic block creation

### Cleaning Reth Data

To reset the node state and start from scratch:

```bash
nix run .#clean-reth
```

### Managing Accounts

Import pre-configured accounts:

```bash
nix run .#import-reth-accounts
```

### Exporting Configuration

Export the Reth configuration for use with other tools:

```bash
nix run .#export-reth-config
```

This generates a config file at `./config/reth/config.json` that contains:
- JSON-RPC URL
- WebSocket URL
- Chain ID
- Private key for the default account

## Using with Valence Contracts

### Configuration Compatibility

Reth is configured to use the same chain ID (31337) and RPC port (8545) as Anvil by default, making it a drop-in replacement for most use cases. The pre-configured accounts also have the same private keys.

### Deploying Contracts to Reth

To deploy Valence Ethereum contracts to Reth:

1. Start the Reth node:
   ```bash
   nix run .#start-reth
   ```

2. Deploy the contracts using the Reth-specific deployment script:
   ```bash
   nix run .#deploy-valence-ethereum-contracts-reth
   ```

This script is specifically optimized for Reth and handles any subtle differences in behavior compared to Anvil. It:

- Uses the Reth configuration from `./config/reth/config.json`
- Verifies the Reth node is running and properly synced
- Sets appropriate gas price parameters for Reth
- Creates a helpful script for interacting with the deployed contracts

You can also continue using the standard deployment script, which should work in most cases:

```bash
nix run .#deploy-valence-ethereum-contracts
```

However, the Reth-specific script provides additional optimizations and reliability improvements.

## Complete Workflow

For a complete workflow using Reth for Valence contract deployment:

```bash
# Start the Reth node
nix run .#start-reth

# Deploy Ethereum contracts to Reth
nix run .#deploy-valence-ethereum-contracts-reth

# Start a Cosmos node for cross-chain testing
nix run .#wasmd-node

# Deploy Cosmos contracts
nix run .#deploy-valence-cosmos-contracts

# Run tests or other integrations
# ...
```

## Advanced Configuration

### Custom Chain ID

To use a different chain ID:

```nix
# In your flake.nix or a custom module
packages.start-reth-custom = rethModule.makeRethStartScript { 
  name = "start-reth-custom";
  config = { 
    chainId = 1337; 
  }; 
};
```

### Custom Port Configuration

To change the default ports:

```nix
packages.start-reth-custom = rethModule.makeRethStartScript { 
  config = { 
    rpcPort = 8555;
    wsPort = 8556; 
    p2pPort = 30304;
  }; 
};
```

### Adding Custom Genesis Accounts

Add additional pre-funded accounts:

```nix
packages.start-reth-custom = rethModule.makeRethStartScript { 
  config = { 
    genesisAccounts = defaultGenesisAccounts ++ [
      {
        address = "0x123...";
        balance = "10000000000000000000000";
        privateKey = "0xabc...";
      }
    ];
  }; 
};
```

## Troubleshooting

### Common Issues

#### Node fails to start

If the Reth node fails to start, try:

```bash
nix run .#clean-reth
nix run .#start-reth
```

#### Contract deployment fails

Ensure the Reth node is running and check its logs for any errors:

```bash
curl -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' http://localhost:8545
```

#### Incompatibility with existing contracts

If you experience compatibility issues:

1. Reset the Reth node state: `nix run .#clean-reth`
2. Start the node again: `nix run .#start-reth`
3. Re-export the configuration: `nix run .#export-reth-config`
4. Re-deploy contracts using the Reth-specific script: `nix run .#deploy-valence-ethereum-contracts-reth`

## Performance Considerations

Reth is optimized for performance, but for development purposes, you may want to adjust:

- Block time (default: 2 seconds, can be lowered for faster development iterations)
- Mining settings (enabled by default)

## Migration from Anvil

### Step-by-Step Migration

1. Stop your Anvil instance
2. Start Reth: `nix run .#start-reth`
3. Export Reth configuration: `nix run .#export-reth-config`
4. Deploy contracts using: `nix run .#deploy-valence-ethereum-contracts-reth`
5. Update any environment variables or configurations to point to Reth

### API Compatibility

Reth supports the same Ethereum JSON-RPC API as Anvil, so your existing code should work without changes. However, there might be subtle behavior differences in edge cases, which is why we provide a Reth-specific deployment script. 