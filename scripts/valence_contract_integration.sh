#!/usr/bin/env bash
# valence_contract_integration.sh
# Purpose: Set up Valence Protocol contracts for integration testing with Almanac

set -e

# Define colors for better output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Define paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VALENCE_DIR="$PROJECT_ROOT/valence-protocol"
COSMOS_CONTRACTS_DIR="$VALENCE_DIR/contracts"
ETH_CONTRACTS_DIR="$VALENCE_DIR/solidity"
BUILD_DIR="$PROJECT_ROOT/build/valence-contracts"
WASMD_CONFIG_DIR="$PROJECT_ROOT/config/wasmd"
ANVIL_CONFIG_DIR="$PROJECT_ROOT/config/anvil"

# Create necessary directories
mkdir -p "$BUILD_DIR"
mkdir -p "$WASMD_CONFIG_DIR"
mkdir -p "$ANVIL_CONFIG_DIR"

# Log function for better output
log() {
  local level=$1
  local message=$2
  
  case $level in
    "info")
      echo -e "${GREEN}[INFO]${NC} $message"
      ;;
    "warn")
      echo -e "${YELLOW}[WARN]${NC} $message"
      ;;
    "error")
      echo -e "${RED}[ERROR]${NC} $message"
      ;;
    *)
      echo "$message"
      ;;
  esac
}

# Check if Valence Protocol repo exists
check_valence_repo() {
  if [ ! -d "$VALENCE_DIR" ]; then
    log "error" "Valence Protocol repository not found at $VALENCE_DIR"
    log "info" "Cloning Valence Protocol repository..."
    git clone https://github.com/valence-protocol/valence.git "$VALENCE_DIR"
  else
    log "info" "Found Valence Protocol repository at $VALENCE_DIR"
    
    # Check for updates
    cd "$VALENCE_DIR"
    git fetch
    
    LOCAL=$(git rev-parse HEAD)
    REMOTE=$(git rev-parse @{u})
    
    if [ "$LOCAL" != "$REMOTE" ]; then
      log "warn" "Valence Protocol repository is not up to date."
      log "info" "Updating Valence Protocol repository..."
      git pull
    else
      log "info" "Valence Protocol repository is up to date."
    fi
  fi
}

# Configure wasmd network for Cosmos contract testing
configure_wasmd_network() {
  log "info" "Configuring wasmd network for Cosmos contract testing..."
  
  # Create wasmd configuration
  cat > "$WASMD_CONFIG_DIR/config.json" << EOL
{
  "chain_id": "wasmchain",
  "rpc_url": "http://localhost:26657",
  "api_url": "http://localhost:1317",
  "keyring_backend": "test",
  "node_home": "$HOME/.wasmd-test",
  "broadcast_mode": "block",
  "gas_prices": "0.025stake",
  "gas_adjustment": "1.3"
}
EOL

  log "info" "wasmd network configuration created at $WASMD_CONFIG_DIR/config.json"
}

# Configure Anvil for Ethereum contract testing
configure_anvil_network() {
  log "info" "Configuring Anvil for Ethereum contract testing..."
  
  # Create Anvil configuration
  cat > "$ANVIL_CONFIG_DIR/config.json" << EOL
{
  "rpc_url": "http://localhost:8545",
  "chain_id": 31337,
  "private_key": "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
  "accounts": [
    {
      "address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
      "private_key": "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
      "balance": "10000000000000000000000"
    },
    {
      "address": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
      "private_key": "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
      "balance": "10000000000000000000000"
    }
  ],
  "block_time": 1,
  "logging": true
}
EOL

  log "info" "Anvil network configuration created at $ANVIL_CONFIG_DIR/config.json"
}

# Setup Nix configuration for building contracts
setup_nix_config() {
  log "info" "Setting up Nix configuration for building Valence contracts..."
  
  # Create a valence-contracts.nix module
  cat > "$PROJECT_ROOT/nix/valence-contracts.nix" << EOL
# valence-contracts.nix - Module for building and deploying Valence contracts
{
  perSystem = { config, self', inputs', pkgs, ... }:
  let
    system = pkgs.system;
  in
  {
    # Add a package for building Valence contracts
    packages.build-valence-contracts = pkgs.writeShellScriptBin "build-valence-contracts" ''
      cd ${"\${VALENCE_DIR:-\$PWD/valence-protocol}"}
      
      # Build Ethereum contracts
      echo "Building Ethereum contracts..."
      cd solidity
      npm install
      npm run compile
      
      # Build Cosmos contracts
      echo "Building Cosmos contracts..."
      cd ../contracts
      cargo build --release --target wasm32-unknown-unknown
      
      # Optimize wasm binaries
      echo "Optimizing WASM binaries..."
      if ! command -v wasm-opt &> /dev/null; then
        echo "Installing wasm-opt..."
        npm install -g wasm-opt
      fi
      
      for wasm in \$(find ../target/wasm32-unknown-unknown/release -name "*.wasm"); do
        wasm-opt -Os \$wasm -o \$wasm
      done
      
      echo "Valence contracts built successfully"
    '';
    
    # Add an app for the Nix flake
    apps.build-valence-contracts = {
      type = "app";
      program = "\${self'.packages.build-valence-contracts}/bin/build-valence-contracts";
    };
  };
}
EOL

  log "info" "Updated Nix configuration for building Valence contracts"
}

# Create deployment scripts for test environments
create_deployment_scripts() {
  log "info" "Creating deployment scripts for test environments..."
  
  # Create script for deploying Cosmos contracts
  cat > "$SCRIPT_DIR/deploy_valence_cosmos_contracts.sh" << 'EOL'
#!/usr/bin/env bash
# deploy_valence_cosmos_contracts.sh - Deploy Valence Cosmos contracts to local wasmd node

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VALENCE_DIR="$PROJECT_ROOT/valence-protocol"
WASMD_CONFIG="$PROJECT_ROOT/config/wasmd/config.json"

# Parse wasmd config
CHAIN_ID=$(jq -r '.chain_id' "$WASMD_CONFIG")
RPC_URL=$(jq -r '.rpc_url' "$WASMD_CONFIG")
API_URL=$(jq -r '.api_url' "$WASMD_CONFIG")
KEYRING_BACKEND=$(jq -r '.keyring_backend' "$WASMD_CONFIG")
NODE_HOME=$(jq -r '.node_home' "$WASMD_CONFIG")
BROADCAST_MODE=$(jq -r '.broadcast_mode' "$WASMD_CONFIG")
GAS_PRICES=$(jq -r '.gas_prices' "$WASMD_CONFIG")

# Set environment variables for wasmd commands
export CHAIN_ID="$CHAIN_ID"
export WASMD_NODE="$RPC_URL"
export WASMD_HOME="$NODE_HOME"
export WASMD_KEYRING_BACKEND="$KEYRING_BACKEND"
export WASMD_BROADCAST_MODE="$BROADCAST_MODE"
export WASMD_GAS_PRICES="$GAS_PRICES"

# Check if wasmd node is running
if ! curl -s "$RPC_URL/status" > /dev/null; then
  echo "wasmd node is not running. Please start it with 'nix run .#wasmd-node'"
  exit 1
fi

# Get validator address
VALIDATOR_ADDR=$(wasmd keys show validator -a --keyring-backend="$KEYRING_BACKEND" --home="$NODE_HOME")
echo "Using validator address: $VALIDATOR_ADDR"

# Function to deploy a wasm contract
deploy_contract() {
  local contract_name=$1
  local wasm_file=$2
  
  echo "Storing contract: $contract_name"
  TX_HASH=$(wasmd tx wasm store "$wasm_file" \
    --from validator \
    --gas auto --gas-adjustment 1.3 \
    --gas-prices "$GAS_PRICES" \
    --broadcast-mode "$BROADCAST_MODE" \
    --chain-id "$CHAIN_ID" \
    --keyring-backend "$KEYRING_BACKEND" \
    --home "$NODE_HOME" \
    --output json -y | jq -r '.txhash')
    
  echo "Waiting for transaction to be included in a block..."
  sleep 6
  
  # Get code ID from transaction result
  CODE_ID=$(wasmd query tx "$TX_HASH" --chain-id "$CHAIN_ID" --node "$RPC_URL" --output json | \
    jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
  
  if [ -z "$CODE_ID" ]; then
    echo "Failed to get code ID for $contract_name"
    exit 1
  fi
  
  echo "Contract $contract_name stored with code ID: $CODE_ID"
  echo "$CODE_ID" > "$PROJECT_ROOT/build/valence-contracts/${contract_name}_code_id.txt"
  
  return 0
}

# Deploy core Valence contracts
echo "Deploying Valence Cosmos contracts..."

# Account contract
deploy_contract "account" "$VALENCE_DIR/target/wasm32-unknown-unknown/release/valence_account.wasm"

# Processor contract
deploy_contract "processor" "$VALENCE_DIR/target/wasm32-unknown-unknown/release/valence_processor.wasm" 

# Authorization contract
deploy_contract "authorization" "$VALENCE_DIR/target/wasm32-unknown-unknown/release/valence_authorization.wasm"

# Library contract
deploy_contract "library" "$VALENCE_DIR/target/wasm32-unknown-unknown/release/valence_library.wasm"

echo "Valence Cosmos contracts deployed successfully!"
EOL

  # Create script for deploying Ethereum contracts
  cat > "$SCRIPT_DIR/deploy_valence_ethereum_contracts.sh" << 'EOL'
#!/usr/bin/env bash
# deploy_valence_ethereum_contracts.sh - Deploy Valence Ethereum contracts to local Anvil node

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VALENCE_DIR="$PROJECT_ROOT/valence-protocol"
ANVIL_CONFIG="$PROJECT_ROOT/config/anvil/config.json"

# Parse Anvil config
RPC_URL=$(jq -r '.rpc_url' "$ANVIL_CONFIG")
PRIVATE_KEY=$(jq -r '.private_key' "$ANVIL_CONFIG")
CHAIN_ID=$(jq -r '.chain_id' "$ANVIL_CONFIG")

# Set environment variables for Ethereum commands
export ETH_RPC_URL="$RPC_URL"
export ETH_PRIVATE_KEY="$PRIVATE_KEY"
export CHAIN_ID="$CHAIN_ID"

# Check if Anvil node is running
if ! curl -s -X POST "$RPC_URL" -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' > /dev/null; then
  echo "Anvil node is not running. Please start it with 'nix run .#start-anvil'"
  exit 1
fi

# Deploy Ethereum contracts using Foundry
cd "$VALENCE_DIR/solidity"

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
  echo "Installing dependencies..."
  npm install
fi

# Compile contracts if needed
if [ ! -d "artifacts" ]; then
  echo "Compiling contracts..."
  npm run compile
fi

# Deploy contracts
echo "Deploying Valence Ethereum contracts..."
forge script scripts/DeployLocal.s.sol --rpc-url "$RPC_URL" --private-key "$PRIVATE_KEY" --broadcast

# Copy deployment artifacts to build directory
echo "Copying deployment artifacts..."
mkdir -p "$PROJECT_ROOT/build/valence-contracts/ethereum"
cp -r artifacts "$PROJECT_ROOT/build/valence-contracts/ethereum/"
cp -r deployments "$PROJECT_ROOT/build/valence-contracts/ethereum/" 2>/dev/null || true

echo "Valence Ethereum contracts deployed successfully!"
EOL

  # Make scripts executable
  chmod +x "$SCRIPT_DIR/deploy_valence_cosmos_contracts.sh"
  chmod +x "$SCRIPT_DIR/deploy_valence_ethereum_contracts.sh"
  
  log "info" "Deployment scripts created"
}

# Create network state persistence scripts
create_network_persistence() {
  log "info" "Creating network state persistence scripts..."
  
  # Create script for persisting wasmd state
  cat > "$SCRIPT_DIR/persist_wasmd_state.sh" << 'EOL'
#!/usr/bin/env bash
# persist_wasmd_state.sh - Save and restore wasmd state for reproducible testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WASMD_CONFIG="$PROJECT_ROOT/config/wasmd/config.json"
PERSIST_DIR="$PROJECT_ROOT/data/wasmd-persistence"

# Parse wasmd config
NODE_HOME=$(jq -r '.node_home' "$WASMD_CONFIG")

# Create persistence directory if it doesn't exist
mkdir -p "$PERSIST_DIR"

# Function to save wasmd state
save_state() {
  local snapshot_name=$1
  
  if [ -z "$snapshot_name" ]; then
    snapshot_name="default"
  fi
  
  echo "Saving wasmd state as '$snapshot_name'..."
  
  # Stop wasmd if it's running
  if pgrep -f "wasmd start" > /dev/null; then
    echo "Stopping wasmd..."
    pkill -f "wasmd start" || true
    sleep 2
  fi
  
  # Create snapshot directory
  local snapshot_dir="$PERSIST_DIR/$snapshot_name"
  mkdir -p "$snapshot_dir"
  
  # Copy wasmd data
  echo "Copying wasmd data to $snapshot_dir..."
  rsync -a --delete "$NODE_HOME/" "$snapshot_dir/"
  
  echo "wasmd state saved as '$snapshot_name'"
}

# Function to restore wasmd state
restore_state() {
  local snapshot_name=$1
  
  if [ -z "$snapshot_name" ]; then
    snapshot_name="default"
  fi
  
  local snapshot_dir="$PERSIST_DIR/$snapshot_name"
  
  if [ ! -d "$snapshot_dir" ]; then
    echo "Snapshot '$snapshot_name' not found"
    exit 1
  fi
  
  echo "Restoring wasmd state from '$snapshot_name'..."
  
  # Stop wasmd if it's running
  if pgrep -f "wasmd start" > /dev/null; then
    echo "Stopping wasmd..."
    pkill -f "wasmd start" || true
    sleep 2
  fi
  
  # Copy wasmd data
  echo "Copying wasmd data from $snapshot_dir..."
  rsync -a --delete "$snapshot_dir/" "$NODE_HOME/"
  
  echo "wasmd state restored from '$snapshot_name'"
}

# Function to list available snapshots
list_snapshots() {
  echo "Available wasmd state snapshots:"
  
  if [ ! -d "$PERSIST_DIR" ] || [ -z "$(ls -A "$PERSIST_DIR")" ]; then
    echo "  No snapshots found"
    return
  fi
  
  for snapshot in "$PERSIST_DIR"/*; do
    if [ -d "$snapshot" ]; then
      snapshot_name=$(basename "$snapshot")
      snapshot_date=$(stat -c "%y" "$snapshot" 2>/dev/null || stat -f "%Sm" "$snapshot")
      echo "  - $snapshot_name (created: $snapshot_date)"
    fi
  done
}

# Parse command line arguments
case "$1" in
  save)
    save_state "$2"
    ;;
  restore)
    restore_state "$2"
    ;;
  list)
    list_snapshots
    ;;
  *)
    echo "Usage: $0 {save|restore|list} [snapshot_name]"
    echo ""
    echo "Commands:"
    echo "  save [name]    - Save current wasmd state"
    echo "  restore [name] - Restore wasmd state"
    echo "  list           - List available snapshots"
    exit 1
    ;;
esac
EOL

  # Create script for persisting Anvil state
  cat > "$SCRIPT_DIR/persist_anvil_state.sh" << 'EOL'
#!/usr/bin/env bash
# persist_anvil_state.sh - Save and restore Anvil state for reproducible testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ANVIL_CONFIG="$PROJECT_ROOT/config/anvil/config.json"
PERSIST_DIR="$PROJECT_ROOT/data/anvil-persistence"

# Parse Anvil config
RPC_URL=$(jq -r '.rpc_url' "$ANVIL_CONFIG")

# Create persistence directory if it doesn't exist
mkdir -p "$PERSIST_DIR"

# Function to save Anvil state
save_state() {
  local snapshot_name=$1
  
  if [ -z "$snapshot_name" ]; then
    snapshot_name="default"
  fi
  
  echo "Saving Anvil state as '$snapshot_name'..."
  
  # Create snapshot using Anvil's built-in snapshot capability
  local snapshot_file="$PERSIST_DIR/$snapshot_name.json"
  
  # Call anvil_dumpState RPC method
  curl -s -X POST "$RPC_URL" \
    -H "Content-Type: application/json" \
    --data "{\"jsonrpc\":\"2.0\",\"method\":\"anvil_dumpState\",\"params\":[],\"id\":1}" \
    | jq '.result' > "$snapshot_file"
  
  echo "Anvil state saved as '$snapshot_name'"
}

# Function to restore Anvil state
restore_state() {
  local snapshot_name=$1
  
  if [ -z "$snapshot_name" ]; then
    snapshot_name="default"
  fi
  
  local snapshot_file="$PERSIST_DIR/$snapshot_name.json"
  
  if [ ! -f "$snapshot_file" ]; then
    echo "Snapshot '$snapshot_name' not found"
    exit 1
  fi
  
  echo "Restoring Anvil state from '$snapshot_name'..."
  
  # Call anvil_loadState RPC method
  local state=$(cat "$snapshot_file")
  curl -s -X POST "$RPC_URL" \
    -H "Content-Type: application/json" \
    --data "{\"jsonrpc\":\"2.0\",\"method\":\"anvil_loadState\",\"params\":[$state],\"id\":1}" \
    > /dev/null
  
  echo "Anvil state restored from '$snapshot_name'"
}

# Function to list available snapshots
list_snapshots() {
  echo "Available Anvil state snapshots:"
  
  if [ ! -d "$PERSIST_DIR" ] || [ -z "$(ls -A "$PERSIST_DIR")" ]; then
    echo "  No snapshots found"
    return
  fi
  
  for snapshot_file in "$PERSIST_DIR"/*.json; do
    if [ -f "$snapshot_file" ]; then
      snapshot_name=$(basename "$snapshot_file" .json)
      snapshot_date=$(stat -c "%y" "$snapshot_file" 2>/dev/null || stat -f "%Sm" "$snapshot_file")
      echo "  - $snapshot_name (created: $snapshot_date)"
    fi
  done
}

# Parse command line arguments
case "$1" in
  save)
    save_state "$2"
    ;;
  restore)
    restore_state "$2"
    ;;
  list)
    list_snapshots
    ;;
  *)
    echo "Usage: $0 {save|restore|list} [snapshot_name]"
    echo ""
    echo "Commands:"
    echo "  save [name]    - Save current Anvil state"
    echo "  restore [name] - Restore Anvil state"
    echo "  list           - List available snapshots"
    exit 1
    ;;
esac
EOL

  # Make scripts executable
  chmod +x "$SCRIPT_DIR/persist_wasmd_state.sh"
  chmod +x "$SCRIPT_DIR/persist_anvil_state.sh"
  
  log "info" "Network state persistence scripts created"
}

# Main function
main() {
  log "info" "Starting Valence Contract Integration Setup"
  
  # Check and update Valence Protocol repository
  check_valence_repo
  
  # Configure test networks
  configure_wasmd_network
  configure_anvil_network
  
  # Setup Nix configuration for building contracts
  setup_nix_config
  
  # Create deployment scripts
  create_deployment_scripts
  
  # Create network state persistence scripts
  create_network_persistence
  
  log "info" "Valence Contract Integration Setup completed successfully"
}

# Run the main function
main "$@" 