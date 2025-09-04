# Environment Tests

This directory contains environment-specific tests for validating cross-platform Nix environments and blockchain development tools.

## Test Environments ✅

### `cosmos/` - Cosmos/Neutron Environment
- **Status**: ✅ Verified on macOS and NixOS
- **Components**: Neutron v6.0.3 testnet, account management, token transfers
- **Features**: Unified test script, cross-platform support, timing-optimized transfers

### `solana/` - Solana Development Environment  
- **Status**: ✅ Verified on macOS and NixOS
- **Components**: Solana CLI, Geyser plugin framework, PostgreSQL integration
- **Features**: Consolidated plugin structure, validator scripts, database schema

### `ethereum/` - Ethereum Development Environment
- **Status**: ✅ Verified on macOS and NixOS  
- **Components**: Geth execution client, Foundry framework, Lighthouse consensus
- **Features**: Anvil local testnet, RPC testing, cross-platform tooling

## Quick Test Overview

Each environment provides unified testing with consistent interface:

```bash
# Quick build verification (10-30 seconds)
./test.sh

# Extended functionality tests  
./test.sh --transfer      # Cosmos: token transfer tests
./test.sh --geyser        # Solana: Geyser plugin tests  
./test.sh --integration   # Ethereum: Anvil testnet tests

# Help and usage
./test.sh --help
```

## Cross-Platform Support

All environments support:
- **macOS**: Local development and testing
- **NixOS**: Production-ready server deployments
- **Automatic Detection**: Platform-specific flake selection
- **Unified Interface**: Consistent command-line experience

## Requirements

- Nix package manager with flakes enabled
- Network connectivity for package downloads
- Platform-specific requirements documented in individual READMEs

## Running Environment Tests

Each subdirectory contains comprehensive documentation and test scripts. Refer to the individual README files for detailed testing instructions, platform-specific notes, and development guidance.