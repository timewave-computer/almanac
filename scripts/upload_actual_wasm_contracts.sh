#!/usr/bin/env bash
# Script to upload pre-compiled Valence WASM contracts from contracts/wasm_compiled

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
KEY_NAME="validator"
CHAIN_ID="testing"
RPC_URL="http://localhost:26657"
CONTRACTS_DIR="contracts/wasm_compiled"
OUTPUT_FILE="cosmos_contract_addresses.env"
DEFAULT_FEES="5000uatom"
CORE_CONTRACTS=("valence_authorization" "valence_base_account" "valence_processor")

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
        --fees)
            DEFAULT_FEES="$2"
            shift 2
            ;;
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --contracts)
            IFS=',' read -ra CORE_CONTRACTS <<< "$2"
            shift 2
            ;;
        --all)
            UPLOAD_ALL=true
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --key KEY_NAME        Key name to use for transactions (default: validator)"
            echo "  --chain-id CHAIN_ID   Chain ID (default: testing)"
            echo "  --rpc RPC_URL         RPC URL (default: http://localhost:26657)"
            echo "  --fees FEES           Transaction fees (default: 5000uatom)"
            echo "  --output FILE         Output file for contract addresses (default: cosmos_contract_addresses.env)"
            echo "  --contracts LIST      Comma-separated list of contract names to upload (without .wasm extension)"
            echo "                        Default: valence_authorization,valence_base_account,valence_processor"
            echo "  --all                 Upload all contracts in the wasm_compiled directory"
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

log_info "Starting Valence WASM contract upload process"
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

# Check connection to chain
log_info "Testing connection to chain at ${RPC_URL}..."
if ! wasmd status --node "${RPC_URL}" &> /dev/null; then
    log_error "Failed to connect to chain at ${RPC_URL}"
    log_info "Make sure the node is running. You can start a local node with 'nix run .#wasmd-node'"
    exit 1
fi
log_success "Connection to chain successful"

# Initialize output file
echo "# Valence Cosmos contract addresses" > "${OUTPUT_FILE}"
echo "# Generated on $(date)" >> "${OUTPUT_FILE}"
echo "# Chain ID: ${CHAIN_ID}" >> "${OUTPUT_FILE}"
echo "# RPC URL: ${RPC_URL}" >> "${OUTPUT_FILE}"
echo "" >> "${OUTPUT_FILE}"

# Get list of contracts to upload
if [ "${UPLOAD_ALL:-false}" = true ]; then
    log_info "Uploading all contracts in ${FULL_CONTRACTS_DIR}"
    CONTRACTS=()
    for contract_file in "${FULL_CONTRACTS_DIR}"/*.wasm; do
        if [ -f "${contract_file}" ]; then
            contract_name=$(basename "${contract_file}" .wasm)
            CONTRACTS+=("${contract_name}")
        fi
    done
else
    log_info "Uploading specified core contracts: ${CORE_CONTRACTS[*]}"
    CONTRACTS=("${CORE_CONTRACTS[@]}")
fi

# Verify contract files exist
for contract_name in "${CONTRACTS[@]}"; do
    contract_file="${FULL_CONTRACTS_DIR}/${contract_name}.wasm"
    if [ ! -f "${contract_file}" ]; then
        log_error "Contract file not found: ${contract_file}"
        exit 1
    fi
    
    # Check file size
    size=$(wc -c < "${contract_file}")
    log_info "Contract ${contract_name}.wasm size: ${size} bytes"
    
    if [ "${size}" -lt 100 ]; then
        log_warning "Contract file ${contract_name}.wasm is suspiciously small (${size} bytes)"
        read -p "Continue with this file? (y/N): " confirm
        if [[ ! "${confirm}" =~ ^[Yy]$ ]]; then
            log_info "Skipping ${contract_name}.wasm"
            continue
        fi
    fi
done

# Confirm before proceeding
log_warning "You are about to upload ${#CONTRACTS[@]} contract(s) to chain ${CHAIN_ID}."
log_warning "This will incur gas fees for each transaction."
read -p "Do you want to continue? (y/N): " confirm
if [[ ! "${confirm}" =~ ^[Yy]$ ]]; then
    log_info "Operation cancelled by user."
    exit 0
fi

# Upload contracts
for contract_name in "${CONTRACTS[@]}"; do
    contract_file="${FULL_CONTRACTS_DIR}/${contract_name}.wasm"
    log_info "Uploading contract: ${contract_name}"
    
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
    init_msg='{"owner":"'$(wasmd keys show "${KEY_NAME}" -a)'"}'
    
    instantiate_result=$(wasmd tx wasm instantiate "${code_id}" "${init_msg}" \
        --from "${KEY_NAME}" \
        --label "${contract_name}" \
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
    echo "# ${contract_name}" >> "${OUTPUT_FILE}"
    echo "${contract_name^^}_CODE_ID=${code_id}" >> "${OUTPUT_FILE}"
    echo "${contract_name^^}_ADDRESS=${contract_addr}" >> "${OUTPUT_FILE}"
    echo "" >> "${OUTPUT_FILE}"
done

log_success "Contract upload process completed!"
log_info "Contract addresses saved to ${OUTPUT_FILE}" 