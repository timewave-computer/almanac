#!/usr/bin/env bash
set -euo pipefail

# Colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TEST_DIR=$(mktemp -d)
OSMOSIS_SOURCE="/tmp/osmosis-source-test"
BUILD_MODE="fauxmosis" # Using fauxmosis for faster tests
VALIDATORS=1
BLOCK_TIME=10
PID_FILE="$TEST_DIR/ufo-node.pid"
LOG_FILE="$TEST_DIR/ufo-node.log"
MINT_LOG_FILE1="$TEST_DIR/ufo-mint1.log"
MINT_LOG_FILE2="$TEST_DIR/ufo-mint2.log"

# Test wallet addresses (simulated)
TEST_WALLET1="osmo1exampleaddress1"
TEST_WALLET2="osmo1exampleaddress2"

echo -e "${BLUE}UFO Node End-to-End Test${NC}"
echo -e "Test directory: $TEST_DIR"

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    if [ -f "$PID_FILE" ]; then
        PID=$(cat $PID_FILE)
        if ps -p $PID > /dev/null 2>&1; then
            kill $PID
            echo -e "UFO node process $PID terminated"
        fi
    fi
    echo -e "Test logs available at: $LOG_FILE"
    echo -e "${GREEN}Cleanup complete${NC}"
}

# Set up the trap to call cleanup on exit
trap cleanup EXIT

# Test Steps
echo -e "\n${BLUE}Test Step 1: Setting up test environment${NC}"
mkdir -p "$OSMOSIS_SOURCE"
echo -e "${GREEN}✓ Test environment set up${NC}"

echo -e "\n${BLUE}Test Step 2: Starting UFO node in $BUILD_MODE mode${NC}"
scripts/run-ufo-node.sh \
    --build-mode "$BUILD_MODE" \
    --validators "$VALIDATORS" \
    --block-time "$BLOCK_TIME" \
    --osmosis-source "$OSMOSIS_SOURCE" > "$LOG_FILE" 2>&1 &

NODE_PID=$!
echo $NODE_PID > "$PID_FILE"
echo -e "${GREEN}✓ UFO node started with PID: $NODE_PID${NC}"

# Give the node some time to start up
sleep 3

# Check if the node is still running
if ! ps -p $NODE_PID > /dev/null; then
    echo -e "${RED}✗ UFO node failed to start or crashed!${NC}"
    cat "$LOG_FILE"
    exit 1
fi

echo -e "\n${BLUE}Test Step 3: Verifying node operation${NC}"
# Check the log file for expected output
if grep -q "UFO node is running" "$LOG_FILE"; then
    echo -e "${GREEN}✓ Node is running correctly${NC}"
else
    echo -e "${RED}✗ Node does not appear to be running correctly!${NC}"
    cat "$LOG_FILE"
    exit 1
fi

echo -e "\n${BLUE}Test Step 4: Waiting for blocks to be produced${NC}"
# Wait for blocks to be produced
sleep 5

# Check the log file for block production
if grep -q "Block .* produced" "$LOG_FILE"; then
    echo -e "${GREEN}✓ Blocks are being produced${NC}"
    BLOCK_COUNT=$(grep -c "Block .* produced" "$LOG_FILE")
    echo -e "   $BLOCK_COUNT blocks produced so far"
else
    echo -e "${RED}✗ No blocks being produced!${NC}"
    cat "$LOG_FILE"
    exit 1
fi

echo -e "\n${BLUE}Test Step 5: Testing faucet functionality${NC}"

# Test minting tokens to wallet 1
echo -e "   Testing minting 100 tokens to $TEST_WALLET1"
scripts/ufo-mint-tokens.sh "$TEST_WALLET1" 100 > "$MINT_LOG_FILE1" 2>&1
if grep -q "Successfully minted 100 tokens" "$MINT_LOG_FILE1"; then
    echo -e "${GREEN}✓ Tokens successfully minted to $TEST_WALLET1${NC}"
else
    echo -e "${RED}✗ Failed to mint tokens to $TEST_WALLET1!${NC}"
    cat "$MINT_LOG_FILE1"
    exit 1
fi

# Test minting tokens to wallet 2
echo -e "   Testing minting 50 tokens to $TEST_WALLET2"
scripts/ufo-mint-tokens.sh "$TEST_WALLET2" 50 > "$MINT_LOG_FILE2" 2>&1
if grep -q "Successfully minted 50 tokens" "$MINT_LOG_FILE2"; then
    echo -e "${GREEN}✓ Tokens successfully minted to $TEST_WALLET2${NC}"
else
    echo -e "${RED}✗ Failed to mint tokens to $TEST_WALLET2!${NC}"
    cat "$MINT_LOG_FILE2"
    exit 1
fi

# Test verifying balances
echo -e "   Verifying balances"
if grep -q "New balance verified: 100 UFO" "$MINT_LOG_FILE1"; then
    echo -e "${GREEN}✓ Balance for $TEST_WALLET1 verified: 100 UFO${NC}"
else
    echo -e "${RED}✗ Failed to verify balance for $TEST_WALLET1!${NC}"
    cat "$MINT_LOG_FILE1"
    exit 1
fi

if grep -q "New balance verified: 50 UFO" "$MINT_LOG_FILE2"; then
    echo -e "${GREEN}✓ Balance for $TEST_WALLET2 verified: 50 UFO${NC}"
else
    echo -e "${RED}✗ Failed to verify balance for $TEST_WALLET2!${NC}"
    cat "$MINT_LOG_FILE2"
    exit 1
fi

echo -e "\n${BLUE}Test Step 6: Testing node shutdown${NC}"
# Send signal to the node
if [ -f "$PID_FILE" ]; then
    PID=$(cat $PID_FILE)
    kill -TERM $PID
    
    # Wait for process to terminate
    for i in {1..5}; do
        if ! ps -p $PID > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Node shut down gracefully${NC}"
            break
        fi
        sleep 1
        if [ $i -eq 5 ]; then
            echo -e "${RED}✗ Node did not shut down in time!${NC}"
            exit 1
        fi
    done
else
    echo -e "${RED}✗ PID file not found!${NC}"
    exit 1
fi

# Final report
echo -e "\n${GREEN}=============================${NC}"
echo -e "${GREEN}All UFO node tests passed!${NC}"
echo -e "${GREEN}=============================${NC}"
echo -e "Test logs available at: $LOG_FILE"
echo -e "Mint logs available at: $MINT_LOG_FILE1 and $MINT_LOG_FILE2"

# Clean up test environment
rm -f "$PID_FILE"
exit 0 