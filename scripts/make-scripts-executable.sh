#!/bin/bash
# Purpose: Make all simulation scripts executable

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Making Simulation Scripts Executable ===${NC}"

# Check if we're in the root directory of the project
if [ ! -d "simulation" ]; then
    echo -e "${RED}Error: Must run from the root directory of the project${NC}"
    exit 1
fi

# Database scripts
echo -e "${BLUE}Making database scripts executable...${NC}"
chmod +x simulation/databases/setup-postgres.sh
chmod +x simulation/databases/create-postgres-tables.sh
chmod +x simulation/databases/setup-rocksdb.sh
chmod +x simulation/databases/reset-databases.sh
chmod +x simulation/databases/populate-rocksdb.sh
echo -e "${GREEN}✓ Database scripts made executable${NC}"

# Ethereum scripts
echo -e "${BLUE}Making Ethereum scripts executable...${NC}"
chmod +x simulation/ethereum/setup-anvil.sh
chmod +x simulation/ethereum/setup-reth.sh
chmod +x simulation/ethereum/deploy-valence-contracts-anvil.sh
chmod +x simulation/ethereum/deploy-valence-contracts-reth.sh
echo -e "${GREEN}✓ Ethereum scripts made executable${NC}"

# Cosmos scripts
echo -e "${BLUE}Making Cosmos scripts executable...${NC}"
chmod +x simulation/cosmos/setup-wasmd.sh
chmod +x simulation/cosmos/deploy-valence-contracts-wasmd.sh
echo -e "${GREEN}✓ Cosmos scripts made executable${NC}"

# Almanac scripts
echo -e "${BLUE}Making Almanac scripts executable...${NC}"
chmod +x simulation/almanac/index-ethereum-anvil.sh
chmod +x simulation/almanac/index-ethereum-reth.sh
chmod +x simulation/almanac/index-cosmwasm.sh
echo -e "${GREEN}✓ Almanac scripts made executable${NC}"

# Test scripts
echo -e "${BLUE}Making test scripts executable...${NC}"
chmod +x simulation/tests/run-e2e-test.sh
echo -e "${GREEN}✓ Test scripts made executable${NC}"

# Main scripts
echo -e "${BLUE}Making main scripts executable...${NC}"
chmod +x simulation/start-all.sh
chmod +x simulation/stop-all.sh
chmod +x simulation/make-scripts-executable.sh
echo -e "${GREEN}✓ Main scripts made executable${NC}"

echo -e "${GREEN}✓ All simulation scripts are now executable${NC}"
echo -e "${BLUE}You can now run any script using ./simulation/script-name.sh${NC}" 