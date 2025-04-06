#!/usr/bin/env bash
set -euo pipefail

# Define colors for better output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Define logging functions
log_info() { echo -e "${BLUE}ℹ ${NC}$1"; }
log_success() { echo -e "${GREEN}✓ ${NC}$1"; }
log_warning() { echo -e "${YELLOW}⚠ ${NC}$1"; }
log_error() { echo -e "${RED}✗ ${NC}$1"; }

# Set directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
VALENCE_DIR="$ROOT_DIR/valence-protocol"
CONTRACTS_DIR="$VALENCE_DIR/contracts"
TEMP_DIR="${ROOT_DIR}/tmp/valence_contract_deploy"
WASMD_HOME="$HOME/.wasmd-test"
RPC_URL="http://localhost:26657"
REST_URL="http://localhost:1317"
mkdir -p "$TEMP_DIR"

# Check if wasmd node is running
log_info "Checking if wasmd node is running..."
if ! curl -s "$RPC_URL/status" > /dev/null; then
    log_error "The wasmd node does not appear to be running."
    log_info "Please run ./scripts/run_wasmd_with_fixed_timeout.sh in another terminal first."
    exit 1
fi

# Get chain info
CHAIN_INFO=$(curl -s "$RPC_URL/status")
CHAIN_ID=$(echo "$CHAIN_INFO" | jq -r '.result.node_info.network')
LATEST_BLOCK=$(echo "$CHAIN_INFO" | jq -r '.result.sync_info.latest_block_height')
VALIDATOR_PUBKEY=$(curl -s "$RPC_URL/validators" | jq -r '.result.validators[0].pub_key.value')
log_success "Connected to chain: $CHAIN_ID (Block height: $LATEST_BLOCK)"

# Choose a contract to build
CONTRACT_PATH="program-registry"
CONTRACT_DIR="$CONTRACTS_DIR/$CONTRACT_PATH"

log_info "Building contract: $CONTRACT_PATH"
cd "$VALENCE_DIR"

# Check if cargo and rustup are installed
if ! command -v cargo &> /dev/null || ! command -v rustup &> /dev/null; then
    log_error "Cargo or rustup is not installed. Please install Rust and Cargo."
    exit 1
fi

# Ensure wasm32 target is installed
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    log_info "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Build the contract in release mode - explicitly building the library only
log_info "Building contract in release mode (library only)..."
cd "$CONTRACT_DIR"
RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release --lib

# Copy the built wasm file to the temp directory
WASM_FILE="$VALENCE_DIR/target/wasm32-unknown-unknown/release/valence_program_registry.wasm"
TARGET_WASM="$TEMP_DIR/program_registry.wasm"

if [ -f "$WASM_FILE" ]; then
    cp "$WASM_FILE" "$TARGET_WASM"
    log_success "Contract built and copied to $TARGET_WASM"
else
    log_error "Failed to build contract. Wasm file not found at $WASM_FILE"
    exit 1
fi

# Optimize the wasm file for smaller size if wasm-opt is available
if command -v wasm-opt &> /dev/null; then
    log_info "Optimizing wasm file for deployment..."
    wasm-opt -Os "$TARGET_WASM" -o "$TARGET_WASM.optimized"
    mv "$TARGET_WASM.optimized" "$TARGET_WASM"
    log_success "Wasm file optimized"
else
    log_warning "wasm-opt not found. Skipping optimization step."
fi

# Get contract size and hash for verification
CONTRACT_SIZE=$(wc -c < "$TARGET_WASM")
CONTRACT_HASH=$(shasum -a 256 "$TARGET_WASM" | cut -d ' ' -f 1)
log_info "Contract size: $CONTRACT_SIZE bytes"
log_info "Contract hash: $CONTRACT_HASH"

# Since we don't have direct wasmd access, simulate the store and instantiate
log_info "Simulating contract deployment (we don't have wasmd CLI access)..."

# Convert contract to base64 for future use (commented out due to size)
# log_info "Converting contract to base64 (this takes a moment for large contracts)..."
# WASM_BASE64=$(base64 "$TARGET_WASM" | tr -d '\n')
# log_success "Contract converted to base64"

# For a real deployment we would need:
# 1. Create and sign a StoreCode transaction
# 2. Broadcast the transaction to the network
# 3. Get the code ID from the transaction result
# 4. Create and sign an InstantiateContract transaction
# 5. Broadcast the transaction to the network
# 6. Get the contract address from the transaction result

# In this demo, we'll just use simulated values
log_info "For a real deployment, we would need to:"
log_info "1. Create and sign a StoreCode transaction with the contract WASM"
log_info "2. Submit the signed transaction to the REST endpoint: $REST_URL/cosmos/tx/v1beta1/txs"
log_info "3. Extract the code ID from the transaction result"
log_info "4. Create and sign an InstantiateContract transaction"
log_info "5. Submit this transaction"
log_info "6. Extract the contract address from the result"

# Use simulated values for demonstration
CODE_ID=1
# Generate a deterministic address for demo purposes
VALIDATOR_ADDR_BIN=$(echo -n "$VALIDATOR_PUBKEY" | xxd -r -p 2>/dev/null || echo "demo_validator")
SIMULATED_CHECKSUM=$(echo -n "${CHAIN_ID}${CODE_ID}${VALIDATOR_ADDR_BIN}" | shasum -a 256 | head -c 40)
CONTRACT_ADDR="wasm1${SIMULATED_CHECKSUM}"

# Create a sample initialization message for the program-registry
INIT_MSG='{"owner":"'"$CONTRACT_ADDR"'"}'
log_info "Contract initialization message would be: $INIT_MSG"

# Print summary
echo ""
log_info "====== VALENCE CONTRACT BUILD SUMMARY ======"
echo "Chain ID:          $CHAIN_ID"
echo "Latest Block:      $LATEST_BLOCK"
echo "Validator Pubkey:  ${VALIDATOR_PUBKEY:0:16}... (truncated)"
echo "Contract:          $CONTRACT_PATH"
echo "Contract Size:     $CONTRACT_SIZE bytes"
echo "Contract Hash:     $CONTRACT_HASH"
echo "Simulated Code ID: $CODE_ID"
echo "Simulated Address: $CONTRACT_ADDR"
echo "RPC URL:           $RPC_URL"
echo "REST URL:          $REST_URL"
echo "------------------------------------"

log_success "Valence contract build completed!"
log_warning "Note: The contract was not deployed to the chain as this would require a properly signed transaction."
log_info "For actual deployment, you would need to:"
log_info "1. Install wasmd CLI or"
log_info "2. Create and sign transactions with a wallet and submit them via the REST API" 