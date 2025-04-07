#!/bin/bash
# valence_test_utils.sh - Common variables and functions for Valence contract testing
#
# Purpose: Provides shared configuration and utility functions for all contract test scripts

# Define paths and binaries
SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build/valence-contracts"
ETHEREUM_DIR="$BUILD_DIR/ethereum"
COSMOS_DIR="$BUILD_DIR/cosmos"
RETH_CONFIG="$PROJECT_ROOT/config/reth/config.json"
RETH_CLIENT="$PROJECT_ROOT/reth_client"

# Check if reth client is available
if [ ! -f "$RETH_CLIENT" ]; then
    echo "Building reth client..."
    cd "$PROJECT_ROOT"
    cargo build --release --bin reth_client
    RETH_CLIENT="$PROJECT_ROOT/target/release/reth_client"
fi

# Define chain-specific configuration
ETHEREUM_RPC_URL=$(jq -r '.rpc_url' "$RETH_CONFIG")
ETHEREUM_WS_URL=$(jq -r '.ws_url' "$RETH_CONFIG")
ETHEREUM_CHAIN_ID=$(jq -r '.chain_id' "$RETH_CONFIG")
ETHEREUM_PRIVATE_KEY=$(jq -r '.private_key' "$RETH_CONFIG")

# Helper function to wait for transaction to be mined
wait_for_tx() {
    local tx_hash=$1
    local max_retries=${2:-30}
    local retry_count=0
    
    echo "Waiting for transaction $tx_hash to be mined..."
    
    while [ $retry_count -lt $max_retries ]; do
        local tx_receipt=$(curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_getTransactionReceipt","params":["'"$tx_hash"'"],"id":1}' $ETHEREUM_RPC_URL)
        
        if [[ $tx_receipt != *"null"* ]]; then
            local status=$(echo $tx_receipt | jq -r '.result.status')
            if [ "$status" == "0x1" ]; then
                echo "Transaction successful!"
                return 0
            elif [ "$status" == "0x0" ]; then
                echo "Transaction failed!"
                return 1
            fi
        fi
        
        retry_count=$((retry_count + 1))
        sleep 1
    done
    
    echo "Timed out waiting for transaction to be mined"
    return 1
}

# Helper function to check if contract exists
contract_exists() {
    local contract_name=$1
    local address=$(cat "$ETHEREUM_DIR/deployment-info.json" | jq -r ".$contract_name.address")
    
    if [ -z "$address" ] || [ "$address" == "null" ]; then
        return 1
    fi
    
    return 0
}

# Helper function to get contract address
get_contract_address() {
    local contract_name=$1
    local address=$(cat "$ETHEREUM_DIR/deployment-info.json" | jq -r ".$contract_name.address")
    
    if [ -z "$address" ] || [ "$address" == "null" ]; then
        echo ""
        return 1
    fi
    
    echo "$address"
    return 0
} 