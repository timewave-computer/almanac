# Nix Integration

## Overview

This project uses Nix to manage the development environment, dependencies, and deployment workflows. The goal is to provide a reproducible environment for both development and deployment of the Almanac cross-chain indexer.

## Nix Flake Structure

Our `flake.nix` defines the following components:

1. Development environment with all required tools
2. Build configuration using `crate2nix`
3. Runtime dependencies for PostgreSQL and RocksDB
4. Testing infrastructure for multiple blockchain environments
5. Workflow automation for development tasks

## Key Nix Components

### Development Shell

The development shell provides all necessary tools and sets up appropriate environment variables:

```bash
nix develop
```

This loads a shell with:
- Rust toolchain with proper versions
- PostgreSQL 15 and database tools
- RocksDB libraries and dependencies
- Foundry (Anvil, Forge, Cast) for Ethereum development
- `wasmd` tools for Cosmos development
- `sqlx-cli` for database migrations
- Additional utilities (git, curl, jq, etc.)

### Database Management

Database initialization and management:

```bash
# Initialize and start databases
init_databases

# Stop databases
stop_databases

# Generate Cargo.nix for Nix builds
generate_cargo_nix

# Run comprehensive test suite
run_almanac_tests
```

### Blockchain Node Support

We maintain Nix configurations for both Ethereum and Cosmos test environments:

```bash
# Start Ethereum test node (Anvil)
nix run .#start-anvil

# Start Reth Ethereum node
nix run .#start-reth

# Start Cosmos test node (wasmd)
nix run .#wasmd-node
```

### Build System

Build specific components of the Almanac system:

```bash
# Build the main indexer API service
nix build .#indexer-api

# Build specific workspace crates
nix build .#indexer-core
nix build .#indexer-storage
nix build .#indexer-ethereum
nix build .#indexer-cosmos
nix build .#indexer-causality
```

### Testing Infrastructure

Comprehensive testing commands for different environments:

```bash
# Test Ethereum adapter against Anvil
nix run .#test-ethereum-adapter-anvil

# Test Ethereum adapter against Reth
nix run .#test-ethereum-adapter-reth

# Test Cosmos adapter
nix run .#test-cosmos-adapter

# Run cross-chain end-to-end tests
nix run .#cross-chain-e2e-test
```

### Development Workflows

Interactive workflow selection and automation:

```bash
# Interactive workflow menu
cd nix/environments
nix run .

# Run specific workflows
nix run ./nix/environments#anvil-workflow
nix run ./nix/environments#reth-workflow
nix run ./nix/environments#cosmwasm-workflow
nix run ./nix/environments#all-workflows
```

## Custom Nix Configurations

### Environment Variables

The development shell automatically sets up essential environment variables:

```bash
# Database configuration
export PGDATA="$PROJECT_ROOT/data/postgres"
export PGUSER="postgres"
export PGPASSWORD="postgres"
export PGDATABASE="indexer"
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/indexer"

# SQLx offline mode for compilation
export SQLX_OFFLINE="true"

# macOS compatibility
export MACOSX_DEPLOYMENT_TARGET="11.0"
export DEVELOPER_DIR=""
```

### Adding New Commands

To add a new command to the flake:

1. Define a package in the `packages` attribute of `flake.nix`
2. Define an app in the `apps` attribute to expose it as a runnable command

Example:

```nix
packages.${system} = {
  my-indexer-tool = pkgs.writeShellApplication {
    name = "my-indexer-tool";
    runtimeInputs = with pkgs; [ curl jq ];
    text = ''
      echo "Custom indexer tool"
      # Tool implementation...
    '';
  };
};

apps.${system} = {
  my-indexer-tool = {
    type = "app";
    program = "${self.packages.${system}.my-indexer-tool}/bin/my-indexer-tool";
  };
};
```

This can be run with `nix run .#my-indexer-tool`.

### Setting Up a New Development Environment

To create a specialized environment for specific tasks:

```nix
devShells.${system}.migration = pkgs.mkShell {
  buildInputs = with pkgs; [
    postgresql_15
    sqlx-cli
    rust-bin.stable.latest.default
  ];
  
  shellHook = ''
    export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/indexer"
    export SQLX_OFFLINE=true
    echo "Migration development environment ready!"
    echo "Run 'sqlx migrate run' to apply database migrations"
  '';
};
```

Load this environment with `nix develop .#migration`.

### Crate-Specific Overrides

The flake includes specific build overrides for system dependencies:

```nix
# RocksDB system dependencies
librocksdb-sys = attrs: {
  nativeBuildInputs = with pkgs; [ pkg-config cmake ];
  buildInputs = with pkgs; [
    zlib bzip2 lz4 zstd snappy
  ] ++ lib.optionals pkgs.stdenv.isDarwin [
    pkgs.darwin.apple_sdk.frameworks.Security
    pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
  ];
};

# PostgreSQL/SQLx dependencies
indexer-storage = attrs: {
  buildInputs = with pkgs; [
    postgresql_15
    sqlx-cli
  ] ++ lib.optionals pkgs.stdenv.isDarwin [
    pkgs.darwin.apple_sdk.frameworks.Security
    pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
  ];
  
  preBuild = ''
    export SQLX_OFFLINE=true
  '';
};
```

## Advantages of Nix Integration

1. **Reproducibility**: Every developer gets exactly the same environment across platforms
2. **Isolation**: Dependencies don't conflict with system packages
3. **Versioning**: Exact versions of tools are specified and pinned in `flake.lock`
4. **Cross-platform**: Works consistently across macOS (Apple Silicon/Intel) and Linux
5. **Incremental Builds**: Efficient caching and incremental compilation
6. **Testing Automation**: Consistent test environments across different blockchain nodes

## Platform Support

The flake is designed to work across multiple platforms:

- **macOS (Apple Silicon)**: Full support with proper framework linking
- **macOS (Intel)**: Full support with Intel-specific optimizations  
- **Linux (x86_64)**: Full support with native Linux tooling
- **Nix on other platforms**: Basic support where Nix is available

## Troubleshooting

### Common Issues

1. **Database Connection Issues**:
   ```bash
   # Check if PostgreSQL is running
   pg_isready -h localhost -p 5432
   
   # Restart databases if needed
   stop_databases
   init_databases
   ```

2. **Build Failures**:
   ```bash
   # Regenerate Cargo.nix if dependencies changed
   generate_cargo_nix
   
   # Clean and rebuild
   nix build .#indexer-api --rebuild
   ```

3. **Missing Environment Variables**:
   ```bash
   # Ensure you're in the development shell
   nix develop
   
   # Check environment variables
   env | grep -E "(DATABASE_URL|SQLX_OFFLINE|PG)"
   ```

### Log Files

Check log files for debugging workflow issues:
- Anvil logs: `logs/anvil.log`
- Reth logs: `logs/reth.log`
- wasmd logs: `logs/wasmd.log`
- PostgreSQL logs: `data/postgres/postgres.log`

## Performance Optimizations

The Nix configuration includes several performance optimizations:

1. **Parallel Builds**: Configured for optimal CPU utilization
2. **Caching**: Aggressive caching of dependencies and build artifacts
3. **System Libraries**: Direct linking to system libraries where possible
4. **Cross-Compilation**: Support for efficient cross-platform builds 