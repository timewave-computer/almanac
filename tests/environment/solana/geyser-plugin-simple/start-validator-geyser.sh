#!/usr/bin/env bash
set -euo pipefail

echo "Starting Solana validator with Geyser plugin (macOS)..."
cd ../geyser-plugin-simple
nix develop ../../../nix/solana/flake.nix --command cargo build --release
cd ../geyser-test

# Note: This uses the macOS Solana flake which includes development tools
echo "Validator start script created for macOS Solana development"
