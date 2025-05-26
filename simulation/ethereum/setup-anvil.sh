#!/bin/bash
# Purpose: Set up an Anvil Ethereum node for development and testing

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Setting up Anvil Ethereum Node ===${NC}"

# Create necessary directories
mkdir -p logs

# Check if anvil is available
if ! command -v anvil >/dev/null 2>&1; then
    echo -e "${RED}Error: anvil command not found${NC}"
    echo -e "${YELLOW}Please ensure Foundry is installed and in your PATH${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Found anvil command${NC}"

# Stop any running Anvil instances
if pgrep -f "anvil" > /dev/null; then
    echo -e "${YELLOW}Stopping running Anvil instances...${NC}"
    pkill -f "anvil" || true
    sleep 2
fi

# Start Anvil node
echo -e "${BLUE}Starting Anvil node...${NC}"
anvil --host 0.0.0.0 --port 8545 > logs/anvil.log 2>&1 &
ANVIL_PID=$!
echo $ANVIL_PID > /tmp/anvil-almanac.pid

echo -e "${GREEN}✓ Anvil started with PID: ${ANVIL_PID}${NC}"

# Verify the node is running
echo -e "${BLUE}Verifying Anvil node is running...${NC}"
RETRY_COUNT=0
MAX_RETRIES=10

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    CHAIN_ID=$(curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
    
    if [ -n "$CHAIN_ID" ]; then
        echo -e "${GREEN}✓ Anvil node is running with chain ID: ${CHAIN_ID}${NC}"
        break
    else
        echo -e "${YELLOW}Waiting for Anvil node to start... (attempt $((RETRY_COUNT+1))/${MAX_RETRIES})${NC}"
        RETRY_COUNT=$((RETRY_COUNT+1))
        sleep 2
    fi
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    echo -e "${RED}Error: Anvil node failed to start or is not responding to RPC calls${NC}"
    echo -e "${YELLOW}Check logs/anvil.log for details${NC}"
    exit 1
fi

echo -e "${GREEN}=== Anvil setup completed successfully! ===${NC}"
echo -e "${BLUE}Anvil is running at: http://localhost:8545${NC}"
echo -e "${YELLOW}To stop Anvil, run: pkill -F /tmp/anvil-almanac.pid${NC}" 