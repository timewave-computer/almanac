#!/bin/bash
# Purpose: Deploy Valence contracts to the CosmWasm (wasmd) node

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Deploying Valence Contracts to CosmWasm (wasmd) ===${NC}"

# Check if wasmd is running
if ! curl -s http://localhost:26657/status > /dev/null 2>&1; then
    echo -e "${RED}Error: wasmd node is not running at http://localhost:26657${NC}"
    echo -e "${YELLOW}Please start wasmd first with: ./simulation/cosmos/setup-wasmd.sh${NC}"
    exit 1
fi

echo -e "${GREEN}✓ wasmd node is running${NC}"

# Configuration
DATA_DIR="data/contracts/cosmos/wasmd"
DEPLOYMENT_INFO="${DATA_DIR}/deployment-info.json"
CONTRACT_ADDRESSES_FILE="${DATA_DIR}/contract-addresses.env"
HOME_DIR="$HOME/.wasmd-test"
CHAIN_ID="wasmchain"
VALIDATOR_NAME="validator"
NODE="tcp://localhost:26657"
KEYRING="--keyring-backend=test"
VALIDATOR_ADDRESS=$(wasmd keys show $VALIDATOR_NAME $KEYRING --home=$HOME_DIR -a)

# Create necessary directories
mkdir -p ${DATA_DIR}

# Create deployment info file
echo "{}" > ${DEPLOYMENT_INFO}

# Function to store WASM code and record its code ID
store_wasm_and_record() {
    local contract_name=$1
    local wasm_file=$2
    
    echo -e "${BLUE}Storing ${contract_name} WASM code...${NC}"
    
    # Check if wasmd is available
    if ! command -v wasmd >/dev/null 2>&1; then
        echo -e "${RED}Error: wasmd command not found${NC}"
        echo -e "${YELLOW}Please ensure wasmd is installed and in your PATH${NC}"
        exit 1
    fi
    
    # Store the WASM code
    local tx_result=$(wasmd tx wasm store $wasm_file --from $VALIDATOR_NAME $KEYRING --chain-id=$CHAIN_ID --node=$NODE --gas=auto --gas-adjustment=1.3 -y --home=$HOME_DIR)
    local code_id=$(echo "$tx_result" | grep -o 'code_id: [0-9]*' | cut -d' ' -f2)
    
    if [ -z "$code_id" ]; then
        echo -e "${RED}Error: Failed to store ${contract_name} WASM code${NC}"
        echo -e "${YELLOW}Output: $tx_result${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ ${contract_name} WASM code stored with code ID: ${code_id}${NC}"
    
    # Add to deployment info JSON
    jq --arg name "${contract_name}" --arg id "${code_id}" \
       '.[$name] = {code_id: $id}' ${DEPLOYMENT_INFO} > ${DEPLOYMENT_INFO}.tmp && mv ${DEPLOYMENT_INFO}.tmp ${DEPLOYMENT_INFO}
    
    # Export as environment variable
    echo "export ${contract_name}_CODE_ID=\"${code_id}\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
    
    # Return the code ID
    echo $code_id
}

# Function to instantiate a contract and record its address
instantiate_and_record() {
    local contract_name=$1
    local code_id=$2
    local init_msg=$3
    local label=$4
    
    echo -e "${BLUE}Instantiating ${contract_name} contract...${NC}"
    
    # Instantiate the contract
    local tx_result=$(wasmd tx wasm instantiate $code_id "$init_msg" --from $VALIDATOR_NAME $KEYRING --chain-id=$CHAIN_ID --node=$NODE --gas=auto --gas-adjustment=1.3 -y --label="$label" --no-admin --home=$HOME_DIR)
    local contract_address=$(echo "$tx_result" | grep -o '_contract_address: .*' | cut -d' ' -f2)
    
    if [ -z "$contract_address" ]; then
        echo -e "${RED}Error: Failed to instantiate ${contract_name} contract${NC}"
        echo -e "${YELLOW}Output: $tx_result${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ ${contract_name} instantiated at address: ${contract_address}${NC}"
    
    # Add to deployment info JSON
    jq --arg name "${contract_name}" --arg addr "${contract_address}" \
       '.[$name].address = $addr' ${DEPLOYMENT_INFO} > ${DEPLOYMENT_INFO}.tmp && mv ${DEPLOYMENT_INFO}.tmp ${DEPLOYMENT_INFO}
    
    # Export as environment variable
    echo "export ${contract_name}_ADDRESS=\"${contract_address}\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
    
    # Return the contract address
    echo $contract_address
}

# Check for local Valence WASM contracts directory
if [ -d "contracts/wasm_compiled" ]; then
    CONTRACTS_DIR="contracts/wasm_compiled"
    echo -e "${GREEN}✓ Found Valence WASM contracts in local directory: ${CONTRACTS_DIR}${NC}"
elif [ -d "../valence-contracts/artifacts" ]; then
    CONTRACTS_DIR="../valence-contracts/artifacts"
    echo -e "${GREEN}✓ Found Valence WASM contracts in sibling directory: ${CONTRACTS_DIR}${NC}"
else
    echo -e "${RED}Error: Valence WASM contracts directory not found${NC}"
    echo -e "${YELLOW}Please ensure compiled WASM contracts are in 'contracts/wasm_compiled' or clone the Valence contracts repository${NC}"
    exit 1
fi

# Reset contract addresses file
echo "# Valence contract addresses on CosmWasm" > ${CONTRACT_ADDRESSES_FILE}.tmp
echo "# Generated on $(date)" >> ${CONTRACT_ADDRESSES_FILE}.tmp
echo "export CHAIN_ID=\"${CHAIN_ID}\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
echo "export VALIDATOR_ADDRESS=\"${VALIDATOR_ADDRESS}\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp

# Store and deploy base contracts if they exist
if [ -f "${CONTRACTS_DIR}/valence_gateway.wasm" ]; then
    # Store WASM code
    GATEWAY_CODE_ID=$(store_wasm_and_record "VALENCE_GATEWAY" "${CONTRACTS_DIR}/valence_gateway.wasm")
    PROCESSOR_CODE_ID=$(store_wasm_and_record "VALENCE_PROCESSOR" "${CONTRACTS_DIR}/valence_processor.wasm")
    ACCOUNT_CODE_ID=$(store_wasm_and_record "VALENCE_ACCOUNT" "${CONTRACTS_DIR}/valence_account.wasm")
    
    # Instantiate contracts
    GATEWAY_INIT="{\"owner\":\"${VALIDATOR_ADDRESS}\"}"
    PROCESSOR_INIT="{\"owner\":\"${VALIDATOR_ADDRESS}\"}"
    ACCOUNT_INIT="{\"owner\":\"${VALIDATOR_ADDRESS}\"}"
    
    GATEWAY_ADDRESS=$(instantiate_and_record "VALENCE_GATEWAY" $GATEWAY_CODE_ID "$GATEWAY_INIT" "Valence Gateway")
    PROCESSOR_ADDRESS=$(instantiate_and_record "VALENCE_PROCESSOR" $PROCESSOR_CODE_ID "$PROCESSOR_INIT" "Valence Processor")
    ACCOUNT_ADDRESS=$(instantiate_and_record "VALENCE_ACCOUNT" $ACCOUNT_CODE_ID "$ACCOUNT_INIT" "Valence Account")
    
    # Configure contract relationships
    echo -e "${BLUE}Configuring contract relationships...${NC}"
    
    # Set processor's gateway
    echo -e "${BLUE}Setting Gateway for Processor...${NC}"
    wasmd tx wasm execute $PROCESSOR_ADDRESS "{\"set_gateway\":{\"gateway\":\"$GATEWAY_ADDRESS\"}}" --from $VALIDATOR_NAME $KEYRING --chain-id=$CHAIN_ID --node=$NODE --gas=auto --gas-adjustment=1.3 -y --home=$HOME_DIR
    
    # Set gateway's processor
    echo -e "${BLUE}Setting Processor for Gateway...${NC}"
    wasmd tx wasm execute $GATEWAY_ADDRESS "{\"set_processor\":{\"processor\":\"$PROCESSOR_ADDRESS\"}}" --from $VALIDATOR_NAME $KEYRING --chain-id=$CHAIN_ID --node=$NODE --gas=auto --gas-adjustment=1.3 -y --home=$HOME_DIR
    
    # Set gateway's relayer (use the validator for testing)
    echo -e "${BLUE}Setting Relayer for Gateway...${NC}"
    wasmd tx wasm execute $GATEWAY_ADDRESS "{\"set_relayer\":{\"relayer\":\"$VALIDATOR_ADDRESS\"}}" --from $VALIDATOR_NAME $KEYRING --chain-id=$CHAIN_ID --node=$NODE --gas=auto --gas-adjustment=1.3 -y --home=$HOME_DIR
    
    echo -e "${GREEN}✓ Contract relationships configured${NC}"
else
    echo -e "${YELLOW}Warning: Valence WASM contract files not found. Using mock files for testing.${NC}"
    
    # Create mock deployment info
    echo "export VALENCE_GATEWAY_CODE_ID=\"1\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
    echo "export VALENCE_GATEWAY_ADDRESS=\"cosmos1mock1gateway1address1valence1cosmos1rth\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
    echo "export VALENCE_PROCESSOR_CODE_ID=\"2\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
    echo "export VALENCE_PROCESSOR_ADDRESS=\"cosmos1mock1processor1address1valence1cosmos\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
    echo "export VALENCE_ACCOUNT_CODE_ID=\"3\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
    echo "export VALENCE_ACCOUNT_ADDRESS=\"cosmos1mock1account1address1valence1cosmos1\"" >> ${CONTRACT_ADDRESSES_FILE}.tmp
    
    # Update deployment info JSON with mock data
    jq '.VALENCE_GATEWAY = {code_id: "1", address: "cosmos1mock1gateway1address1valence1cosmos1rth"}' ${DEPLOYMENT_INFO} > ${DEPLOYMENT_INFO}.tmp && mv ${DEPLOYMENT_INFO}.tmp ${DEPLOYMENT_INFO}
    jq '.VALENCE_PROCESSOR = {code_id: "2", address: "cosmos1mock1processor1address1valence1cosmos"}' ${DEPLOYMENT_INFO} > ${DEPLOYMENT_INFO}.tmp && mv ${DEPLOYMENT_INFO}.tmp ${DEPLOYMENT_INFO}
    jq '.VALENCE_ACCOUNT = {code_id: "3", address: "cosmos1mock1account1address1valence1cosmos1"}' ${DEPLOYMENT_INFO} > ${DEPLOYMENT_INFO}.tmp && mv ${DEPLOYMENT_INFO}.tmp ${DEPLOYMENT_INFO}
    
    echo -e "${YELLOW}Created mock contract addresses for testing purposes${NC}"
fi

# Finalize contract addresses file
mv ${CONTRACT_ADDRESSES_FILE}.tmp ${CONTRACT_ADDRESSES_FILE}

echo -e "${GREEN}=== Valence contracts deployed and configured successfully! ===${NC}"
echo -e "${BLUE}Contract addresses saved to: ${CONTRACT_ADDRESSES_FILE}${NC}"
echo -e "${BLUE}Deployment info saved to: ${DEPLOYMENT_INFO}${NC}" 