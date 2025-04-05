#!/usr/bin/env bash
set -e

# Script to test the Valence contract indexers
echo "Running Valence Contract Indexer tests..."

# Make sure we're in the project root directory
cd "$(dirname "$0")/.."

# Check if we're in nix-shell
if [ -z "$IN_NIX_SHELL" ] && [ -z "$NIX_BUILD_TOP" ]; then
    echo "Error: This script should be run inside a Nix shell."
    echo "Please enter the nix development shell first using:"
    echo "  nix develop"
    exit 1
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
export RUN_CONTRACT_TESTS=1
export COSMOS_TEST_ENDPOINT=http://localhost:26657
export ETHEREUM_TEST_ENDPOINT=http://localhost:8545

echo "Running contract indexer tests..."

# Run the contract indexer tests
echo "Testing Account Contract Indexer..."
cargo test -p indexer-cosmos --test valence_account_tests -- --nocapture

echo "Testing Processor Contract Indexer..."
cargo test -p indexer-cosmos --test valence_processor_tests -- --nocapture

echo "Testing Authorization Contract Indexer..."
cargo test -p indexer-cosmos --test valence_authorization_tests -- --nocapture

echo "Testing Library Contract Indexer..."
cargo test -p indexer-cosmos --test valence_library_tests -- --nocapture

echo "All Valence contract indexer tests completed!" 