#!/bin/bash
set -e

WASMD_HOME="/Users/hxrts/.wasmd-test"
echo "Setting up wasmd test node at $WASMD_HOME"

# Kill any existing wasmd processes
echo "Stopping any existing wasmd processes..."
pkill -f "wasmd" || true
sleep 2

# Clean up and create directory if needed
if [ -d "$WASMD_HOME" ]; then
  echo "Found existing wasmd home directory, creating a fresh one..."
  rm -rf "$WASMD_HOME"
fi

mkdir -p "$WASMD_HOME"

# Initialize wasmd node
echo "Initializing wasmd node..."
wasmd init --chain-id=wasmchain testing --home="$WASMD_HOME"

# Configure node basic settings
wasmd config chain-id wasmchain --home="$WASMD_HOME"
wasmd config keyring-backend test --home="$WASMD_HOME"
wasmd config broadcast-mode block --home="$WASMD_HOME"
wasmd config node "tcp://127.0.0.1:26657" --home="$WASMD_HOME"

# Create test validator account
echo "Creating validator account..."
wasmd keys add validator --keyring-backend=test --home="$WASMD_HOME"
VALIDATOR_ADDR=$(wasmd keys show validator -a --keyring-backend=test --home="$WASMD_HOME")
  
echo "Adding genesis account: $VALIDATOR_ADDR"
wasmd add-genesis-account "$VALIDATOR_ADDR" 1000000000stake,1000000000validatortoken --home="$WASMD_HOME"
  
# Generate gentx
echo "Generating genesis transaction..."
wasmd gentx validator 1000000stake --chain-id=wasmchain --keyring-backend=test --home="$WASMD_HOME"
  
# Collect gentxs
wasmd collect-gentxs --home="$WASMD_HOME"

# Update app.toml
echo "Updating app.toml..."
sed -i '' 's|tcp://0.0.0.0:1317|tcp://0.0.0.0:1317|g' "$WASMD_HOME/config/app.toml"
sed -i '' 's|0.0.0.0:9090|0.0.0.0:9090|g' "$WASMD_HOME/config/app.toml"
sed -i '' 's|enabled-unsafe-cors = false|enabled-unsafe-cors = true|g' "$WASMD_HOME/config/app.toml"
sed -i '' 's|swagger = false|swagger = true|g' "$WASMD_HOME/config/app.toml"
# Set minimum gas price to avoid warnings
sed -i '' 's|minimum-gas-prices = ""|minimum-gas-prices = "0.025stake"|g' "$WASMD_HOME/config/app.toml"

# Update config.toml for better local performance and prevent validator timeout
echo "Updating config.toml for local performance..."
sed -i '' 's|timeout_commit = "5s"|timeout_commit = "1s"|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|timeout_propose = "3s"|timeout_propose = "10s"|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|timeout_precommit = "1s"|timeout_precommit = "10s"|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|timeout_prevote = "1s"|timeout_prevote = "10s"|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|skip_timeout_commit = false|skip_timeout_commit = true|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|timeout_broadcast_tx_commit = "10s"|timeout_broadcast_tx_commit = "30s"|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|addr_book_strict = true|addr_book_strict = false|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|allow_duplicate_ip = false|allow_duplicate_ip = true|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|max_num_outbound_peers = 10|max_num_outbound_peers = 5|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|max_num_inbound_peers = 40|max_num_inbound_peers = 5|g' "$WASMD_HOME/config/config.toml"
sed -i '' 's|flush_throttle_timeout = "100ms"|flush_throttle_timeout = "10ms"|g' "$WASMD_HOME/config/config.toml"

# IMPORTANT: Empty the priv_validator_laddr to disable socket mode
sed -i '' 's|priv_validator_laddr = ""|priv_validator_laddr = ""|g' "$WASMD_HOME/config/config.toml"

echo "Starting wasmd node..."
wasmd start --home "$WASMD_HOME" --rpc.laddr "tcp://0.0.0.0:26657" > "$WASMD_HOME/node.log" 2>&1 &
PID=$!
echo "Started wasmd node with PID $PID"

# Write PID to file for later reference
echo $PID > "$WASMD_HOME/wasmd.pid"

echo "Waiting for node to start..."
sleep 5

# Test if the node is responsive
if curl -s http://localhost:26657/status > /dev/null; then
  echo "Wasmd node is running successfully!"
  exit 0
else
  echo "Error: Node did not start properly. Check logs at $WASMD_HOME/node.log"
  exit 1
fi 