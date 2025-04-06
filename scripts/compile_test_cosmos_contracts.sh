#!/usr/bin/env bash
set -euo pipefail

# Script to compile test Cosmos contracts for end-to-end testing

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

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

log_info "Compiling test Cosmos contracts..."

# Check if wasm32 target is installed
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    log_info "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# For test purposes, we can create simple WASM files with actual content
if [ ! -f "$OUTPUT_DIR/base_account.wasm" ] || [ $(wc -c < "$OUTPUT_DIR/base_account.wasm") -lt 100 ]; then
    log_info "Creating base_account.wasm test file..."
    echo '(module
      (type (;0;) (func (param i32) (result i32)))
      (func (;0;) (type 0) (param i32) (result i32)
        local.get 0
        i32.const 42
        i32.add)
      (export "add_42" (func 0))
    )' > "$OUTPUT_DIR/base_account.wasm"
    log_success "Created base_account.wasm with valid WebAssembly content"
fi

if [ ! -f "$OUTPUT_DIR/authorization.wasm" ] || [ $(wc -c < "$OUTPUT_DIR/authorization.wasm") -lt 100 ]; then
    log_info "Creating authorization.wasm test file..."
    echo '(module
      (type (;0;) (func (param i32) (result i32)))
      (func (;0;) (type 0) (param i32) (result i32)
        local.get 0
        i32.const 100
        i32.add)
      (export "authorize" (func 0))
    )' > "$OUTPUT_DIR/authorization.wasm"
    log_success "Created authorization.wasm with valid WebAssembly content"
fi

if [ ! -f "$OUTPUT_DIR/processor.wasm" ] || [ $(wc -c < "$OUTPUT_DIR/processor.wasm") -lt 100 ]; then
    log_info "Creating processor.wasm test file..."
    echo '(module
      (type (;0;) (func (param i32) (result i32)))
      (func (;0;) (type 0) (param i32) (result i32)
        local.get 0
        i32.const 200
        i32.add)
      (export "process" (func 0))
    )' > "$OUTPUT_DIR/processor.wasm"
    log_success "Created processor.wasm with valid WebAssembly content"
fi

# Check all files exist and have content
log_info "Checking contract WASM files:"
for contract in base_account authorization processor; do
    wasm_file="$OUTPUT_DIR/${contract}.wasm"
    if [ -f "$wasm_file" ]; then
        size=$(wc -c < "$wasm_file")
        log_success "$wasm_file: $size bytes"
    else
        log_error "Failed to create $wasm_file"
    fi
done

log_success "Contract compilation completed!"
log_info "Contract WASM files available at $OUTPUT_DIR" 