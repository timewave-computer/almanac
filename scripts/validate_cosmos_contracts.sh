#!/usr/bin/env bash
# Script to validate Cosmos contracts for end-to-end testing

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
CONTRACTS_DIR="$ROOT_DIR/tests/cosmos-contracts"
OUTPUT_DIR="$CONTRACTS_DIR/output"

log_info "Validating Cosmos contract WASM files..."

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Check if each contract exists and is valid
needs_recompilation=false

validate_wasm() {
    local wasm_file="$1"
    local name="$2"
    
    if [ ! -f "$wasm_file" ]; then
        log_warning "$name contract not found at $wasm_file"
        return 1
    fi
    
    size=$(wc -c < "$wasm_file")
    if [ "$size" -lt 100 ]; then
        log_warning "$name contract file is too small ($size bytes), may be invalid"
        return 1
    fi
    
    # Simple validity check - WebAssembly files start with "\0asm"
    if ! hexdump -n 4 "$wasm_file" | grep -q "0000 0061 736d"; then
        log_warning "$name contract does not appear to be a valid WebAssembly file"
        return 1
    fi
    
    log_success "$name contract is valid ($size bytes)"
    return 0
}

for contract in base_account authorization processor; do
    wasm_file="$OUTPUT_DIR/${contract}.wasm"
    if ! validate_wasm "$wasm_file" "$contract"; then
        needs_recompilation=true
    fi
done

# Recompile if necessary
if [ "$needs_recompilation" = true ]; then
    log_info "One or more contracts need to be recompiled, running compilation script..."
    "$SCRIPT_DIR/compile_test_cosmos_contracts.sh"
    
    # Validate again after compilation
    log_info "Validating contracts after recompilation..."
    all_valid=true
    for contract in base_account authorization processor; do
        wasm_file="$OUTPUT_DIR/${contract}.wasm"
        if ! validate_wasm "$wasm_file" "$contract"; then
            all_valid=false
        fi
    done
    
    if [ "$all_valid" = true ]; then
        log_success "All contracts successfully compiled and validated"
    else
        log_error "Some contracts failed validation even after recompilation"
        exit 1
    fi
else
    log_success "All Cosmos contracts are valid"
fi

log_info "Cosmos contracts validation completed!" 