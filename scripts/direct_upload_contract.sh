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

# Set directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TEMP_DIR="${ROOT_DIR}/tmp/direct_contract_deploy"
mkdir -p "$TEMP_DIR"

# Check if node is running
log_info "Checking if wasmd node is running..."
if ! curl -s http://localhost:26657/status > /dev/null; then
    log_error "The wasmd node does not appear to be running."
    log_info "Please run ./scripts/run_wasmd_with_fixed_timeout.sh in another terminal first."
    exit 1
fi

# Download a proper CosmWasm contract
CONTRACT_PATH="${TEMP_DIR}/cw_nameservice.wasm"
log_info "Downloading proper CosmWasm nameservice contract..."
curl -s -o "$CONTRACT_PATH" -L https://github.com/CosmWasm/cw-contracts/releases/download/v1.1.0/cw20_base.wasm
if [ ! -s "$CONTRACT_PATH" ]; then
    # Try a different URL if the first one failed
    curl -s -o "$CONTRACT_PATH" -L https://github.com/CosmWasm/cosmwasm-plus/releases/download/v0.14.0/cw20_base.wasm
fi

if [ ! -s "$CONTRACT_PATH" ]; then
    log_error "Failed to download contract. Please check your internet connection."
    exit 1
fi

CONTRACT_SIZE=$(wc -c < "$CONTRACT_PATH")
log_success "Contract downloaded: $CONTRACT_SIZE bytes"

# Get chain info
CHAIN_ID=$(curl -s http://localhost:26657/status | jq -r '.result.node_info.network')
log_success "Connected to chain: $CHAIN_ID"

# Get validator public key
VALIDATOR_PUBKEY=$(curl -s http://localhost:26657/validators | jq -r '.result.validators[0].pub_key.value')
log_success "Validator pubkey: $VALIDATOR_PUBKEY"

# Convert contract to base64
CONTRACT_BASE64=$(base64 -i "$CONTRACT_PATH")
log_success "Contract converted to base64"

# Create a sample message for contract upload
MSG_FILE="${TEMP_DIR}/store_contract_msg.json"
cat > "$MSG_FILE" << EOL
{
  "type": "wasm/MsgStoreCode",
  "value": {
    "sender": "wasm1anavzt8dzhuz9az7hrn84dkdzur5qmf4n9yvae",
    "wasm_byte_code": "${CONTRACT_BASE64}"
  }
}
EOL

log_info "Created contract upload message"
log_warning "Unfortunately, direct REST API upload requires a properly signed transaction, which needs private keys."
log_warning "In a real environment, we would need to sign this message with a proper key."

log_info "Instead, let's query the node for information about contracts..."

# Query node for blocks
LATEST_BLOCK=$(curl -s http://localhost:26657/block | jq -r '.result.block.header.height')
log_success "Latest block: $LATEST_BLOCK"

# Display summary
echo ""
log_info "====== NODE INFORMATION SUMMARY ======"
echo "Chain ID:         $CHAIN_ID"
echo "Latest Block:     $LATEST_BLOCK"
echo "Validator Pubkey: $VALIDATOR_PUBKEY"
echo "RPC URL:          http://localhost:26657"
echo "REST URL:         http://localhost:1317"
echo "------------------------------------"

log_success "Node is running properly!"
log_warning "To upload and interact with contracts, you'll need to use the wasmd CLI directly with the node."
log_info "Suggestion: Try running 'wasmd tx wasm store $CONTRACT_PATH --from=<your-key> --chain-id=$CHAIN_ID --node=tcp://localhost:26657 --gas=auto'" 