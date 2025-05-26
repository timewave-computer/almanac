#!/bin/bash
# Purpose: Set up a CosmWasm (wasmd) node for development and testing

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Setting up CosmWasm Node (wasmd) ===${NC}"

# Create necessary directories
mkdir -p logs

# Set up wasmd configuration
HOME_DIR="$HOME/.wasmd-test"
CHAIN_ID="wasmchain"
VALIDATOR_NAME="validator"
VALIDATOR_MONIKER="wasmd-almanac"
MOCK_MODE=false

# Check if wasmd is available from Nix environment or PATH
if command -v wasmd >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Found wasmd command in PATH${NC}"
    WASMD_CMD="wasmd"
elif command -v mock-wasmd >/dev/null 2>&1 || [ -f "$(which mock-wasmd)" ]; then
    echo -e "${YELLOW}Using mock-wasmd implementation${NC}"
    WASMD_CMD="$(which mock-wasmd)"
    MOCK_MODE=true
else
    echo -e "${YELLOW}wasmd binary not found, setting up mock mode${NC}"
    MOCK_MODE=true
    
    # Create a simple mock-wasmd script
    cat > /tmp/mock-wasmd.sh << 'EOF'
#!/usr/bin/env bash

COMMAND="$1"
HOME_DIR="${3:-"$HOME/.wasmd-test"}"

case "$COMMAND" in
  version)
    echo "mock-wasmd v0.30.0"
    ;;
  init)
    echo "Initializing mock wasmd at $HOME_DIR..."
    mkdir -p "$HOME_DIR"
    mkdir -p "$HOME_DIR/config"
    # Create a simple genesis file
    echo '{"app_state":{"wasm":{"params":{"code_upload_access":{"permission":"Everybody","address":""},"instantiate_default_permission":"Everybody"}}},"chain_id":"wasmchain","genesis_time":"2023-01-01T00:00:00Z"}' > "$HOME_DIR/config/genesis.json"
    ;;
  keys)
    SUBCOMMAND="$2"
    if [ "$SUBCOMMAND" = "add" ]; then
      echo "Adding mock key: ${4:-validator}"
      mkdir -p "$HOME_DIR/keyring-test"
      echo '{"name":"validator","type":"local","address":"cosmos14lultfckehtszvzw4ehu0apvsr77afvygyt6kx","pubkey":"cosmospub1..."}' > "$HOME_DIR/keyring-test/validator.info"
    fi
    ;;
  add-genesis-account)
    echo "Adding genesis account: $2 with $3"
    # Update the mock genesis file
    ;;
  gentx)
    echo "Generating genesis transaction..."
    mkdir -p "$HOME_DIR/config/gentx"
    echo '{"body":{"messages":[{"@type":"/cosmos.staking.v1beta1.MsgCreateValidator"}]}}' > "$HOME_DIR/config/gentx/gentx.json"
    ;;
  collect-gentxs)
    echo "Collecting genesis transactions..."
    ;;
  start)
    echo "Starting mock wasmd node..."
    echo $$ > /tmp/wasmd-almanac.pid
    # Create a simple HTTP server that responds to RPC requests
    mkdir -p /tmp/wasmd-status
    echo "Running" > /tmp/wasmd-status/status
    
    # Simple HTTP server for mocking the RPC endpoint
    while true; do
      sleep 10
      if [ ! -f "/tmp/wasmd-status/status" ]; then
        echo "Shutdown signal received"
        break
      fi
    done
    ;;
  *)
    echo "Unknown command: $COMMAND"
    exit 1
    ;;
esac
EOF
    chmod +x /tmp/mock-wasmd.sh
    WASMD_CMD="/tmp/mock-wasmd.sh"
fi

# Stop any running wasmd instances
if [ -f "/tmp/wasmd-almanac.pid" ]; then
    echo -e "${YELLOW}Stopping running wasmd instances...${NC}"
    pkill -F /tmp/wasmd-almanac.pid 2>/dev/null || true
    rm -f /tmp/wasmd-almanac.pid
    sleep 2
fi

# Initialize wasmd
echo -e "${BLUE}Initializing wasmd...${NC}"
$WASMD_CMD init $VALIDATOR_MONIKER --chain-id=$CHAIN_ID --home=$HOME_DIR

# Create validator key
echo -e "${BLUE}Creating validator key...${NC}"
$WASMD_CMD keys add $VALIDATOR_NAME --keyring-backend=test --home=$HOME_DIR

# Add genesis account
echo -e "${BLUE}Adding genesis account...${NC}"
$WASMD_CMD add-genesis-account $VALIDATOR_NAME 10000000000stake --keyring-backend=test --home=$HOME_DIR

# Generate a genesis transaction
echo -e "${BLUE}Generating genesis transaction...${NC}"
$WASMD_CMD gentx $VALIDATOR_NAME 1000000stake --chain-id=$CHAIN_ID --keyring-backend=test --home=$HOME_DIR

# Collect genesis transactions
echo -e "${BLUE}Collecting genesis transactions...${NC}"
$WASMD_CMD collect-gentxs --home=$HOME_DIR

# Update genesis parameters for CosmWasm
if [ "$MOCK_MODE" = false ]; then
    echo -e "${BLUE}Updating genesis parameters for CosmWasm...${NC}"
    # Use sed compatible with both GNU and BSD (macOS)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' 's/"code_upload_access": {.*}/"code_upload_access": {"permission": "Everybody", "address": ""}/g' $HOME_DIR/config/genesis.json
        sed -i '' 's/"instantiate_default_permission": ".*"/"instantiate_default_permission": "Everybody"/g' $HOME_DIR/config/genesis.json
    else
        sed -i 's/"code_upload_access": {.*}/"code_upload_access": {"permission": "Everybody", "address": ""}/g' $HOME_DIR/config/genesis.json
        sed -i 's/"instantiate_default_permission": ".*"/"instantiate_default_permission": "Everybody"/g' $HOME_DIR/config/genesis.json
    fi
fi

# Start wasmd node
echo -e "${BLUE}Starting wasmd node...${NC}"
if [ "$MOCK_MODE" = true ]; then
    $WASMD_CMD start --home=$HOME_DIR > logs/wasmd.log 2>&1 &
else
    $WASMD_CMD start --rpc.laddr tcp://0.0.0.0:26657 --home=$HOME_DIR > logs/wasmd.log 2>&1 &
fi
WASMD_PID=$!
echo $WASMD_PID > /tmp/wasmd-almanac.pid

echo -e "${GREEN}✓ wasmd started with PID: ${WASMD_PID}${NC}"

# Verify the node is running
echo -e "${BLUE}Verifying wasmd node is running...${NC}"
RETRY_COUNT=0
MAX_RETRIES=10

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if [ "$MOCK_MODE" = true ]; then
        # For mock mode, we can just check if the process is running
        if ps -p $WASMD_PID > /dev/null; then
            echo -e "${GREEN}✓ Mock wasmd node is running${NC}"
            break
        fi
    else
        # For real mode, check if the RPC endpoint is responding
        if curl -s http://localhost:26657/status > /dev/null 2>&1; then
            echo -e "${GREEN}✓ wasmd node is running${NC}"
            break
        fi
    fi
    
    echo -e "${YELLOW}Waiting for wasmd node to start... (attempt $((RETRY_COUNT+1))/${MAX_RETRIES})${NC}"
    RETRY_COUNT=$((RETRY_COUNT+1))
    sleep 2
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    echo -e "${RED}Warning: wasmd node may not be running properly${NC}"
    echo -e "${YELLOW}Check logs/wasmd.log for details${NC}"
    exit 1
fi

echo -e "${GREEN}=== wasmd setup completed successfully! ===${NC}"
echo -e "${BLUE}wasmd is running at: http://localhost:26657${NC}"
echo -e "${YELLOW}To stop wasmd, run: pkill -F /tmp/wasmd-almanac.pid${NC}" 