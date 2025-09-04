# Solana Nix Environments

This directory contains multiple Nix configurations for Solana development across different platforms and use cases.

## Nix Files Overview

### Core Flakes

| File | Purpose | Platforms | Solana Version | Use Case |
|------|---------|-----------|----------------|----------|
| **`flake.nix`** | Main cross-platform development environment | All (x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin) | 2.0.22 | Primary development with Rust overlay and Anchor |
| **`flake-cross-platform.nix`** | Simplified cross-platform environment | All | System default | Lightweight development without complex dependencies |
| **`flake-macos.nix`** | macOS-optimized environment | Darwin only | System default | macOS local development |
| **`flake-nixos.nix`** | NixOS-optimized with KVM patches | Linux only | 1.18.26 (patched) | NixOS server deployment with Geyser |
| **`flake-nixos-stable.nix`** | Production-stable NixOS environment | Linux only | 1.14.23 | Production deployments (KVM-compatible) |

### Supporting Files

| File | Purpose | Description |
|------|---------|-------------|
| **`shell.nix`** | Legacy shell wrapper | Auto-selects appropriate flake based on current platform |
| **`geyser-builder.nix`** | Geyser plugin builder | Builds the Almanac Geyser plugin for blockchain data streaming |
| **`geyser-plugin/`** | Geyser plugin source | Rust source code for the Almanac Geyser plugin |

## When to Use Each Flake

### For Development

```bash
# Primary development (recommended)
nix develop ./flake.nix
# Provides: Solana 2.0.22, Anchor 0.31.1, Rust overlay, development tools

# Cross-platform compatibility testing  
nix develop ./flake-cross-platform.nix
# Provides: System Solana, basic toolchain, platform detection

# macOS-specific development
nix develop ./flake-macos.nix  
# Provides: macOS-optimized Solana CLI, Anchor
```

### For Production Deployment

```bash
# Stable production (recommended for servers)
nix develop ./flake-nixos-stable.nix
# Provides: Solana 1.14.23 (KVM-compatible), battle-tested

# Latest with patches (for testing new features)
nix develop ./flake-nixos.nix
# Provides: Solana 1.18.26 with KVM socket patches, Geyser support
```

### For Legacy Compatibility

```bash
# Auto-platform detection (legacy)
nix-shell shell.nix
# Automatically chooses flake-macos.nix or flake-nixos.nix
```

## Detailed File Descriptions

### `flake.nix` - Primary Development Environment
- **Target**: All platforms (cross-platform)
- **Features**: Rust overlay, Anchor framework, comprehensive development tools
- **Solana**: Version 2.0.22 (latest stable)
- **Use Case**: Main development environment with full toolchain
- **Includes**: IDL generation, reproducible builds, macOS deployment targets

### `flake-cross-platform.nix` - Lightweight Cross-Platform
- **Target**: All platforms
- **Features**: Basic Rust toolchain, Geyser plugin support
- **Solana**: System default version
- **Use Case**: Simple development without heavy dependencies
- **Includes**: Platform detection, basic development shell

### `flake-macos.nix` - macOS Optimized
- **Target**: macOS (Darwin) only
- **Features**: macOS-optimized packages, system integration
- **Solana**: System default (typically latest)
- **Use Case**: Local macOS development
- **Includes**: macOS-specific optimizations, GUI application support

### `flake-nixos.nix` - NixOS with KVM Patches
- **Target**: Linux/NixOS only
- **Features**: KVM socket patches, Geyser plugin, PostgreSQL integration
- **Solana**: Version 1.18.26 with custom patches
- **Use Case**: NixOS server deployment with latest features
- **Includes**: Socket timeout handling, virtualization compatibility

### `flake-nixos-stable.nix` - Production Stable
- **Target**: Linux/NixOS only  
- **Features**: Production-tested, KVM-compatible
- **Solana**: Version 1.14.23 (stable)
- **Use Case**: Production server deployments
- **Includes**: Proven compatibility, minimal dependencies

### `shell.nix` - Platform Auto-Detection
- **Target**: All platforms
- **Features**: Automatic flake selection based on current platform
- **Solana**: Varies (delegates to platform-specific flake)
- **Use Case**: Legacy compatibility, simple platform abstraction
- **Includes**: Platform detection logic, fallback mechanisms

## Architecture Notes

### KVM Compatibility Issue
Solana versions 1.15+ have socket timeout assertions that fail in KVM/virtualized environments:

```rust
// This fails in KVM environments
assert!(res.is_ok(), "streamer::receiver set_read_timeout error");
```

**Solutions**:
- **Stable**: Use `flake-nixos-stable.nix` (Solana 1.14.23)
- **Patched**: Use `flake-nixos.nix` (1.18.26 with graceful error handling)

### Geyser Plugin Integration
```
Solana Validator → Geyser Plugin → PostgreSQL → Almanac Storage
```

The Geyser plugin enables real-time blockchain data streaming to external systems.

## Quick Start Examples

```bash
# Development on any platform
nix develop

# Production deployment
nix develop ./flake-nixos-stable.nix
solana-test-validator

# Geyser plugin development  
nix develop ./flake-nixos.nix
# Setup database first time only
setup-almanac-db
# Start validator with plugin
solana-test-validator-geyser
```

## Testing

Each flake is tested through the unified test framework in `../../tests/environment/solana/`:

```bash
cd ../../tests/environment/solana
./test.sh                # Test current platform flake
./test.sh --geyser       # Test with Geyser plugin functionality
```

## Migration Guide

- **From solana-cross**: Use `flake-cross-platform.nix` instead
- **Legacy shell.nix**: Migrate to specific flakes for better control
- **Production**: Always use `flake-nixos-stable.nix` for new deployments
- **Development**: Use main `flake.nix` for comprehensive toolchain