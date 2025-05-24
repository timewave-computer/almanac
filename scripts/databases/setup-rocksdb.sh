#!/bin/bash
# Purpose: Set up RocksDB for testing and development

set -e

echo "=== Setting up RocksDB Environment ==="

# Create RocksDB data directory
mkdir -p data/rocksdb
echo "✓ Created RocksDB data directory"

# Use nix develop to initialize RocksDB
nix develop --command bash -c "
  # Initialize RocksDB
  echo \"Initializing RocksDB storage...\"
  
  # Clean any existing data
  if [ -d \"data/rocksdb\" ]; then
    echo \"Cleaning existing RocksDB data...\"
    rm -rf data/rocksdb/*
  fi
  
  # Create required directories
  mkdir -p data/rocksdb/{blocks,events,metadata,temp}
  chmod -R 755 data/rocksdb
  
  echo \"✓ RocksDB directories initialized\"
  
  # Create a test file to verify permissions
  echo 'This is a test file to verify RocksDB permissions' > data/rocksdb/test-file.txt
  
  if [ -f data/rocksdb/test-file.txt ]; then
    echo \"✓ RocksDB permissions verified\"
    rm data/rocksdb/test-file.txt
  else
    echo \"ERROR: Could not create test file in RocksDB directory\"
    exit 1
  fi
"

echo "=== RocksDB setup complete ==="
echo "RocksDB storage is ready for use at: $(pwd)/data/rocksdb" 