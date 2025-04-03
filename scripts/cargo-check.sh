#!/usr/bin/env bash

# Script to run cargo check with proper environment variables for macOS
# This script doesn't rely on any external environment setup

# Set environment variables
export MACOSX_DEPLOYMENT_TARGET="11.0"
export SOURCE_DATE_EPOCH="1672531200"

# Print the environment variables
echo "Running with environment variables:"
echo "MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET}"
echo "SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}"

# Set database variables if not already set
export DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@localhost:5432/indexer}"
export ROCKSDB_PATH="${ROCKSDB_PATH:-./data/rocksdb}"
export ETH_RPC_URL="${ETH_RPC_URL:-http://localhost:8545}"
export RETH_DATA_DIR="${RETH_DATA_DIR:-./data/reth}"
export COSMOS_RPC_URL="${COSMOS_RPC_URL:-http://localhost:26657}"

# Create necessary directories
mkdir -p ./data/rocksdb
mkdir -p ./data/reth
mkdir -p ./data/postgres
mkdir -p ./logs

# Display Rust version information
echo "Rust version:"
rustc --version
echo ""

# Check if we need to run with specific features
if [ "$1" == "--features" ] || [ "$1" == "--all-features" ]; then
  # Pass all arguments directly to cargo
  echo "Running: cargo check $@"
  cargo check "$@"
else
  # Run with all features by default
  echo "Running: cargo check --all-features"
  cargo check --all-features
fi 