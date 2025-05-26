#!/bin/bash
# Purpose: Run end-to-end integration tests with all components

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Running End-to-End Integration Tests ===${NC}"

# Create logs directory
mkdir -p logs

# Step 1: Reset all databases
echo -e "${BLUE}Step 1: Resetting all databases...${NC}"
bash simulation/databases/reset-databases.sh

# Step 2: Set up Ethereum node
echo -e "${BLUE}Step 2: Setting up Ethereum node...${NC}"
bash simulation/ethereum/setup-anvil.sh

# Step 3: Set up CosmWasm node
echo -e "${BLUE}Step 3: Setting up CosmWasm node...${NC}"
bash simulation/cosmos/setup-wasmd.sh

# Step 4: Run integration tests
echo -e "${BLUE}Step 4: Running integration tests...${NC}"

# Create a function to check if services are running correctly
check_services() {
    # Check if Anvil is running
    if curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 > /dev/null; then
        echo -e "${GREEN}✓ Ethereum node (Anvil) is running${NC}"
    else
        echo -e "${RED}✗ Ethereum node (Anvil) is not running${NC}"
        return 1
    fi

    # Check if wasmd is running
    if curl -s http://localhost:26657/status > /dev/null; then
        echo -e "${GREEN}✓ CosmWasm node (wasmd) is running${NC}"
    else
        echo -e "${RED}✗ CosmWasm node (wasmd) is not running${NC}"
        return 1
    fi

    # Check if PostgreSQL is running
    if nix develop --command bash -c "pg_isready -h localhost -p 5432" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PostgreSQL is running${NC}"
    else
        echo -e "${RED}✗ PostgreSQL is not running${NC}"
        return 1
    fi

    return 0
}

# Check if all services are running
if ! check_services; then
    echo -e "${RED}Error: Some required services are not running. Cannot continue with tests.${NC}"
    exit 1
fi

# Run the cross-chain e2e test using nix develop
echo -e "${BLUE}Running cross-chain E2E test...${NC}"
nix develop --command bash -c "cargo test -p tests --features postgres -- --nocapture cross_chain_e2e" 2>&1 | tee logs/e2e-test.log

# Check if the test succeeded
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo -e "${GREEN}✓ Cross-chain E2E test succeeded${NC}"
else
    echo -e "${RED}✗ Cross-chain E2E test failed. Check logs/e2e-test.log for details.${NC}"
    exit 1
fi

# Run valence contract integration test
echo -e "${BLUE}Running Valence contract integration test...${NC}"
nix develop --command bash -c "cargo test -p tests --features postgres -- --nocapture valence_contract_integration" 2>&1 | tee -a logs/e2e-test.log

# Check if the test succeeded
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo -e "${GREEN}✓ Valence contract integration test succeeded${NC}"
else
    echo -e "${RED}✗ Valence contract integration test failed. Check logs/e2e-test.log for details.${NC}"
    exit 1
fi

echo -e "${GREEN}=== All End-to-End Integration Tests Completed Successfully ===${NC}" 