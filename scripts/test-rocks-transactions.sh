#!/usr/bin/env bash
set -e

# Script to run RocksDB transaction isolation and atomicity tests
echo "Running RocksDB transaction tests..."

# Make sure we're in the project root directory
cd "$(dirname "$0")/.."

# Run the benchmark
cd crates/storage && cargo run --bin test_rocks_transactions 