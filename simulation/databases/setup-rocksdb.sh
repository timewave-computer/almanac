#!/bin/bash
# Purpose: Set up RocksDB directory for Almanac

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Setting up RocksDB for Almanac ===${NC}"

# Configuration
ROCKSDB_DIR="data/rocksdb"
PROJECT_ROOT=$(pwd)

# Create necessary directories
echo -e "${BLUE}Creating RocksDB directory...${NC}"
mkdir -p ${ROCKSDB_DIR}/ethereum
mkdir -p ${ROCKSDB_DIR}/cosmos

echo -e "${GREEN}✓ RocksDB directory created at: ${PROJECT_ROOT}/${ROCKSDB_DIR}${NC}"

# Check for existing RocksDB data
if [ "$(find ${ROCKSDB_DIR} -type f | wc -l)" -gt 0 ]; then
    echo -e "${YELLOW}Found existing RocksDB data. This is expected if you've run Almanac before.${NC}"
else
    echo -e "${GREEN}✓ Empty RocksDB directory is ready for use${NC}"
fi

echo -e "${GREEN}=== RocksDB setup completed successfully! ===${NC}"
echo -e "${BLUE}RocksDB directory: ${PROJECT_ROOT}/${ROCKSDB_DIR}${NC}" 