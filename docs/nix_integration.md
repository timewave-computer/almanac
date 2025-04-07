# Nix Integration

## Overview

This project uses Nix to manage the development environment, dependencies, and deployment workflows. The goal is to provide a reproducible environment for both development and deployment.

## Nix Flake Structure

Our `flake.nix` defines the following components:

1. Development environment
2. Build configuration
3. Runtime dependencies
4. Testing infrastructure

## Key Nix Components

### Development Shell

The development shell provides all necessary tools and sets up appropriate environment variables:

```bash
nix develop
```

This loads a shell with:
- Rust toolchain
- PostgreSQL libraries
- RocksDB libraries
- Foundry (Anvil, Forge, Cast)
- Wasmd tools
- Additional utilities

### Blockchain Nodes

We maintain Nix configurations for both Ethereum and Cosmos test nodes:

```bash
# Start Ethereum test node
nix run .#start-anvil

# Start Cosmos test node
nix run .#wasmd-node
```

### Contract Deployment

Valence contract deployment is managed through Nix functions in `nix/valence-contracts.nix`:

```bash
# Build all contracts
nix run .#build-valence-contracts

# Deploy individual contract types
nix run .#deploy-valence-account
nix run .#deploy-valence-processor
nix run .#deploy-valence-authorization
nix run .#deploy-valence-library

# Deploy all Cosmos contracts
nix run .#deploy-valence-cosmos-contracts

# Deploy Ethereum contracts
nix run .#deploy-valence-ethereum-contracts
```

### Nix Utilities

The Valence contract deployment functionality is implemented using Nix functions:

- `valenceUtils.makeWasmdConfig`: Creates configuration for wasmd nodes
- `valenceUtils.makeAnvilConfig`: Creates configuration for Anvil nodes
- `valenceUtils.deployContract`: Generates deployment scripts for Valence contracts

Example of generating a custom wasmd config:

```nix
let
  wasmdConfig = valenceUtils.makeWasmdConfig {
    chainId = "custom-chain";
    rpcUrl = "http://localhost:26657";
    apiUrl = "http://localhost:1317";
  };
in
  # Use the config...
```

### Testing Infrastructure

We provide Nix-based testing commands:

```bash
# Run all tests
nix run .#test-all

# Run contract-specific tests
nix run .#test-valence-contracts
```

## Custom Nix Configurations

### Adding New Commands

To add a new command to the flake:

1. Define a package in the `packages` attribute of `flake.nix`
2. Define an app in the `apps` attribute to expose it as a runnable command

Example:

```nix
packages.${system} = {
  my-command = pkgs.writeShellScriptBin "my-command" ''
    echo "Hello, world!"
  '';
};

apps.${system} = {
  my-command = {
    type = "app";
    program = "${self.packages.${system}.my-command}/bin/my-command";
  };
};
```

This can be run with `nix run .#my-command`.

### Setting Up a New Development Environment

To create a new environment for a specific task:

```nix
devShells.${system}.migration = pkgs.mkShell {
  buildInputs = [
    pkgs.postgresql_14
    pkgs.sqlx-cli
  ];
  
  shellHook = ''
    export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/indexer"
    echo "Migration development environment ready!"
  '';
};
```

Load this environment with `nix develop .#migration`.

## Advantages of Nix Integration

1. **Reproducibility**: Every developer gets exactly the same environment
2. **Isolation**: Dependencies don't conflict with system packages
3. **Versioning**: Exact versions of tools are specified and pinned
4. **Configuration as Code**: Node and deployment configurations are managed as code
5. **Cross-platform**: Works consistently across macOS and Linux
6. **Custom Commands**: Simple addition of project-specific commands

## Nix-Native Contract Deployment

Contract deployment scripts have been implemented as Nix functions rather than standalone shell scripts, providing better integration, reproducibility, and maintainability:

1. Configuration files are generated through Nix functions
2. Deployment scripts are templated with Nix functions
3. Command-line interfaces are exposed through the Nix flake

The primary implementation is in `nix/valence-contracts.nix`. 