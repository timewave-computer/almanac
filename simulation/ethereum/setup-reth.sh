#!/bin/bash
# Purpose: Set up a Reth Ethereum node for development and testing

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Setting up Reth Ethereum Node ===${NC}"

# Create necessary directories
mkdir -p logs
mkdir -p data/ethereum/reth

# Check if reth is available from Nix environment
if command -v reth >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Found reth command in PATH${NC}"
    RETH_CMD="reth"
    MOCK_MODE=false
elif [ -f "$HOME/.cargo/bin/reth" ]; then
    echo -e "${YELLOW}Using locally built reth command${NC}"
    RETH_CMD="$HOME/.cargo/bin/reth"
    MOCK_MODE=false
elif [ -f "$(which mock-reth)" ]; then
    echo -e "${YELLOW}Using mock-reth implementation${NC}"
    RETH_CMD="$(which mock-reth)"
    MOCK_MODE=true
else
    echo -e "${YELLOW}reth binary not found, setting up mock mode${NC}"
    MOCK_MODE=true
    
    # Create a simple mock-reth script
    cat > /tmp/mock-reth.sh << 'EOF'
#!/usr/bin/env bash

COMMAND="$1"
shift

case "$COMMAND" in
    --help)
        echo "mock-reth - Mock implementation of Reth"
        echo "Usage: mock-reth [OPTIONS]"
        ;;
    *)
        # Start a simple HTTP server on port 8545 that responds to RPC calls
        echo "Starting mock Reth node on port 8545..."
        
        # Create PID file
        echo $$ > /tmp/reth-almanac.pid
        
        # Create a netcat listener that responds to eth_chainId calls
        while true; do
            DATA=$(nc -l 8545)
            if echo "$DATA" | grep -q "eth_chainId"; then
                echo -e "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"0x539\"}"
            else
                echo -e "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"ok\"}"
            fi
        done
        ;;
esac
EOF
    chmod +x /tmp/mock-reth.sh
    RETH_CMD="/tmp/mock-reth.sh"
fi

# Stop any running Reth instances
if [ -f "/tmp/reth-almanac.pid" ]; then
    echo -e "${YELLOW}Stopping running Reth instances...${NC}"
    pkill -F /tmp/reth-almanac.pid 2>/dev/null || true
    rm -f /tmp/reth-almanac.pid
    sleep 2
fi

# Start Reth node
echo -e "${BLUE}Starting Reth node...${NC}"

if [ "$MOCK_MODE" = true ]; then
    $RETH_CMD > logs/reth.log 2>&1 &
    echo $! > /tmp/reth-almanac.pid
else
    $RETH_CMD --datadir data/ethereum/reth --dev --http --http.addr 0.0.0.0 --http.port 8545 > logs/reth.log 2>&1 &
    RETH_PID=$!
    echo $RETH_PID > /tmp/reth-almanac.pid
fi

echo -e "${GREEN}✓ Reth started with PID: $(cat /tmp/reth-almanac.pid)${NC}"

# Verify the node is running
echo -e "${BLUE}Verifying Reth node is running...${NC}"
RETRY_COUNT=0
MAX_RETRIES=10

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    CHAIN_ID=$(curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
    
    if [ -n "$CHAIN_ID" ]; then
        echo -e "${GREEN}✓ Reth node is running with chain ID: ${CHAIN_ID}${NC}"
        break
    else
        echo -e "${YELLOW}Waiting for Reth node to start... (attempt $((RETRY_COUNT+1))/${MAX_RETRIES})${NC}"
        RETRY_COUNT=$((RETRY_COUNT+1))
        sleep 2
    fi
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    echo -e "${RED}Error: Reth node failed to start or is not responding to RPC calls${NC}"
    echo -e "${YELLOW}Check logs/reth.log for details${NC}"
    exit 1
fi

echo -e "${GREEN}=== Reth setup completed successfully! ===${NC}"
echo -e "${BLUE}Reth is running at: http://localhost:8545${NC}"
echo -e "${YELLOW}To stop Reth, run: pkill -F /tmp/reth-almanac.pid${NC}" 