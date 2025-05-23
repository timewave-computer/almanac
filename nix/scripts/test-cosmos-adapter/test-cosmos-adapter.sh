#!/usr/bin/env bash
set -euo pipefail

# Script to run Cosmos adapter tests against local wasmd node
echo "Running Cosmos adapter tests..."

# Stay in the current directory where the tests are
# We don't need to change directory as it might cause issues

# Define expected path for wasmd node - use from nix if available
if command -v wasmd-node &> /dev/null; then
    echo "Using wasmd-node from Nix environment"
    WASMD_RUN_CMD="wasmd-node"
else
    echo "Error: wasmd node command not found"
    echo "Please enter the nix development shell first using:"
    echo "  nix develop"
    exit 1
fi

# Start local wasmd node if it's not already running
WASMD_PID=""
if ! pgrep -f "wasmd start" > /dev/null; then
    echo "Starting local wasmd node..."
    # Run in background
    $WASMD_RUN_CMD &
    WASMD_NODE_PID=$!
    # Give it time to start
    sleep 5
    # Check if the process actually started
    WASMD_PID_FILE="$HOME/.wasmd-test/wasmd.pid"
    if [ ! -f "$WASMD_PID_FILE" ]; then
        echo "Error: Failed to start wasmd node (no PID file found)."
        kill $WASMD_NODE_PID 2>/dev/null || true
        exit 1
    fi
    WASMD_PID=$(cat "$WASMD_PID_FILE")
    # Check if the process actually started
    if ! kill -0 $WASMD_PID > /dev/null 2>&1; then
      echo "Error: Failed to start wasmd node."
      kill $WASMD_NODE_PID 2>/dev/null || true
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

# Run the tests - let the user specify the package or run default tests
if [ $# -gt 0 ]; then
    echo "Running specified test command: cargo test $@"
    cargo test "$@"
else
    echo "Running default cosmos tests: cargo test -p indexer-cosmos -- --nocapture"
    cargo test -p indexer-cosmos -- --nocapture
fi

echo "All Cosmos adapter tests completed!" 