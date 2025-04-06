#!/usr/bin/env bash
# Script to copy pre-compiled WASM contracts to the correct location for e2e testing

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

# Set directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
SOURCE_DIR="${ROOT_DIR}/contracts/wasm_compiled"
TARGET_DIR="${ROOT_DIR}/tests/cosmos-contracts"
OUTPUT_DIR="${TARGET_DIR}/output"

log_info "Setting up WASM contracts for cross-chain e2e test"
log_info "Source directory: ${SOURCE_DIR}"
log_info "Target directory: ${TARGET_DIR}"
log_info "Output directory: ${OUTPUT_DIR}"

# Check if source directory exists
if [ ! -d "${SOURCE_DIR}" ]; then
    log_error "Source directory not found: ${SOURCE_DIR}"
    exit 1
fi

# Create output directory if it doesn't exist
mkdir -p "${OUTPUT_DIR}"

# Mapping of compiled contract names to the names expected by the e2e test
declare -A CONTRACT_MAP=(
    ["valence_authorization"]="authorization"
    ["valence_base_account"]="base_account"
    ["valence_processor"]="processor"
)

# Copy and rename the contracts
for source_name in "${!CONTRACT_MAP[@]}"; do
    target_name="${CONTRACT_MAP[$source_name]}"
    source_file="${SOURCE_DIR}/${source_name}.wasm"
    target_file="${OUTPUT_DIR}/${target_name}.wasm"
    
    if [ ! -f "${source_file}" ]; then
        log_error "Source file not found: ${source_file}"
        exit 1
    fi
    
    log_info "Copying ${source_name}.wasm to ${target_file}"
    cp "${source_file}" "${target_file}"
    
    if [ -f "${target_file}" ]; then
        size=$(wc -c < "${target_file}")
        log_success "Copied ${target_name}.wasm (${size} bytes)"
    else
        log_error "Failed to copy ${target_name}.wasm"
        exit 1
    fi
done

# Also copy to the main test directory for compatibility with some scripts
for source_name in "${!CONTRACT_MAP[@]}"; do
    target_name="${CONTRACT_MAP[$source_name]}"
    source_file="${SOURCE_DIR}/${source_name}.wasm"
    target_file="${TARGET_DIR}/${target_name}.wasm"
    
    log_info "Copying ${source_name}.wasm to ${target_file}"
    cp "${source_file}" "${target_file}"
    
    if [ -f "${target_file}" ]; then
        size=$(wc -c < "${target_file}")
        log_success "Copied ${target_name}.wasm (${size} bytes)"
    else
        log_error "Failed to copy ${target_name}.wasm"
        exit 1
    fi
done

log_success "WASM contracts set up successfully for e2e testing!"
log_info "You can now run the cross-chain e2e test with: nix run .#cross-chain-e2e-test" 