#!/usr/bin/env bash
# Purpose: Set up and configure test nodes for Ethereum (Anvil) and Cosmos (UFO)

set -euo pipefail

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
SETUP_ANVIL=true
SETUP_UFO=true
RESET=false

# Help function
function show_help {
  echo -e "${BLUE}Usage:${NC} $0 [OPTIONS]"
  echo "Configure test nodes for development and testing"
  echo
  echo "Options:"
  echo "  --help         Show this help message"
  echo "  --anvil-only   Only setup Anvil (Ethereum) node"
  echo "  --ufo-only     Only setup UFO (Cosmos) node"
  echo "  --reset        Reset existing configurations"
  echo
  echo "This script will:"
  echo "1. Configure an Anvil node with testing parameters"
  echo "2. Deploy the Faucet contract to Anvil"
  echo "3. Setup a UFO node with testing parameters"
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
      SETUP_ANVIL=true
      SETUP_UFO=false
      shift
      ;;
    --ufo-only)
      SETUP_ANVIL=false
      SETUP_UFO=true
      shift
      ;;
    --reset)
      RESET=true
      shift
      ;;
    *)
      echo -e "${RED}Error:${NC} Unknown option $1"
      show_help
      exit 1
      ;;
  esac
done

# Create data directories
mkdir -p ./data/anvil
mkdir -p ./data/ufo
mkdir -p ./deployments

# Setup Anvil (Ethereum) node
function setup_anvil {
  echo -e "${BLUE}Setting up Anvil node...${NC}"
  
  # Check if Anvil is already running
  if pgrep -f "anvil" > /dev/null; then
    echo -e "${YELLOW}Anvil is already running. Stopping it...${NC}"
    pkill -f "anvil" || true
    sleep 2
  fi
  
  # Start Anvil with specified parameters
  echo -e "${GREEN}Starting Anvil...${NC}"
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
      kill $ANVIL_PID
      exit 1
    fi
  done
  
  # Deploy Faucet contract
  echo -e "${BLUE}Deploying Faucet contract...${NC}"
  
  # Predefined private key for deterministic deployment
  PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
  
  # Deploy using forge
  forge create ./contracts/Faucet.sol:Faucet \
    --private-key $PRIVATE_KEY \
    --json \
    --rpc-url http://localhost:8545 > ./deployments/faucet.json
  
  # Extract contract address
  FAUCET_ADDRESS=$(cat ./deployments/faucet.json | jq -r '.deployedTo')
  echo "$FAUCET_ADDRESS" > ./deployments/faucet_address.txt
  
  echo -e "${GREEN}Faucet contract deployed at:${NC} $FAUCET_ADDRESS"
  
  # Mint some initial tokens to test accounts
  echo -e "${BLUE}Minting initial tokens to test accounts...${NC}"
  
  # Test accounts
  TEST_ACCOUNT_1="0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
  TEST_ACCOUNT_2="0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"
  
  # Mint tokens to test accounts
  cast send --private-key $PRIVATE_KEY \
    --rpc-url http://localhost:8545 \
    $FAUCET_ADDRESS \
    "mint(address,uint256)" $TEST_ACCOUNT_1 1000000000000000000000
  
  cast send --private-key $PRIVATE_KEY \
    --rpc-url http://localhost:8545 \
    $FAUCET_ADDRESS \
    "mint(address,uint256)" $TEST_ACCOUNT_2 1000000000000000000000
  
  # Verify balances
  echo -e "${BLUE}Verifying token balances...${NC}"
  
  BALANCE_1=$(cast call --rpc-url http://localhost:8545 $FAUCET_ADDRESS "balanceOf(address)(uint256)" $TEST_ACCOUNT_1)
  BALANCE_2=$(cast call --rpc-url http://localhost:8545 $FAUCET_ADDRESS "balanceOf(address)(uint256)" $TEST_ACCOUNT_2)
  
  echo -e "${GREEN}Balance of${NC} $TEST_ACCOUNT_1: $BALANCE_1"
  echo -e "${GREEN}Balance of${NC} $TEST_ACCOUNT_2: $BALANCE_2"
  
  # Stop Anvil
  echo -e "${YELLOW}Stopping Anvil...${NC}"
  kill $ANVIL_PID
  
  echo -e "${GREEN}Anvil setup complete!${NC}"
}

# Setup UFO (Cosmos) node
function setup_ufo {
  echo -e "${BLUE}Setting up UFO node...${NC}"
  
  # Check if UFO configuration exists
  if [ -d "$HOME/.ufo" ] && [ "$RESET" != "true" ]; then
    echo -e "${YELLOW}UFO configuration already exists.${NC}"
    read -p "Do you want to reset it? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
      echo -e "${GREEN}Keeping existing UFO configuration.${NC}"
      return
    fi
  fi
  
  # Create configuration directory
  mkdir -p "$HOME/.ufo/config"
  
  # Create minimal genesis.json
  cat > "$HOME/.ufo/config/genesis.json" << EOF
{
  "chain_id": "ufo-local-testnet",
  "validators": [
    {
      "name": "validator",
      "power": 100
    }
  ],
  "consensus_params": {
    "block": {
      "time_iota_ms": "1000",
      "max_bytes": "22020096",
      "max_gas": "-1"
    }
  },
  "initial_height": "1",
  "app_state": {
    "auth": {
      "accounts": [
        {
          "address": "ufo1sy8xq3l4nn7zxa9d6fcl8l4jszdprf789r5587",
          "coins": [
            {
              "denom": "ufo",
              "amount": "1000000000"
            }
          ]
        },
        {
          "address": "ufo1j2ld29k7wxh6vx7mglt8dmrxtz5gsp7m5ry48e",
          "coins": [
            {
              "denom": "ufo",
              "amount": "1000000000"
            }
          ]
        }
      ]
    }
  }
}
EOF
  
  echo -e "${GREEN}UFO setup complete!${NC}"
}

# Main execution
if [ "$SETUP_ANVIL" = true ]; then
  setup_anvil
fi

if [ "$SETUP_UFO" = true ]; then
  setup_ufo
fi

echo -e "${GREEN}All test nodes configured successfully!${NC}"
echo -e "${BLUE}You can now run:${NC}"
echo "  nix run .#start-anvil     - Start Ethereum node"
echo "  nix run .#run-ufo-node    - Start UFO node"
echo "  nix run .#run-all-nodes   - Start all nodes together" 