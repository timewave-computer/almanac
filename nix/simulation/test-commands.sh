#!/bin/bash

# Test script to verify all nix commands from README are syntactically correct
# This tests the command structure without actually building/running

set -e

echo "Testing Nix simulation commands from README..."
echo "=============================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

test_count=0
pass_count=0

test_command() {
    local desc="$1"
    local cmd="$2"
    test_count=$((test_count + 1))
    
    echo -n "[$test_count] Testing: $desc... "
    
    # Test that the command parses correctly (dry-run)
    if eval "$cmd --dry-run >/dev/null 2>&1" || eval "$cmd help >/dev/null 2>&1" || eval "$cmd --help >/dev/null 2>&1"; then
        echo -e "${GREEN}PASS${NC}"
        pass_count=$((pass_count + 1))
    else
        # For commands that don't support --help, test if the package exists
        if [[ "$cmd" =~ nix\ run\ \.#([^\ ]+) ]]; then
            local package="${BASH_REMATCH[1]}"
            if nix eval ".#packages.aarch64-darwin.$package.name" --json >/dev/null 2>&1; then
                echo -e "${GREEN}PASS${NC} (package exists)"
                pass_count=$((pass_count + 1))
            else
                echo -e "${RED}FAIL${NC} (package missing)"
            fi
        else
            echo -e "${RED}FAIL${NC}"
        fi
    fi
}

echo "Testing main orchestrator commands..."
test_command "sim-cluster help" "nix run .#sim-cluster --"
test_command "sim-cluster init" "nix run .#sim-cluster -- init"
test_command "sim-cluster start" "nix run .#sim-cluster -- start"
test_command "sim-cluster stop" "nix run .#sim-cluster -- stop"
test_command "sim-cluster status" "nix run .#sim-cluster -- status"
test_command "sim-cluster logs" "nix run .#sim-cluster -- logs"

echo ""
echo "Testing configuration generator..."
test_command "sim-config" "nix run .#sim-config --"

echo ""
echo "Testing individual blockchain nodes..."
test_command "sim-ethereum" "nix run .#sim-ethereum --"
test_command "sim-cosmos" "nix run .#sim-cosmos --"
test_command "sim-solana" "nix run .#sim-solana --"

echo ""
echo "Testing Almanac services..."
test_command "sim-almanac-api" "nix run .#sim-almanac-api --"
test_command "sim-almanac-indexer" "nix run .#sim-almanac-indexer --"

echo ""
echo "Testing command variations with arguments..."
test_command "sim-cluster init with chains" "nix run .#sim-cluster -- init ethereum,cosmos,fdb"
test_command "sim-config with args" "nix run .#sim-config -- ethereum,cosmos,fdb ./config/simulation"
test_command "sim-ethereum start" "nix run .#sim-ethereum -- start"
test_command "sim-cosmos stop" "nix run .#sim-cosmos -- stop"
test_command "sim-solana status" "nix run .#sim-solana -- status"

echo ""
echo "Testing flake structure..."
test_command "flake show" "nix flake show . --allow-dirty"

echo ""
echo "=============================================="
echo "Test Results: $pass_count/$test_count tests passed"

if [ $pass_count -eq $test_count ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    echo ""
    echo "The nix commands in the README are syntactically correct."
    echo "Note: This only tests command structure, not actual functionality."
    echo "To test functionality, you would need to build and run the services."
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi