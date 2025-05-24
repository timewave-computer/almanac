#!/bin/bash
# Purpose: Run isolated RocksDB tests without requiring PostgreSQL connections

set -e

# Color definitions
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Running Isolated RocksDB Tests ===${NC}"

# Create temporary directory for RocksDB
TEMP_DIR="${PWD}/tmp/isolated-rocks-test"
mkdir -p "$TEMP_DIR"

echo "Setting up test environment..."
echo "Using RocksDB path: $TEMP_DIR"

# Run the rocks_only test with SQLX_OFFLINE=true to skip PostgreSQL connection attempts
nix develop --command bash -c "cd crates/storage && \
  RUST_BACKTRACE=1 \
  TEST_ROCKSDB_PATH=$TEMP_DIR \
  SQLX_OFFLINE=true \
  cargo test --test rocks_only -- --nocapture"

EXIT_CODE=$?

# Check if tests passed or failed
if [ $EXIT_CODE -eq 0 ]; then
  echo -e "${GREEN}✅ RocksDB tests passed!${NC}"
else
  echo -e "${RED}❌ RocksDB tests failed with exit code $EXIT_CODE${NC}"
fi

# Clean up
echo -e "${YELLOW}Cleaning up temporary directory...${NC}"
rm -rf "$TEMP_DIR"

exit $EXIT_CODE 