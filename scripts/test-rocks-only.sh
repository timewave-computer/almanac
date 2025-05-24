#!/bin/bash
# Purpose: Run only RocksDB tests with PostgreSQL features disabled

set -e

echo "=== Running RocksDB Storage Tests Only ==="

# Run from nix environment with PostgreSQL features disabled
nix develop --command bash -c "
  cd crates/storage
  
  # Backup current Cargo.toml
  cp Cargo.toml Cargo.toml.bak
  
  # Update Cargo.toml to disable postgres feature
  sed -i '' 's/default = \\[\"postgres\", \"rocks\"\\]/default = \\[\"rocks\"\\]/' Cargo.toml
  
  echo 'Modified Cargo.toml to disable PostgreSQL features'
  
  # Run only RocksDB tests with SQLX_OFFLINE=true
  SQLX_OFFLINE=true cargo test rocks --no-default-features --features rocks
  
  # Restore original Cargo.toml
  mv Cargo.toml.bak Cargo.toml
  
  echo 'Restored original Cargo.toml configuration'
"

echo "✓ RocksDB tests completed" 