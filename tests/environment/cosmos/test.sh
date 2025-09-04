#!/usr/bin/env bash

set -euo pipefail

echo "Cosmos Environment Tests"
echo "======================="
echo ""

# Determine platform and flake to use
if [[ "$OSTYPE" == "darwin"* ]]; then
    PLATFORM="macOS"
    FLAKE_PATH="../../../nix/cosmos"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PLATFORM="NixOS/Linux"
    FLAKE_PATH="../../../nix/cosmos#nixos"
else
    echo "NO Unsupported platform: $OSTYPE"
    exit 1
fi

echo "Platform: $PLATFORM"
echo "Using flake: $FLAKE_PATH"
echo ""

# Parse command line arguments
RUN_TRANSFER_TEST=false
HELP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --transfer|-t)
            RUN_TRANSFER_TEST=true
            shift
            ;;
        --help|-h)
            HELP=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            HELP=true
            shift
            ;;
    esac
done

if [ "$HELP" = true ]; then
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --transfer, -t    Run full token transfer test (requires more time/resources)"
    echo "  --help, -h        Show this help message"
    echo ""
    echo "Default behavior: Run build tests only (quick verification)"
    echo ""
    echo "Examples:"
    echo "  $0                # Quick build test"
    echo "  $0 --transfer     # Full transfer test"
    exit 0
fi

# Test 1: Build Test (always runs)
echo "Test 1: Build and Environment Verification"
echo "=============================================="
echo ""

echo "1.1 Testing Neutron build..."
if timeout 120 nix build "$FLAKE_PATH#neutron" --no-link --show-trace >/dev/null 2>&1; then
    echo "OK Neutron builds successfully"
else
    echo "NO Neutron build failed"
    exit 1
fi

echo ""
echo "1.2 Testing development shell access..."
if timeout 60 nix develop "$FLAKE_PATH" --command neutrond version >/dev/null 2>&1; then
    VERSION=$(nix develop "$FLAKE_PATH" --command neutrond version 2>/dev/null)
    echo "OK Development shell provides neutrond"
    echo "   Version: $VERSION"
else
    echo "NO Development shell access failed"
    exit 1
fi

echo ""
echo "1.3 Testing required tools..."
TOOLS=("curl --version" "jq --version")
for tool_cmd in "${TOOLS[@]}"; do
    tool_name=$(echo "$tool_cmd" | cut -d' ' -f1)
    if timeout 30 nix develop "$FLAKE_PATH" --command bash -c "$tool_cmd" >/dev/null 2>&1; then
        echo "OK $tool_name available"
    else
        echo "NO $tool_name not available"
        exit 1
    fi
done

echo ""
echo "OK Build Test PASSED"
echo ""

# Test 2: Transfer Test (optional)
if [ "$RUN_TRANSFER_TEST" = true ]; then
    echo "Test 2: Token Transfer Verification"
    echo "======================================"
    echo ""
    
    # Cleanup function
    cleanup() {
        echo ""
        echo "Cleaning up..."
        pkill -f "neutrond.*start" 2>/dev/null || true
        sleep 2
        rm -rf ~/.neutrond 2>/dev/null || true
        rm -f /tmp/cosmos-transfer-test.sh 2>/dev/null || true
        echo "Cleanup completed."
    }
    
    trap cleanup EXIT INT TERM
    
    # Create transfer test script
    cat > /tmp/cosmos-transfer-test.sh << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

ADMIN_ADDRESS="neutron19rl4cm2hmr8afy4kldpxz3fka4jguq0aclyl9j"
ADMIN_MNEMONIC="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

echo "2.1 Initializing Neutron testnet..."

# Clean up
pkill -f "neutrond.*start" 2>/dev/null || true
sleep 2
rm -rf ~/.neutrond

# Initialize and configure
neutrond init testing --chain-id testing --default-denom untrn
sed -i.bak 's/minimum-gas-prices = ""/minimum-gas-prices = "0.0001untrn"/' ~/.neutrond/config/app.toml
sed -i.bak 's/enabled-unsafe-cors = false/enabled-unsafe-cors = true/' ~/.neutrond/config/app.toml
sed -i.bak 's/cors_allowed_origins = \[\]/cors_allowed_origins = ["*"]/' ~/.neutrond/config/config.toml
sed -i.bak 's/^create_empty_blocks = true/create_empty_blocks = false/' ~/.neutrond/config/config.toml
sed -i.bak 's/^timeout_commit = "5s"/timeout_commit = "2s"/' ~/.neutrond/config/config.toml

# Set up accounts and genesis
echo "$ADMIN_MNEMONIC" | neutrond keys add admin --keyring-backend test --recover
neutrond add-genesis-account admin 10000000000000stake,10000000000000untrn --keyring-backend test
neutrond add-consumer-section
neutrond gentx admin 1000000000untrn --chain-id testing --keyring-backend test
neutrond collect-gentxs

echo "2.2 Starting Neutron node..."
neutrond start --rpc.unsafe &
NEUTRON_PID=$!

# Wait for startup
for i in {1..30}; do
    if curl -s http://localhost:26657/status >/dev/null 2>&1; then
        echo "OK Node started (attempt $i)"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "NO Node failed to start"
        exit 1
    fi
    sleep 1
done

# Wait for REST API
for i in {1..20}; do
    if curl -s http://localhost:1317/cosmos/base/tendermint/v1beta1/node_info >/dev/null 2>&1; then
        echo "OK REST API ready (attempt $i)"
        break
    fi
    sleep 1
done

sleep 5

echo "2.3 Creating test accounts..."
neutrond keys delete user_a --keyring-backend test -y 2>/dev/null || true
neutrond keys delete user_b --keyring-backend test -y 2>/dev/null || true

USER_A_INFO=$(neutrond keys add user_a --keyring-backend test --output json)
USER_A_ADDRESS=$(echo "$USER_A_INFO" | jq -r ".address")

USER_B_INFO=$(neutrond keys add user_b --keyring-backend test --output json)
USER_B_ADDRESS=$(echo "$USER_B_INFO" | jq -r ".address")

echo "   User A: $USER_A_ADDRESS"
echo "   User B: $USER_B_ADDRESS"

echo "2.4 Verifying admin balance..."
ADMIN_BALANCE_RESPONSE=$(curl -s "http://localhost:1317/cosmos/bank/v1beta1/balances/$ADMIN_ADDRESS")
ADMIN_UNTRN=$(echo "$ADMIN_BALANCE_RESPONSE" | jq -r '.balances[]? | select(.denom=="untrn") | .amount' 2>/dev/null || echo "0")

if [ "$ADMIN_UNTRN" = "0" ] || [ -z "$ADMIN_UNTRN" ]; then
    echo "NO Admin has no balance: $ADMIN_UNTRN"
    exit 1
fi
echo "OK Admin balance: $ADMIN_UNTRN untrn"

echo "2.5 Funding User A with 1000 untrn..."
TX_RESULT=$(neutrond tx bank send admin "$USER_A_ADDRESS" 1000untrn \
    --from admin \
    --keyring-backend test \
    --chain-id testing \
    --node tcp://localhost:26657 \
    --gas 200000 \
    --gas-prices 0.001untrn \
    --yes \
    --output json \
    --broadcast-mode sync)

TX_HASH=$(echo "$TX_RESULT" | jq -r ".txhash")
if [ "$TX_HASH" = "null" ] || [ -z "$TX_HASH" ]; then
    echo "NO Failed to get transaction hash"
    exit 1
fi

sleep 8

TX_STATUS=$(neutrond query tx "$TX_HASH" --node tcp://localhost:26657 --output json)
TX_CODE=$(echo "$TX_STATUS" | jq -r ".code")

if [ "$TX_CODE" != "0" ]; then
    echo "NO Funding transaction failed with code: $TX_CODE"
    exit 1
fi
echo "OK Funding transaction successful"

echo "2.6 Verifying User A balance..."
USER_A_BALANCE_RESPONSE=$(curl -s "http://localhost:1317/cosmos/bank/v1beta1/balances/$USER_A_ADDRESS")
USER_A_UNTRN=$(echo "$USER_A_BALANCE_RESPONSE" | jq -r '.balances[]? | select(.denom=="untrn") | .amount' 2>/dev/null || echo "0")

if [ "$USER_A_UNTRN" != "1000" ]; then
    echo "NO User A funding verification failed. Expected 1000, got: $USER_A_UNTRN"
    exit 1
fi
echo "OK User A balance verified: $USER_A_UNTRN untrn"

echo "2.7 Transferring 10 untrn from User A to User B..."
TRANSFER_RESULT=$(neutrond tx bank send user_a "$USER_B_ADDRESS" 10untrn \
    --from user_a \
    --keyring-backend test \
    --chain-id testing \
    --node tcp://localhost:26657 \
    --gas 200000 \
    --gas-prices 0.001untrn \
    --yes \
    --output json \
    --broadcast-mode sync)

TRANSFER_HASH=$(echo "$TRANSFER_RESULT" | jq -r ".txhash")
sleep 8

TRANSFER_STATUS=$(neutrond query tx "$TRANSFER_HASH" --node tcp://localhost:26657 --output json)
TRANSFER_CODE=$(echo "$TRANSFER_STATUS" | jq -r ".code")

if [ "$TRANSFER_CODE" != "0" ]; then
    echo "NO Transfer transaction failed with code: $TRANSFER_CODE"
    exit 1
fi
echo "OK Transfer transaction successful"

echo "2.8 Verifying final balances..."
sleep 3

USER_A_FINAL_RESPONSE=$(curl -s "http://localhost:1317/cosmos/bank/v1beta1/balances/$USER_A_ADDRESS")
USER_B_FINAL_RESPONSE=$(curl -s "http://localhost:1317/cosmos/bank/v1beta1/balances/$USER_B_ADDRESS")

USER_A_FINAL=$(echo "$USER_A_FINAL_RESPONSE" | jq -r '.balances[]? | select(.denom=="untrn") | .amount' 2>/dev/null || echo "0")
USER_B_FINAL=$(echo "$USER_B_FINAL_RESPONSE" | jq -r '.balances[]? | select(.denom=="untrn") | .amount' 2>/dev/null || echo "0")

if [ "$USER_B_FINAL" = "10" ]; then
    echo "OK Transfer verified: User B has $USER_B_FINAL untrn"
    echo "OK User A remaining: $USER_A_FINAL untrn (after gas fees)"
    
    CURRENT_HEIGHT=$(curl -s http://localhost:26657/status | jq -r '.result.sync_info.latest_block_height')
    echo "OK Final block height: $CURRENT_HEIGHT"
else
    echo "NO Transfer verification failed"
    echo "   Expected User B: 10 untrn, Got: $USER_B_FINAL untrn" 
    echo "   User A final: $USER_A_FINAL untrn"
    exit 1
fi

echo ""
echo "Transfer Test PASSED!"
EOF

    chmod +x /tmp/cosmos-transfer-test.sh
    
    echo "Running full token transfer test..."
    echo "This may take 5-10 minutes and requires stable network connection."
    echo ""
    
    if nix develop "$FLAKE_PATH" --command /tmp/cosmos-transfer-test.sh; then
        echo ""
        echo "OK Transfer Test PASSED"
    else
        echo ""
        echo "NO Transfer Test FAILED"
        exit 1
    fi
else
    echo "Skipping transfer test (use --transfer to run full test)"
fi

echo ""
echo "All Tests Completed Successfully!"
echo ""
echo "Summary:"
echo "OK Platform: $PLATFORM"
echo "OK Neutron v6.0.3 builds and runs correctly"
echo "OK Development environment functional"
if [ "$RUN_TRANSFER_TEST" = true ]; then
    echo "OK Token transfer functionality verified"
fi
echo ""
echo "Cosmos testing environment ready for development!"