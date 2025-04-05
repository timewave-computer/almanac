#!/usr/bin/env bash
set -euo pipefail

# Set up paths
export GOPATH="$HOME/go"
export PATH="$GOPATH/bin:$PATH"

# Check if wasmd is installed
if [ ! -f "$GOPATH/bin/wasmd" ]; then
  echo "wasmd not found. Please run wasmd-setup first."
  exit 1
fi

WASMD_CMD="$GOPATH/bin/wasmd"

# Set up wasmd test node
TEST_DIR="$HOME/.wasmd-test"
echo "Setting up wasmd test node at $TEST_DIR"

# Create test directory if it doesn't exist
mkdir -p "$TEST_DIR"

# Initialize wasmd node config if it doesn't exist
if [ ! -d "$TEST_DIR/config" ]; then
  echo "Initializing wasmd node configuration..."
  "$WASMD_CMD" init --chain-id=testing testing --home="$TEST_DIR"
  
  # Configure node
  "$WASMD_CMD" config chain-id testing --home="$TEST_DIR"
  "$WASMD_CMD" config keyring-backend test --home="$TEST_DIR"
  "$WASMD_CMD" config broadcast-mode block --home="$TEST_DIR"
  
  # Create test accounts
  "$WASMD_CMD" keys add validator --keyring-backend=test --home="$TEST_DIR"
  VALIDATOR_ADDR=$("$WASMD_CMD" keys show validator -a --keyring-backend=test --home="$TEST_DIR")
  "$WASMD_CMD" add-genesis-account "$VALIDATOR_ADDR" 1000000000stake,1000000000validatortoken --home="$TEST_DIR"
  "$WASMD_CMD" gentx validator 1000000stake --chain-id=testing --keyring-backend=test --home="$TEST_DIR"
  "$WASMD_CMD" collect-gentxs --home="$TEST_DIR"
  
  echo "Node configuration completed."
fi

# Check if a wasmd node is already running
PID_FILE="$TEST_DIR/wasmd.pid"
if [ -f "$PID_FILE" ]; then
  PID=$(cat "$PID_FILE")
  if ps -p "$PID" > /dev/null; then
    kill "$PID"
    echo "Stopped existing wasmd node (PID $PID)"
  fi
  rm -f "$PID_FILE"
fi

# Start the wasmd node
echo "Starting wasmd node..."
"$WASMD_CMD" start --home="$TEST_DIR" &
NODE_PID=$!
echo "$NODE_PID" > "$PID_FILE"

# Give node time to start up
sleep 2

# Show node status
echo "Testing node connection..."
"$WASMD_CMD" status --node=tcp://localhost:26657 | jq '.node_info.network, .sync_info.latest_block_height'

echo ""
echo "wasmd node is running! (Simulated for development)"
echo "RPC URL: http://localhost:26657"
echo "REST URL: http://localhost:1317"
echo "Chain ID: testing"
echo ""
echo "Press Ctrl+C to stop the node"
echo ""

# Wait for user to press Ctrl+C
wait "$NODE_PID" 