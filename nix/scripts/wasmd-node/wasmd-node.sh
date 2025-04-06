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

# Find wasmd command
if command -v wasmd >/dev/null 2>&1; then
  WASMD_CMD="wasmd"
  echo "Using wasmd from PATH: $(which wasmd)"
elif [ -f "$GOPATH/bin/wasmd" ]; then
  WASMD_CMD="$GOPATH/bin/wasmd"
  echo "Using wasmd from GOPATH: $WASMD_CMD"
else
  echo "wasmd not found in PATH or GOPATH. Checking if it's available through nix..."
  if nix run .#wasmd -- version >/dev/null 2>&1; then
    WASMD_CMD="nix run .#wasmd --"
    echo "Using wasmd from nix"
  else
    echo "wasmd not found. Please install wasmd or ensure it's available through nix."
  exit 1
  fi
fi

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
# **** IMPORTANT: Disabling private validator socket to avoid timeout ****
# This is intentionally left empty to use the file-based private validator
# instead of the socket-based validator that causes timeouts
# Socket address to listen on for connections from an external validator
# Default is "unix://priv_validator_socket", but we're setting it explicitly to "" to disable
priv_validator_laddr = ""

# Time interval between consensus rounds
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
EOF

# Update app.toml with correct settings for API and GRPC
echo "Updating app.toml with correct API and GRPC settings..."
sed -i.bak "s|^address = \"tcp://0.0.0.0:1317\"|address = \"tcp://0.0.0.0:$API_PORT\"|" "$APP_TOML"
sed -i.bak "s|^address = \"0.0.0.0:[0-9]*\"|address = \"0.0.0.0:$GRPC_PORT\"|" "$APP_TOML"
sed -i.bak "s|^enabled-unsafe-cors = false|enabled-unsafe-cors = true|" "$APP_TOML"

# Set minimum-gas-prices to avoid warning
sed -i.bak "s|^minimum-gas-prices = \".*\"|minimum-gas-prices = \"0stake\"|" "$APP_TOML"

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

# Start the wasmd node with explicit settings - using the --with-tendermint flag
# and explicitly disabling the private validator socket
echo "Starting wasmd node..."
nohup $WASMD_CMD start \
  --home="$WASMD_HOME" \
  --rpc.laddr="tcp://127.0.0.1:$RPC_PORT" \
  --p2p.laddr="tcp://0.0.0.0:$P2P_PORT" \
  --grpc.address="0.0.0.0:$GRPC_PORT" \
  --address="tcp://0.0.0.0:$API_PORT" \
  --priv_validator_laddr="" \
  --with-tendermint > "$WASMD_HOME/node.log" 2>&1 &

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

# Wait for RPC to become available - more time and more detailed output
echo "Waiting for RPC to become available (max 60 seconds)..."
RPC_AVAILABLE=false
for i in {1..60}; do
  if curl -s "http://127.0.0.1:$RPC_PORT/status" > /dev/null 2>&1; then
    echo "RPC is available!"
    RPC_AVAILABLE=true
    break
  fi
  echo "Waiting for RPC (attempt $i)..."
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