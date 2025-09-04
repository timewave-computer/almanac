# Cosmos Test Environment

This directory contains cross-platform tests for the Cosmos development environment using Neutron v6.0.3 testnet.

## Test Status: ✅ VERIFIED WORKING

The Cosmos testing environment has been successfully verified on both macOS and NixOS with the following components:

✅ **Neutron v6.0.3** builds successfully from source on both platforms  
✅ **Cross-platform support** confirmed for macOS and NixOS  
✅ **Development tools** (curl, jq, neutrond CLI) available in both environments  
✅ **Unified test script** works seamlessly across platforms  
✅ **Token transfer functionality** verified with proper timing and error handling  
✅ **Database integration** ready with PostgreSQL schema and configuration  

## Unified Test Script

### test.sh
Single consolidated test script that handles both build verification and optional token transfer tests:

- **Quick Build Test** (default): Verifies Neutron builds and development environment
- **Full Transfer Test** (with `--transfer` flag): Complete token transfer verification including testnet setup, account funding, and transfer execution

## Platform Support

- **macOS**: Uses `../../../nix/cosmos` flake
- **NixOS**: Uses `../../../nix/cosmos#nixos` flake  

## Running Tests

### Quick Build Test (Recommended)
```bash
cd tests/environment/cosmos

# Fast verification - builds packages and tests CLI tools
./test.sh
```

### Full Transfer Test
```bash
cd tests/environment/cosmos

# Complete test with token transfers (5-10 minutes)
./test.sh --transfer
```

### Help
```bash
./test.sh --help
```

### Remote Testing (NixOS server)
```bash
ssh tw-nix
cd /root/projects/timewave/almanac/tests/environment/cosmos
NIX_CONFIG='experimental-features = nix-command flakes' ./test.sh                # Quick build test  
NIX_CONFIG='experimental-features = nix-command flakes' ./test.sh --transfer     # Full transfer test
```

## Manual Testing

For guaranteed results, you can run tests manually:

```bash
# Enter development environment
nix develop ../../../nix/cosmos

# Verify tools
neutrond version  # Should show 6.0.3
curl --version
jq --version

# Manual testnet (optional)
neutrond init test --chain-id test
# ... follow setup in test-transfer-working.sh
```

## Latest Test Results

### macOS Test Results (September 2, 2025)
```
Platform: macOS
Using flake: ../../../nix/cosmos

✅ Neutron builds successfully  
✅ Development shell provides neutrond v6.0.3
✅ curl and jq available
✅ Build Test PASSED in ~30 seconds
```

### NixOS Test Results (Previously Verified)
```
Platform: NixOS/Linux
Using flake: ../../../nix/cosmos#nixos

✅ Neutron builds successfully with NIX_CONFIG
✅ Development shell functional
✅ All development tools available
✅ Transfer test functionality verified
```

## Requirements

- Nix with flakes enabled (`experimental-features = nix-command flakes`)
- Access to the cosmos flake in `../../../nix/cosmos/`
- Network connectivity for downloading nixpkgs  
- For transfer tests: ~2GB RAM, stable network connection
- Time: Build test ~30 seconds, Full transfer test ~5-10 minutes

## Architecture

- **Neutron v6.0.3**: Built from source with consumer chain support
- **libwasmvm v2.1.5**: Cross-platform WebAssembly virtual machine
- **Cross-platform flakes**: Supports both Darwin and Linux systems
- **Test framework**: Bash scripts with comprehensive error handling and cleanup