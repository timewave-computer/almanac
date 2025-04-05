#!/usr/bin/env bash
set -e

# Script to run Cosmos adapter tests against local wasmd node
echo "Running Cosmos adapter tests..."

# Make sure we're in the project root directory
cd "$(dirname "$0")/.."

# Define expected path for wasmd node - use from nix if available
if command -v run-wasmd-node &> /dev/null; then
    echo "Using run-wasmd-node from Nix environment"
    WASMD_RUN_CMD="run-wasmd-node"
else
    echo "Error: wasmd node command not found"
    echo "Please enter the nix development shell first using:"
    echo "  nix develop"
    exit 1
fi

# Start local wasmd node if it's not already running
if ! pgrep -f "wasmd start" > /dev/null; then
    echo "Starting local wasmd node..."
    # Run in background
    $WASMD_RUN_CMD &
    # Give it time to start
    sleep 5
    # Get the PID from the PID file
    if [ ! -f "/tmp/wasmd-node.pid" ]; then
        echo "Error: Failed to start wasmd node (no PID file found)."
        exit 1
    fi
    WASMD_PID=$(cat /tmp/wasmd-node.pid)
    # Check if the process actually started
    if ! kill -0 $WASMD_PID > /dev/null 2>&1; then
      echo "Error: Failed to start wasmd node."
      exit 1
    fi
    echo "wasmd node started with PID $WASMD_PID"
    # Register cleanup function to kill wasmd node on exit
    function cleanup {
        echo "Stopping wasmd node..."
        kill $WASMD_PID || true # Use || true to ignore error if already stopped
    }
    trap cleanup EXIT
else
    echo "Using already running wasmd node"
fi

# Set environment variables for tests
export RUN_COSMOS_TESTS=1
export COSMOS_TEST_ENDPOINT=http://localhost:26657

# Run the tests
echo "Running tests from directory: $(pwd)"
cargo test -p indexer-cosmos -- --nocapture

echo "All Cosmos adapter tests completed!" 