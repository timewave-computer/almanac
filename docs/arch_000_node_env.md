# System Architecture

This document describes the overall architecture of the integrated Ethereum and UFO node system.

## Overview

The system consists of two main components:
1. An Ethereum node with a faucet contract for token management
2. A UFO (Universal Fast Orderer) node for Cosmos SDK chain consensus with its own faucet

These components are packaged together in a Nix flake that provides a unified interface for development, testing, and operation. Both faucets share a similar API for consistency.

## Component Architecture

```mermaid
graph TD
    A[Nix Flake] --> B[Ethereum Module]
    A --> C[UFO Module]
    
    B --> D[Anvil Node]
    B --> E[Faucet Contract]
    B --> F[Ethereum Scripts]
    
    C --> G[UFO Node]
    C --> H[Osmosis Integration]
    C --> I[UFO Scripts]
    
    J[Shared Commands] --> K[Run All Nodes]
    J --> L[E2E Tests]
```

## Flow Diagrams

### Ethereum Node Workflow

```mermaid
sequenceDiagram
    actor User
    participant Anvil
    participant Faucet
    participant Account1
    participant Account2
    
    User->>Anvil: Start node
    User->>Faucet: Deploy contract
    User->>Faucet: Mint tokens to Account1
    Faucet->>Account1: Transfer tokens
    User->>Account1: Transfer to Account2
    Account1->>Account2: Send tokens
    User->>Account1: Check balance
    User->>Account2: Check balance
```

### UFO Node Workflow

```mermaid
sequenceDiagram
    actor User
    participant Script
    participant UFO
    participant UFOFaucet
    participant Osmosis
    participant Account1
    participant Account2
    
    User->>Script: Start UFO node
    Script->>Osmosis: Check source code
    alt Patched Mode
        Script->>Osmosis: Apply UFO patches
        Osmosis->>UFO: Integrate consensus
    else Bridged Mode
        Script->>UFO: Start UFO process
        Script->>Osmosis: Start Osmosis process
        UFO->>Osmosis: Connect via bridge
    else Fauxmosis Mode
        Script->>UFO: Start with mock app
    end
    User->>UFOFaucet: Mint tokens to Account1
    UFOFaucet->>Account1: Transfer tokens
    User->>UFOFaucet: Mint tokens to Account2
    UFOFaucet->>Account2: Transfer tokens
```

## Faucet Architecture

```mermaid
classDiagram
    class FaucetInterface {
        mint(address, amount)
        getBalance(address)
    }
    
    class EthereumFaucet {
        contract address
        rpcUrl
        privateKey
        mint(address, amount)
        getBalance(address)
    }
    
    class UFOFaucet {
        buildMode
        validators
        blockTime
        mint(address, amount)
        getBalance(address)
    }
    
    FaucetInterface <|-- EthereumFaucet
    FaucetInterface <|-- UFOFaucet
```

## Module Structure

```mermaid
classDiagram
    class FlakeOutputs {
        packages
        apps
        devShells
    }
    
    class EthereumModule {
        rpcUrl: string
        privateKey: string
        start-anvil()
        deploy-contract()
        mint-tokens()
        e2e-test()
    }
    
    class UFOModule {
        osmosisSource: string
        buildMode: enum
        validators: number
        blockTimes: number[]
        faucet: {
            enabled: boolean
            initialSupply: number
            tokenName: string
            tokenSymbol: string
        }
        run-ufo-node()
        ufo-mint-tokens()
        build-osmosis-ufo()
        benchmark-ufo()
        ufo-e2e-test()
    }
    
    FlakeOutputs --> EthereumModule
    FlakeOutputs --> UFOModule
```

## Configuration Management

Configuration for both modules is defined in the main `flake.nix` file. The Ethereum module configuration includes RPC URL and private key information, while the UFO module configuration includes the path to the Osmosis source code, the build mode, the number of validators, the block times for benchmarking, and faucet settings.

Example configuration:

```nix
# Ethereum configuration
ethereum = {
  rpcUrl = "http://localhost:8545";
  privateKey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
};

# UFO configuration
ufo = {
  osmosisSource = "/tmp/osmosis-source";
  buildMode = "patched";
  validators = 1;
  blockTimes = [ 1000 100 10 1 ];
  faucet = {
    enabled = true;
    initialSupply = 1000000;
    tokenName = "UFO";
    tokenSymbol = "UFO";
  };
};
```

## Integration Points

The main integration point between the Ethereum and UFO components is the `run-all-nodes` command, which starts both nodes and manages their lifecycle. This allows developers to work with both systems simultaneously.

Both faucets share a similar API, allowing for consistent token management across both chains:

```bash
# Ethereum faucet
nix run .#mint-tokens 0xAddress 100

# UFO faucet
nix run .#ufo-mint-tokens osmo1address 100
```

## Future Considerations

Potential future enhancements:
- Cross-chain communication between Ethereum and Cosmos
- Shared token standards
- Unified monitoring and logging
- State synchronization between chains 