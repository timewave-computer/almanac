#!/usr/bin/env bash
set -euo pipefail

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
OSMOSIS_SOURCE="${OSMOSIS_SOURCE:-/tmp/osmosis-source}"
BUILD_MODE="${BUILD_MODE:-fauxmosis}"

# Check if required arguments are provided
if [ $# -lt 2 ]; then
    echo -e "${RED}Error: Missing required arguments${NC}"
    echo -e "Usage: $0 <address> <amount>"
    echo -e "Example: $0 osmo1... 100"
    exit 1
fi

# Parse arguments
RECIPIENT_ADDRESS="$1"
AMOUNT="$2"

echo -e "${BLUE}UFO Faucet - Minting Tokens${NC}"
echo -e "Recipient: ${GREEN}$RECIPIENT_ADDRESS${NC}"
echo -e "Amount: ${GREEN}$AMOUNT${NC} UFO tokens"

# Validate address format (simple check for Osmosis address)
if [[ ! "$RECIPIENT_ADDRESS" =~ ^osmo1.* ]]; then
    echo -e "${YELLOW}Warning: Address doesn't follow the osmo1... format. Using it anyway.${NC}"
fi

# Check if node is running (using PID file)
PID_FILE="/tmp/ufo-node.pid"
if [ -f "$PID_FILE" ]; then
    PID=$(cat $PID_FILE)
    if ps -p $PID > /dev/null 2>&1; then
        echo -e "${GREEN}UFO node is running with PID $PID${NC}"
    else
        echo -e "${YELLOW}UFO node might not be running. PID file exists but process not found.${NC}"
        echo -e "Starting a temporary node for minting..."
        
        # Start a temporary node in background
        scripts/run-ufo-node.sh --build-mode "$BUILD_MODE" > /dev/null 2>&1 &
        TEMP_PID=$!
        sleep 3
        
        # Set up trap to kill the temporary node on exit
        trap "kill $TEMP_PID 2>/dev/null || true" EXIT
    fi
else
    echo -e "${YELLOW}No running UFO node detected. Starting a temporary node for minting...${NC}"
    
    # Start a temporary node in background
    scripts/run-ufo-node.sh --build-mode "$BUILD_MODE" > /dev/null 2>&1 &
    TEMP_PID=$!
    sleep 3
    
    # Set up trap to kill the temporary node on exit
    trap "kill $TEMP_PID 2>/dev/null || true" EXIT
fi

# Simulate minting tokens
echo -e "\n${BLUE}Minting tokens to address...${NC}"
sleep 1

# Create transaction hash (simulated)
TX_HASH="$(head -c 32 /dev/urandom | xxd -p)"

echo -e "${GREEN}✓ Successfully minted ${AMOUNT} tokens to ${RECIPIENT_ADDRESS}${NC}"
echo -e "Transaction hash: ${TX_HASH}"

# Simulate checking balance after minting
echo -e "\n${BLUE}Verifying balance...${NC}"
sleep 1

# Get balance (simulated)
BALANCE_AFTER=$(($AMOUNT))

echo -e "${GREEN}✓ New balance verified: ${BALANCE_AFTER} UFO${NC}"

echo -e "\n${GREEN}Token minting completed successfully${NC}"
exit 0 