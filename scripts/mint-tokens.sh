#!/usr/bin/env bash

set -euo pipefail

# Default values
RPC_URL="${RPC_URL:-http://localhost:8545}"
PRIVATE_KEY="${PRIVATE_KEY:-0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80}"

if [ $# -lt 2 ]; then
  echo "Usage: $0 <recipient-address> <amount>"
  echo "Example: $0 0x70997970C51812dc3A010C7d01b50e0d17dc79C8 1.5"
  exit 1
fi

RECIPIENT=$1
AMOUNT=$2

# Convert amount to wei if it contains a decimal point
if [[ "$AMOUNT" == *.* ]]; then
  AMOUNT_WEI=$(cast --to-wei "$AMOUNT")
else
  AMOUNT_WEI="$AMOUNT"
fi

if [ ! -f "./deployments/faucet_address.txt" ]; then
  echo "Faucet not deployed yet. Run deploy-ethereum-contracts first."
  exit 1
fi

FAUCET_ADDRESS=$(cat ./deployments/faucet_address.txt)
SENDER=$(cast wallet address --private-key "$PRIVATE_KEY")

echo "=== Faucet Token Mint ==="
echo "Network: $(cast chain --rpc-url "$RPC_URL")"
echo "Chain ID: $(cast chain-id --rpc-url "$RPC_URL")"
echo "Faucet Address: $FAUCET_ADDRESS"
echo "Sender: $SENDER"
echo "Sender Balance: $(cast balance --rpc-url "$RPC_URL" "$SENDER" | cast --from-wei) ETH"
echo "Recipient: $RECIPIENT"
echo "Amount: $AMOUNT (${AMOUNT_WEI} wei)"
echo "Previous Token Balance: $(cast call --rpc-url "$RPC_URL" "$FAUCET_ADDRESS" "balanceOf(address)(uint256)" "$RECIPIENT" | cast --from-wei) FCT"
echo "==========================="

echo -e "\nMinting tokens..."
TX_HASH=$(cast send --rpc-url "$RPC_URL" \
  --private-key "$PRIVATE_KEY" \
  "$FAUCET_ADDRESS" \
  "mint(address,uint256)" "$RECIPIENT" "$AMOUNT_WEI" | grep -oP '(?<=tx: )0x[a-fA-F0-9]+')

echo "Transaction hash: $TX_HASH"
echo "Transaction details:"
cast tx --rpc-url "$RPC_URL" "$TX_HASH"

echo -e "\nNew token balance of $RECIPIENT:"
NEW_BALANCE=$(cast call --rpc-url "$RPC_URL" "$FAUCET_ADDRESS" "balanceOf(address)(uint256)" "$RECIPIENT" | cast --from-wei)
echo "$NEW_BALANCE FCT"

echo -e "\nMinting completed successfully." 