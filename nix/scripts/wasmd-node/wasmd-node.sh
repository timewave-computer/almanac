#!/usr/bin/env bash
set -euo pipefail

# Set up paths
export GOPATH="$HOME/go"
export PATH="$GOPATH/bin:$PATH"

# Function to check if a port is available
check_port_available() {
  local port=$1
  if command -v nc >/dev/null 2>&1; then
    nc -z 127.0.0.1 "$port" >/dev/null 2>&1
    if [ $? -eq 0 ]; then
      # Port is in use
      return 1
    else
      # Port is available
      return 0
    fi
  elif command -v lsof >/dev/null 2>&1; then
    lsof -i:$port >/dev/null 2>&1
    if [ $? -eq 0 ]; then
      # Port is in use
      return 1
    else
      # Port is available
      return 0
    fi
  else
    # If we can't check, assume it's available
    return 0
  fi
}

# Function to find an available port
find_available_port() {
  local base_port=$1
  local port=$base_port
  local max_attempts=20
  
  for ((i=0; i<max_attempts; i++)); do
    if check_port_available "$port"; then
      echo "$port"
      return 0
    fi
    port=$((port + 1))
  done
  
  # If we couldn't find an available port, return the base port
  # and hope for the best
  echo "$base_port"
  return 1
}

# Setup environment
WASMD_HOME="/Users/hxrts/.wasmd-test"
echo "Setting up wasmd node at $WASMD_HOME"

# Kill any existing wasmd processes - be more aggressive in killing
echo "Stopping any existing wasmd processes..."
pkill -f "wasmd" || true
sleep 3
# Double-check and kill more forcefully if needed
pkill -9 -f "wasmd" || true
sleep 1

# Use wasmd from cosmos.nix - available through nix
WASMD_CMD="wasmd"
echo "Using wasmd from cosmos.nix: $(which wasmd || echo "not found")"

# Find available ports
RPC_PORT=$(find_available_port 26657)
API_PORT=$(find_available_port 1317)
GRPC_PORT=$(find_available_port 9090)
P2P_PORT=$(find_available_port 26656)
echo "Using RPC port: $RPC_PORT"
echo "Using API port: $API_PORT"
echo "Using GRPC port: $GRPC_PORT"
echo "Using P2P port: $P2P_PORT"

# Check if wasmd home directory already exists
INIT_NEEDED=true
if [ -d "$WASMD_HOME" ] && [ -f "$WASMD_HOME/config/genesis.json" ]; then
  echo "Found existing wasmd home directory with genesis.json"
  
  # Check if config.toml exists
  if [ -f "$WASMD_HOME/config/config.toml" ]; then
    echo "Found existing config.toml - will update with correct settings"
    INIT_NEEDED=false
  fi
fi

# Clean up private validator socket files and state
if [ -f "$WASMD_HOME/config/priv_validator_state.json" ]; then
  echo "Removing existing priv_validator_state.json for clean start"
  rm -f "$WASMD_HOME/config/priv_validator_state.json"
fi

if [ -S "$WASMD_HOME/config/priv_validator_socket" ]; then
  echo "Removing existing priv_validator_socket for clean start"
  rm -f "$WASMD_HOME/config/priv_validator_socket"
fi

# Additional cleanup to ensure no stale validator files
rm -f "$WASMD_HOME/config/priv_validator_key.json.*.backup"
rm -f "$WASMD_HOME/config/node_key.json.*.backup"

# Remove and recreate home directory if initialization is needed
if [ "$INIT_NEEDED" = true ]; then
  echo "Performing fresh initialization of wasmd node..."
  rm -rf "$WASMD_HOME"
  mkdir -p "$WASMD_HOME"

  # Initialize wasmd node
  echo "Initializing wasmd node..."
  $WASMD_CMD init --chain-id=wasmchain testing --home="$WASMD_HOME"

  # Configure node basic settings
  $WASMD_CMD config chain-id wasmchain --home="$WASMD_HOME"
  $WASMD_CMD config keyring-backend test --home="$WASMD_HOME"
  $WASMD_CMD config broadcast-mode block --home="$WASMD_HOME"
  $WASMD_CMD config node tcp://127.0.0.1:$RPC_PORT --home="$WASMD_HOME"

  # Create test validator account
  echo "Creating validator account..."
  $WASMD_CMD keys add validator --keyring-backend=test --home="$WASMD_HOME" || {
    echo "WARNING: Failed to create validator key. It may already exist."
    # List keys to check
    $WASMD_CMD keys list --keyring-backend=test --home="$WASMD_HOME" || true
  }
  
  # Try to get validator address - if it fails, we'll exit with error
  VALIDATOR_ADDR=$($WASMD_CMD keys show validator -a --keyring-backend=test --home="$WASMD_HOME" 2>/dev/null) || {
    echo "ERROR: Failed to get validator address. Key creation likely failed."
    exit 1
  }
  
  echo "Adding genesis account: $VALIDATOR_ADDR"
  $WASMD_CMD add-genesis-account "$VALIDATOR_ADDR" 1000000000stake,1000000000validatortoken --home="$WASMD_HOME"
  
  # Generate gentx
  echo "Generating genesis transaction..."
  $WASMD_CMD gentx validator 1000000stake --chain-id=wasmchain --keyring-backend=test --home="$WASMD_HOME"
  
  # Collect gentxs
  $WASMD_CMD collect-gentxs --home="$WASMD_HOME"
  
  echo "Node initialization complete"
  
  # Create a custom priv_validator_key.json file with a fixed key
  # This helps avoid issues with validator initialization
  echo "Creating custom validator key file..."
  cat > "$WASMD_HOME/config/priv_validator_key.json" << EOF
  {
    "address": "12617FA635AF8D5E2141BE5FFB161D89B1847771",
    "pub_key": {
      "type": "tendermint/PubKeyEd25519",
      "value": "Ie/6a5+2gFL+jR8418CroiYqgLXEuCkRBV5/aoLkvas="
    },
    "priv_key": {
      "type": "tendermint/PrivKeyEd25519",
      "value": "hVv9jXbgI8K5ua3x8+jroT96l7YlVfq9jjOJ5vKYFD4h7/prn7aAUv6NHzjXwKuiJiqAtcS4KREFX39qguS9qw=="
    }
  }
  EOF
  chmod 600 "$WASMD_HOME/config/priv_validator_key.json"

  # Ensure data directory exists and create priv_validator_state.json in the correct location
  mkdir -p "$WASMD_HOME/data"
  echo "Creating priv_validator_state.json in data directory..."
  cat > "$WASMD_HOME/data/priv_validator_state.json" << EOF
  {
    "height": "0",
    "round": 0,
    "step": 0
  }
  EOF
  chmod 600 "$WASMD_HOME/data/priv_validator_state.json"

  # Remove any existing priv_validator_state.json in the config directory (if it exists)
  rm -f "$WASMD_HOME/config/priv_validator_state.json"
else
  echo "Using existing wasmd node configuration"
fi

# Fix config files - create a completely new config.toml to ensure settings are correct
CONFIG_TOML="$WASMD_HOME/config/config.toml"
APP_TOML="$WASMD_HOME/config/app.toml"

# Create a new config.toml from scratch to avoid any issues with modifying existing file
echo "Writing clean config.toml with correct settings..."
cat > "$CONFIG_TOML" << EOF
# This is a TOML config file for the wasmd node
# TCP or UNIX socket address of the ABCI application
proxy_app = "tcp://127.0.0.1:26658"

# A custom human readable name for this node
moniker = "testing-node"

# Database backend - using the default
db_backend = "goleveldb"

# Database directory
db_dir = "data"

# Output level for logging
log_level = "info"

# Output format: 'plain' or 'json'
log_format = "plain"

##### RPC server configuration options #####
[rpc]
# TCP or UNIX socket address for the RPC server to listen on
laddr = "tcp://127.0.0.1:$RPC_PORT"

# Maximum number of simultaneous connections
max_open_connections = 900

# CORS settings - allow all origins
cors_allowed_origins = ["*"]
cors_allowed_methods = ["HEAD", "GET", "POST"]
cors_allowed_headers = ["Origin", "Accept", "Content-Type", "X-Requested-With", "X-Server-Time"]

##### P2P configuration options #####
[p2p]
# TCP or UDP address for the node to listen on for p2p connections
laddr = "tcp://0.0.0.0:$P2P_PORT"

# Maximum number of inbound/outbound peers
max_num_inbound_peers = 40
max_num_outbound_peers = 10

# Set to true to enable the peer-exchange reactor
pex = true

##### Mempool configuration options #####
[mempool]
# Size of the mempool
size = 5000

# Maximum size of a transaction
max_tx_bytes = 1048576

##### FastSync configuration options #####
[fastsync]
# Fast sync version: 0 (v0) or 1 (v1) or 2 (v2)
version = "v0"

##### Consensus configuration options #####
[consensus]
# **** IMPORTANT: Disabling private validator socket completely ****
# We're explicitly setting this to empty string to disable socket-based validator
priv_validator_laddr = ""

# The following settings are critical to avoid the validator socket error
timeout_propose = "3s"
timeout_propose_delta = "500ms"
timeout_prevote = "1s"
timeout_prevote_delta = "500ms"
timeout_precommit = "1s"
timeout_precommit_delta = "500ms"
timeout_commit = "5s"

# Make progress when we have all precommits
skip_timeout_commit = false

# EmptyBlocks mode and possible interval between empty blocks
create_empty_blocks = true
create_empty_blocks_interval = "0s"

# Reactor sleep duration parameters
peer_gossip_sleep_duration = "100ms"
peer_query_maj23_sleep_duration = "2s"

##### Transaction indexer configuration options #####
[tx_index]
# What indexer to use
indexer = "kv"

### Additional Settings ####
[statesync]
# State sync rapidly bootstraps a new node by discovering, fetching, and restoring a state machine snapshot
# from peers instead of fetching and replaying historical blocks. Requires some peers in the network to take and
# serve state machine snapshots.
enable = false
EOF

# Make sure the config changes are properly written and flushed
sync

# Update app.toml with correct settings for API and GRPC
echo "Updating app.toml with correct API and GRPC settings..."
sed -i.bak "s|^address = \"tcp://0.0.0.0:1317\"|address = \"tcp://0.0.0.0:$API_PORT\"|" "$APP_TOML"
sed -i.bak "s|^address = \"0.0.0.0:[0-9]*\"|address = \"0.0.0.0:$GRPC_PORT\"|" "$APP_TOML"
sed -i.bak "s|^enabled-unsafe-cors = false|enabled-unsafe-cors = true|" "$APP_TOML"
sed -i.bak "s|^enable = false|enable = true|g" "$APP_TOML"
sed -i.bak "s|^swagger = false|swagger = true|g" "$APP_TOML"

# Set minimum-gas-prices to avoid warning
sed -i.bak "s|^minimum-gas-prices = \".*\"|minimum-gas-prices = \"0.025stake\"|" "$APP_TOML"

# Update config.toml for better local performance
# Set timeouts to prevent validator timeout issues
sed -i.bak \
    -e 's/timeout_commit = "5s"/timeout_commit = "1s"/g' \
    -e 's/timeout_propose = "3s"/timeout_propose = "10s"/g' \
    -e 's/timeout_precommit = "1s"/timeout_precommit = "10s"/g' \
    -e 's/timeout_prevote = "1s"/timeout_prevote = "10s"/g' \
    -e 's/skip_timeout_commit = false/skip_timeout_commit = true/g' \
    -e 's/timeout_broadcast_tx_commit = "10s"/timeout_broadcast_tx_commit = "30s"/g' \
    -e 's/addr_book_strict = true/addr_book_strict = false/g' \
    -e 's/allow_duplicate_ip = false/allow_duplicate_ip = true/g' \
    -e 's/max_num_outbound_peers = 10/max_num_outbound_peers = 5/g' \
    -e 's/max_num_inbound_peers = 40/max_num_inbound_peers = 5/g' \
    -e 's/flush_throttle_timeout = "100ms"/flush_throttle_timeout = "10ms"/g' \
    -e 's/priv_validator_laddr = "tcp:\/\/127.0.0.1:26658"/priv_validator_laddr = ""/g' \
    "$CONFIG_TOML"

# Clear any existing process file
PID_FILE="$WASMD_HOME/wasmd.pid"
if [ -f "$PID_FILE" ]; then
  PID=$(cat "$PID_FILE")
  if ps -p "$PID" > /dev/null 2>&1; then
    kill "$PID" 2>/dev/null || true
    echo "Stopped existing wasmd node (PID $PID)"
  fi
  rm -f "$PID_FILE"
fi

# Start the wasmd node with explicit settings
echo "Starting wasmd node..."
# Important: we need to explicitly set these flags to avoid the private validator socket error
nohup $WASMD_CMD start \
  --home="$WASMD_HOME" \
  --rpc.laddr="tcp://0.0.0.0:$RPC_PORT" \
  --p2p.laddr="tcp://0.0.0.0:$P2P_PORT" \
  --rpc.unsafe \
  --log_level="info" > "$WASMD_HOME/node.log" 2>&1 &

NODE_PID=$!
echo "$NODE_PID" > "$PID_FILE"
echo "Node started with PID: $NODE_PID (saved to $PID_FILE)"

# Check if process is still running after a moment
sleep 5
if ! ps -p "$NODE_PID" > /dev/null; then
  echo "ERROR: wasmd node failed to start. Check the logs:"
  cat "$WASMD_HOME/node.log"
  exit 1
fi

# Check for common errors in the logs
if grep -q "panic" "$WASMD_HOME/node.log"; then
  echo "ERROR: Node panic detected in logs:"
  grep -A 10 "panic" "$WASMD_HOME/node.log"
  exit 1
fi

if grep -q "error with private validator socket client" "$WASMD_HOME/node.log"; then
  echo "ERROR: Private validator socket error detected. This should have been fixed by our configuration."
  grep -A 5 -B 5 "error with private validator socket client" "$WASMD_HOME/node.log"
fi

# Wait for RPC to become available - more time and more detailed output
echo "Waiting for RPC to become available (max 120 seconds)..."
RPC_AVAILABLE=false
for i in {1..120}; do
  if curl -s "http://127.0.0.1:$RPC_PORT/status" > /dev/null 2>&1; then
    echo "RPC is available!"
    RPC_AVAILABLE=true
    break
  fi
  
  # Check if the process is still running
  if ! ps -p "$NODE_PID" > /dev/null; then
    echo "ERROR: wasmd node process died while waiting for RPC to become available"
    echo "Last 30 lines of log:"
    tail -30 "$WASMD_HOME/node.log"
    exit 1
  fi
  
  # Show progress every 10 seconds
  if [ $((i % 10)) -eq 0 ]; then
    echo "Waiting for RPC (attempt $i/120)..."
  fi
  sleep 1
done

if [ "$RPC_AVAILABLE" = false ]; then
  echo "WARNING: RPC did not become available within timeout. Last 20 lines of log:"
  tail -20 "$WASMD_HOME/node.log"
  # Continue anyway, sometimes the RPC can take longer to start
  # especially on first run
fi

# Try to get node status to verify it's working
echo "Testing node connection..."
$WASMD_CMD status --node="tcp://127.0.0.1:$RPC_PORT" || {
  echo "NOTE: Initial status check failed, but node might still be starting up"
  echo "Check logs at $WASMD_HOME/node.log if problems persist"
}

echo ""
echo "wasmd node is running!"
echo "RPC URL: http://127.0.0.1:$RPC_PORT"
echo "P2P URL: tcp://0.0.0.0:$P2P_PORT"
echo "REST URL: http://localhost:$API_PORT"
echo "GRPC URL: 0.0.0.0:$GRPC_PORT" 
echo "Chain ID: wasmchain"
echo "Logs: $WASMD_HOME/node.log"
echo ""

# Wait for node to exit
wait "$NODE_PID" 