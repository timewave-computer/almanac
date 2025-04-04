#!/usr/bin/env bash
set -e

# Script to run RocksDB and filesystem benchmarks
echo "Running RocksDB and filesystem performance benchmarks..."

# Make sure we're in the project root directory
cd "$(dirname "$0")/.."

# Run the benchmark
cd crates/storage && cargo run --bin run_rocks_benchmark 