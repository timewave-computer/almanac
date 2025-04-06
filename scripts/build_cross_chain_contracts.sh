#!/usr/bin/env bash
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
VALENCE_DIR="$ROOT_DIR/valence-protocol"
CONTRACTS_DIR="$VALENCE_DIR/contracts"
ARTIFACTS_DIR="${ROOT_DIR}/tmp/cosmos_e2e_artifacts"

# Create artifacts directory if it doesn't exist
mkdir -p "$ARTIFACTS_DIR"

log_info "Building specific contracts for cross-chain e2e test..."

# Check if wasm32 target is installed
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    log_info "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Function to build a specific contract
build_contract() {
    local contract_dir=$1
    local target_name=$2
    local output_name=$3
    
    log_info "Building contract: $contract_dir"
    
    if [ ! -d "$contract_dir" ]; then
        log_error "Contract directory $contract_dir does not exist."
        return 1
    fi
    
    # First try to use wasm-opt if it's available
    if command -v wasm-opt &> /dev/null; then
        log_info "Using wasm-opt tool to optimize the build..."
        
        pushd "$contract_dir" > /dev/null
        
        # Build the contract with cargo
        cargo build --target wasm32-unknown-unknown --release
        
        local wasm_file="$VALENCE_DIR/target/wasm32-unknown-unknown/release/${target_name}.wasm"
        
        if [ -f "$wasm_file" ]; then
            # Optimize the wasm with wasm-opt
            local optimized_wasm="$ARTIFACTS_DIR/${output_name}.wasm"
            wasm-opt -Os "$wasm_file" -o "$optimized_wasm"
            
            log_success "Contract built and optimized to $optimized_wasm"
            popd > /dev/null
            return 0
        else
            log_warning "Wasm file not found, trying alternate method..."
            popd > /dev/null
        fi
    fi
    
    # If wasm-opt isn't available or build failed, try docker method
    if command -v docker &> /dev/null; then
        log_info "Using Docker to build the contract..."
        
        # Create a temp directory 
        local tmp_dir=$(mktemp -d)
        log_info "Created temp directory $tmp_dir"
        
        # Copy the contract to the temp directory
        cp -r "$contract_dir"/* "$tmp_dir/"
        
        # Run the build in Docker
        docker run --rm -v "$tmp_dir:/code" \
            -v "$ARTIFACTS_DIR:/artifacts" \
            cosmwasm/rust-optimizer:0.13.0
        
        # Copy the optimized wasm to the artifacts directory
        if [ -f "$tmp_dir/artifacts/${target_name}.wasm" ]; then
            cp "$tmp_dir/artifacts/${target_name}.wasm" "$ARTIFACTS_DIR/${output_name}.wasm"
            log_success "Contract built with Docker and copied to $ARTIFACTS_DIR/${output_name}.wasm"
            rm -rf "$tmp_dir"
            return 0
        else
            log_error "Docker build failed: No wasm artifact found"
            rm -rf "$tmp_dir"
        fi
    fi
    
    # Last resort: Try to download a prebuilt version if available
    log_warning "All build methods failed. Trying to use prebuilt artifacts..."
    
    # Check if any of the typical contracts is downloadable from a release
    if [[ "$output_name" == "cw20_base" ]]; then
        log_info "Downloading cw20_base.wasm..."
        curl -L -o "$ARTIFACTS_DIR/${output_name}.wasm" \
            "https://github.com/CosmWasm/cw-plus/releases/download/v1.0.0/cw20_base.wasm"
            
        if [ -f "$ARTIFACTS_DIR/${output_name}.wasm" ]; then
            log_success "Downloaded ${output_name}.wasm"
            return 0
        fi
    fi
    
    log_error "All build methods failed for $contract_dir"
    return 1
}

# Try to build each contract
build_contract "$CONTRACTS_DIR/accounts/base_account" "valence_base_account" "accounts" || log_warning "Failed to build base_account contract"
build_contract "$CONTRACTS_DIR/authorization" "valence_authorization" "authorization" || log_warning "Failed to build authorization contract"
build_contract "$CONTRACTS_DIR/processor" "valence_processor" "processor" || log_warning "Failed to build processor contract"

# Try to get cw20 contract if not available
if [ ! -f "$ARTIFACTS_DIR/cw20_base.wasm" ]; then
    log_info "Downloading cw20_base.wasm..."
    curl -L -o "$ARTIFACTS_DIR/cw20_base.wasm" \
        "https://github.com/CosmWasm/cw-plus/releases/download/v1.0.0/cw20_base.wasm"
fi

# Get contract sizes and check if they exist
log_info "Contract build summary:"
CONTRACTS_FOUND=0
for contract in "$ARTIFACTS_DIR"/*.wasm; do
    if [ -f "$contract" ]; then
        CONTRACTS_FOUND=$((CONTRACTS_FOUND+1))
        CONTRACT_NAME=$(basename "$contract")
        CONTRACT_SIZE=$(wc -c < "$contract")
        CONTRACT_HASH=$(shasum -a 256 "$contract" | cut -d ' ' -f 1)
        printf "%-30s %10d bytes   %s\n" "$CONTRACT_NAME" "$CONTRACT_SIZE" "$CONTRACT_HASH"
    fi
done

if [ $CONTRACTS_FOUND -eq 0 ]; then
    log_error "No contracts were built or found!"
    exit 1
fi

# Create a basic deployment script if none exists
DEPLOY_SCRIPT="$ARTIFACTS_DIR/deployment_instructions.sh"
if [ ! -f "$DEPLOY_SCRIPT" ]; then
    log_info "Creating basic deployment script..."
    
    cat > "$DEPLOY_SCRIPT" << 'EOL'
#!/usr/bin/env bash
# E2E Test Contract Deployment Instructions

# Set variables
CHAIN_ID="testing"
COSMOS_ACCOUNT_B="cosmos1phaxpevm5wecex2jyaqty2a4v02qj7qmhz2r5e"

# Store Cosmos contracts
echo "Storing Cosmos contracts..."

# Store and instantiate contracts as needed
for contract in *.wasm; do
    echo "Storing $contract..."
    CODE_ID=$(wasmd tx wasm store "$contract" --from validator --chain-id=$CHAIN_ID --gas=4000000 --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
    echo "Code ID for $contract: $CODE_ID"
    
    CONTRACT_NAME="${contract%.*}"
    echo "Instantiating $CONTRACT_NAME..."
    CONTRACT_ADDR=$(wasmd tx wasm instantiate $CODE_ID '{"owner":"'$COSMOS_ACCOUNT_B'"}' --from validator --chain-id=$CHAIN_ID --label "$CONTRACT_NAME" --no-admin --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
    echo "$CONTRACT_NAME address: $CONTRACT_ADDR"
    
    # Save the contract address to environment file
    echo "${CONTRACT_NAME^^}_ADDRESS=$CONTRACT_ADDR" >> cosmos_contracts.env
done

echo "Cosmos contracts deployed successfully!"
echo "Contract addresses saved to cosmos_contracts.env"
EOL

    chmod +x "$DEPLOY_SCRIPT"
    log_success "Created basic deployment script at $DEPLOY_SCRIPT"
fi

log_success "Contract build process completed!"
log_info "Contract artifacts available at $ARTIFACTS_DIR"
log_info "To deploy contracts: cd $ARTIFACTS_DIR && ./deployment_instructions.sh" 