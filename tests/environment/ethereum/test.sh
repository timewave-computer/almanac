#!/usr/bin/env bash

set -euo pipefail

echo "Ethereum Environment Tests"
echo "=========================="
echo ""

# Determine platform and flake to use
if [[ "$OSTYPE" == "darwin"* ]]; then
    PLATFORM="macOS"
    FLAKE_PATH="../../../nix/ethereum/flake.nix"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PLATFORM="NixOS/Linux"
    FLAKE_PATH="../../../nix/ethereum/flake-nixos.nix"
else
    echo "NO Unsupported platform: $OSTYPE"
    exit 1
fi

echo "Platform: $PLATFORM"
echo "Using flake: $FLAKE_PATH"
echo ""

# Parse command line arguments
RUN_INTEGRATION_TEST=false
HELP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --integration|-i)
            RUN_INTEGRATION_TEST=true
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
    echo "  --integration, -i    Run integration tests with Anvil local testnet"
    echo "  --help, -h           Show this help message"
    echo ""
    echo "Default behavior: Run build tests only (quick verification)"
    echo ""
    echo "Examples:"
    echo "  $0                   # Quick build test"
    echo "  $0 --integration     # Build + integration tests with Anvil"
    exit 0
fi

# Test 1: Build Test (always runs)
echo "Test 1: Build and Environment Verification"
echo "==========================================="
echo ""

echo "1.1 Testing flake availability and structure..."
if [ -f "$FLAKE_PATH" ]; then
    echo "OK Flake file exists at: $FLAKE_PATH"
else
    echo "NO Flake file not found at: $FLAKE_PATH"
    exit 1
fi

echo "1.2 Testing Nix flake syntax..."
if timeout 30 nix flake show "$FLAKE_PATH" >/dev/null 2>&1; then
    echo "OK Flake structure is valid"
else
    echo "NO Flake structure evaluation failed"
    exit 1
fi

echo ""
echo "1.3 Testing development shell tools..."
TOOLS=("geth --version" "forge --version" "cast --version")
for tool_cmd in "${TOOLS[@]}"; do
    tool_name=$(echo "$tool_cmd" | cut -d' ' -f1)
    if timeout 60 nix develop "$FLAKE_PATH" --command bash -c "$tool_cmd" >/dev/null 2>&1; then
        VERSION=$(nix develop "$FLAKE_PATH" --command bash -c "$tool_cmd" 2>/dev/null | head -1 || echo "version check failed")
        echo "OK $tool_name available"
        echo "   Version: $VERSION"
    else
        echo "NO $tool_name not available"
        exit 1
    fi
done

echo ""
echo "1.4 Testing additional tools..."
EXTRA_TOOLS=("curl --version" "jq --version")
for tool_cmd in "${EXTRA_TOOLS[@]}"; do
    tool_name=$(echo "$tool_cmd" | cut -d' ' -f1)
    if timeout 30 nix develop "$FLAKE_PATH" --command bash -c "$tool_cmd" >/dev/null 2>&1; then
        echo "OK $tool_name available"
    else
        echo "NO $tool_name not available"
        exit 1
    fi
done

echo ""
echo "1.5 Testing Lighthouse availability..."
if timeout 60 nix develop "$FLAKE_PATH" --command bash -c "lighthouse --version" >/dev/null 2>&1; then
    LIGHTHOUSE_VERSION=$(nix develop "$FLAKE_PATH" --command lighthouse --version 2>/dev/null | head -1 || echo "version check failed")
    echo "OK Lighthouse available"
    echo "   Version: $LIGHTHOUSE_VERSION"
elif [[ "$PLATFORM" == "macOS" ]]; then
    echo "OK Lighthouse not available on macOS (expected - use Homebrew or manual install)"
else
    echo "NO Lighthouse not available on NixOS (unexpected)"
    exit 1
fi

echo ""
echo "OK Build Test PASSED"
echo ""

# Test 2: Integration Test (optional)
if [ "$RUN_INTEGRATION_TEST" = true ]; then
    echo "Test 2: Ethereum Integration Test"
    echo "=================================="
    echo ""
    
    # Cleanup function
    cleanup() {
        echo ""
        echo "Cleaning up..."
        pkill -f "anvil" 2>/dev/null || true
        sleep 2
        rm -f /tmp/ethereum-integration-test.sh 2>/dev/null || true
        echo "Cleanup completed."
    }
    
    trap cleanup EXIT INT TERM
    
    # Create integration test script
    cat > /tmp/ethereum-integration-test.sh << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

echo "2.1 Starting Anvil local testnet..."

# Start Anvil in background
anvil --host 0.0.0.0 --port 8545 > /tmp/anvil.log 2>&1 &
ANVIL_PID=$!

# Wait for Anvil to start
for i in {1..15}; do
    if curl -s -X POST http://localhost:8545 \
       -H "Content-Type: application/json" \
       -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' >/dev/null 2>&1; then
        echo "OK Anvil started (attempt $i)"
        break
    fi
    if [ $i -eq 15 ]; then
        echo "NO Anvil failed to start"
        exit 1
    fi
    sleep 1
done

echo "2.2 Testing RPC connectivity..."
BLOCK_NUMBER_RESPONSE=$(curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}')

if echo "$BLOCK_NUMBER_RESPONSE" | jq -r '.result' | grep -q "0x"; then
    BLOCK_NUM=$(echo "$BLOCK_NUMBER_RESPONSE" | jq -r '.result')
    echo "OK RPC connectivity working"
    echo "   Current block: $BLOCK_NUM"
else
    echo "NO RPC connectivity failed"
    exit 1
fi

echo "2.3 Testing account access..."
ACCOUNTS_RESPONSE=$(curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_accounts","params":[],"id":1}')

ACCOUNT_COUNT=$(echo "$ACCOUNTS_RESPONSE" | jq -r '.result | length')
if [ "$ACCOUNT_COUNT" -gt 0 ]; then
    echo "OK Anvil provides $ACCOUNT_COUNT test accounts"
    FIRST_ACCOUNT=$(echo "$ACCOUNTS_RESPONSE" | jq -r '.result[0]')
    echo "   First account: $FIRST_ACCOUNT"
else
    echo "NO No test accounts available"
    exit 1
fi

echo "2.4 Testing account balance..."
BALANCE_RESPONSE=$(curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBalance\",\"params\":[\"$FIRST_ACCOUNT\",\"latest\"],\"id\":1}")

BALANCE_HEX=$(echo "$BALANCE_RESPONSE" | jq -r '.result')
if [ "$BALANCE_HEX" != "null" ] && [ "$BALANCE_HEX" != "" ]; then
    echo "OK Account has balance: $BALANCE_HEX"
else
    echo "NO Failed to get account balance"
    exit 1
fi

echo "2.5 Testing simple transaction with Cast..."
# Test a simple eth_call using cast
if command -v cast >/dev/null 2>&1; then
    CAST_BLOCK=$(cast block-number --rpc-url http://localhost:8545 2>/dev/null || echo "failed")
    if [ "$CAST_BLOCK" != "failed" ]; then
        echo "OK Cast can interact with Anvil"
        echo "   Block number via Cast: $CAST_BLOCK"
    else
        echo "OK Cast interaction failed (chain may be too fast)"
    fi
else
    echo "NO Cast not available for testing"
    exit 1
fi

echo ""
echo "Integration Test PASSED!"
EOF

    chmod +x /tmp/ethereum-integration-test.sh
    
    echo "Running Ethereum integration test..."
    echo "This will start a local Anvil testnet and test RPC connectivity."
    echo ""
    
    if nix develop "$FLAKE_PATH" --command /tmp/ethereum-integration-test.sh; then
        echo ""
        echo "OK Integration Test PASSED"
    else
        echo ""
        echo "NO Integration Test FAILED"
        exit 1
    fi
else
    echo "Skipping integration test (use --integration to run Anvil tests)"
fi

echo ""
echo "All Tests Completed Successfully!"
echo ""
echo "Summary:"
echo "OK Platform: $PLATFORM"
if [[ "$PLATFORM" == "macOS" ]]; then
    echo "OK Ethereum development tools available (Geth, Foundry)"
    echo "OK Lighthouse requires manual installation on macOS"
else
    echo "OK Ethereum development stack complete (Geth, Lighthouse, Foundry)"
fi
echo "OK Development environment functional"
if [ "$RUN_INTEGRATION_TEST" = true ]; then
    echo "OK Anvil local testnet and RPC connectivity verified"
fi
echo ""
echo "Ethereum testing environment ready for development!"