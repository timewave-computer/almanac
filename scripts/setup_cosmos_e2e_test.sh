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
ARTIFACTS_DIR="${ROOT_DIR}/tmp/cosmos_e2e_artifacts"
WASMD_HOME="$HOME/.wasmd-test"
RPC_URL="http://localhost:26657"
REST_URL="http://localhost:1317"

mkdir -p "$ARTIFACTS_DIR"

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
log_success "Connected to chain: $CHAIN_ID (Block height: $LATEST_BLOCK)"

# Generate a deterministic test account from test mnemonic
COSMOS_ACCOUNT_B="cosmos1phaxpevm5wecex2jyaqty2a4v02qj7qmhz2r5e" # First address from test mnemonic
log_info "Using test account: $COSMOS_ACCOUNT_B"

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

# Function to build a contract
build_contract() {
    local contract_path=$1
    local target_name=$2
    local dir_name=$contract_path
    
    log_info "Building contract: $contract_path"
    
    # Handle special cases with different directory structures
    if [ "$contract_path" = "accounts" ]; then
        dir_name="accounts/base_account"
        target_name="valence_base_account"
    fi
    
    # Check if directory exists
    if [ ! -d "$CONTRACTS_DIR/$dir_name" ]; then
        log_error "Contract directory $CONTRACTS_DIR/$dir_name does not exist."
        return 1
    fi
    
    # Build contract
    cd "$CONTRACTS_DIR/$dir_name"
    RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release --lib
    
    # Get output file name from Cargo.toml if target_name is not provided
    if [ -z "$target_name" ]; then
        target_name=$(grep -m 1 "name" Cargo.toml | cut -d '=' -f2 | tr -d ' "' | tr '-' '_')
    fi
    
    # Copy built file to artifacts directory
    local wasm_file="$VALENCE_DIR/target/wasm32-unknown-unknown/release/${target_name}.wasm"
    if [ -f "$wasm_file" ]; then
        cp "$wasm_file" "$ARTIFACTS_DIR/${contract_path}.wasm"
        log_success "Contract built and copied to $ARTIFACTS_DIR/${contract_path}.wasm"
        return 0
    else
        log_error "Failed to build contract. Wasm file not found at $wasm_file"
        return 1
    fi
}

# Build all required contracts
log_info "Building contracts for e2e test..."

# 1. Build Processor Contract
build_contract "processor" "valence_processor"

# 2. Build Base Account Contract
build_contract "accounts" "valence_base_account"

# 3. Build Authorization Contract
build_contract "authorization" "valence_authorization"

# 4. Find and build CW20 Token Contract
# Check if CW20 is in the valence protocol contracts
if [ -d "$CONTRACTS_DIR/token" ]; then
    build_contract "token" "valence_cw20_token"
elif [ -d "$CONTRACTS_DIR/libraries/cw20" ]; then
    build_contract "libraries/cw20" "valence_cw20_token"
else
    log_warning "CW20 contract not found in Valence protocol. Will use an external CW20 implementation."
    # Download cw20_base for the tests
    CW20_URL="https://github.com/CosmWasm/cw-plus/releases/download/v1.0.0/cw20_base.wasm"
    log_info "Downloading CW20 base contract from $CW20_URL"
    curl -L -o "$ARTIFACTS_DIR/cw20_base.wasm" "$CW20_URL"
    if [ -f "$ARTIFACTS_DIR/cw20_base.wasm" ]; then
        log_success "CW20 base contract downloaded to $ARTIFACTS_DIR/cw20_base.wasm"
    else
        log_error "Failed to download CW20 base contract."
        exit 1
    fi
fi

# Get contract sizes and hashes for verification
log_info "Contract build summary:"
for contract in "$ARTIFACTS_DIR"/*.wasm; do
    if [ -f "$contract" ]; then
        CONTRACT_NAME=$(basename "$contract")
        CONTRACT_SIZE=$(wc -c < "$contract")
        CONTRACT_HASH=$(shasum -a 256 "$contract" | cut -d ' ' -f 1)
        printf "%-30s %10d bytes   %s\n" "$CONTRACT_NAME" "$CONTRACT_SIZE" "$CONTRACT_HASH"
    fi
done

# Generate deployment instructions
log_info "Generating deployment instructions for the e2e test..."

# Create deployment instructions file
DEPLOY_FILE="$ARTIFACTS_DIR/deployment_instructions.sh"
cat > "$DEPLOY_FILE" << EOL
#!/usr/bin/env bash
# E2E Test Contract Deployment Instructions
# Chain ID: $CHAIN_ID
# Generated: $(date)

# Start wasmd node in a separate terminal:
# ./scripts/run_wasmd_with_fixed_timeout.sh

# Set variables
WASMD_HOME="$WASMD_HOME"
CHAIN_ID="$CHAIN_ID"
COSMOS_ACCOUNT_B="$COSMOS_ACCOUNT_B"

# Store and instantiate Processor Contract
echo "Storing Cosmos Processor Contract..."
PROCESSOR_CODE_ID=\$(wasmd tx wasm store $ARTIFACTS_DIR/processor.wasm --from validator --chain-id=\$CHAIN_ID --gas=4000000 --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
echo "Processor Code ID: \$PROCESSOR_CODE_ID"

echo "Instantiating Cosmos Processor Contract..."
COSMOS_PROCESSOR_ADDRESS=\$(wasmd tx wasm instantiate \$PROCESSOR_CODE_ID '{"owner":"'\$COSMOS_ACCOUNT_B'"}' --from validator --chain-id=\$CHAIN_ID --label "processor" --no-admin --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
echo "Cosmos Processor: \$COSMOS_PROCESSOR_ADDRESS"

# Store and instantiate Base Account Contract
echo "Storing Cosmos Base Account Contract..."
BASE_ACCOUNT_CODE_ID=\$(wasmd tx wasm store $ARTIFACTS_DIR/accounts.wasm --from validator --chain-id=\$CHAIN_ID --gas=4000000 --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
echo "Base Account Code ID: \$BASE_ACCOUNT_CODE_ID"

echo "Instantiating Cosmos Base Account Contract..."
COSMOS_BASE_ACCOUNT_ADDRESS=\$(wasmd tx wasm instantiate \$BASE_ACCOUNT_CODE_ID '{"owner":"'\$COSMOS_ACCOUNT_B'"}' --from validator --chain-id=\$CHAIN_ID --label "base_account" --no-admin --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
echo "Cosmos Base Account: \$COSMOS_BASE_ACCOUNT_ADDRESS"

# Store and instantiate Authorization Contract
echo "Storing Cosmos Authorization Contract..."
AUTH_CODE_ID=\$(wasmd tx wasm store $ARTIFACTS_DIR/authorization.wasm --from validator --chain-id=\$CHAIN_ID --gas=4000000 --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
echo "Authorization Code ID: \$AUTH_CODE_ID"

echo "Instantiating Cosmos Authorization Contract..."
COSMOS_AUTH_ADDRESS=\$(wasmd tx wasm instantiate \$AUTH_CODE_ID '{"owner":"'\$COSMOS_ACCOUNT_B'"}' --from validator --chain-id=\$CHAIN_ID --label "authorization" --no-admin --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
echo "Cosmos Authorization: \$COSMOS_AUTH_ADDRESS"

# Store and instantiate MOON Token Contract (CW20)
echo "Storing CW20 Token Contract..."
CW20_CODE_ID=\$(wasmd tx wasm store $ARTIFACTS_DIR/cw20_base.wasm --from validator --chain-id=\$CHAIN_ID --gas=4000000 --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
echo "CW20 Code ID: \$CW20_CODE_ID"

echo "Instantiating MOON Token Contract..."
MOON_TOKEN_ADDRESS=\$(wasmd tx wasm instantiate \$CW20_CODE_ID '{"name":"MOON Token","symbol":"MOON","decimals":6,"initial_balances":[{"address":"'\$COSMOS_BASE_ACCOUNT_ADDRESS'","amount":"10000000"}],"mint":{"minter":"'\$COSMOS_ACCOUNT_B'"}}' --from validator --chain-id=\$CHAIN_ID --label "moon_token" --no-admin --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
echo "MOON Token: \$MOON_TOKEN_ADDRESS"

# Set EOA Account B as authorized for Cosmos Base Account
echo "Authorizing COSMOS Account B to control COSMOS Base Account..."
wasmd tx wasm execute \$COSMOS_AUTH_ADDRESS '{"grant_permission":{"grant_id":"auth1","grantee":"'\$COSMOS_ACCOUNT_B'","permissions":["execute"],"resources":["\$COSMOS_BASE_ACCOUNT_ADDRESS"]}}' --from validator --chain-id=\$CHAIN_ID --output json --keyring-backend=test --yes | jq

# Save contract addresses to environment file for the e2e test
cat > "$ARTIFACTS_DIR/cosmos_contracts.env" << EOF
# Cosmos contract addresses for e2e test
COSMOS_ACCOUNT_B=\$COSMOS_ACCOUNT_B
COSMOS_PROCESSOR_ADDRESS=\$COSMOS_PROCESSOR_ADDRESS
COSMOS_BASE_ACCOUNT_ADDRESS=\$COSMOS_BASE_ACCOUNT_ADDRESS
COSMOS_AUTH_ADDRESS=\$COSMOS_AUTH_ADDRESS
MOON_TOKEN_ADDRESS=\$MOON_TOKEN_ADDRESS
EOF

echo "Environment variables saved to $ARTIFACTS_DIR/cosmos_contracts.env"
echo "Cosmos contract setup completed!"
EOL

chmod +x "$DEPLOY_FILE"

log_success "Contracts built successfully for e2e test!"
log_info "To deploy the contracts, you need to have the wasmd CLI installed."
log_info "Deployment instructions have been generated at $DEPLOY_FILE"
log_info "Review the file and run it manually to deploy the contracts."
log_info "You can also copy the contract artifacts to the location expected by your e2e test."

# Create a script to run the e2e test
E2E_TEST_FILE="$ARTIFACTS_DIR/run_e2e_test.sh"
cat > "$E2E_TEST_FILE" << EOL
#!/usr/bin/env bash
# E2E Test Runner Script
# Chain ID: $CHAIN_ID
# Generated: $(date)

# Start prerequisites in separate terminals:
# 1. Ethereum node: anvil --mnemonic "test test test test test test test test test test test junk" --block-time 2
# 2. Cosmos node: ./scripts/run_wasmd_with_fixed_timeout.sh
# 3. Indexer: cargo run --bin indexer -- --config config/test_config.toml

# Load contract addresses
source "$ARTIFACTS_DIR/cosmos_contracts.env"

# Set Ethereum variables
ETH_ACCOUNT_A="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266" # First address from test mnemonic
ETH_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" # First private key from test mnemonic
BURN_ADDRESS="0x000000000000000000000000000000000000dEaD"

# Deploy Ethereum contracts
echo "Deploying Ethereum contracts..."
# [You need to add the actual deployment commands for Ethereum contracts here]

# Run the e2e test
echo "Starting the e2e test monitoring scripts..."
# [You need to add the actual e2e test monitoring script execution here]

echo "E2E test setup completed!"
EOL

chmod +x "$E2E_TEST_FILE"

log_info "An e2e test runner script template has been generated at $E2E_TEST_FILE"
log_info "You will need to customize it with your specific Ethereum contract deployment commands"
log_info "and the actual e2e test monitoring script execution."
EOL 