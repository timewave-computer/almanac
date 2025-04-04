#!/usr/bin/env bash
set -e

# Script to run chain reorganization tests
echo "Running chain reorganization tests..."

# Make sure we're in the project root directory
cd "$(dirname "$0")/.."

# Start local Anvil node in development mode if it's not already running
if ! pgrep -x "anvil" > /dev/null; then
    echo "Starting local Anvil node in development mode..."
    anvil --port 8545 --dev &
    ANVIL_PID=$!
    # Give it time to start
    sleep 2
    echo "Anvil node started with PID $ANVIL_PID"
    # Register cleanup function to kill Anvil on exit
    function cleanup {
        echo "Stopping Anvil node..."
        kill $ANVIL_PID
    }
    trap cleanup EXIT
else
    echo "Using already running Anvil node"
fi

# Run the chain reorganization tests
cd crates/storage && cargo run --bin test_chain_reorg 