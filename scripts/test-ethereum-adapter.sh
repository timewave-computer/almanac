#!/usr/bin/env bash
set -e

# Script to run Ethereum adapter tests against local Anvil node
echo "Running Ethereum adapter tests..."

# Make sure we're in the project root directory
cd "$(dirname "$0")/.."

# Start local Anvil node if it's not already running
if ! pgrep -x "anvil" > /dev/null; then
    echo "Starting local Anvil node..."
    anvil --port 8545 &
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

# Run the Ethereum adapter tests
cd crates/storage && cargo run --bin test_ethereum_adapter 