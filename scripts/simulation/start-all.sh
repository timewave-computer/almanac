#!/bin/bash
# Purpose: Start all services required for development and testing

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Starting All Services ===${NC}"

# Step 1: Stop any running services first
echo -e "${BLUE}Step 1: Stopping any running services...${NC}"
bash simulation/stop-all.sh

# Create logs directory
mkdir -p logs

# Step 2: Set up PostgreSQL database
echo -e "${BLUE}Step 2: Setting up PostgreSQL database...${NC}"
bash simulation/databases/setup-postgres.sh

# Step 3: Create PostgreSQL tables
echo -e "${BLUE}Step 3: Creating PostgreSQL tables...${NC}"
bash simulation/databases/create-postgres-tables.sh

# Step 4: Set up RocksDB
echo -e "${BLUE}Step 4: Setting up RocksDB storage...${NC}"
bash simulation/databases/setup-rocksdb.sh

# Step 5: Set up Ethereum node (Anvil)
echo -e "${BLUE}Step 5: Setting up Ethereum node (Anvil)...${NC}"
bash simulation/ethereum/setup-anvil.sh

# Step 6: Set up CosmWasm node (wasmd)
echo -e "${BLUE}Step 6: Setting up CosmWasm node (wasmd)...${NC}"
bash simulation/cosmos/setup-wasmd.sh

# Final check: Verify all services are running
echo -e "${BLUE}Final check: Verifying all services are running...${NC}"

# Check PostgreSQL
if nix develop --command bash -c "pg_isready -h localhost -p 5432" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ PostgreSQL is running${NC}"
else
    echo -e "${RED}✗ PostgreSQL is not running${NC}"
    exit 1
fi

# Check RocksDB directories
if [ -d "$(pwd)/data/rocksdb" ]; then
    echo -e "${GREEN}✓ RocksDB directories are set up${NC}"
else
    echo -e "${RED}✗ RocksDB directories are not set up${NC}"
    exit 1
fi

# Check Anvil
if curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 > /dev/null; then
    echo -e "${GREEN}✓ Ethereum node (Anvil) is running${NC}"
else
    echo -e "${RED}✗ Ethereum node (Anvil) is not running${NC}"
    exit 1
fi

# Check wasmd
if curl -s http://localhost:26657/status > /dev/null; then
    echo -e "${GREEN}✓ CosmWasm node (wasmd) is running${NC}"
else
    echo -e "${RED}✗ CosmWasm node (wasmd) is not running${NC}"
    exit 1
fi

echo -e "${GREEN}=== All Services Started Successfully ===${NC}"
echo -e "${BLUE}PostgreSQL: running on localhost:5432${NC}"
echo -e "${BLUE}RocksDB: data directory at $(pwd)/data/rocksdb${NC}"
echo -e "${BLUE}Ethereum (Anvil): running on http://localhost:8545${NC}"
echo -e "${BLUE}CosmWasm (wasmd): running on http://localhost:26657${NC}"
echo -e "${YELLOW}To stop all services, run: bash simulation/stop-all.sh${NC}" 