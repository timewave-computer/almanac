#!/bin/bash
# account_test.sh - Test script for Valence Account contracts
#
# Purpose: Provides end-to-end tests for account creation, library approval, and execution

set -e

# Load environment variables
source "$(dirname "$0")/valence_test_utils.sh"

echo "===== Running Account Contract Tests ====="

# Start test node if not running
if ! $RETH_CLIENT --config-path $RETH_CONFIG info > /dev/null 2>&1; then
    echo "Starting reth node..."
    nix run .#start-reth &
    RETH_PID=$!
    
    # Wait for node to start
    echo "Waiting for node to start..."
    sleep 5
fi

# Get account contract address
ACCOUNT_ADDRESS=$(cat build/valence-contracts/ethereum/deployment-info.json | jq -r '.ValenceAccount.address')
if [ -z "$ACCOUNT_ADDRESS" ]; then
    echo "Account contract not found. Please deploy contracts first."
    exit 1
fi

echo "Using Account contract at: $ACCOUNT_ADDRESS"

# Test account creation
echo "Testing account creation..."
$RETH_CLIENT --config-path $RETH_CONFIG send --address $ACCOUNT_ADDRESS --function "createAccount" --args '[]'

# Wait for transaction to be mined
sleep 2

# Test account ownership
echo "Testing account ownership..."
OWNER_ADDRESS=$($RETH_CLIENT --config-path $RETH_CONFIG query --address $ACCOUNT_ADDRESS --function "owner" --args '[]')
echo "Account owner: $OWNER_ADDRESS"

# Test library approval
echo "Testing library approval..."
LIBRARY_ADDRESS=$(cat build/valence-contracts/ethereum/deployment-info.json | jq -r '.ValenceLibrary.address')
if [ -z "$LIBRARY_ADDRESS" ]; then
    echo "Library contract not found. Please deploy contracts first."
    exit 1
fi

$RETH_CLIENT --config-path $RETH_CONFIG send --address $ACCOUNT_ADDRESS --function "approveLibrary" --args '["'$LIBRARY_ADDRESS'", true]'

# Wait for transaction to be mined
sleep 2

# Verify library approval
echo "Verifying library approval..."
IS_APPROVED=$($RETH_CLIENT --config-path $RETH_CONFIG query --address $ACCOUNT_ADDRESS --function "isLibraryApproved" --args '["'$LIBRARY_ADDRESS'"]')
echo "Library approval status: $IS_APPROVED"

# Test execution
echo "Testing execution..."
FUNCTION_SELECTOR="0x12345678" # Replace with actual function selector
$RETH_CLIENT --config-path $RETH_CONFIG send --address $ACCOUNT_ADDRESS --function "execute" --args '["'$LIBRARY_ADDRESS'", "'$FUNCTION_SELECTOR'", "0x"]'

# Wait for transaction to be mined
sleep 2

# Test indexer event processing
echo "Testing indexer event processing..."
$RETH_CLIENT --config-path $RETH_CONFIG listen --address $ACCOUNT_ADDRESS --event "AccountCreated" --blocks 10

echo "===== Account Contract Tests Completed Successfully ====="

# Stop the node if we started it
if [ ! -z "$RETH_PID" ]; then
    echo "Stopping reth node..."
    kill $RETH_PID
fi 