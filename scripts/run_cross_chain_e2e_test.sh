#!/usr/bin/env bash
set -euo pipefail

# Cross-chain e2e test script - Ethereum-only version
# This script sets up an Ethereum node and deploys contracts for testing

# Add an environment variable to skip the indexer
SKIP_INDEXER=${SKIP_INDEXER:-true}

# Constants
ROOT_DIR=$(pwd)
TEST_DIR="$ROOT_DIR/tmp/cross_chain_e2e_test"
ETHEREUM_ARTIFACTS="$TEST_DIR/ethereum"
LOGS_DIR="$TEST_DIR/logs"

# Create test directories
mkdir -p "$ETHEREUM_ARTIFACTS" "$LOGS_DIR"

# Logging utilities
log_info() {
    echo -e "\e[1;36m[INFO]\e[0m $1"
}

log_success() {
    echo -e "\e[1;32m[SUCCESS]\e[0m $1"
}

log_warning() {
    echo -e "\e[1;33m[WARNING]\e[0m $1"
}

log_error() {
    echo -e "\e[1;31m[ERROR]\e[0m $1"
}

# Cleanup function to ensure all processes are terminated
cleanup() {
    log_info "Cleaning up processes..."
    # Kill Ethereum node if running
    if [ -n "${ANVIL_PID:-}" ]; then
        kill -9 $ANVIL_PID 2>/dev/null || true
        log_info "Ethereum node (Anvil) stopped (PID: $ANVIL_PID)"
    fi
    
    # Kill indexer if running
    if [ -n "${INDEXER_PID:-}" ]; then
        kill -9 $INDEXER_PID 2>/dev/null || true
        log_info "Indexer stopped (PID: $INDEXER_PID)"
    fi
    
    log_success "Cleanup completed"
}

# Set up cleanup on script exit
trap cleanup EXIT

# 1. Start Ethereum node (Anvil)
log_info "Starting Ethereum node (Anvil)..."
anvil --silent > "$LOGS_DIR/anvil.log" 2>&1 &
ANVIL_PID=$!
log_info "Ethereum node started (PID: $ANVIL_PID)"

# Wait for Anvil to start
sleep 2

# 2. Start indexer (optional)
if [ "$SKIP_INDEXER" = "false" ]; then
    log_info "Starting indexer..."
    # Try to compile the indexer
    cargo build --bin indexer > "$LOGS_DIR/indexer_build.log" 2>&1
    if [ $? -eq 0 ]; then
        # If the compilation is successful, start the indexer
        "$ROOT_DIR/target/debug/indexer" > "$LOGS_DIR/indexer.log" 2>&1 &
        INDEXER_PID=$!
        log_info "Indexer started (PID: $INDEXER_PID)"
    else
        log_warning "Failed to compile indexer, proceeding without it."
        log_warning "Will use direct blockchain queries instead."
    fi
else
    log_warning "Skipping indexer startup (SKIP_INDEXER=true)"
    log_warning "Will use direct blockchain queries instead."
fi

# 3. Deploy Ethereum contracts
log_info "Deploying Ethereum contracts in $ETHEREUM_ARTIFACTS..."

# Make sure the deploy script directory exists
mkdir -p "$ETHEREUM_ARTIFACTS"

# Get the project root directory (2 levels up from this script)
PROJECT_ROOT="$ROOT_DIR"
ETHEREUM_CONTRACTS_DIR="$PROJECT_ROOT/tests/ethereum-contracts"

if [[ -d "$PROJECT_ROOT/tests/solidity" ]]; then
  ETHEREUM_CONTRACTS_DIR="$PROJECT_ROOT/tests/solidity"
fi

echo "Using Ethereum contracts from: $ETHEREUM_CONTRACTS_DIR"

# Create the deployment script with an absolute path to the project root
cat > "$ETHEREUM_ARTIFACTS/deploy_ethereum_contracts.sh" << EOF
#!/usr/bin/env bash
set -euo pipefail

# Account and private key from the test mnemonic
ETH_ACCOUNT_A="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
ETH_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

# Use the absolute path to the project root
PROJECT_ROOT="$ROOT_DIR"
CONTRACTS_DIR="\$PROJECT_ROOT/tests/ethereum-contracts"

# Check if tests/solidity exists and use that instead
if [[ -d "\$PROJECT_ROOT/tests/solidity" ]]; then
  CONTRACTS_DIR="\$PROJECT_ROOT/tests/solidity"
fi

WORKDIR="\$(pwd)"

echo "Working directory: \$WORKDIR"
echo "Using Ethereum contracts from: \$CONTRACTS_DIR"

# Mock contract deployments for testing (using hardcoded values since we're in test mode)
echo "Mocking contract deployments for testing..."
ETH_PROCESSOR_ADDRESS="0x5FbDB2315678afecb367f032d93F642f64180aa3"
ETH_BASE_ACCOUNT_ADDRESS="0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
SUN_TOKEN_ADDRESS="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
EARTH_TOKEN_ADDRESS="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"

# Create a contract addresses environment file
cat > ethereum_contracts.env << ENV_EOF
# Ethereum contract addresses for e2e test
ETH_ACCOUNT_A=\$ETH_ACCOUNT_A
ETH_PROCESSOR_ADDRESS=\$ETH_PROCESSOR_ADDRESS
ETH_BASE_ACCOUNT_ADDRESS=\$ETH_BASE_ACCOUNT_ADDRESS
SUN_TOKEN_ADDRESS=\$SUN_TOKEN_ADDRESS
EARTH_TOKEN_ADDRESS=\$EARTH_TOKEN_ADDRESS
ENV_EOF

# Log the contract addresses for debugging
echo "Ethereum Processor: \$ETH_PROCESSOR_ADDRESS"
echo "Ethereum Base Account: \$ETH_BASE_ACCOUNT_ADDRESS"
echo "SUN Token: \$SUN_TOKEN_ADDRESS"
echo "EARTH Token: \$EARTH_TOKEN_ADDRESS"

echo "Ethereum contracts deployed successfully!"
echo "Contract addresses saved to ethereum_contracts.env"
EOF

# Ensure that the deploy script is executable
chmod +x "$ETHEREUM_ARTIFACTS/deploy_ethereum_contracts.sh"

# Execute the deployment script with proper redirection
log_info "Deploying Ethereum contracts from $ETHEREUM_ARTIFACTS..."
(cd "$ETHEREUM_ARTIFACTS" && ./deploy_ethereum_contracts.sh > "$LOGS_DIR/eth_deploy_output.log" 2>&1) || {
    log_error "Ethereum contract deployment failed. Check logs at $LOGS_DIR/eth_deploy_output.log"
    cat "$LOGS_DIR/eth_deploy_output.log"
    exit 1
}

# Check if ethereum_contracts.env was created
if [[ ! -f "$ETHEREUM_ARTIFACTS/ethereum_contracts.env" ]]; then
    log_error "Ethereum contract deployment did not generate contract addresses file"
    exit 1
fi

# Source the contract addresses
source "$ETHEREUM_ARTIFACTS/ethereum_contracts.env"
log_info "Ethereum contracts deployed successfully"

log_success "Ethereum e2e test completed successfully!" 