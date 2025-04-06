#!/usr/bin/env bash

set -euo pipefail

# Default values
RPC_URL="${RPC_URL:-http://localhost:8545}"
PRIVATE_KEY="${PRIVATE_KEY:-0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80}"

# Create deployment directory
DEPLOYMENT_DIR="$PWD/deployments"
mkdir -p "$DEPLOYMENT_DIR"

echo "=== Ethereum Contract Deployment ==="
echo "Network: $(cast chain --rpc-url "$RPC_URL")"
echo "Chain ID: $(cast chain-id --rpc-url "$RPC_URL")"
echo "Deployer: $(cast wallet address --private-key "$PRIVATE_KEY")"
echo "Deployer Balance: $(cast balance --rpc-url "$RPC_URL" "$(cast wallet address --private-key "$PRIVATE_KEY")" | cast --from-wei) ETH"
echo "=================================="

# Deploy Faucet contract
echo -e "\nDeploying Faucet contract..."
RESULT=$(forge create --rpc-url "$RPC_URL" \
  --private-key "$PRIVATE_KEY" \
  contracts/solidity/Faucet.sol:Faucet \
  --constructor-args \
  "$TOKEN_NAME" "$TOKEN_SYMBOL" "$TOKEN_DECIMALS" "$FAUCET_AMOUNT" \
  --broadcast)

echo "$RESULT" > "$DEPLOYMENT_DIR/faucet.json"
FAUCET_ADDRESS=$(echo "$RESULT" | jq -r '.deployedTo')
TRANSACTION_HASH=$(echo "$RESULT" | jq -r '.transactionHash')

echo "Faucet deployed to: $FAUCET_ADDRESS"
echo "Transaction hash: $TRANSACTION_HASH"
echo "Gas used: $(echo "$RESULT" | jq -r '.gasUsed')"
echo "$FAUCET_ADDRESS" > "$DEPLOYMENT_DIR/faucet_address.txt"

# Test the contract
echo -e "\nTesting the contract..."
echo "Token name: $(cast call --rpc-url "$RPC_URL" "$FAUCET_ADDRESS" "name()(string)")"
echo "Token symbol: $(cast call --rpc-url "$RPC_URL" "$FAUCET_ADDRESS" "symbol()(string)")"
echo "Token owner: $(cast call --rpc-url "$RPC_URL" "$FAUCET_ADDRESS" "owner()(address)")"
echo "Deployer balance: $(cast call --rpc-url "$RPC_URL" "$FAUCET_ADDRESS" "balanceOf(address)(uint256)" "$(cast wallet address --private-key "$PRIVATE_KEY")" | cast --from-wei) FCT"

echo -e "\nDeployment completed successfully."
echo "Contract artifacts saved to: $DEPLOYMENT_DIR/faucet.json"
echo "Contract address saved to: $DEPLOYMENT_DIR/faucet_address.txt" 