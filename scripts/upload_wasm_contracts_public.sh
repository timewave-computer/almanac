#!/usr/bin/env bash
# Script to upload Cosmos WASM contracts to a public RPC

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${BLUE}ℹ ${NC}$1"; }
log_success() { echo -e "${GREEN}✓ ${NC}$1"; }
log_warning() { echo -e "${YELLOW}⚠ ${NC}$1"; }
log_error() { echo -e "${RED}✗ ${NC}$1"; }

# Default values
KEY_NAME="mykey"
CHAIN_ID="cosmoshub-4"  # This should be changed to match the target public chain
RPC_URL="https://rpc.cosmos.network:26657"  # Public RPC URL, should be changed for actual use
CONTRACTS_DIR="tests/cosmos-contracts/output"
DEFAULT_FEES="5000uatom"  # Adjust based on the chain's denomination
OUTPUT_FILE="public_cosmos_contracts.env"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --key)
            KEY_NAME="$2"
            shift 2
            ;;
        --chain-id)
            CHAIN_ID="$2"
            shift 2
            ;;
        --rpc)
            RPC_URL="$2"
            shift 2
            ;;
        --contracts-dir)
            CONTRACTS_DIR="$2"
            shift 2
            ;;
        --fees)
            DEFAULT_FEES="$2"
            shift 2
            ;;
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --key KEY_NAME        Key name to use for transactions (default: mykey)"
            echo "  --chain-id CHAIN_ID   Chain ID (default: cosmoshub-4)"
            echo "  --rpc RPC_URL         RPC URL (default: https://rpc.cosmos.network:26657)"
            echo "  --contracts-dir DIR   Directory containing WASM contracts (default: tests/cosmos-contracts/output)"
            echo "  --fees FEES           Transaction fees (default: 5000uatom)"
            echo "  --output FILE         Output file for contract addresses (default: public_cosmos_contracts.env)"
            echo "  --help                Show this help message"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Set directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
FULL_CONTRACTS_DIR="${ROOT_DIR}/${CONTRACTS_DIR}"

log_info "Starting WASM contract upload process to public RPC"
log_info "Using key: ${KEY_NAME}"
log_info "Chain ID: ${CHAIN_ID}"
log_info "RPC URL: ${RPC_URL}"
log_info "Contracts directory: ${FULL_CONTRACTS_DIR}"

# Check if contracts directory exists
if [ ! -d "${FULL_CONTRACTS_DIR}" ]; then
    log_error "Contracts directory not found: ${FULL_CONTRACTS_DIR}"
    exit 1
fi

# Check if wasmd is available
if ! command -v wasmd &> /dev/null; then
    log_error "wasmd command not found. Please ensure it's installed and in your PATH."
    log_info "If using Nix, run 'nix develop' first or prefix this script with 'nix develop --command'"
    exit 1
fi

# Check if key exists
log_info "Checking if key '${KEY_NAME}' exists..."
if ! wasmd keys show "${KEY_NAME}" &> /dev/null; then
    log_error "Key '${KEY_NAME}' not found."
    log_info "You can create a new key with: wasmd keys add ${KEY_NAME}"
    exit 1
fi

# Check connection to chain
log_info "Testing connection to chain at ${RPC_URL}..."
if ! wasmd status --node "${RPC_URL}" &> /dev/null; then
    log_error "Failed to connect to chain at ${RPC_URL}"
    log_info "Please ensure the RPC endpoint is correct and accessible."
    exit 1
fi
log_success "Connection to chain successful"

# Initialize output file
echo "# Cosmos contract addresses on public chain" > "${OUTPUT_FILE}"
echo "# Generated on $(date)" >> "${OUTPUT_FILE}"
echo "# Chain ID: ${CHAIN_ID}" >> "${OUTPUT_FILE}"
echo "# RPC URL: ${RPC_URL}" >> "${OUTPUT_FILE}"
echo "" >> "${OUTPUT_FILE}"

# Find WASM files
WASM_FILES=("${FULL_CONTRACTS_DIR}"/*.wasm)
if [ ${#WASM_FILES[@]} -eq 0 ]; then
    log_error "No WASM files found in ${FULL_CONTRACTS_DIR}"
    exit 1
fi

# Verify user wants to proceed
log_warning "You are about to upload ${#WASM_FILES[@]} contracts to a public chain using key '${KEY_NAME}'."
log_warning "This action will incur gas fees on a live network."
read -p "Do you want to continue? (y/N): " confirm
if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
    log_info "Operation cancelled by user."
    exit 0
fi

for contract_file in "${WASM_FILES[@]}"; do
    contract_name=$(basename "${contract_file}" .wasm)
    log_info "Uploading contract: ${contract_name}"
    
    # Verify WASM file is valid
    if [ ! -s "${contract_file}" ]; then
        log_error "${contract_file} is empty or not readable."
        continue
    fi
    
    # Check file size
    size=$(wc -c < "${contract_file}")
    if [ "$size" -lt 100 ]; then
        log_warning "${contract_file} is very small (${size} bytes). It might not be a valid WASM file."
        read -p "Continue uploading this file? (y/N): " confirm
        if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
            log_info "Skipping ${contract_name}."
            continue
        fi
    fi
    
    # Store contract code
    log_info "Storing ${contract_name} contract code..."
    store_result=$(wasmd tx wasm store "${contract_file}" \
        --from "${KEY_NAME}" \
        --gas auto \
        --fees "${DEFAULT_FEES}" \
        --chain-id "${CHAIN_ID}" \
        --node "${RPC_URL}" \
        --output json \
        --yes)
    
    # Extract code_id
    code_id=$(echo "${store_result}" | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
    
    if [ -z "${code_id}" ]; then
        log_error "Failed to extract code_id for ${contract_name}"
        log_error "Response: ${store_result}"
        continue
    fi
    
    log_success "Stored ${contract_name} with code_id: ${code_id}"
    
    # Instantiate contract
    log_info "Instantiating ${contract_name} contract..."
    wallet_address=$(wasmd keys show "${KEY_NAME}" -a)
    init_msg='{"owner":"'${wallet_address}'"}'
    
    instantiate_result=$(wasmd tx wasm instantiate "${code_id}" "${init_msg}" \
        --from "${KEY_NAME}" \
        --label "${contract_name}" \
        --admin "${wallet_address}" \
        --gas auto \
        --fees "${DEFAULT_FEES}" \
        --chain-id "${CHAIN_ID}" \
        --node "${RPC_URL}" \
        --output json \
        --yes)
    
    # Extract contract address
    contract_addr=$(echo "${instantiate_result}" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
    
    if [ -z "${contract_addr}" ]; then
        log_error "Failed to extract contract address for ${contract_name}"
        log_error "Response: ${instantiate_result}"
        continue
    fi
    
    log_success "Instantiated ${contract_name} at address: ${contract_addr}"
    
    # Save to output file
    echo "${contract_name^^}_CODE_ID=${code_id}" >> "${OUTPUT_FILE}"
    echo "${contract_name^^}_ADDRESS=${contract_addr}" >> "${OUTPUT_FILE}"
    echo "" >> "${OUTPUT_FILE}"
done

log_success "Contract upload process completed!"
log_info "Contract addresses saved to ${OUTPUT_FILE}" 