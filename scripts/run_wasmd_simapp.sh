#!/usr/bin/env bash
set -euo pipefail

# Define colors for better output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Define logging functions
log_info() { echo -e "${BLUE}ℹ ${NC}$1"; }
log_success() { echo -e "${GREEN}✓ ${NC}$1"; }
log_warning() { echo -e "${YELLOW}⚠ ${NC}$1"; }
log_error() { echo -e "${RED}✗ ${NC}$1"; }

# Get script directory and set paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
WASMD_HOME="${ROOT_DIR}/tmp/wasmd-simapp"

# Define the wasmd command using nix
WASMD="nix run .#wasmd-node --"

log_info "Setting up wasmd simapp at $WASMD_HOME"

# Stop any existing wasmd processes
pkill -f "wasmd" || true
log_info "Stopped any existing wasmd processes"

# Create fresh wasmd home directory
if [ -d "$WASMD_HOME" ]; then
    log_info "Found existing wasmd directory, creating backup..."
    backup_dir="${WASMD_HOME}_backup_$(date +%s)"
    mv "$WASMD_HOME" "$backup_dir"
    log_success "Backed up to $backup_dir"
fi

mkdir -p "$WASMD_HOME"
log_success "Created fresh wasmd home directory"

# Initialize wasmd chain
log_info "Initializing wasmd simapp..."
$WASMD init "${HOSTNAME:-tester}" --chain-id="simapp-1" --home="$WASMD_HOME"

# Create validator key
echo "decline lonely primary country relief six milk bleak chaos define adult junk slot shrug almost century sausage lock tumble length update brain hurdle detail" | $WASMD keys add validator --recover --keyring-backend="test" --home="$WASMD_HOME"

# Get validator address
VALIDATOR_ADDRESS=$($WASMD keys show validator -a --keyring-backend="test" --home="$WASMD_HOME")

# Add validator to genesis
$WASMD add-genesis-account "$VALIDATOR_ADDRESS" 10000000000stake --home="$WASMD_HOME"

# Create validator gentx
$WASMD gentx validator 1000000stake --chain-id="simapp-1" --keyring-backend="test" --home="$WASMD_HOME"

# Collect gentxs
$WASMD collect-gentxs --home="$WASMD_HOME"

# Update chain config
CONFIG_FILE="$WASMD_HOME/config/config.toml"
APP_CONFIG_FILE="$WASMD_HOME/config/app.toml"

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
    -e 's|laddr = "tcp://127.0.0.1:26657"|laddr = "tcp://0.0.0.0:26657"|g' \
    "$CONFIG_FILE"

# Enable API server and set minimum gas price
sed -i.bak \
    -e 's/enable = false/enable = true/g' \
    -e 's/swagger = false/swagger = true/g' \
    -e 's/enabled-unsafe-cors = false/enabled-unsafe-cors = true/g' \
    -e 's/minimum-gas-prices = ""/minimum-gas-prices = "0.025stake"/g' \
    "$APP_CONFIG_FILE"

# Fix private validator timeouts that cause the common error
PRIV_VALIDATOR_FILE="$WASMD_HOME/config/priv_validator_key.json"
PRIV_VALIDATOR_STATE="$WASMD_HOME/data/priv_validator_state.json"

if [ -f "$PRIV_VALIDATOR_FILE" ]; then
    log_info "Fixing private validator configuration..."
    chmod 600 "$PRIV_VALIDATOR_FILE"
    
    # Ensure data directory exists
    mkdir -p "$WASMD_HOME/data"
    
    # Create or fix priv_validator_state.json
    if [ ! -f "$PRIV_VALIDATOR_STATE" ]; then
        log_info "Creating priv_validator_state.json..."
        echo '{"height":"0","round":0,"step":0}' > "$PRIV_VALIDATOR_STATE"
        chmod 600 "$PRIV_VALIDATOR_STATE"
    fi
fi

# Disable the external signer socket service in config.toml
sed -i.bak \
    -e 's/priv_validator_laddr = ""/priv_validator_laddr = ""/g' \
    "$CONFIG_FILE"

# Ensure the node binds to 0.0.0.0 to allow external connections and connection checks
sed -i.bak \
    -e 's|laddr = "tcp://127.0.0.1:26657"|laddr = "tcp://0.0.0.0:26657"|g' \
    "$CONFIG_FILE"

# Start wasmd node in background
log_info "Starting wasmd node..."
$WASMD start --home="$WASMD_HOME" --rpc.unsafe > "$WASMD_HOME/node.log" 2>&1 &
WASMD_PID=$!

# Give the node more time to start
log_info "Waiting for node to start..."
sleep 30

# Test if the node is running
log_info "Testing node connection..."
if curl -s http://localhost:26657/status > /dev/null; then
    log_success "Node is running!"
    NODE_INFO=$(curl -s http://localhost:26657/status)
    CHAIN_ID=$(echo "$NODE_INFO" | grep -o '"chain_id":"[^"]*' | sed 's/"chain_id":"//g')
    log_success "Connected to chain: $CHAIN_ID"
else
    log_error "Failed to connect to wasmd node"
    kill $WASMD_PID 2>/dev/null || true
    exit 1
fi

log_success "Wasmd node is running with PID: $WASMD_PID"
echo "RPC URL: http://localhost:26657"
echo "Chain ID: simapp-1"
echo "Validator Address: $VALIDATOR_ADDRESS"

# Return with PID
echo $WASMD_PID 