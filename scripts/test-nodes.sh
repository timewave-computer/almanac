#!/usr/bin/env bash
# Purpose: Test node setup and verify functionality

set -euo pipefail

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
TEST_ANVIL=true
TEST_UFO=true
VERBOSE=false

# Status tracking
PASSED=0
FAILED=0
TOTAL=0

# Help function
function show_help {
  echo -e "${BLUE}Usage:${NC} $0 [OPTIONS]"
  echo "Test nodes setup and verify functionality"
  echo
  echo "Options:"
  echo "  --help         Show this help message"
  echo "  --anvil-only   Only test Anvil (Ethereum) node"
  echo "  --ufo-only     Only test UFO (Cosmos) node"
  echo "  --verbose      Show detailed test output"
  echo
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --help)
      show_help
      exit 0
      ;;
    --anvil-only)
      TEST_ANVIL=true
      TEST_UFO=false
      shift
      ;;
    --ufo-only)
      TEST_ANVIL=false
      TEST_UFO=true
      shift
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    *)
      echo -e "${RED}Error:${NC} Unknown option $1"
      show_help
      exit 1
      ;;
  esac
done

# Test function with result tracking
function run_test {
  local name=$1
  local cmd=$2
  
  echo -e "${BLUE}Testing:${NC} $name"
  
  TOTAL=$((TOTAL + 1))
  
  if $VERBOSE; then
    if eval "$cmd"; then
      echo -e "${GREEN}✓ PASS:${NC} $name"
      PASSED=$((PASSED + 1))
      return 0
    else
      echo -e "${RED}✗ FAIL:${NC} $name"
      FAILED=$((FAILED + 1))
      return 1
    fi
  else
    if eval "$cmd &>/dev/null"; then
      echo -e "${GREEN}✓ PASS:${NC} $name"
      PASSED=$((PASSED + 1))
      return 0
    else
      echo -e "${RED}✗ FAIL:${NC} $name"
      FAILED=$((FAILED + 1))
      return 1
    fi
  fi
}

# Function to test Anvil node
function test_anvil {
  echo -e "\n${BLUE}Testing Anvil Node Setup${NC}"
  
  # Start Anvil node
  echo -e "${YELLOW}Starting Anvil node...${NC}"
  anvil \
    --host 0.0.0.0 \
    --accounts 10 \
    --balance 10000 \
    --gas-limit 30000000 \
    --block-time 1 > ./data/anvil/log.txt 2>&1 &
  
  ANVIL_PID=$!
  
  # Wait for Anvil to be ready
  echo -e "${YELLOW}Waiting for Anvil to be ready...${NC}"
  for i in {1..30}; do
    if curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' http://localhost:8545 | grep -q "result"; then
      echo -e "${GREEN}Anvil is ready!${NC}"
      break
    fi
    sleep 1
    if [ $i -eq 30 ]; then
      echo -e "${RED}Failed to start Anvil${NC}"
      kill $ANVIL_PID 2>/dev/null || true
      exit 1
    fi
  done
  
  # Test 1: Check Anvil RPC connectivity
  run_test "Anvil RPC connectivity" "curl -s -X POST -H 'Content-Type: application/json' --data '{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"params\":[],\"id\":1}' http://localhost:8545 | grep -q 'result'"
  
  # Test 2: Check predefined accounts
  run_test "Anvil predefined accounts" "curl -s -X POST -H 'Content-Type: application/json' --data '{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBalance\",\"params\":[\"0x70997970C51812dc3A010C7d01b50e0d17dc79C8\", \"latest\"],\"id\":1}' http://localhost:8545 | grep -q 'result'"
  
  # Deploy Faucet contract for testing
  echo -e "${YELLOW}Deploying test contract...${NC}"
  PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
  
  # Deploy using forge
  forge create ./contracts/Faucet.sol:Faucet \
    --private-key $PRIVATE_KEY \
    --json \
    --rpc-url http://localhost:8545 > ./deployments/faucet.json
  
  # Extract contract address
  FAUCET_ADDRESS=$(cat ./deployments/faucet.json | jq -r '.deployedTo')
  echo "$FAUCET_ADDRESS" > ./deployments/faucet_address.txt
  
  # Test 3: Check contract deployment
  run_test "Contract deployment" "test -f ./deployments/faucet.json && test -f ./deployments/faucet_address.txt"
  
  # Test 4: Check contract code
  run_test "Contract code exists" "curl -s -X POST -H 'Content-Type: application/json' --data '{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$FAUCET_ADDRESS\", \"latest\"],\"id\":1}' http://localhost:8545 | grep -q '0x'"
  
  # Test 5: Mint tokens and check balance
  echo -e "${YELLOW}Minting test tokens...${NC}"
  TEST_ACCOUNT="0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
  
  # Mint tokens to test account
  cast send --private-key $PRIVATE_KEY \
    --rpc-url http://localhost:8545 \
    $FAUCET_ADDRESS \
    "mint(address,uint256)" $TEST_ACCOUNT 1000000000000000000000
  
  # Test 5: Check token balance
  run_test "Token minting" "cast call --rpc-url http://localhost:8545 $FAUCET_ADDRESS 'balanceOf(address)(uint256)' $TEST_ACCOUNT | grep -q '1000000000000000000000'"
  
  # Test 6: Check block mining
  run_test "Block mining" "sleep 3 && curl -s -X POST -H 'Content-Type: application/json' --data '{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"params\":[],\"id\":1}' http://localhost:8545 | grep -q 'result\":[\"0x[1-9]'"
  
  # Cleanup
  echo -e "${YELLOW}Stopping Anvil...${NC}"
  kill $ANVIL_PID
  wait $ANVIL_PID 2>/dev/null || true
}

# Function to test UFO node
function test_ufo {
  echo -e "\n${BLUE}Testing UFO Node Setup${NC}"
  
  # Start UFO node
  echo -e "${YELLOW}Starting UFO node...${NC}"
  ./scripts/run-ufo-node.sh --block-time 1 > ./data/ufo/log.txt 2>&1 &
  
  UFO_PID=$!
  
  # Wait for UFO to be ready
  echo -e "${YELLOW}Waiting for UFO node to be ready...${NC}"
  for i in {1..60}; do
    if curl -s http://localhost:26657/status | grep -q "result"; then
      echo -e "${GREEN}UFO node is ready!${NC}"
      break
    fi
    sleep 1
    if [ $i -eq 60 ]; then
      echo -e "${RED}Failed to start UFO node${NC}"
      kill $UFO_PID 2>/dev/null || true
      exit 1
    fi
  done
  
  # Test 1: Check UFO RPC connectivity
  run_test "UFO RPC connectivity" "curl -s http://localhost:26657/status | grep -q 'result'"
  
  # Test 2: Check block production
  run_test "UFO block production" "sleep 3 && curl -s http://localhost:26657/block | grep -q 'height'"
  
  # Test 3: Check validator configuration
  run_test "UFO validator configuration" "curl -s http://localhost:26657/validators | grep -q 'validators'"
  
  # Test 4: Check predefined accounts
  run_test "UFO predefined accounts" "curl -s http://localhost:26657/abci_query?path=\\\"/account/balance\\\" | grep -q 'value'"
  
  # Cleanup
  echo -e "${YELLOW}Stopping UFO node...${NC}"
  kill $UFO_PID
  wait $UFO_PID 2>/dev/null || true
}

# Main execution
echo -e "${BLUE}Running node tests...${NC}"

# Create data directories if they don't exist
mkdir -p ./data/anvil
mkdir -p ./data/ufo
mkdir -p ./deployments

# Run tests
if [ "$TEST_ANVIL" = true ]; then
  test_anvil
fi

if [ "$TEST_UFO" = true ]; then
  test_ufo
fi

# Print summary
echo -e "\n${BLUE}Test Summary:${NC}"
echo -e "Total tests: $TOTAL"
echo -e "${GREEN}Passed: $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
  echo -e "${RED}Failed: $FAILED${NC}"
  exit 1
else
  echo -e "${GREEN}All tests passed!${NC}"
  exit 0
fi 