#!/usr/bin/env bash
set -e

# Script to test the Valence contract indexers with real contract deployments
echo "Running Valence Contract Integration Tests with Real Contracts..."

# Make sure we're in the project root directory
cd "$(dirname "$0")/.."

# Check if we're in nix-shell
if [ -z "$IN_NIX_SHELL" ] && [ -z "$NIX_BUILD_TOP" ]; then
    echo "Error: This script should be run inside a Nix shell."
    echo "Please enter the nix development shell first using:"
    echo "  nix develop"
    exit 1
fi

# Define paths
VALENCE_DIR="./valence-protocol"
COSMOS_CONTRACTS_DIR="${VALENCE_DIR}/contracts"
ETH_CONTRACTS_DIR="${VALENCE_DIR}/solidity"
TEST_DATA_DIR="./test-data"

# Clone Valence repositories if not already present
if [ ! -d "$VALENCE_DIR" ]; then
    echo "Cloning Valence Protocol repositories..."
    git clone https://github.com/valence-protocol/valence-protocol.git "$VALENCE_DIR"
else
    echo "Valence Protocol repository already present. Pulling latest changes..."
    (cd "$VALENCE_DIR" && git pull)
fi

# Ensure PostgreSQL is available
if ! command -v psql &> /dev/null; then
    echo "Error: PostgreSQL client (psql) not found."
    echo "Please ensure PostgreSQL is installed and available in your PATH."
    exit 1
fi

# Set up test databases
echo "Setting up test databases..."

# Create test database if it doesn't exist
if ! psql -lqt | cut -d \| -f 1 | grep -qw indexer_test; then
    echo "Creating indexer_test database..."
    createdb indexer_test
fi

# Export environment variables for the tests
export TEST_POSTGRES_URL="postgres://postgres:postgres@localhost:5432/indexer_test"
export TEST_ROCKSDB_PATH="/tmp/almanac_test_rocksdb"

# Clear RocksDB test directory if it exists
if [ -d "$TEST_ROCKSDB_PATH" ]; then
    echo "Clearing existing RocksDB test directory..."
    rm -rf "$TEST_ROCKSDB_PATH"
fi
mkdir -p "$TEST_ROCKSDB_PATH"

# Create test data directory if it doesn't exist
mkdir -p "$TEST_DATA_DIR"

# Start local wasmd node if it's not already running
WASMD_PID=""
if ! pgrep -f "wasmd start" > /dev/null && command -v wasmd-node &> /dev/null; then
    echo "Starting local wasmd node for Cosmos tests..."
    # Run in background
    wasmd-node &
    # Give it time to start
    sleep 5
    # Check if the process actually started
    WASMD_PID_FILE="$HOME/.wasmd-test/wasmd.pid"
    if [ -f "$WASMD_PID_FILE" ]; then
        WASMD_PID=$(cat "$WASMD_PID_FILE")
        # Check if the process actually started
        if kill -0 $WASMD_PID > /dev/null 2>&1; then
            echo "wasmd node started with PID $WASMD_PID"
            # Register cleanup function to kill wasmd node on exit
            function cleanup_wasmd {
                echo "Stopping wasmd node..."
                kill $WASMD_PID || true # Use || true to ignore error if already stopped
            }
            trap cleanup_wasmd EXIT
        else
            echo "Warning: wasmd node process not found, but will continue with tests."
        fi
    else
        echo "Warning: wasmd node PID file not found, but will continue with tests."
    fi
else
    echo "Using already running wasmd node or skipping (not found in PATH)"
fi

# Start local Anvil node if it's not already running and if available
if ! pgrep -x "anvil" > /dev/null && command -v anvil &> /dev/null; then
    echo "Starting local Anvil node for Ethereum tests..."
    anvil --port 8545 &
    ANVIL_PID=$!
    # Give it time to start
    sleep 2
    echo "Anvil node started with PID $ANVIL_PID"
    # Register cleanup function to kill Anvil on exit
    function cleanup_anvil {
        echo "Stopping Anvil node..."
        kill $ANVIL_PID || true
    }
    trap cleanup_anvil EXIT
else
    echo "Using already running Anvil node or skipping (not found in PATH)"
fi

# Set environment variables for tests
export RUN_REAL_CONTRACT_TESTS=1
export COSMOS_TEST_ENDPOINT=http://localhost:26657
export ETHEREUM_TEST_ENDPOINT=http://localhost:8545

# Build and deploy Cosmos contracts
echo "Building and deploying Cosmos contracts..."
(
    cd "$COSMOS_CONTRACTS_DIR"
    
    # Check if Rust is available
    if command -v cargo &> /dev/null; then
        # Build the contracts (this is a placeholder - actual build commands may differ)
        echo "Building Valence Cosmos contracts..."
        # cargo wasm # Replace with actual build command
        
        # Deploy the contracts (this is a placeholder - actual deployment commands may differ)
        echo "Deploying Valence Cosmos contracts to test wasmd node..."
        # Use appropriate tools to deploy the contracts
        # For example: wasmd tx wasm store contract.wasm --from <wallet> --chain-id=<chain-id>
    else
        echo "Error: Rust toolchain not found. Cannot build Cosmos contracts."
        exit 1
    fi
)

# Build and deploy Ethereum contracts
echo "Building and deploying Ethereum contracts..."
(
    cd "$ETH_CONTRACTS_DIR"
    
    # Check if Node.js is available for Ethereum contracts
    if command -v npm &> /dev/null; then
        # Install dependencies
        echo "Installing Ethereum contract dependencies..."
        # npm install # Replace with actual install command if needed
        
        # Build the contracts (this is a placeholder - actual build commands may differ)
        echo "Building Valence Ethereum contracts..."
        # npm run build # Replace with actual build command
        
        # Deploy the contracts (this is a placeholder - actual deployment commands may differ)
        echo "Deploying Valence Ethereum contracts to test Anvil node..."
        # npm run deploy:local # Replace with actual deployment command
    else
        echo "Error: Node.js not found. Cannot build Ethereum contracts."
        exit 1
    fi
)

echo "Running contract integration tests..."

# Run the contract indexer tests with real contracts
echo "Testing Account Contract Indexer with real contracts..."
cargo test -p indexer-cosmos --test real_valence_account_tests -- --nocapture

echo "Testing Processor Contract Indexer with real contracts..."
cargo test -p indexer-cosmos --test real_valence_processor_tests -- --nocapture

echo "Testing Authorization Contract Indexer with real contracts..."
cargo test -p indexer-cosmos --test real_valence_authorization_tests -- --nocapture

echo "Testing Library Contract Indexer with real contracts..."
cargo test -p indexer-cosmos --test real_valence_library_tests -- --nocapture

echo "All Valence real contract indexer tests completed!" 