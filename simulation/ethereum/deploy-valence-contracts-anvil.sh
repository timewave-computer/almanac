#!/bin/bash
# Purpose: Deploy Valence contracts to the Anvil Ethereum node

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Deploying Valence Contracts to Anvil ===${NC}"

# Check if anvil is running
if ! curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 > /dev/null; then
    echo -e "${RED}Error: Anvil node is not running at http://localhost:8545${NC}"
    echo -e "${YELLOW}Please start Anvil first with: ./simulation/ethereum/setup-anvil.sh${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Anvil node is running${NC}"

# Verify chain ID
CHAIN_ID_HEX=$(curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' http://localhost:8545 | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
CHAIN_ID_DEC=$((16#${CHAIN_ID_HEX:2}))
echo -e "${GREEN}✓ Anvil chain ID: ${CHAIN_ID_DEC}${NC}"

# Configuration
DATA_DIR="data/contracts/ethereum/anvil"
DEPLOYMENT_INFO="${DATA_DIR}/deployment-info.json"
CONTRACT_ADDRESSES_FILE="${DATA_DIR}/contract-addresses.env"

# Default private key for Anvil (account 0)
PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
RPC_URL="http://localhost:8545"

# Create necessary directories
mkdir -p ${DATA_DIR}

# Create deployment info file
echo "{}" > ${DEPLOYMENT_INFO}

# Function to deploy a contract and record its address
deploy_and_record() {
    local contract_name=$1
    local contract_path=$2
    local args=$3
    
    echo -e "${BLUE}Deploying ${contract_name}...${NC}"
    
    # Check if Forge is available
    if ! command -v forge >/dev/null 2>&1; then
        echo -e "${RED}Error: forge command not found${NC}"
        echo -e "${YELLOW}Please ensure Foundry is installed and in your PATH${NC}"
        exit 1
    fi
    
    # Deploy the contract
    local output
    if [ -z "$args" ]; then
        output=$(forge create ${contract_path} --rpc-url ${RPC_URL} --private-key ${PRIVATE_KEY} --json)
    else
        output=$(forge create ${contract_path} --rpc-url ${RPC_URL} --private-key ${PRIVATE_KEY} --constructor-args ${args} --json)
    fi
    
    # Extract contract address and transaction hash
    local address=$(echo $output | jq -r '.deployedTo')
    local tx_hash=$(echo $output | jq -r '.transactionHash')
    
    if [ -z "$address" ] || [ "$address" = "null" ]; then
        echo -e "${RED}Error: Failed to deploy ${contract_name}${NC}"
        echo -e "${YELLOW}Output: $output${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ ${contract_name} deployed to: ${address}${NC}"
    
    # Add to deployment info JSON
    jq --arg name "${contract_name}" --arg addr "${address}" --arg tx "${tx_hash}" \
       '.[$name] = {address: $addr, tx_hash: $tx}' ${DEPLOYMENT_INFO} > ${DEPLOYMENT_INFO}.tmp && mv ${DEPLOYMENT_INFO}.tmp ${DEPLOYMENT_INFO}
    
    # Export as environment variable
    echo "export ${contract_name}_ADDRESS=\"${address}\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
}

# Check for local Valence contracts directory
if [ -d "contracts/solidity/valence" ]; then
    CONTRACTS_DIR="contracts/solidity/valence"
    echo -e "${GREEN}✓ Found Valence contracts in local directory: ${CONTRACTS_DIR}${NC}"
elif [ -d "../valence-contracts/src" ]; then
    CONTRACTS_DIR="../valence-contracts/src"
    echo -e "${GREEN}✓ Found Valence contracts in sibling directory: ${CONTRACTS_DIR}${NC}"
else
    echo -e "${RED}Error: Valence contracts directory not found${NC}"
    echo -e "${YELLOW}Please clone the Valence contracts repository or ensure contracts are in 'contracts/solidity/valence'${NC}"
    exit 1
fi

# Reset contract addresses file
echo "# Valence contract addresses on Anvil" > ${CONTRACT_ADDRESSES_FILE}.tmp
echo "# Generated on $(date)" >> ${CONTRACT_ADDRESSES_FILE}.tmp
echo "export CHAIN_ID=\"${CHAIN_ID_DEC}\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp

# Deploy base contracts
deploy_and_record "VALENCE_PROCESSOR" "${CONTRACTS_DIR}/EthereumProcessor.sol:EthereumProcessor"
deploy_and_record "VALENCE_GATEWAY" "${CONTRACTS_DIR}/UniversalGateway.sol:UniversalGateway"
deploy_and_record "VALENCE_ACCOUNT" "${CONTRACTS_DIR}/BaseAccount.sol:BaseAccount"

# Deploy test tokens (optional)
deploy_and_record "TEST_TOKEN_SUN" "${CONTRACTS_DIR}/TestToken.sol:TestToken" "\"Sun Token\" SUN 18"
deploy_and_record "TEST_TOKEN_EARTH" "${CONTRACTS_DIR}/TestToken.sol:TestToken" "\"Earth Token\" EARTH 18"

# Finalize contract addresses file
mv ${CONTRACT_ADDRESSES_FILE}.tmp ${CONTRACT_ADDRESSES_FILE}

# Set up contract relationships
echo -e "${BLUE}Configuring contract relationships...${NC}"

# Import contract addresses to use them
source ${CONTRACT_ADDRESSES_FILE}

# Set processor's gateway
echo -e "${BLUE}Setting Gateway for Processor...${NC}"
cast send --rpc-url ${RPC_URL} --private-key ${PRIVATE_KEY} ${VALENCE_PROCESSOR_ADDRESS} "setGateway(address)" ${VALENCE_GATEWAY_ADDRESS}

# Set gateway's processor
echo -e "${BLUE}Setting Processor for Gateway...${NC}"
cast send --rpc-url ${RPC_URL} --private-key ${PRIVATE_KEY} ${VALENCE_GATEWAY_ADDRESS} "setProcessor(address)" ${VALENCE_PROCESSOR_ADDRESS}

# Set gateway's relayer (use the default anvil account for testing)
echo -e "${BLUE}Setting Relayer for Gateway...${NC}"
cast send --rpc-url ${RPC_URL} --private-key ${PRIVATE_KEY} ${VALENCE_GATEWAY_ADDRESS} "setRelayer(address)" "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

echo -e "${GREEN}=== Valence contracts deployed and configured successfully! ===${NC}"
echo -e "${BLUE}Contract addresses saved to: ${CONTRACT_ADDRESSES_FILE}${NC}"
echo -e "${BLUE}Deployment info saved to: ${DEPLOYMENT_INFO}${NC}" 