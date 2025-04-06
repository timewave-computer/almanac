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
WASMD_HOME="$HOME/.wasmd-test"
TEMP_DIR="${ROOT_DIR}/tmp/contract_deploy"
mkdir -p "$TEMP_DIR"

# Check if wasmd-node is available
if ! command -v wasmd-node &> /dev/null; then
    log_error "wasmd-node command not found. Make sure it's installed via nix."
    exit 1
fi

# Check if the wasmd node is running
log_info "Checking if wasmd node is running..."
if ! curl -s http://localhost:26657/status > /dev/null; then
    log_error "The wasmd node does not appear to be running."
    log_info "Please run ./scripts/run_wasmd_with_fixed_timeout.sh in another terminal first."
    exit 1
fi

# Get chain information
CHAIN_ID=$(curl -s http://localhost:26657/status | jq -r '.result.node_info.network')
log_success "Connected to wasmd node with Chain ID: $CHAIN_ID"

# Get validator information
log_info "Getting validator address..."
VALIDATOR_ADDR=$(wasmd-node keys show validator -a --keyring-backend=test --home="$WASMD_HOME")
log_success "Validator address: $VALIDATOR_ADDR"

# Download sample contract if not exists
CONTRACT_PATH="${TEMP_DIR}/cw_nameservice.wasm"
if [ ! -f "$CONTRACT_PATH" ]; then
  log_info "Downloading sample CosmWasm nameservice contract..."
  CONTRACT_URL="https://github.com/CosmWasm/cw-examples/releases/download/v0.14.0/cw_nameservice.wasm"
  if ! curl -L -o "$CONTRACT_PATH" "$CONTRACT_URL"; then
    log_error "Failed to download contract from $CONTRACT_URL"
    exit 1
  fi
  log_success "Contract downloaded to $CONTRACT_PATH"
else
  log_info "Using existing contract at $CONTRACT_PATH"
fi

# Get contract size and hash for verification
CONTRACT_SIZE=$(wc -c < "$CONTRACT_PATH")
CONTRACT_HASH=$(shasum -a 256 "$CONTRACT_PATH" | cut -d ' ' -f 1)
log_info "Contract size: $CONTRACT_SIZE bytes"
log_info "Contract hash: $CONTRACT_HASH"

# Store the contract on chain
log_info "Storing contract on chain..."
TX_STORE=$(wasmd-node tx wasm store "$CONTRACT_PATH" \
  --from validator \
  --chain-id="$CHAIN_ID" \
  --gas="auto" \
  --gas-adjustment=1.3 \
  --fees="5000stake" \
  --broadcast-mode=block \
  --keyring-backend=test \
  --home="$WASMD_HOME" \
  -y)

# Extract code ID
CODE_ID=$(echo "$TX_STORE" | grep -A1 'code_id:' | tail -n1 | tr -d "[:space:]\"" || echo "")
if [ -z "$CODE_ID" ]; then
    CODE_ID=$(echo "$TX_STORE" | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
fi

if [ -z "$CODE_ID" ]; then
    log_error "Failed to extract code ID from transaction response."
    echo "$TX_STORE"
    exit 1
fi

log_success "Contract uploaded with code ID: $CODE_ID"

# Instantiate the contract
log_info "Instantiating contract..."
INIT_MSG='{"purchase_price":{"amount":"100","denom":"stake"},"transfer_price":{"amount":"999","denom":"stake"}}'
TX_INIT=$(wasmd-node tx wasm instantiate "$CODE_ID" "$INIT_MSG" \
  --from validator \
  --label "name service" \
  --chain-id="$CHAIN_ID" \
  --gas="auto" \
  --gas-adjustment=1.3 \
  --fees="5000stake" \
  --broadcast-mode=block \
  --admin="$VALIDATOR_ADDR" \
  --keyring-backend=test \
  --home="$WASMD_HOME" \
  -y)

# Extract contract address
TXHASH=$(echo "$TX_INIT" | grep -A1 'txhash:' | tail -n1 | tr -d "[:space:]\"" || echo "")
if [ -z "$TXHASH" ]; then
    TXHASH=$(echo "$TX_INIT" | jq -r '.txhash')
fi

if [ -z "$TXHASH" ]; then
    log_error "Failed to extract txhash from instantiation response."
    echo "$TX_INIT"
    exit 1
fi

log_info "Querying contract address from transaction hash: $TXHASH..."
sleep 2  # Give the chain a moment to process
TX_RESULT=$(wasmd-node query tx "$TXHASH" --chain-id="$CHAIN_ID" --home="$WASMD_HOME" -o json)
CONTRACT_ADDR=$(echo "$TX_RESULT" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="contract_address") | .value')

if [ -z "$CONTRACT_ADDR" ]; then
    log_error "Failed to extract contract address from transaction."
    echo "$TX_RESULT"
    exit 1
fi

log_success "Contract instantiated at address: $CONTRACT_ADDR"

# Execute a function (register a name)
log_info "Executing contract function to register a name 'alice'..."
EXECUTE_MSG='{"register":{"name":"alice"}}'
TX_EXEC=$(wasmd-node tx wasm execute "$CONTRACT_ADDR" "$EXECUTE_MSG" \
  --from validator \
  --chain-id="$CHAIN_ID" \
  --gas="auto" \
  --gas-adjustment=1.3 \
  --fees="5000stake" \
  --amount="100stake" \
  --broadcast-mode=block \
  --keyring-backend=test \
  --home="$WASMD_HOME" \
  -y)

# Extract execution txhash for verification
EXEC_TXHASH=$(echo "$TX_EXEC" | grep -A1 'txhash:' | tail -n1 | tr -d "[:space:]\"" || echo "")
if [ -z "$EXEC_TXHASH" ]; then
    EXEC_TXHASH=$(echo "$TX_EXEC" | jq -r '.txhash')
fi

if [ -z "$EXEC_TXHASH" ]; then
    log_warning "Could not extract execution transaction hash, but transaction may have succeeded."
else
    log_success "Contract function executed successfully with txhash: $EXEC_TXHASH"
fi

# Query contract state
log_info "Querying contract state..."
QUERY_RESULT=$(wasmd-node query wasm contract-state all "$CONTRACT_ADDR" \
  --chain-id="$CHAIN_ID" \
  --home="$WASMD_HOME" \
  -o json)

# Pretty print query result
echo "$QUERY_RESULT" | jq . || log_warning "Could not parse query result as JSON"

# Query specific record
log_info "Querying the record for 'alice'..."
QUERY_MSG='{"resolve_record":{"name":"alice"}}'
RESOLVE_RESULT=$(wasmd-node query wasm contract-state smart "$CONTRACT_ADDR" "$QUERY_MSG" \
  --chain-id="$CHAIN_ID" \
  --home="$WASMD_HOME" \
  -o json)

echo "$RESOLVE_RESULT" | jq . || log_warning "Could not parse resolve query result as JSON"

# Print summary
echo ""
log_info "====== CONTRACT DEPLOYMENT SUMMARY ======"
echo "Chain ID:         $CHAIN_ID"
echo "Validator:        $VALIDATOR_ADDR"
echo "Contract Code ID: $CODE_ID"
echo "Contract Address: $CONTRACT_ADDR"
echo "Contract Size:    $CONTRACT_SIZE bytes"
echo "Contract Hash:    $CONTRACT_HASH"
echo "RPC URL:          http://localhost:26657"
echo "REST URL:         http://localhost:1317"
echo "------------------------------------"

log_success "Real contract deployment and execution completed!"
log_info "You can interact with this contract using wasmd-node and the contract address above." 