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

# Set RPC and REST URLs
RPC_URL="http://localhost:26657"
REST_URL="http://localhost:1317"

# Check if wasmd node is running
log_info "Checking if wasmd node is running..."
if ! curl -s "$RPC_URL/status" > /dev/null; then
    log_error "The wasmd node does not appear to be running."
    log_info "Please run ./scripts/run_wasmd_with_fixed_timeout.sh in another terminal first."
    exit 1
fi

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    log_error "jq is not installed. Please install jq to parse JSON responses."
    exit 1
fi

# Get node status
log_info "Querying wasmd node status..."
NODE_STATUS=$(curl -s "$RPC_URL/status")
CHAIN_ID=$(echo "$NODE_STATUS" | jq -r '.result.node_info.network')
NODE_ID=$(echo "$NODE_STATUS" | jq -r '.result.node_info.id')
MONIKER=$(echo "$NODE_STATUS" | jq -r '.result.node_info.moniker')
LATEST_BLOCK_HEIGHT=$(echo "$NODE_STATUS" | jq -r '.result.sync_info.latest_block_height')
LATEST_BLOCK_TIME=$(echo "$NODE_STATUS" | jq -r '.result.sync_info.latest_block_time')
CATCHING_UP=$(echo "$NODE_STATUS" | jq -r '.result.sync_info.catching_up')

log_success "Node status retrieved!"
echo "Chain ID:           $CHAIN_ID"
echo "Node ID:            $NODE_ID"
echo "Moniker:            $MONIKER"
echo "Latest Block:       $LATEST_BLOCK_HEIGHT"
echo "Latest Block Time:  $LATEST_BLOCK_TIME"
echo "Catching Up:        $CATCHING_UP"
echo ""

# Get validator info
log_info "Querying validator information..."
VALIDATORS=$(curl -s "$RPC_URL/validators")
VALIDATOR_COUNT=$(echo "$VALIDATORS" | jq -r '.result.total')
VALIDATOR_ADDRESS=$(echo "$VALIDATORS" | jq -r '.result.validators[0].address')
VALIDATOR_PUBKEY=$(echo "$VALIDATORS" | jq -r '.result.validators[0].pub_key.value')
VALIDATOR_POWER=$(echo "$VALIDATORS" | jq -r '.result.validators[0].voting_power')

log_success "Validator information retrieved!"
echo "Validator Count:    $VALIDATOR_COUNT"
echo "Validator Address:  $VALIDATOR_ADDRESS"
echo "Validator Pubkey:   $VALIDATOR_PUBKEY"
echo "Validator Power:    $VALIDATOR_POWER"
echo ""

# Get blockchain info
log_info "Querying blockchain information..."
BLOCKCHAIN=$(curl -s "$RPC_URL/blockchain?minHeight=1&maxHeight=$LATEST_BLOCK_HEIGHT" | jq -r '.result.block_metas | length')
log_success "Found $BLOCKCHAIN blocks in the blockchain"

# Get net info
log_info "Querying network information..."
NET_INFO=$(curl -s "$RPC_URL/net_info")
LISTENING=$(echo "$NET_INFO" | jq -r '.result.listening')
PEERS_COUNT=$(echo "$NET_INFO" | jq -r '.result.n_peers')

log_success "Network information retrieved!"
echo "Listening:          $LISTENING"
echo "Connected Peers:    $PEERS_COUNT"
echo ""

# Get genesis info
log_info "Querying genesis information..."
GENESIS=$(curl -s "$RPC_URL/genesis" | jq -r '.result.genesis.app_state.bank.balances')
GENESIS_BALANCE_COUNT=$(echo "$GENESIS" | jq -r '. | length')

log_success "Genesis information retrieved!"
echo "Genesis Balances:   $GENESIS_BALANCE_COUNT"
echo ""

# Get any deployed wasm contracts (may be empty if none deployed)
log_info "Checking for deployed wasm contracts..."
CONTRACTS=$(curl -s "$REST_URL/cosmwasm/wasm/v1/code" 2>/dev/null || echo '{"code_infos":[]}')
CONTRACT_COUNT=$(echo "$CONTRACTS" | jq -r '.code_infos | length // 0')

if [ "$CONTRACT_COUNT" -gt 0 ]; then
    log_success "Found $CONTRACT_COUNT deployed contract code(s)!"
    echo "$CONTRACTS" | jq -r '.code_infos[] | "Code ID: \(.code_id), Creator: \(.creator)"'
else
    log_info "No deployed wasm contracts found (this is expected if you haven't deployed any)."
fi
echo ""

# Summary
echo "========================="
log_info "WASMD Node Summary"
echo "========================="
echo "RPC URL:            $RPC_URL"
echo "REST URL:           $REST_URL"
echo "Chain ID:           $CHAIN_ID"
echo "Latest Block:       $LATEST_BLOCK_HEIGHT"
echo "Validator Count:    $VALIDATOR_COUNT"
echo "Deployed Contracts: $CONTRACT_COUNT"
echo "========================="

log_success "wasmd node query completed!"
log_info "To deploy a contract, run: ./scripts/build_and_deploy_valence_contract.sh" 