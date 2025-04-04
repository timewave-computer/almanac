#!/usr/bin/env bash
# Purpose: Connect to live Ethereum and Cosmos nodes for indexing

set -euo pipefail

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
CONNECT_ETH=true
CONNECT_COSMOS=true
ETH_ENDPOINT="https://mainnet.infura.io/v3/$INFURA_KEY"
COSMOS_ENDPOINT="https://rpc.cosmos.network:26657"
VERBOSE=false

# Help function
function show_help {
  echo -e "${BLUE}Usage:${NC} $0 [OPTIONS]"
  echo "Connect to live Ethereum and Cosmos nodes for indexing"
  echo
  echo "Options:"
  echo "  --help               Show this help message"
  echo "  --eth-only           Only connect to Ethereum node"
  echo "  --cosmos-only        Only connect to Cosmos node"
  echo "  --eth-endpoint URL   Specify Ethereum node endpoint (default: Infura mainnet)"
  echo "  --cosmos-endpoint URL Specify Cosmos node endpoint (default: Cosmos Hub)"
  echo "  --verbose            Show detailed output"
  echo
  echo "Environment variables:"
  echo "  INFURA_KEY           Your Infura API key for Ethereum connections"
  echo "  ALCHEMY_KEY          Alternative: Your Alchemy API key"
  echo
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --help)
      show_help
      exit 0
      ;;
    --eth-only)
      CONNECT_ETH=true
      CONNECT_COSMOS=false
      shift
      ;;
    --cosmos-only)
      CONNECT_ETH=false
      CONNECT_COSMOS=true
      shift
      ;;
    --eth-endpoint)
      ETH_ENDPOINT="$2"
      shift 2
      ;;
    --cosmos-endpoint)
      COSMOS_ENDPOINT="$2"
      shift 2
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

# Check for required environment variables
if [ "$CONNECT_ETH" = true ] && [[ "$ETH_ENDPOINT" == *"$INFURA_KEY"* ]] && [ -z "${INFURA_KEY:-}" ]; then
  echo -e "${YELLOW}Warning:${NC} INFURA_KEY environment variable not set"
  
  # Check if we have ALCHEMY_KEY as alternative
  if [ -n "${ALCHEMY_KEY:-}" ]; then
    echo -e "${GREEN}Using Alchemy API instead${NC}"
    ETH_ENDPOINT="https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY"
  else
    echo -e "${YELLOW}Using public node (rate limited)${NC}"
    ETH_ENDPOINT="https://eth.llamarpc.com"
  fi
fi

# Function to test connection to Ethereum node
function connect_ethereum {
  echo -e "\n${BLUE}Connecting to Ethereum node:${NC} $ETH_ENDPOINT"
  
  # Test basic connection
  echo -e "${YELLOW}Testing connection...${NC}"
  
  CURL_OPTS="-s"
  if [ "$VERBOSE" = true ]; then
    CURL_OPTS="-v"
  fi
  
  RESPONSE=$(curl $CURL_OPTS -X POST \
    -H "Content-Type: application/json" \
    --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
    "$ETH_ENDPOINT")
  
  # Check if connection succeeded
  if echo "$RESPONSE" | grep -q "result"; then
    BLOCK_NUM=$(echo "$RESPONSE" | jq -r '.result')
    BLOCK_NUM_DEC=$((16#${BLOCK_NUM:2}))
    echo -e "${GREEN}Connection successful!${NC}"
    echo -e "Current block number: ${BLUE}$BLOCK_NUM_DEC${NC}"
    
    # Test getting a block
    echo -e "${YELLOW}Fetching latest block details...${NC}"
    BLOCK_RESPONSE=$(curl $CURL_OPTS -X POST \
      -H "Content-Type: application/json" \
      --data "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByNumber\",\"params\":[\"$BLOCK_NUM\", false],\"id\":1}" \
      "$ETH_ENDPOINT")
    
    if echo "$BLOCK_RESPONSE" | grep -q "result"; then
      TIMESTAMP_HEX=$(echo "$BLOCK_RESPONSE" | jq -r '.result.timestamp')
      TIMESTAMP_DEC=$((16#${TIMESTAMP_HEX:2}))
      TIMESTAMP_DATE=$(date -r "$TIMESTAMP_DEC" "+%Y-%m-%d %H:%M:%S")
      
      TX_COUNT=$(echo "$BLOCK_RESPONSE" | jq -r '.result.transactions | length')
      
      echo -e "Block timestamp: ${BLUE}$TIMESTAMP_DATE${NC}"
      echo -e "Transaction count: ${BLUE}$TX_COUNT${NC}"
      
      # Test network status
      echo -e "${YELLOW}Checking network status...${NC}"
      NET_VERSION=$(curl $CURL_OPTS -X POST \
        -H "Content-Type: application/json" \
        --data '{"jsonrpc":"2.0","method":"net_version","params":[],"id":1}' \
        "$ETH_ENDPOINT" | jq -r '.result')
      
      echo -e "Network ID: ${BLUE}$NET_VERSION${NC}"
      
      # Connection info
      echo -e "${GREEN}Ethereum node connection verified${NC}"
      echo -e "This connection can be used for the indexer with:"
      echo -e "export ETH_RPC_URL=\"$ETH_ENDPOINT\""
    else
      echo -e "${RED}Failed to fetch block details${NC}"
      if [ "$VERBOSE" = true ]; then
        echo "$BLOCK_RESPONSE"
      fi
    fi
  else
    echo -e "${RED}Connection failed!${NC}"
    if [ "$VERBOSE" = true ]; then
      echo "$RESPONSE"
    fi
    return 1
  fi
}

# Function to test connection to Cosmos node
function connect_cosmos {
  echo -e "\n${BLUE}Connecting to Cosmos node:${NC} $COSMOS_ENDPOINT"
  
  # Test basic connection
  echo -e "${YELLOW}Testing connection...${NC}"
  
  CURL_OPTS="-s"
  if [ "$VERBOSE" = true ]; then
    CURL_OPTS="-v"
  fi
  
  RESPONSE=$(curl $CURL_OPTS "$COSMOS_ENDPOINT/status")
  
  # Check if connection succeeded
  if echo "$RESPONSE" | grep -q "result"; then
    echo -e "${GREEN}Connection successful!${NC}"
    
    # Parse and display node information
    NODE_INFO=$(echo "$RESPONSE" | jq -r '.result.node_info')
    LATEST_BLOCK_HEIGHT=$(echo "$RESPONSE" | jq -r '.result.sync_info.latest_block_height')
    CATCHING_UP=$(echo "$RESPONSE" | jq -r '.result.sync_info.catching_up')
    
    echo -e "Node ID: ${BLUE}$(echo "$NODE_INFO" | jq -r '.id')${NC}"
    echo -e "Network: ${BLUE}$(echo "$NODE_INFO" | jq -r '.network')${NC}"
    echo -e "Latest block: ${BLUE}$LATEST_BLOCK_HEIGHT${NC}"
    echo -e "Catching up: ${BLUE}$CATCHING_UP${NC}"
    
    # Fetch latest blocks
    echo -e "${YELLOW}Fetching latest block details...${NC}"
    BLOCK_RESPONSE=$(curl $CURL_OPTS "$COSMOS_ENDPOINT/block")
    
    if echo "$BLOCK_RESPONSE" | grep -q "result"; then
      BLOCK_HEIGHT=$(echo "$BLOCK_RESPONSE" | jq -r '.result.block.header.height')
      BLOCK_TIME=$(echo "$BLOCK_RESPONSE" | jq -r '.result.block.header.time')
      TX_COUNT=$(echo "$BLOCK_RESPONSE" | jq -r '.result.block.data.txs | length')
      
      echo -e "Block height: ${BLUE}$BLOCK_HEIGHT${NC}"
      echo -e "Block time: ${BLUE}$BLOCK_TIME${NC}"
      echo -e "Transaction count: ${BLUE}$TX_COUNT${NC}"
      
      # Check validators
      echo -e "${YELLOW}Checking validators...${NC}"
      VALIDATORS_RESPONSE=$(curl $CURL_OPTS "$COSMOS_ENDPOINT/validators")
      
      if echo "$VALIDATORS_RESPONSE" | grep -q "result"; then
        VALIDATOR_COUNT=$(echo "$VALIDATORS_RESPONSE" | jq -r '.result.total')
        echo -e "Validator count: ${BLUE}$VALIDATOR_COUNT${NC}"
      else
        echo -e "${RED}Failed to fetch validators${NC}"
        if [ "$VERBOSE" = true ]; then
          echo "$VALIDATORS_RESPONSE"
        fi
      fi
      
      # Connection info
      echo -e "${GREEN}Cosmos node connection verified${NC}"
      echo -e "This connection can be used for the indexer with:"
      echo -e "export COSMOS_RPC_URL=\"$COSMOS_ENDPOINT\""
    else
      echo -e "${RED}Failed to fetch block details${NC}"
      if [ "$VERBOSE" = true ]; then
        echo "$BLOCK_RESPONSE"
      fi
    fi
  else
    echo -e "${RED}Connection failed!${NC}"
    if [ "$VERBOSE" = true ]; then
      echo "$RESPONSE"
    fi
    return 1
  fi
}

# Main execution
SUCCESS=true

# Connect to Ethereum
if [ "$CONNECT_ETH" = true ]; then
  if ! connect_ethereum; then
    SUCCESS=false
  fi
fi

# Connect to Cosmos
if [ "$CONNECT_COSMOS" = true ]; then
  if ! connect_cosmos; then
    SUCCESS=false
  fi
fi

# Summary
echo -e "\n${BLUE}Connection Summary:${NC}"
if [ "$SUCCESS" = true ]; then
  echo -e "${GREEN}All node connections were successful!${NC}"
  echo -e "You can use these connections for the indexer with the following environment variables:"
  
  if [ "$CONNECT_ETH" = true ]; then
    echo -e "export ETH_RPC_URL=\"$ETH_ENDPOINT\""
  fi
  
  if [ "$CONNECT_COSMOS" = true ]; then
    echo -e "export COSMOS_RPC_URL=\"$COSMOS_ENDPOINT\""
  fi
  
  exit 0
else
  echo -e "${RED}Some node connections failed. Please check the errors above.${NC}"
  exit 1
fi 