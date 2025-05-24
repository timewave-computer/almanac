#!/bin/bash

# Purpose: Run RocksDB tests in isolation

set -e

# Color definitions for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Running RocksDB-only Tests ===${NC}"

# Create a temporary directory for RocksDB
TMP_DIR="${PWD}/tmp/rocks-test"
ROCKSDB_DIR="${TMP_DIR}/rocksdb"
mkdir -p "${ROCKSDB_DIR}"

# Set environment variable for RocksDB path
export TEST_ROCKSDB_PATH="${ROCKSDB_DIR}"

# Run the RocksDB tests, using Nix but only running the specific RocksDB test file
echo -e "${BLUE}Running RocksDB tests...${NC}"
nix develop --command bash -c \
  "cd crates/storage && MACOSX_DEPLOYMENT_TARGET=11.0 TEST_ROCKSDB_PATH='${ROCKSDB_DIR}' cargo test --test rocks_only -- --nocapture"

# Check if the tests passed
EXIT_CODE=$?
if [ ${EXIT_CODE} -eq 0 ]; then
  echo -e "${GREEN}✓ All RocksDB tests passed!${NC}"
else
  echo -e "${RED}✗ RocksDB tests failed${NC}"
fi

# Clean up temp directory
echo "Cleaning up..."
rm -rf "${TMP_DIR}"

exit ${EXIT_CODE} 