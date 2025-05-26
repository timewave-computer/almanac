#!/bin/bash
# Purpose: Reset both PostgreSQL and RocksDB databases

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Resetting All Databases ===${NC}"

# Stop any running PostgreSQL instances
echo -e "${BLUE}Stopping any running PostgreSQL instances...${NC}"
nix develop --command bash -c "stop_databases || true"

# Remove existing data directories
echo -e "${BLUE}Removing existing data directories...${NC}"
rm -rf data/postgres
rm -rf data/rocksdb
echo -e "${GREEN}✓ Removed old data directories${NC}"

# Initialize databases
echo -e "${BLUE}Setting up PostgreSQL server...${NC}"
./simulation/databases/setup-postgres.sh

# Create and populate tables
echo -e "${BLUE}Creating PostgreSQL tables...${NC}"
./simulation/databases/create-postgres-tables.sh

# Set up RocksDB
echo -e "${BLUE}Setting up RocksDB storage...${NC}"
./simulation/databases/setup-rocksdb.sh

echo -e "${GREEN}=== All Databases Reset Successfully ===${NC}"
echo -e "${GREEN}PostgreSQL is running at localhost:5432${NC}"
echo -e "${GREEN}RocksDB storage is ready at $(pwd)/data/rocksdb${NC}" 